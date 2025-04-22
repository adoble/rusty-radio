// Code taken from the project rust-projects/edge-hhtp-embassy-esp

use embassy_net::{
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
    IpAddress, Stack,
};

use embedded_nal::Dns;
use embedded_nal::TcpClientStack;

use embassy_time::{Duration, Timer};

use embedded_io_async::Read;

use reqwless::request::{Method, RequestBody};
use reqwless::{client::HttpClient, request::RequestBuilder};

use crate::constants::NUMBER_SOCKETS_TCP_CLIENT_STATE;

use crate::task::sync::MUSIC_CHANNEL;

use super::sync::ACCESS_WEB_SIGNAL;

// It is important to use a buffer size that is big enough.
const BUFFER_SIZE: usize = 2048;

// NOTE: This station does a number of redirects by setting the response header "location". Note that it does
// not give a return code 3xx which is strange.
// Anaylsed with Google HAR analyser https://toolbox.googleapps.com/apps/har_analyzer/
// For a description of the location field see: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Location
// const STATION_URL: &str = "http://liveradio.swr.de/sw282p3/swr3/play.mp3";

// NOTE: This station doesn't seem to have redirects (as of now) so couldl you it to test the basic functionality
const STATION_URL: &str = "http://listen.181fm.com/181-classical_128k.mp3";

// This my "own" station for testing
//const STATION_URL: &str = "http://192.168.2.123/music/1";

/// This task  accesses an internet radio station and send the data to MUSIC_CHANNEL.
#[embassy_executor::task]
pub async fn stream2(stack: Stack<'static>) {
    let mut headers_buf = [0u8; 1024];
    esp_println::println!("DEBUG: Streaming task started");

    loop {
        let start_access = ACCESS_WEB_SIGNAL.wait().await;
        if start_access {
            break;
        }
    }
    esp_println::println!("DEBUG: Start streaming from provider");

    // This is important. Need to make sure the DHCP is up so
    // that the ip address can be found from the host name
    esp_println::println!("INFO: waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    esp_println::println!("INFO: DHCP is now up!");

    stack.wait_config_up().await;
    esp_println::println!("DEBUG: Stack is up!");

    //let url = Url::parse(STATION_URL).unwrap();

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
    esp_println::println!("DEBUG: Stack link is up!");

    let dns = DnsSocket::new(stack);
    let client_state = TcpClientState::<NUMBER_SOCKETS_TCP_CLIENT_STATE, 512, BUFFER_SIZE>::new();
    //TcpClientState::<NUMBER_SOCKETS_TCP_CLIENT_STATE, BUFFER_SIZE, BUFFER_SIZE>::new();
    let tcp_client = TcpClient::new(stack, &client_state);

    let mut http_client = HttpClient::new(&tcp_client, &dns);

    let request = http_client.request(Method::GET, STATION_URL);

    let mut client = HttpClient::new(&tcp_client, &dns); // Types implementing embedded-nal-async
    let mut rx_buf = [0; 4096];
    let headers: [(&str, &str); 1] = [("User-Agent", "Rusty-Radio/0.5")];
    let response = client
        .request(Method::POST, &STATION_URL)
        .await
        .unwrap()
        .content_type(reqwless::headers::ContentType::TextPlain)
        .headers(&[("User-Agent", "Rusty-Radio/0.5")])
        .send(&mut rx_buf)
        .await;

    match response {
        Ok(response) => {
            // Find headers
            for header in response.headers() {
                if header.0.len() > 0 {
                    esp_println::println!(
                        "Header: {} = {:?}",
                        header.0,
                        core::str::from_utf8(header.1).unwrap()
                    );
                }
            }

            // REDIRECTS go here

            let mut reader = response.body().reader();

            // let mut read_buffer = [0u8; 32];
            let mut read_buffer = [0u8; 10000];

            loop {
                match reader.read(&mut read_buffer).await {
                    Ok(0) => {
                        esp_println::println!("ERROR: EOF of stream");

                        break;
                    }
                    Ok(n) => {
                        for i in 0..n {
                            MUSIC_CHANNEL.send(read_buffer[i]).await;
                        }

                        // Wait until the channel is nearly full before playing
                        // if MUSIC_CHANNEL.free_capacity() < 100_000 {
                        //     START_PLAYING.signal(true);
                        // }
                        //continue;
                    }
                    Err(err) => esp_println::println!("ERROR: Cannot read from socket [{:?}]", err),
                }
                // }
            }
        }
        Err(err) => {
            esp_println::println!("ERROR: Cannot send request [{:?}]", err);
            return;
        }
    }
}
