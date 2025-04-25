// Code taken from the project rust-projects/edge-hhtp-embassy-esp

use embassy_net::{
    tcp::{
        //client::TcpClient,
        //client::TcpClientState,
        TcpSocket,
    },
    IpAddress, Stack,
};
use embassy_time::{Duration, Instant, Timer};

use embedded_io_async::{Read, Write};

use core::net::Ipv4Addr;

use nourl::Url;

use crate::task::sync::{MUSIC_PIPE, MUSIC_PIPE_LEN, START_PLAYING};

// use crate::task::sync::MUSIC_CHANNEL;

//use super::sync::ACCESS_WEB_SIGNAL;
use crate::task::sync::ACCESS_WEB_SIGNAL;

use http_builder::{Method, Request};

//const BUFFER_SIZE: usize = 32;
// Suggestion from CoPilot to make this bigger
// This has significantly improved the performance of the radio stream
//const BUFFER_SIZE: usize = 1024;
//const BUFFER_SIZE: usize = 2048;
const BUFFER_SIZE: usize = 8192;

//const MUSIC_CHUNK_SIZE: usize = 32;
//const MUSIC_CHUNK_SIZE: usize = 2048;
const MUSIC_CHUNK_SIZE: usize = 8192;
//const MUSIC_CHUNK_SIZE: usize = 16384; // Did not work

// NOTE: This station does a number of redirects by setting the response header "location". Note that it does
// not give a return code 3xx which is strange.
// Anaylsed with Google HAR analyser https://toolbox.googleapps.com/apps/har_analyzer/
// For a description of the location field see: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Location
// const STATION_URL: &str = "http://liveradio.swr.de/sw282p3/swr3/play.mp3";

// NOTE: This station doesn't seem to have redirects (as of now) so could you it to test the basic functionality
const STATION_URL: &str = "http://listen.181fm.com/181-classical_128k.mp3";

/// This task  accesses an internet radio station and send the data to MUSIC_CHANNEL.
#[embassy_executor::task]
pub async fn stream(stack: Stack<'static>) {
    let mut rx_buffer = [0; BUFFER_SIZE];
    let mut tx_buffer = [0; BUFFER_SIZE];
    //let mut buf: [u8; 512] = [0; 512];
    //let mut buf: [u8; 32] = [0; 32];

    esp_println::println!("DEBUG: read web page task");

    loop {
        let start_access = ACCESS_WEB_SIGNAL.wait().await;
        if start_access {
            break;
        }
    }

    // This is important. Need to make sure the DHCP is up so
    // that the ip address can be found from the host name
    esp_println::println!("INFO: waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    esp_println::println!("INFO: DHCP is now up!");

    esp_println::println!("DEBUG:: waiting for stack to be up...");
    stack.wait_config_up().await;
    esp_println::println!("DEBUG: Stack is up!");
    let config = stack.config_v4().unwrap();

    esp_println::println!(
        "Network: IP: {}, DNS: {:?}, GATEWAY: {:?}",
        config.address,
        config.dns_servers,
        config.gateway,
    );

    let url = Url::parse(STATION_URL).unwrap();

    loop {
        if stack.is_link_up() {
            break;
        }
        esp_println::println!("DEBUG: Waiting for stack link.");
        Timer::after(Duration::from_millis(500)).await;
    }
    esp_println::println!("DEBUG: Stack link is up!");

    // let client_state =
    //     TcpClientState::<NUMBER_SOCKETS_TCP_CLIENT_STATE, BUFFER_SIZE, BUFFER_SIZE>::new();
    //let tcp_client = TcpClient::new(stack, &client_state);
    //let dns = DnsSocket::new(stack);

    let host = url.host();
    let port = url.port_or_default();
    let path = url.path();
    esp_println::println!("DEBUG: Host = {}, Path = {}, Port = {}", host, path, port);

    let remote_ip_addresses = stack
        .dns_query(host, embassy_net::dns::DnsQueryType::A)
        .await
        .unwrap();
    let remote_ip_addr = remote_ip_addresses[0]; //TODO Error case!

    let remote_endpoint = match remote_ip_addr {
        IpAddress::Ipv4(ipv4_addr) => {
            let octets = ipv4_addr.octets();
            (Ipv4Addr::from(octets), port)
        }
    };

    esp_println::println!("DEBUG: IPS = {:?} , Port = {} ", remote_ip_addr, port);

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

    // Timeout longer than keep alive (see https://docs.embassy.dev/embassy-net/git/default/tcp/struct.TcpSocket.html#method.set_keep_alive)
    socket.set_timeout(Some(embassy_time::Duration::from_secs(15)));

    // Optimisations
    socket.set_keep_alive(Some(embassy_time::Duration::from_secs(10)));
    //socket.set_nodelay(true); // Disable Nagle's algorithm

    socket.connect(remote_endpoint).await.unwrap();

    // Now read the page
    let mut request = Request::new(Method::GET, path).unwrap();
    request.host(host).unwrap();
    request.header("User-Agent", "RustyRadio/0.1.0").unwrap();

    esp_println::println!("DEBUG: HTTP Request:\n{}", request.to_string());

    socket
        .write_all(request.to_string().as_bytes())
        .await
        .expect("ERROR: Could not write request");
    socket
        .flush()
        .await
        .expect("ERROR: Could not flush request");

    esp_println::println!("DEBUG: Starting to read");

    // let mut body_read_buffer = [0u8; 32]; // Small buffer that matches to other buffers
    let mut body_read_buffer = [0u8; MUSIC_CHUNK_SIZE]; // Small buffer that matches to music channel message size

    // Skip HTTP headers
    let mut header_buffer = [0u8; 2048];
    let mut header_pos = 0;
    let mut found_end = false;

    // Read until we find the end of headers (\r\n\r\n)
    while header_pos < header_buffer.len() && !found_end {
        match socket
            .read(&mut header_buffer[header_pos..header_pos + 1])
            .await
        {
            Ok(0) => {
                esp_println::println!("Connection closed while reading headers");
                break;
            }
            Ok(n) => {
                //esp_println::println!("DEBUG:: Read {} bytes", n);
                header_pos += n;

                // Check for end of headers
                if header_pos >= 4
                    && header_buffer[header_pos - 4] == b'\r'
                    && header_buffer[header_pos - 3] == b'\n'
                    && header_buffer[header_pos - 2] == b'\r'
                    && header_buffer[header_pos - 1] == b'\n'
                {
                    found_end = true;
                }
            }
            Err(e) => {
                esp_println::println!("Error reading headers: {:?}", e);
                break;
            }
        }
    }

    if !found_end {
        esp_println::println!("ERROR: Failed to find end of headers");
        Timer::after_secs(5).await;
        return;
    } else {
        esp_println::println!("DEBUG: Found end of headers at position {}", header_pos);
    }

    let start_time = Instant::now();

    esp_println::println!("DEBUG: Start filling channel ...");

    // Fill up the pipe to 75% of its capacity before starting to play
    let initial_fill_len = 3 * MUSIC_PIPE.capacity() / 4;

    'initial_fill: loop {
        match socket.read(&mut body_read_buffer).await {
            Ok(0) => {
                esp_println::println!("ERROR: Connection closed");
                return;
            }
            Ok(n) => {
                let write_start = Instant::now();
                MUSIC_PIPE.write(&body_read_buffer[..n]).await;

                if MUSIC_PIPE.len() >= initial_fill_len {
                    START_PLAYING.signal(true);
                    break 'initial_fill;
                }

                let read_time = write_start.elapsed().as_micros();
                if read_time > 1000 {
                    esp_println::println!("Slow write: {}us", read_time);
                }
            }
            Err(err) => {
                esp_println::println!("ERROR: Cannot read from socket [{:?}]", err);
                Timer::after(Duration::from_millis(100)).await;
            }
        }
    }

    // Continue streaming with performance monitoring
    let mut total_bytes = 0u32;
    let mut last_stats = Instant::now();

    loop {
        match socket.read(&mut body_read_buffer).await {
            Ok(0) => {
                esp_println::println!("Connection closed");
                break;
            }
            Ok(n) => {
                let read_start = Instant::now();
                total_bytes += n as u32;

                // Write immediately without trying to read more
                let write_start = Instant::now();
                MUSIC_PIPE.write(&body_read_buffer[..n]).await;

                // Add network statistics
                if last_stats.elapsed().as_millis() >= 1000 {
                    let pipe_usage =
                        (MUSIC_PIPE.len() as f32 / MUSIC_PIPE.capacity() as f32) * 100.0;
                    let read_time = read_start.elapsed().as_micros();
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
