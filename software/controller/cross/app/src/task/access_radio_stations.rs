use embassy_net::{
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
    Stack,
};
use embassy_time::{Duration, Timer};

use embedded_io_async::Read;

use reqwless::{client::HttpClient, request};

use crate::constants::NUMBER_SOCKETS_TCP_CLIENT_STATE;

use crate::task::sync::{ACCESS_WEB_SIGNAL, TEST_CHANNEL};

const BUFFER_SIZE: usize = 2560;

/// This task only accesses the web when  ACCESS_WEB_SIGNAL is signalled.
#[embassy_executor::task]
pub async fn access_radio_stations(stack: Stack<'static>) {
    let mut rx_buffer = [0; BUFFER_SIZE];

    loop {
        ACCESS_WEB_SIGNAL.wait().await;

        esp_println::println!("Access web task");

        loop {
            if stack.is_link_up() {
                break;
            }
            Timer::after(Duration::from_millis(500)).await;
        }

        let client_state =
            TcpClientState::<NUMBER_SOCKETS_TCP_CLIENT_STATE, BUFFER_SIZE, BUFFER_SIZE>::new();
        let tcp_client = TcpClient::new(stack, &client_state);
        let dns = DnsSocket::new(stack);
        let mut http_client = HttpClient::new(&tcp_client, &dns);

        esp_println::println!("Setting up request");

        let mut request = http_client
            .request(
                request::Method::GET,
                "http://andrew-doble.hier-im-netz.de/ir/stations.txt",
            )
            .await
            .unwrap();

        esp_println::println!("Sending request, reading response");
        let response = request.send(&mut rx_buffer).await.unwrap();

        esp_println::println!("Getting body");

        esp_println::println!("Http body:");

        // This approach can be used to read a stream
        let mut reader = response.body().reader();
        let mut small_buffer: [u8; 32] = [0; 32];

        loop {
            let res = reader.read(&mut small_buffer).await;
            match res {
                Ok(0) => {
                    esp_println::println!("EOF");
                    break;
                }
                Ok(size) if size > 0 => {
                    //let content = from_utf8(&small_buffer).unwrap();
                    //esp_println::print!("{content}");
                    TEST_CHANNEL.send(small_buffer).await;
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

        // This approach is better to read a page (for instance, the stations list)
        // let body = from_utf8(response.body().read_to_end().await.unwrap()).unwrap();
        // esp_println::println!("Http body:");
        // esp_println::println!("{body}");

        ACCESS_WEB_SIGNAL.reset();

        // TODO Is this delay required?
        //Timer::after(Duration::from_millis(3000)).await;
    }
}
