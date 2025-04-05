use embassy_net::{
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
    Stack,
};
use embassy_time::{Duration, Timer};

use embedded_io_async::Read;

use esp_alloc::HeapStats;
use reqwless::{client::HttpClient, request};

use crate::constants::NUMBER_SOCKETS_TCP_CLIENT_STATE;

use crate::task::sync::MUSIC_CHANNEL;

use super::sync::ACCESS_WEB_SIGNAL;

const BUFFER_SIZE: usize = 2560;

/// This task  accesses an internet radio station and send the data to MUSIC_CHANNEL.
#[embassy_executor::task]
pub async fn stream(stack: Stack<'static>) {
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
}
