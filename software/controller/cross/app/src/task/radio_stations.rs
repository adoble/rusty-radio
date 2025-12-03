use embassy_executor::Spawner;
use embassy_net::{
    dns::DnsSocket,
    tcp::{
        self,
        client::{TcpClient, TcpClientState},
        TcpSocket,
    },
    Stack,
    // IpAddress
};
//use embedded_io_async::Write;

// use core::net::Ipv4Addr;
use embassy_time::{Duration, Timer};
// use embassy_net::tcp::client::{TcpClient, TcpClientState};
// use embassy_net::Stack;
use core::sync::atomic::{AtomicBool, Ordering};
// use nourl::Url;
use reqwless::{client::HttpClient, request::Method};
use static_cell::StaticCell;

//
//use http::{Method, Request, RequestError, Response, ResponseStatusCode, MAX_URL_LEN};
use stations::{Station, StationError, Stations};

use crate::{front_panel::FrontPanel, task::tuner::tuner};

pub const MAX_STATION_NAME_LEN: usize = 40;
pub const MAX_STATION_URL_LEN: usize = 256;
pub const NUMBER_PRESETS: usize = 4;

pub type RadioStation = Station<MAX_STATION_NAME_LEN, MAX_STATION_URL_LEN>;
pub type RadioStations = Stations<MAX_STATION_NAME_LEN, MAX_STATION_URL_LEN, NUMBER_PRESETS>;

static RADIO_STATIONS: StaticCell<RadioStations> = StaticCell::new();
static RADIO_STATIONS_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Read the internet stations from the web.
// Development note: This version uses reqwless. A  previous version used TCP sockets directly.
// This can be found at https://gist.github.com/adoble/6ae04bff12d76949743be39f9222f06d
#[embassy_executor::task]
pub async fn radio_stations(
    spawner: Spawner,
    stack: Stack<'static>,
    front_panel: &'static FrontPanel,
    stations_url: &'static str,
) {
    let mut rx_buffer = [0; 16000];
    let client_state = TcpClientState::<1, 1024, 1024>::new();
    let tcp_client = TcpClient::new(stack, &client_state);
    let dns_client = DnsSocket::new(stack);

    let mut http_client = HttpClient::new(&tcp_client, &dns_client);

    // Get initial stations and spawn tuner
    loop {
        // Only load the stations if they are not initialised
        if !RADIO_STATIONS_INITIALIZED.load(Ordering::Acquire) {
            if let Ok(mut request) = http_client.request(Method::GET, stations_url).await {
                if let Ok(response) = request.send(&mut rx_buffer).await {
                    if let Ok(body) = response.body().read_to_end().await {
                        if let Ok(stations) = Stations::<
                            MAX_STATION_NAME_LEN,
                            MAX_STATION_URL_LEN,
                            NUMBER_PRESETS,
                        >::load(body)
                        {
                            let stations = RADIO_STATIONS.init(stations);
                            RADIO_STATIONS_INITIALIZED.store(true, Ordering::Release);

                            spawner.must_spawn(tuner(stations, front_panel));
                        }
                    }
                }
            }
        }

        Timer::after(Duration::from_secs(1)).await;
    }
}

// TODO This is a duplicate from stream. Maybe put it  http::response??
/// Read the headers into the header buffer
#[deprecated]
async fn read_headers(
    socket: &mut TcpSocket<'_>,
    header_buffer: &mut [u8],
    //) -> Result<Option<String<MAX_URL_LEN>>, embassy_net::tcp::Error> {
) -> Result<(), RadioStationsError> {
    let mut header_pos = 0;
    let mut found_end = false;

    while header_pos < header_buffer.len() && !found_end {
        match socket
            .read(&mut header_buffer[header_pos..header_pos + 1])
            .await
            .map_err(RadioStationsError::Tcp)?
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
        Err(RadioStationsError::HeadersEndNotFound)
    } else {
        Ok(())
    }
}

#[derive(Debug)]
pub enum RadioStationsError {
    StationConstruction(StationError),
    Dns,
    Tcp(tcp::Error),
    IpConnection,
    MalformedUrl,
    HttpRequest,
    HeadersEndNotFound,
}
// impl From<nourl::Error> for RadioStationsError {
//     fn from(_error: nourl::Error) -> Self {
//         Self::MalformedUrl
//     }
// }
