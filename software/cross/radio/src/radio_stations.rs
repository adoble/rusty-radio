use crate::task::sync::StationChangeSender;

use embassy_net::{
    dns::DnsSocket,
    tcp::{
        self,
        client::{TcpClient, TcpClientState},
    },
    Stack,
    // IpAddress
};

#[allow(unused_imports)]
use log::*;

// use core::sync::atomic::{AtomicBool, Ordering};
use reqwless::{client::HttpClient, request::Method};

use stations::{Station, StationError, Stations};

pub const MAX_STATION_NAME_LEN: usize = 40;
pub const MAX_STATION_URL_LEN: usize = 256;
pub const NUMBER_PRESETS: usize = 4;

pub type RadioStation = Station<MAX_STATION_NAME_LEN, MAX_STATION_URL_LEN>;
pub type RadioStations = Stations<MAX_STATION_NAME_LEN, MAX_STATION_URL_LEN, NUMBER_PRESETS>;

/// Read the internet stations from the web.
// Development note: This version uses reqwless. A  previous version used TCP sockets directly.
// This can be found at https://gist.github.com/adoble/6ae04bff12d76949743be39f9222f06d
pub async fn radio_stations(
    stack: Stack<'static>,
    stations_url: &'static str,
    station_change_sender: StationChangeSender,
) -> Result<RadioStations, RadioStationsError> {
    let mut rx_buffer = [0; 16000];
    let client_state = TcpClientState::<1, 1024, 1024>::new();
    let tcp_client = TcpClient::new(stack, &client_state);
    let dns_client = DnsSocket::new(stack);

    let mut http_client = HttpClient::new(&tcp_client, &dns_client);

    let mut request = http_client.request(Method::GET, stations_url).await?;

    let response = request.send(&mut rx_buffer).await?;

    let body = response.body().read_to_end().await?;

    let stations = RadioStations::load(body)?;

    // Signal the initial station
    //let station_change_sender = STATION_CHANGE_WATCH.sender();

    // 1. The last set station - TODO
    // 2. The first preset stations if set
    // 3. The first station in the station list
    let initial_station = stations
        .preset(0)
        .map(|s| s.1) // Get the preset station from the tuple
        .or_else(|| stations.get_station(0)); //.expect("No initial station found");

    // Send the inital station
    station_change_sender.send(initial_station);

    Ok(stations)
}

#[derive(Debug)]
pub enum RadioStationsError {
    Http(reqwless::Error),

    StationConstruction(StationError),
    // Old ???? TODO remove??
    Dns,
    Tcp(tcp::Error),
    IpConnection,
    MalformedUrl,
    HttpRequest,
    HeadersEndNotFound,
}

impl From<reqwless::Error> for RadioStationsError {
    fn from(err: reqwless::Error) -> Self {
        Self::Http(err)
    }
}

impl From<StationError> for RadioStationsError {
    fn from(err: StationError) -> Self {
        Self::StationConstruction(err)
    }
}
