use core::num::ParseIntError;
use core::str::Utf8Error;

use embedded_io_async::{Read, Write}; // Import the Write trait
use heapless::{CapacityError, String, Vec};
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
pub type RadioStationName = String<40>;

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
    ) -> Result<String<MAX_PARAMETER_LEN>, RadioControlProtocolError> {
        let cmd = Command::Station;
        let mut tx_parameters = Vec::<String<MAX_PARAMETER_LEN>, 5>::new();

        let mut buffer = Buffer::new();
        let mut station_id_str = String::<MAX_PARAMETER_LEN>::new();
        station_id_str.push_str(buffer.format(station_id))?;

        // SAFETY - Only ever pushing one parameter.
        tx_parameters.push(station_id_str).unwrap();

        //TODO make parameter a new type Parameters (and also Parameter).
        //Use Deref to manipulat at the underlying type
        self.uart_handler
            .send_command(cmd, tx_parameters)
            .await
            .map_err(|_| UartHandlerError::SerialWrite)?;

        let mut rx_parameters: Vec<String<MAX_PARAMETER_LEN>, MAX_NUMBER_PARAMETERS> = Vec::new();

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
    ) -> Result<String<MAX_PARAMETER_LEN>, RadioControlProtocolError> {
        let cmd = Command::Preset;
        let mut tx_parameters = Vec::<String<MAX_PARAMETER_LEN>, 5>::new();

        let mut buffer = Buffer::new();
        let mut preset_id_str = String::<MAX_PARAMETER_LEN>::new();
        preset_id_str.push_str(buffer.format(preset_id))?;

        // SAFETY - Only ever pushing one parameter.
        tx_parameters.push(preset_id_str).unwrap();

        self.uart_handler
            .send_command(cmd, tx_parameters)
            .await
            .map_err(|_| UartHandlerError::SerialWrite)?;

        let mut rx_parameters: Vec<String<MAX_PARAMETER_LEN>, MAX_NUMBER_PARAMETERS> = Vec::new();

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
        let tx_parameters = Vec::<String<MAX_PARAMETER_LEN>, 5>::new();

        self.uart_handler
            .send_command(cmd, tx_parameters)
            .await
            .map_err(|_| UartHandlerError::SerialWrite)?;

        let mut rx_parameters: Vec<String<MAX_PARAMETER_LEN>, MAX_NUMBER_PARAMETERS> = Vec::new();

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
        let tx_parameters = Vec::<String<MAX_PARAMETER_LEN>, 5>::new();

        self.uart_handler
            .send_command(cmd, tx_parameters)
            .await
            .map_err(|_| UartHandlerError::SerialWrite)?;

        let mut rx_parameters: Vec<String<MAX_PARAMETER_LEN>, MAX_NUMBER_PARAMETERS> = Vec::new();

        self.uart_handler
            .receive_response(&mut rx_parameters)
            .await?;

        Ok(())
    }

    // TODO move this out to another struct just for receiving
    pub async fn receive_command(&mut self) -> Result<RadioCommand, RadioControlProtocolError> {
        let number_bytes = self.uart_handler.receive_frame().await?;

        if number_bytes < 5 {
            return Err(RadioControlProtocolError::IllFormedReponse);
        }

        let framebuffer = self.uart_handler.framebuffer();

        let parsed_command = match &framebuffer[0..3] {
            b"STA" => self.parse_station_parameters(&framebuffer[3..])?,
            b"DTN" if &framebuffer[3..5] == b":;" => RadioCommand::Detune,
            b"CFG" if &framebuffer[3..5] == b":;" => RadioCommand::Config,
            _ => Err(RadioControlProtocolError::UnknownCommand)?,
        };

        Ok(parsed_command)
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
pub enum RadioControlProtocolError {
    Uart(UartHandlerError),
    StationNameNotReceived,
    ParseParameter,
    IncorrectNumberParametersReturned,
    ParameterLengthTooSmall,
    CannotLoadStationConfig,
    IllFormedReponse,
    Timeout,
    UnknownCommand,
}

impl From<UartHandlerError> for RadioControlProtocolError {
    fn from(error: UartHandlerError) -> Self {
        RadioControlProtocolError::Uart(error)
    }
}

impl From<CapacityError> for RadioControlProtocolError {
    fn from(_: CapacityError) -> Self {
        Self::ParameterLengthTooSmall
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    use embedded_io_async_mock::{Mock as SerialMock, Transaction as SerialTransaction};

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
}
