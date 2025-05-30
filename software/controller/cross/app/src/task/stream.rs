// Code taken from the project rust-projects/edge-hhtp-embassy-esp

use embassy_net::{
    tcp::{State, TcpSocket},
    IpAddress, Stack,
};
use embassy_time::{Duration, Instant, Timer};

use embedded_io_async::Write;
use stations::Station;

use core::net::Ipv4Addr;

use nourl::Url;

use crate::task::sync::{AUDIO_BUFFER_SIZE, MUSIC_PIPE, START_PLAYING};

use http::{Method, Request, Response, ResponseStatusCode};

// Empirically determined value. This value  has to be used in
// conjunction with the wifi tuning parameters in .cargo/config.toml
const BUFFER_SIZE: usize = 6000; // THIS WORKS with good enough performance

// Max size for a url
//const MAX_URL_LEN: usize = 256;
// TODO Actual URLS after redirects come close to this limit
// TODO This needs to be the same size as a the PATH_LEN in crate::http_builder::Request

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamingState {
    FillingPipe,
    Playing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StreamError {
    Tcp(embassy_net::tcp::Error),
    HeadersEndNotFound,
}

/// This task accesses an internet radio station and sends the data to MUSIC_CHANNEL.
#[embassy_executor::task]
pub async fn stream(stack: Stack<'static>, initial_station: &'static Station) {
    let mut rx_buffer = [0; BUFFER_SIZE];
    let mut tx_buffer = [0; BUFFER_SIZE];

    // This is important. Need to make sure the DHCP is up so
    // that the ip address can be found from the host name
    //esp_println::println!("INFO: waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    //esp_println::println!("INFO: DHCP is now up!");

    stack.wait_config_up().await;
    //esp_println::println!("INFO: Stack is up!");
    let config = stack.config_v4().unwrap();

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
    //esp_println::println!("INFO: Stack link is now up!");
    esp_println::println!("\nPLAYING: {}\n", initial_station.name());

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

    // Optimisations
    // Timeout longer than keep alive
    // (see https://docs.embassy.dev/embassy-net/git/default/tcp/struct.TcpSocket.html#method.set_keep_alive)
    socket.set_timeout(Some(embassy_time::Duration::from_secs(15)));
    socket.set_keep_alive(Some(embassy_time::Duration::from_secs(10)));

    let mut body_buffer = [0u8; AUDIO_BUFFER_SIZE];

    //let mut url_str: String<MAX_URL_LEN> = String::new();
    let initial_url = initial_station.url();
    let mut url_str = initial_url.clone();
    //url_str.push_str(initial_url).expect("ERROR: url to big");

    'redirect: loop {
        // while let StationUrl::Redirect(url) = station_url {
        let url = Url::parse(&url_str).unwrap();

        let host = url.host();
        let port = url.port_or_default();
        let path = url.path();
        //esp_println::println!("INFO: Host = {}, Path = {}, Port = {}", host, path, port);

        let remote_ip_addresses = stack
            .dns_query(host, embassy_net::dns::DnsQueryType::A)
            .await
            .unwrap();

        //esp_println::println!("INFO: DNS Query OK");

        let remote_ip_addr = remote_ip_addresses[0]; //TODO Error case!

        let remote_endpoint = match remote_ip_addr {
            IpAddress::Ipv4(ipv4_addr) => {
                let octets = ipv4_addr.octets();
                (Ipv4Addr::from(octets), port)
            }
        };

        // Connect to the socket using the IP address from the DNS
        socket.connect(remote_endpoint).await.unwrap();

        // Request the data
        let mut request = Request::new(Method::GET, path).unwrap();
        request.host(host).unwrap();

        // Set the user agent. Note this does not have to be a spoof of
        // a "normal" browser agent such as
        // "Mozilla/5.0 (X11; Linux x86_64; rv:138.0) Gecko/20100101 Firefox/138.0"
        request.header("User-Agent", "RustyRadio/0.1.0").unwrap();

        request.header("Connection", "keep-alive").unwrap();

        //esp_println::println!("DEBUG: HTTP Request:\n{}", request.to_string());

        socket
            .write_all(request.to_string().as_bytes())
            .await
            .expect("ERROR: Could not write request");
        socket
            .flush()
            .await
            .expect("ERROR: Could not flush request");

        let mut header_buffer = [0u8; 2048];
        if read_headers(&mut socket, &mut header_buffer).await.is_err() {
            panic!("Cannot read headers!");
        };
        let Ok(response) = Response::new(&header_buffer) else {
            panic!("Cannot process HTTP response!");
        };

        match response.status_code() {
            ResponseStatusCode::Successful(_) => break 'redirect, // Start streaming the audiocontent
            ResponseStatusCode::Redirection(_) => {
                url_str = response
                    .location
                    .expect("ERROR: Redirect, but no redirection location specifed!");
                // socket.close();
                socket.abort();
                socket.flush().await.unwrap();
                esp_println::println!("DEBUG: Redirecting: {url_str}");
                continue 'redirect;
            }
            other => panic!("Received invalid HTTP response code {:?}", other),
        }
    }

    // Now stream the body
    stream_body(&mut socket, &mut body_buffer).await;
}

/// Read the headers into the header buffer
async fn read_headers(
    socket: &mut TcpSocket<'_>,
    header_buffer: &mut [u8],
    //) -> Result<Option<String<MAX_URL_LEN>>, embassy_net::tcp::Error> {
) -> Result<(), StreamError> {
    let mut header_pos = 0;
    let mut found_end = false;

    while header_pos < header_buffer.len() && !found_end {
        match socket
            .read(&mut header_buffer[header_pos..header_pos + 1])
            .await
            .map_err(|e| StreamError::Tcp(e))?
        {
            0 => break,
            n => {
                header_pos += n;
                if header_pos >= 4
                    && header_buffer[header_pos - 4..header_pos] == [b'\r', b'\n', b'\r', b'\n']
                {
                    found_end = true;
                }
            }
        }
    }

    if !found_end {
        Err(StreamError::HeadersEndNotFound)
    } else {
        Ok(())
    }
}

// Hndle streaming of body, i.e. the mp3 data.
async fn stream_body(socket: &mut TcpSocket<'_>, audio_buffer: &mut [u8]) {
    let mut total_bytes = 0u32;
    let mut last_stats = Instant::now();
    let mut read_state = StreamingState::FillingPipe;
    let initial_fill_len = 3 * MUSIC_PIPE.capacity() / 4;

    loop {
        let read_start = Instant::now();
        match socket.read(audio_buffer).await {
            Ok(0) => {
                //esp_println::println!("Connection closed");
                break;
            }
            Ok(n) => {
                let read_time = read_start.elapsed().as_micros();
                total_bytes += n as u32;

                // Write immediately without trying to read more
                let write_start = Instant::now();
                MUSIC_PIPE.write_all(&audio_buffer[..n]).await;

                if read_state == StreamingState::FillingPipe && MUSIC_PIPE.len() >= initial_fill_len
                {
                    // If the pipe is more than 75% full, start playing (and emptying the pipe)
                    START_PLAYING.signal(true);
                    read_state = StreamingState::Playing;
                };

                // Add network statistics
                if last_stats.elapsed().as_millis() >= 1000 {
                    let pipe_usage =
                        (MUSIC_PIPE.len() as f32 / MUSIC_PIPE.capacity() as f32) * 100.0;
                    esp_println::println!(
                        "Stats: {:.2} KB/s, Pipe: {:.1}%, Read: {} bytes in {}us, Write: {}us",
                        (total_bytes as f32) / 1024.0,
                        pipe_usage,
                        n,
                        read_time,
                        write_start.elapsed().as_micros()
                    );
                    total_bytes = 0;
                    last_stats = Instant::now();
                }
            }
            Err(err) => {
                esp_println::println!("ERROR: Cannot read from socket [{:?}]", err);
                Timer::after(Duration::from_millis(10)).await;
            }
        }
    }
}

// // Helper function to find a particular header. It is case insenstive
// #[deprecated]
// fn find_header(buf: &[u8], header: &[u8]) -> Option<usize> {
//     buf.windows(header.len())
//         .position(|window| {
//             window.len() == header.len()
//                 && window
//                     .iter()
//                     .zip(header.iter())
//                     .all(|(a, b)| a.to_ascii_lowercase() == b.to_ascii_lowercase())
//         })
//         .map(|p| p + header.len())
// }

// // Helper function to find a new line.
// #[deprecated]
// fn find_newline(buf: &[u8]) -> Option<usize> {
//     buf.iter().position(|&b| b == b'\r' || b == b'\n')
// }
