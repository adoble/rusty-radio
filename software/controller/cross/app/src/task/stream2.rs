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

use reqwless::client::HttpClient;
use reqwless::request::Method;

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

/// This task  accesses an internet radio station and send the data to MUSIC_CHANNEL.
#[embassy_executor::task]
pub async fn stream2(stack: Stack<'static>) {
    let mut headers_buf = [0u8; 1024];
    esp_println::println!("DEBUG: Start streaming");

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

    //let url = Url::parse(STATION_URL).unwrap();

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
    let dns = DnsSocket::new(stack);
    let mut http_client = HttpClient::new(&tcp_client, &dns);

    let mut req = http_client
        .request(Method::GET, STATION_URL)
        .await
        .expect("ERROR: Unable to build HTTP request");

    let resp = req
        .send(&mut headers_buf)
        .await
        .expect("ERROR: Unable to send HTTP request");

    for header in resp.headers() {
        esp_println::println!(
            "Header: {} = {:?}",
            header.0,
            core::str::from_utf8(header.1).unwrap()
        );
    }

    // loop {
    //     match socket.read(&mut read_buffer).await {
    //         Ok(0) => {
    //             esp_println::println!("ERROR: EOF of stream");
    //             esp_println::println!("DEBUG: Number of bytes read: {n_bytes}");

    //             break;
    //         }
    //         Ok(n) => {
    //             n_bytes += n;
    //             // let s = core::str::from_utf8(&(read_buffer[0..n])).expect("Cannot convert string");
    //             // string_buffer.clear();
    //             // string_buffer.push_str(s).expect("ERROR: String too long");
    //             // esp_println::print!("{}", string_buffer);
    //             for i in 0..n {
    //                 MUSIC_CHANNEL.send(read_buffer[i]).await;
    //             }
    //             continue;
    //         }
    //         Err(err) => esp_println::println!("ERROR: Cannot read from socket [{:?}]", err),
    //     }
    // }
}
