// Code taken from the project rust-projects/edge-hhtp-embassy-esp

use embassy_net::{
    dns::DnsSocket,
    tcp::{
        client::{TcpClient, TcpClientState},
        TcpSocket,
    },
    IpAddress, Stack,
};
use embassy_time::{Duration, Timer};

use embedded_io_async::Write;

use core::net::Ipv4Addr;

use heapless::String;
use nourl::Url;

use esp_alloc::HeapStats;

use crate::constants::NUMBER_SOCKETS_TCP_CLIENT_STATE;

use crate::task::sync::MUSIC_CHANNEL;

use super::sync::ACCESS_WEB_SIGNAL;

const BUFFER_SIZE: usize = 32;

// NOTE: This station does a number of redirects by setting the response header "location". Note that it does
// not give a return code 3xx which is strange.
// Anaylsed with Google HAR analyser https://toolbox.googleapps.com/apps/har_analyzer/
// For a description of the location field see: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Location
// const STATION_URL: &str = "http://liveradio.swr.de/sw282p3/swr3/play.mp3";


// NOTE: This station doesn't seem to have redirects (as of now) so couldl you it to test the basic functionality
const STATION_URL: &str = "http://listen.181fm.com/181-classical_128k.mp3";

https://liveradio.swr.de/sw282p3/swr3/play.mp3

/// This task  accesses an internet radio station and send the data to MUSIC_CHANNEL.
#[embassy_executor::task]
pub async fn stream(stack: Stack<'static>) {
    /*
    let mut rx_buffer = [0; BUFFER_SIZE];
    //let mut buf: [u8; 512] = [0; 512];
    let mut buf: [u8; 32] = [0; 32];

    esp_println::println!("Stream task");

    let stats: HeapStats = esp_alloc::HEAP.stats();
    esp_println::println!("{}", stats);

    loop {
        //ACCESS_WEB_SIGNAL.wait().await;
        let start_access = ACCESS_WEB_SIGNAL.wait().await;
        if start_access {
            break;
        }
    }
    esp_println::println!("Web access enabled!");

    // let station_url = "https://liveradio.swr.de/sw282p3/swr3/play.mp3";
    let station_url = "http://liveradio.swr.de/sw282p3/swr3/play.mp3";

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
    esp_println::println!("Stack link is up!");

    let client_state =
        TcpClientState::<NUMBER_SOCKETS_TCP_CLIENT_STATE, BUFFER_SIZE, BUFFER_SIZE>::new();
    let tcp_client = TcpClient::new(stack, &client_state);
    let dns = DnsSocket::new(stack);
    let mut http_client = HttpClient::new(&tcp_client, &dns);

    // esp_println::println!("Setting up request");

    let mut request = http_client
        .request(request::Method::GET, &station_url)
        .await
        .unwrap();

    // let http_connection = request.conn;
    // match http_connection {
    //     reqwless::client::HttpConnection::Plain(connection) => connection.read(buf)
    //     reqwless::client::HttpConnection::PlainBuffered(buffered_write) => todo!(),
    //     reqwless::client::HttpConnection::Tls(tls_connection) => todo!(),
    // }

    esp_println::println!("Sending request, reading response");

    // With reqwless akways have to be reading into buffer that have to be adequatly sized.
    // With a streamed input this is not possible.
    // Maybe need a new way of reading in http response and reqwless is not the right way.
    let response = request.send(&mut rx_buffer).await.unwrap();

    esp_println::println!("Getting body");

    let mut reader = response.body().reader();

    loop {
        let res = reader.read(&mut buf).await; //GOES WRONG HERE
        match res {
            Ok(0) => {
                esp_println::println!("ERROR: EOF of stream");
                break;
            }
            Ok(size) if size > 0 => {
                for i in 0..size {
                    MUSIC_CHANNEL.send(buf[i]).await;
                }
                continue;
            }
            Err(err) => {
                esp_println::println!("Error in reading {:?}", err);
                break;
            }
            _ => {
                esp_println::println!("Unknown read condition");
                break;
            }
        }
    }
    */

    let mut rx_buffer = [0; BUFFER_SIZE];
    let mut tx_buffer = [0; BUFFER_SIZE];
    //let mut buf: [u8; 512] = [0; 512];
    //let mut buf: [u8; 32] = [0; 32];

    esp_println::println!("DEBUG: read web page task");

    // Only used for debugging
    // let stats: HeapStats = esp_alloc::HEAP.stats();
    // esp_println::println!("{}", stats);

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

    let url = Url::parse(STATION_URL).unwrap();

    loop {
        if stack.is_link_up() {
            break;
        }
        esp_println::println!("DEBUG: Waiting for stack link.");
        Timer::after(Duration::from_millis(500)).await;
    }
    esp_println::println!("DEBUG: Stack link is up!");

    let client_state =
        TcpClientState::<NUMBER_SOCKETS_TCP_CLIENT_STATE, BUFFER_SIZE, BUFFER_SIZE>::new();
    let tcp_client = TcpClient::new(stack, &client_state);
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
    //let remote_ip_addr = dns.get_host_by_name(host, AddrType::Either).await.unwrap();

    esp_println::println!("DEBUG: IPS = {:?} , Port = {} ", remote_ip_addr, port);

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

    // let ip_address = IpAddress::from(remote_ip_addr);

    // let endpoint = IpEndpoint::from(remote_ip_addr);
    // let endpoint = IpEndpoint::from(remote_ip_addr);
    //let remote_endpoint = (Ipv4Addr::new(142, 250, 185, 115), 80);
    socket.connect(remote_endpoint).await.unwrap();

    // Now read the page
    let mut request: String<128> = String::new();
    request.push_str("GET ").expect("ERROR: HTTP request build");
    request.push_str(path).expect("ERROR: Path too long");
    request
        .push_str(" / HTTP/1.0\r\nHost: ")
        .expect("ERROR:HTTP preamble build error");
    request
        .push_str(host)
        .expect("ERROR:Cannot add host name to http request");
    request
        .push_str("\r\n\r\n")
        .expect("ERROR: Cannot add HTTP request postamble");

    esp_println::println!("DEBUG: HTTP Request:\n{}", request);

    socket
        .write_all(request.as_bytes())
        .await
        .expect("ERROR: Could not write request");
    socket
        .flush()
        .await
        .expect("ERROR: Could not flush request");

    let mut read_buffer = [0u8; 32]; // Samll buffer tat matches to other buffers
                                     //let mut string_buffer: String<32> = String::new();
    let mut n_bytes = 0;

    loop {
        match socket.read(&mut read_buffer).await {
            Ok(0) => {
                esp_println::println!("ERROR: EOF of stream");
                esp_println::println!("DEBUG: Number of bytes read: {n_bytes}");

                break;
            }
            Ok(n) => {
                n_bytes += n;
                // let s = core::str::from_utf8(&(read_buffer[0..n])).expect("Cannot convert string");
                // string_buffer.clear();
                // string_buffer.push_str(s).expect("ERROR: String too long");
                // esp_println::print!("{}", string_buffer);
                for i in 0..n {
                    MUSIC_CHANNEL.send(read_buffer[i]).await;
                }
                continue;
            }
            Err(err) => esp_println::println!("ERROR: Cannot read from socket [{:?}]", err),
        }
    }
}
