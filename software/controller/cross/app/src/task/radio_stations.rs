use embassy_executor::Spawner;
use embassy_net::{
    dns::DnsSocket,
    tcp::{
        self,
        client::{TcpClient, TcpClientState},
        TcpSocket,
    },
    IpAddress, Stack,
};
use embedded_io_async::Write;

use core::net::Ipv4Addr;
use embassy_time::{Duration, Instant, Timer};
// use embassy_net::tcp::client::{TcpClient, TcpClientState};
// use embassy_net::Stack;
use core::sync::atomic::{AtomicBool, Ordering};
use nourl::Url;
use reqwless::{
    client::HttpClient,
    request::{Method, Request},
    response::Response,
};
use static_cell::StaticCell;

use esp_println::{dbg, println};

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

//static RADIO_STATIONS: StaticCell<Option<RadioStations>> = StaticCell::new();

// pub async fn read_stations(
//     _stack: Stack<'static>,
//     _stations_url: &str,
// ) -> Result<&'static mut RadioStations, RadioStationError> {
//     let stations_data = include_bytes!("../../../resources/rr-stations.txt");

//     let stations =
//         RadioStations::load(stations_data).map_err(RadioStationError::StationConstruction)?;

//     Ok(RADIO_STATIONS.init(stations))
// }

#[embassy_executor::task]
pub async fn radio_stations_reqwless(
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

#[deprecated]
#[embassy_executor::task]
pub async fn radio_stations(
    spawner: Spawner,
    stack: Stack<'static>,
    front_panel: &'static FrontPanel,
    stations_url: &'static str,
) {
    use http::{Method, Request, Response, ResponseStatusCode, MAX_URL_LEN};
    println!("DEBUG: radio_stations task entered");

    let mut rx_buffer = [0; 8196];
    let mut tx_buffer = [0; 1024];
    println!("DEBUG: buffers set up");

    let client_state = TcpClientState::<1, 1024, 1024>::new();
    let tcp_client = TcpClient::new(stack, &client_state);
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    let dns_client = DnsSocket::new(stack);

    println!("DEBUG: tcp client set up");

    // Get initial stations and spawn tuner
    loop {
        println!("DEBUG: loop entered");
        // Only load the stations if they are not initialised
        if !RADIO_STATIONS_INITIALIZED.load(Ordering::Acquire) {
            println!("DEBUG: stations unint");
            let url = Url::parse(stations_url).expect("Malformed URL");

            let remote_ip_addresses = stack
                .dns_query(url.host(), embassy_net::dns::DnsQueryType::A)
                .await
                .expect("DNS lookup failure");

            let remote_ip_address = remote_ip_addresses
                .first()
                .ok_or(RadioStationsError::Dns)
                .expect("ERROR: IP address not found");

            let remote_endpoint = match remote_ip_address {
                IpAddress::Ipv4(ipv4_addr) => {
                    let octets = ipv4_addr.octets();
                    (Ipv4Addr::from(octets), url.port_or_default())
                }
            };

            println!("DEBUG: remote endpoint: {:?}", remote_endpoint);

            socket
                .connect(remote_endpoint)
                .await
                .expect("ERROR: Could not connect socket");

            // Request the data
            let mut request =
                Request::new(Method::GET, url.path()).expect("ERROR: Could not create request");
            request
                .host(url.host())
                .expect("ERROR: Could not set request host");

            // Set the user agent.Note that this is based on the data in cross/app/Cargo.toml
            let user_agent = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
            request
                .header("User-Agent", user_agent)
                .expect("ERROR: Could not set user agant");

            dbg!(request.to_string());

            socket
                .write_all(request.to_string().as_bytes())
                .await
                .expect("ERROR: Could not wrtie data to socket");
            socket.flush().await.expect("ERROR: Could not flush socket");

            let mut header_buffer = [0u8; 2048]; // TODO header buffer size

            read_headers(&mut socket, &mut header_buffer)
                .await
                .expect("ERROR: Could not read headers");

            let response = Response::new(&header_buffer)
                .expect("ERROR: Response could not be created from headers");

            match response.status_code() {
                ResponseStatusCode::Successful(_) => (),
                _ => panic!("ERROR: Could not read stations list"),
            }

            println!("DEBUG: status code: {:?}", response.status_code());

            // read in the body
            let mut body = [0; 8196]; // TODO constants
            let mut pos = 0;
            let start = Instant::now();
            const READ_TIMEOUT: Duration = Duration::from_secs(5);

            // socket.read(&mut body).await.unwrap();
            // println!("DEBUG: read first byte");
            loop {
                println!("DEBUG: pos = {pos}");
                match socket.read(&mut body[pos..]).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        pos += n;
                        println!("n = {n}");
                        if start.elapsed() > READ_TIMEOUT {
                            println!("DEBUG: Read timeout after {pos} bytes");
                            break;
                        }
                    }
                    Err(e) => panic!("ERROR: Cannot read from socket: {:?}", e),
                }
                if pos > body.len() {
                    println!("DEBUG: exceded body buffer len");
                    break;
                }
            }

            if pos >= body.len() {
                println!("DEBUG: Buffer full at {pos} bytes");
                break;
            }

            // if let Ok(stations) =
            //     Stations::<MAX_STATION_NAME_LEN, MAX_STATION_URL_LEN, NUMBER_PRESETS>::load(&body)
            // {
            //     let stations = RADIO_STATIONS.init(stations);
            RADIO_STATIONS_INITIALIZED.store(true, Ordering::Release);
            socket.abort();
            //         println!("DEBUG: Stations initialised");
            //         spawner.must_spawn(tuner(stations, front_panel));
            //     }
        }

        Timer::after(Duration::from_secs(1)).await;
    } // end loop
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
