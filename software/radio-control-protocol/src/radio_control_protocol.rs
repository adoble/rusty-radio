use core::num::ParseIntError;
use core::str::Utf8Error;

use embedded_io_async::{Read, Write}; // Import the Write trait
use heapless::{String, Vec};
use itoa::Buffer;
use static_assertions::const_assert;

#[allow(unused_imports)]
use log::{debug, error, info};

use crate::station_config::StationConfig;
// use crate::uart_handler::RadioCommand;
use crate::uart_handler::{UartHandler, UartHandlerError, command::Command};

pub const MAX_NUMBER_PARAMETERS: usize = 5;
const_assert!(MAX_NUMBER_PARAMETERS > 0);

pub const MAX_PARAMETER_LEN: usize = 40;

pub type RadioStationId = usize;
pub type RadioStationName = String<MAX_PARAMETER_LEN>;

pub type Parameter = String<MAX_PARAMETER_LEN>;
pub type Parameters = Vec<Parameter, MAX_NUMBER_PARAMETERS>;

pub struct RadioControlProtocol<'a, S>
where
    S: Write + Read,
{
    uart_handler: UartHandler<'a, S, MAX_PARAMETER_LEN, MAX_NUMBER_PARAMETERS>,
}

impl<'a, S> RadioControlProtocol<'a, S>
where
    S: Write + Read,
{
    pub fn new(serial: &'a mut S) -> Self {
        let uart_handler = UartHandler::new(serial);
        Self { uart_handler }
    }

    /// Sets a radio station based on it's id. The radio station name is returned.
    pub async fn set_station(
        &mut self,
        station_id: u8,
    ) -> Result<RadioStationName, RadioControlProtocolError> {
        let cmd = Command::Station;
        let mut tx_parameters = Parameters::new();

        let mut buffer = Buffer::new();
        let mut station_id_str = RadioStationName::new();
        station_id_str
            .push_str(buffer.format(station_id))
            .map_err(|_| RadioControlProtocolError::StationIdInvalid)?;

        // SAFETY - Only ever pushing one parameter.
        tx_parameters.push(station_id_str).unwrap();

        self.uart_handler
            .send_command(cmd, tx_parameters)
            .await
            .map_err(|_| UartHandlerError::SerialWrite)?;

        let mut rx_parameters = Parameters::new();

        self.uart_handler
            .receive_response(&mut rx_parameters)
            .await?;
        if !rx_parameters.is_empty() {
            Ok(rx_parameters[0].clone())
        } else {
            error!("Station name not received");
            Err(RadioControlProtocolError::StationNameNotReceived)
        }
    }

    /// Sets a station based its preset id. The station name is returned.
    pub async fn set_preset(
        &mut self,
        preset_id: u8,
    ) -> Result<RadioStationName, RadioControlProtocolError> {
        let cmd = Command::Preset;
        let mut tx_parameters = Parameters::new();

        let mut buffer = Buffer::new();
        let mut preset_id_str = RadioStationName::new();
        preset_id_str
            .push_str(buffer.format(preset_id))
            .map_err(|_| RadioControlProtocolError::StationIdInvalid)?;

        // SAFETY - Only ever pushing one parameter.
        tx_parameters.push(preset_id_str).unwrap();

        self.uart_handler
            .send_command(cmd, tx_parameters)
            .await
            .map_err(|_| UartHandlerError::SerialWrite)?;

        let mut rx_parameters = Parameters::new();

        self.uart_handler
            .receive_response(&mut rx_parameters)
            .await?;

        if !rx_parameters.is_empty() {
            Ok(rx_parameters[0].clone())
        } else {
            Err(RadioControlProtocolError::StationNameNotReceived)
        }
    }

    /// Queries the configuation of the stations and return the number of stations
    pub async fn query_config(&mut self) -> Result<StationConfig, RadioControlProtocolError> {
        let cmd = Command::Config;
        let tx_parameters = Parameters::new();

        self.uart_handler
            .send_command(cmd, tx_parameters)
            .await
            .map_err(|_| UartHandlerError::SerialWrite)?;

        let mut rx_parameters = Parameters::new();

        self.uart_handler
            .receive_response(&mut rx_parameters)
            .await?;

        if rx_parameters.is_empty() {
            error!("No parameters returned from query config");
            return Err(RadioControlProtocolError::IncorrectNumberParametersReturned);
        };

        let mut station_config = StationConfig::new();
        station_config.number_stations = rx_parameters[0]
            .parse()
            .map_err(|_| RadioControlProtocolError::ParseParameter)?;

        // Now extract the presets
        for param in rx_parameters.iter().skip(1) {
            let preset_station_id: usize = param
                .parse()
                .map_err(|_| RadioControlProtocolError::ParseParameter)?;
            station_config
                .presets
                .push(preset_station_id)
                .map_err(|_| RadioControlProtocolError::IncorrectNumberParametersReturned)?;
        }

        Ok(station_config)
    }

    /// Detunes the radio.
    pub async fn detune(&mut self) -> Result<(), RadioControlProtocolError> {
        let cmd = Command::Detune;
        let tx_parameters = Parameters::new();

        self.uart_handler
            .send_command(cmd, tx_parameters)
            .await
            .map_err(|_| UartHandlerError::SerialWrite)?;

        let mut rx_parameters = Parameters::new();

        self.uart_handler
            .receive_response(&mut rx_parameters)
            .await?;

        Ok(())
    }

    // TODO move this out to another struct just for receiving
    pub async fn receive_command(&mut self) -> Result<RadioCommand, RadioControlProtocolError> {
        debug!("Entered receive_command");

        let number_bytes: usize;
        const RETRIES: usize = 20; // TODO where to put this constant
        let mut count: usize = 0;

        loop {
            match self.uart_handler.receive_frame().await {
                Ok(n) => {
                    number_bytes = n;
                    break;
                }
                Err(UartHandlerError::FrameBufferOverflow) => {
                    if count < RETRIES {
                        count += 1;
                        continue;
                    } else {
                        return Err(RadioControlProtocolError::IllFormedReponse);
                    }
                }
                Err(e) => return Err(RadioControlProtocolError::Uart(e)),
            };
        }

        // let number_bytes = self.uart_handler.receive_frame().await?;

        debug!("Received {} bytes", number_bytes);

        if number_bytes < 5 {
            return Err(RadioControlProtocolError::IllFormedReponse);
        }

        let framebuffer = self.uart_handler.framebuffer();
        let command_bytes = self
            .command_span(framebuffer)
            .ok_or(RadioControlProtocolError::CommandNotFound)?; //TODO review the error handling here

        debug!("receive_command::command_bytes: {:?}", command_bytes);

        let parsed_command = match &command_bytes[0..3] {
            b"STA" => self.parse_station_parameters(&command_bytes[3..])?,
            b"DTN" if &command_bytes[3..5] == b":;" => RadioCommand::Detune,
            b"CFG" if &command_bytes[3..5] == b":;" => RadioCommand::Config,
            _ => Err(RadioControlProtocolError::UnknownCommand)?,
        };

        Ok(parsed_command)
    }

    ///Get a slice from the framebuffer thet contains a command.
    fn command_span(&self, framebuffer: &'a [u8]) -> Option<&'a [u8]> {
        let mut start = framebuffer.windows(4).position(|w| w == b"CFG:");

        if start.is_none() {
            start = framebuffer.windows(4).position(|w| w == b"STA:");
        }

        if start.is_none() {
            start = framebuffer.windows(4).position(|w| w == b"DTN:");
        }

        let start = start?; //Ugh!

        let end = framebuffer[start..].windows(1).position(|w| w == b";")?;
        Some(&framebuffer[start..(end + start + 1)])
    }

    // TODO this is a thin veneer over  UartHandler::send_acknowledge. Maybe move that to here as
    // it  handles the semantics.
    pub async fn send_acknowledge(
        &mut self,
        parameters: &Parameters,
    ) -> Result<(), RadioControlProtocolError> {
        self.uart_handler
            .send_acknowledge(parameters)
            .await
            .map_err(|_| UartHandlerError::SerialWrite)?;

        Ok(())
    }

    pub async fn send_error(&mut self, error_number: u8) {
        let result = self.uart_handler.send_error(error_number).await;

        // As we cannot display errors as the connection has failed, panic.
        if let Err(e) = result {
            error!("Unable to send error back: {:?}", e);
            panic!("Unable to send error back: {:?}", e);
        }
    }

    pub async fn flush(&mut self) -> Result<(), RadioControlProtocolError> {
        self.uart_handler
            .flush()
            .await
            .map_err(|_| UartHandlerError::SerialFlush)?;
        Ok(())
    }

    fn parse_station_parameters(
        &self,
        remaining: &[u8],
    ) -> Result<RadioCommand, RadioControlProtocolError> {
        let colon_position: Option<usize> = remaining.iter().position(|&b| b == b':');
        let terminator_position = remaining.iter().position(|&b| b == b';');

        // Error handling
        let colon_position = colon_position.ok_or(RadioControlProtocolError::IllFormedReponse)?;
        let terminator_position =
            terminator_position.ok_or(RadioControlProtocolError::IllFormedReponse)?;

        let station_id = &remaining[(colon_position + 1)..terminator_position];

        let station_id = str::from_utf8(station_id)?; //.map_err(|e| RadioControlProtocolError::IllFormedReponse)?;
        let station_id: RadioStationId = station_id.parse::<usize>()?;

        Ok(RadioCommand::Station(station_id))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RadioCommand {
    /// Set a station based on it's ID.
    Station(RadioStationId),

    /// Detune between stations
    Detune,

    /// Configure from the web
    Config,
    // /// An error has been detected
    // Error(RadioControlProtocolError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TunerDirection {
    Clockwise,
    CounterClockwise,
}

#[derive(PartialEq, Debug, Clone)]
#[repr(u8)]
pub enum RadioControlProtocolError {
    Uart(UartHandlerError),
    StationNameNotReceived = 102,
    StationIdInvalid = 103,
    ParseParameter = 104,
    IncorrectNumberParametersReturned = 105,
    ParameterLengthTooSmall = 106,
    CannotLoadStationConfig = 107,
    IllFormedReponse = 108,
    Timeout = 109,
    UnknownCommand = 110,
    CommandNotFound = 111,
}

impl From<UartHandlerError> for RadioControlProtocolError {
    fn from(error: UartHandlerError) -> Self {
        RadioControlProtocolError::Uart(error)
    }
}

// impl From<CapacityError> for RadioControlProtocolError {
//     fn from(_: CapacityError) -> Self {
//         Self::ParameterLengthTooSmall
//     }
// }

// TODO should IllFormreposnse carry some extra error information?
impl From<Utf8Error> for RadioControlProtocolError {
    fn from(_: Utf8Error) -> Self {
        Self::IllFormedReponse
    }
}

impl From<ParseIntError> for RadioControlProtocolError {
    fn from(_: ParseIntError) -> Self {
        Self::IllFormedReponse
    }
}

impl From<RadioControlProtocolError> for u8 {
    fn from(e: RadioControlProtocolError) -> Self {
        match e {
            RadioControlProtocolError::Uart(_) => 101,
            other => other.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use embedded_io_async_mock::Mock as SerialMock;

    #[test]
    fn test_parse_station_parameters() {
        // <station_select_command> ::= "STA" ":" (<station_id> |  ";")

        let parameters = b":42;";

        let expectations = [];

        let mut serial = SerialMock::new(&expectations);

        let radio_command_protocol = RadioControlProtocol::new(&mut serial);

        let command = radio_command_protocol
            .parse_station_parameters(parameters)
            .unwrap();

        assert_eq!(command, RadioCommand::Station(42));

        serial.done();
    }

    #[test]
    fn test_parse_station_parameters_error() {
        // <station_select_command> ::= "STA" ":" (<station_id> |  ";")

        let parameters = b":AB;";

        let expectations = [];

        let mut serial = SerialMock::new(&expectations);

        let radio_command_protocol = RadioControlProtocol::new(&mut serial);

        let r = radio_command_protocol.parse_station_parameters(parameters);

        assert_eq!(r, Err(RadioControlProtocolError::IllFormedReponse));

        serial.done();
    }

    #[test]
    fn test_command_span() {
        let framebuffer = b"abcdefCFG:;12345";

        let mut serial = SerialMock::new(&[]);

        let rcp = RadioControlProtocol::new(&mut serial);

        let v = rcp.command_span(framebuffer);

        assert!(v.is_some());
        assert_eq!(5, v.unwrap().len());

        assert_eq!(b"CFG:;", v.unwrap());

        serial.done();
    }

    #[test]
    fn test_command_span_with_no_command() {
        let framebuffer = b"abcdef12345";

        let mut serial = SerialMock::new(&[]);

        let rcp = RadioControlProtocol::new(&mut serial);

        let v = rcp.command_span(framebuffer);

        assert!(v.is_none());

        serial.done();
    }

    #[test]
    fn test_command_span_boundaries() {
        let framebuffer = b"STA:23;abcdef12345"; // At the start

        let mut serial = SerialMock::new(&[]);

        let rcp = RadioControlProtocol::new(&mut serial);

        let v = rcp
            .command_span(framebuffer)
            .expect("Could not find command");

        assert_eq!(b"STA:23;", v);

        let framebuffer = b"abcdef12345DTN:;"; // At the end

        let v = rcp
            .command_span(framebuffer)
            .expect("Could not find command");

        assert_eq!(b"DTN:;", v);

        serial.done();
    }
}
