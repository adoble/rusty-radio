// [ ] (Finally) set up a config so that the streaming statistics are not printed.
// [ ]  Maybe the code will be simplified if Stations used nourl::Url instead of Strings for URLS.

use crate::RadioStation;

use embassy_net::{tcp::TcpSocket, IpAddress, Stack};
#[cfg(feature = "stats")]
use embassy_time::Instant;
use embassy_time::{Duration, Timer};

use embedded_io_async::Write;

use m3u::{M3UError, M3U};
use static_assertions::const_assert;

use core::net::Ipv4Addr;

use nourl::Url;

use heapless::String;

use crate::task::sync::{
    StationChangeReceiver, AUDIO_BUFFER_SIZE, MUSIC_PIPE, START_PLAYING, STATION_CHANGE_WATCH,
};

use http::{Method, Request, Response, ResponseStatusCode, MAX_URL_LEN};

// Empirically determined value. This value  has to be used in
// conjunction with the wifi tuning parameters in .cargo/config.toml
// Reducing it can give problems with some stations.
const TCP_BUFFER_SIZE: usize = 6000;

// Enough space to store all the HTTP header information
const HEADER_SIZE: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamingState {
    FillingPipe,
    Playing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ContentType {
    Audio,
    SimpleM3U(String<MAX_URL_LEN>),
    ExtendedM3U(String<MAX_URL_LEN>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StreamError {
    // Recoverabe errors. These are problems with the station.
    // The user is given is ability to change the station if it
    // does not work.
    Dns(embassy_net::dns::Error),
    IpAddressNotFound,
    ConnectionError(embassy_net::tcp::ConnectError),
    ConnectionPrematurelyClosed,
    Tcp(embassy_net::tcp::Error),

    StationUrlTooLong,
    MalformedUrl,

    HttpRequest(http::RequestError),
    HttpResponse(http::ResponseError),
    HeadersEndNotFound,
    UnableToReadContentType(embassy_net::tcp::Error),
    InvalidHttpCode(ResponseStatusCode),
    InvalidContent,
    TooManyBytesReadIn(usize),

    NoRedirectionLocationFound,
    RedirectionUrlTooLong,

    EmptyM3U,
    InvalidM3U(M3UError),
    M3uUrlTooLong,
    UrlNotFoundInM3U,
    // Non recoverable errors. These are due to program errors and
    // should not happen
    //StringAllocationTooSmall,
}

impl From<http::ResponseError> for StreamError {
    fn from(error: http::ResponseError) -> Self {
        StreamError::HttpResponse(error)
    }
}

impl From<http::RequestError> for StreamError {
    fn from(error: http::RequestError) -> Self {
        StreamError::HttpRequest(error)
    }
}

impl From<embassy_net::tcp::ConnectError> for StreamError {
    fn from(error: embassy_net::tcp::ConnectError) -> Self {
        StreamError::ConnectionError(error)
    }
}

impl From<embassy_net::dns::Error> for StreamError {
    fn from(error: embassy_net::dns::Error) -> Self {
        Self::Dns(error)
    }
}
impl From<core::str::Utf8Error> for StreamError {
    fn from(_error: core::str::Utf8Error) -> Self {
        StreamError::MalformedUrl
    }
}
impl From<embassy_net::tcp::Error> for StreamError {
    fn from(error: embassy_net::tcp::Error) -> Self {
        StreamError::Tcp(error)
    }
}
impl From<M3UError> for StreamError {
    fn from(error: M3UError) -> Self {
        Self::InvalidM3U(error)
    }
}
impl From<nourl::Error> for StreamError {
    fn from(_error: nourl::Error) -> Self {
        Self::MalformedUrl
    }
}

// This is the number of characters that have to be read in the determine the content type
// The assumption is that each content type contains at least this number of characters.
const TOKEN_LEN: usize = 7;

/// This task is the core of the rusty-radio project.
/// It accesses an internet radio station and sends the data to MUSIC_CHANNEL.
#[embassy_executor::task]
pub async fn stream(stack: Stack<'static>) {
    // Set up the receiver for changes in the station
    let Some(mut station_change_receiver) = STATION_CHANGE_WATCH.receiver() else {
        panic!("Cannot get station change watch receiver in task:stream");
    };

    let station_change_sender = STATION_CHANGE_WATCH.sender();

    loop {
        match stream_station(stack, &mut station_change_receiver).await {
            Ok(_) => (), //  stream_station will only return if there is an error

            Err(e) => {
                esp_println::println!("ERROR: {:?}", e);
                // Wait until the station changes

                let station = station_change_receiver.changed().await;

                // Resignal the station so that stream_station will pick it up again
                station_change_sender.send(station);

                // Try again with the new station
                continue;
            }
        }
    }
}

//#[embassy_executor::task]
async fn stream_station(
    stack: Stack<'static>,
    station_change_receiver: &mut StationChangeReceiver,
) -> Result<(), StreamError> {
    let mut rx_buffer = [0; TCP_BUFFER_SIZE];
    let mut tx_buffer = [0; TCP_BUFFER_SIZE];

    // This is important. Need to make sure the DHCP is up so
    // that the ip address can be found from the host name
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }

    stack.wait_config_up().await;

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

    // Optimisations
    // Timeout longer than keep alive
    // (see https://docs.embassy.dev/embassy-net/git/default/tcp/struct.TcpSocket.html#method.set_keep_alive)
    socket.set_timeout(Some(embassy_time::Duration::from_secs(15)));
    socket.set_keep_alive(Some(embassy_time::Duration::from_secs(10)));

    let mut body_buffer = [0u8; AUDIO_BUFFER_SIZE];

    // Get the initial station.
    // TODO assuming that this is always Some(station)
    let initial_station = station_change_receiver.get().await.unwrap();

    let initial_url = initial_station.url();
    let mut url_str = String::<MAX_URL_LEN>::new();
    url_str
        .push_str(initial_url)
        .map_err(|_| StreamError::StationUrlTooLong)?;

    'redirect: loop {
        let url = if !url_str.is_empty() {
            Url::parse(&url_str)?
        } else {
            //The URL is empty meaning no station selected so wait until one has been
            let new_station = station_change_receiver.get().await;
            match new_station {
                Some(station) => {
                    // A new station has been selected
                    url_str.clear();
                    url_str
                        .push_str(station.url())
                        .map_err(|_| StreamError::StationUrlTooLong)?;

                    Url::parse(&url_str)?
                }
                None => {
                    // No station selected so wait a bit and then check again
                    Timer::after(Duration::from_millis(10)).await;
                    continue 'redirect;
                }
            }
        };

        let host = url.host();
        let port = url.port_or_default();
        let path = url.path();

        let remote_ip_addresses = stack
            .dns_query(host, embassy_net::dns::DnsQueryType::A)
            .await?;

        let remote_ip_addr = if !remote_ip_addresses.is_empty() {
            remote_ip_addresses[0]
        } else {
            return Err(StreamError::IpAddressNotFound);
        };

        let remote_endpoint = match remote_ip_addr {
            IpAddress::Ipv4(ipv4_addr) => {
                let octets = ipv4_addr.octets();
                (Ipv4Addr::from(octets), port)
            }
        };

        // Connect to the socket using the IP address from the DNS
        socket.connect(remote_endpoint).await?;

        // Request the data
        let mut request = Request::new(Method::GET, path)?;
        request.host(host)?;

        // Set the user agent. Note this does not have to be a spoof of
        // a "normal" browser agent such as
        // "Mozilla/5.0 (X11; Linux x86_64; rv:138.0) Gecko/20100101 Firefox/138.0"
        // Note that this is based on the data in cross/app/Cargo.toml
        let user_agent = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
        request.header("User-Agent", user_agent)?;

        request.header("Connection", "keep-alive")?;

        socket.write_all(request.to_string().as_bytes()).await?;
        socket.flush().await?;

        let mut header_buffer = [0u8; HEADER_SIZE];

        read_headers(&mut socket, &mut header_buffer).await?;

        let response = Response::new(&header_buffer)?;

        match response.status_code() {
            ResponseStatusCode::Successful(_) => {
                let content_type = determine_content_type(&mut socket).await?;
                match content_type {
                    ContentType::Audio => (),
                    ContentType::SimpleM3U(location) => {
                        url_str.clear();
                        url_str
                            .push_str(&location)
                            .map_err(|_| StreamError::M3uUrlTooLong)?;

                        socket.abort();
                        socket.flush().await?;
                        continue 'redirect;
                    }
                    ContentType::ExtendedM3U(location) => {
                        url_str.clear();
                        url_str
                            .push_str(&location)
                            .map_err(|_| StreamError::M3uUrlTooLong)?;

                        socket.abort();
                        socket.flush().await?;
                        continue 'redirect;
                    }
                }
            }

            ResponseStatusCode::Redirection(_) => {
                url_str = response
                    .location
                    .ok_or(StreamError::NoRedirectionLocationFound)?;
                socket.abort();
                socket.flush().await?;
                continue 'redirect;
            }

            other => return Err(StreamError::InvalidHttpCode(other)),
        }

        // // Stream the audio until a new station has been selected by the tuner
        // let new_station =
        //     stream_audio(&mut socket, &mut body_buffer, station_change_receiver).await?;
        // url_str.clear();
        // url_str
        //     .push_str(new_station.url())
        //     .map_err(|_| StreamError::RedirectionUrlTooLong)?;
        // socket.abort();
        // socket.flush().await?;

        // Stream the audio until a new station has been selected by the tuner
        let new_station =
            stream_audio(&mut socket, &mut body_buffer, station_change_receiver).await?;

        match new_station {
            Some(station) => {
                // A new station has been selected
                url_str.clear();
                url_str
                    .push_str(station.url())
                    .map_err(|_| StreamError::RedirectionUrlTooLong)?;
            }
            None => {
                // No station has been selected just clear the url
                url_str.clear();
            }
        }
        // Close the socket properly. This happens if a new station has been selected AND also if no station
        // selected, so no music plays.
        socket.abort();
        socket.flush().await?;
    }
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
            .map_err(StreamError::Tcp)?
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

// Handle streaming of body, i.e. the mp3 data.
async fn stream_audio(
    socket: &mut TcpSocket<'_>,
    audio_buffer: &mut [u8],
    station_change_receiver: &mut StationChangeReceiver,
) -> Result<Option<RadioStation>, StreamError> {
    // let mut total_bytes = 0u32;
    // let mut last_stats = Instant::now();
    let mut read_state = StreamingState::FillingPipe;
    let initial_fill_len = 3 * MUSIC_PIPE.capacity() / 4;

    #[cfg(feature = "stats")]
    let (mut total_bytes, mut last_stats) = (0u32, Instant::now());

    loop {
        #[cfg(feature = "stats")]
        let read_start = Instant::now();

        match socket.read(audio_buffer).await {
            Ok(0) => {
                return Err(StreamError::ConnectionPrematurelyClosed);
            }
            Ok(n) => {
                #[cfg(feature = "stats")]
                let (read_time, write_start) = {
                    let read_time = read_start.elapsed().as_micros();
                    total_bytes += n as u32;
                    let write_start = Instant::now();
                    (read_time, write_start)
                };

                // Write immediately without trying to read more
                MUSIC_PIPE.write_all(&audio_buffer[..n]).await;

                if read_state == StreamingState::FillingPipe && MUSIC_PIPE.len() >= initial_fill_len
                {
                    // If the pipe is more than 75% full, start playing (and emptying the pipe)
                    START_PLAYING.signal(true);
                    read_state = StreamingState::Playing;
                };

                // Display network statistics if required
                #[cfg(feature = "stats")]
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

        if let Some(new_station) = station_change_receiver.try_changed() {
            break Ok(new_station);
        }
    }
}

async fn determine_content_type(socket: &mut TcpSocket<'_>) -> Result<ContentType, StreamError> {
    let mut token_buffer = [0u8; TOKEN_LEN];

    // Only readinng in one byte at a time for maximum control when it comes to errors
    let mut buf = [0u8; 1];

    let mut pos: usize = 0;

    // Read in the token at the start of the content
    while pos < TOKEN_LEN {
        match socket.read(&mut buf).await {
            Ok(0) => {
                return Err(StreamError::EmptyM3U);
            }
            Ok(n) => {
                token_buffer[pos..pos + n].copy_from_slice(&buf);
                pos += n;
            }
            Err(e) => return Err(StreamError::UnableToReadContentType(e)),
        }
    }

    // Safety check
    if pos < TOKEN_LEN {
        return Err(StreamError::InvalidContent);
    }

    let token_read_result = core::str::from_utf8(&token_buffer);
    let token = match token_read_result {
        Ok(token) => token,
        Err(_) => {
            // Unable to convert to UTF8so assume that the content type is audio
            return Ok(ContentType::Audio);
        }
    };

    let content_type = match token {
        "http://" => parse_simple_m3u(socket).await?,
        "#EXTM3U" => parse_extended_m3u(socket).await?,
        // Return audio content type in the unlikely event that the first byte scan be intepretated as UTF8
        _ => ContentType::Audio,
    };

    Ok(content_type)
}

async fn parse_simple_m3u(socket: &mut TcpSocket<'_>) -> Result<ContentType, StreamError> {
    // Make sure that  MAX_URL_LEN can contain the token and, at least, a minimal url (http://a.de);
    const_assert!(MAX_URL_LEN > TOKEN_LEN + 4);

    let mut url_buffer = [0u8; MAX_URL_LEN - TOKEN_LEN];

    match socket.read(&mut url_buffer).await {
        Ok(0) => Err(StreamError::EmptyM3U),
        Ok(n) => {
            let url_str = core::str::from_utf8(&url_buffer[0..n])?;

            // Sometimes more then one url is recorded. Just take the first one.
            let first_url_str = url_str.lines().next();

            let first_url_str = first_url_str.ok_or(StreamError::UrlNotFoundInM3U)?;

            let mut url = String::<MAX_URL_LEN>::new();
            // This has already been read as the token
            // SAFETY: We know this fits due to const_assert
            url.push_str("http://").unwrap();
            url.push_str(first_url_str)
                .map_err(|_| StreamError::StationUrlTooLong)?;
            Ok(ContentType::SimpleM3U(url))
        }
        Err(e) => Err(StreamError::Tcp(e)),
    }
}

// Extracts the first URL it finds in an extended M3U file and uses that as the next location
// It is designed to be sparing with memory.
//async fn parse_extended_m3u(socket: &mut TcpSocket<'_>) -> Result<ContentType, StreamError> {
async fn parse_extended_m3u(socket: &mut TcpSocket<'_>) -> Result<ContentType, StreamError> {
    let mut m3u = M3U::<MAX_URL_LEN>::new();

    let mut char_buf = [0u8; 1];

    loop {
        match socket.read(&mut char_buf).await {
            Ok(0) => {
                // EOF
                let url = m3u.terminate()?;
                return Ok(ContentType::ExtendedM3U(url));
            }
            Ok(1) => {
                let url = m3u.parse_m3u(char_buf[0])?;
                if let Some(url) = url {
                    return Ok(ContentType::ExtendedM3U(url));
                } else {
                    continue;
                }
            }
            Ok(n) => return Err(StreamError::TooManyBytesReadIn(n)),
            Err(e) => return Err(StreamError::Tcp(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mock_embedded_io::{MockError, Source};

    #[test]
    fn test_parse_extended_m3u_invalid_data() {
        futures::executor::block_on(async {
            let data_bytes = "#EXTM3U".as_bytes();
            let mut mock_source = Source::new()
                .data(data_bytes)
                .error(MockError(embedded_io_async::ErrorKind::BrokenPipe));

            let result = parse_extended_m3u(&mut mock_source).await;
            assert!(matches!(result, Err(StreamError::InvalidM3U)));
        });
    }

    #[test]
    fn test_parse_extended_m3u_valid_url() {
        futures::executor::block_on(async {
            let data = "#EXTM3U\nhttp://example.com/stream\n";
            let mut mock_source = Source::new().data(data.as_bytes());

            let result = parse_extended_m3u(&mut mock_source).await;
            match result {
                Ok(ContentType::ExtendedM3U(url)) => {
                    assert_eq!(url.as_str(), "http://example.com/stream");
                }
                _ => panic!("Expected ExtendedM3U content type"),
            }
        });
    }

    #[test]
    fn test_determine_content_type() {
        futures::executor::block_on(async {
            let data = "http://example.com/stream\n";
            let mut mock_source = Source::new().data(data.as_bytes());

            let result = determine_content_type(&mut mock_source).await;
            assert!(matches!(result, Ok(ContentType::SimpleM3U(_))));
        });
    }
}
