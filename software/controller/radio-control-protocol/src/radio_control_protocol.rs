use embedded_hal_nb::serial::{Error, Read, Write}; // Import the Write trait
use heapless::{String, Vec};
use itoa::Buffer;

use crate::uart_handler::{UartHandler, UartHandlerError, command::Command};

const MAX_NUMBER_PARAMETERS: usize = 5;

pub const MAX_PARAMETER_LEN: usize = 40;

pub struct RadioControlProtocol<'a, S>
where
    S: Write<u8> + Read<u8>,
{
    uart_handler: UartHandler<'a, S, MAX_PARAMETER_LEN, MAX_NUMBER_PARAMETERS>,
}

impl<'a, S> RadioControlProtocol<'a, S>
where
    S: Write<u8> + Read<u8>,
{
    pub fn new(serial: &'a mut S) -> Self {
        let uart_handler = UartHandler::new(serial);
        Self { uart_handler }
    }

    /// Sets a radio station based on it's id. The radio station name is returned.
    pub fn set_station(
        &mut self,
        station_id: u8,
    ) -> Result<String<MAX_PARAMETER_LEN>, RadioControlProtocolError> {
        let cmd = Command::Station;
        let mut tx_parameters = Vec::<&str, 5>::new();

        let mut buffer = Buffer::new();
        let station_id_str = buffer.format(station_id);

        // SAFETY - Ony ever pushing one parameter.
        tx_parameters.push(station_id_str).unwrap();

        self.uart_handler
            .send_command(cmd, tx_parameters)
            .map_err(|e| UartHandlerError::SerialWrite(e.kind()))?;

        let mut rx_parameters: Vec<String<MAX_PARAMETER_LEN>, MAX_NUMBER_PARAMETERS> = Vec::new();

        self.uart_handler.receive_response(&mut rx_parameters)?;

        if !rx_parameters.is_empty() {
            Ok(rx_parameters[0].clone())
        } else {
            Err(RadioControlProtocolError::StationNameNotReceived)
        }
    }

    /// Sets a station based its preset id. The station name is returned.
    pub fn set_preset(
        &mut self,
        preset_id: u8,
    ) -> Result<String<MAX_PARAMETER_LEN>, RadioControlProtocolError> {
        let cmd = Command::Preset;
        let mut tx_parameters = Vec::<&str, 5>::new();

        let mut buffer = Buffer::new();
        let preset_id_str = buffer.format(preset_id);

        // SAFETY - Only ever pushing one parameter.
        tx_parameters.push(preset_id_str).unwrap();

        self.uart_handler
            .send_command(cmd, tx_parameters)
            .map_err(|e| UartHandlerError::SerialWrite(e.kind()))?;

        let mut rx_parameters: Vec<String<MAX_PARAMETER_LEN>, MAX_NUMBER_PARAMETERS> = Vec::new();

        self.uart_handler.receive_response(&mut rx_parameters)?;

        if !rx_parameters.is_empty() {
            Ok(rx_parameters[0].clone())
        } else {
            Err(RadioControlProtocolError::StationNameNotReceived)
        }
    }

    /// Sets a station based its preset id. The station name is returned.
    pub fn query_config(&mut self) -> Result<usize, RadioControlProtocolError> {
        let cmd = Command::Config;
        let tx_parameters = Vec::<&str, 5>::new();

        // let mut buffer = Buffer::new();
        // let preset_id_str = buffer.format(preset_id);

        // SAFETY - Only ever pushing one parameter.
        // tx_parameters.push(preset_id_str).unwrap();

        self.uart_handler
            .send_command(cmd, tx_parameters)
            .map_err(|e| UartHandlerError::SerialWrite(e.kind()))?;

        let mut rx_parameters: Vec<String<MAX_PARAMETER_LEN>, MAX_NUMBER_PARAMETERS> = Vec::new();

        self.uart_handler.receive_response(&mut rx_parameters)?;

        if !rx_parameters.is_empty() {
            let number_stations: usize = rx_parameters[0]
                .parse()
                .map_err(|_| RadioControlProtocolError::ParseParameter)?;

            Ok(number_stations)
        } else {
            Err(RadioControlProtocolError::IncorrectNumberParametersReturned)
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum RadioControlProtocolError {
    Uart(UartHandlerError),
    StationNameNotReceived,
    ParseParameter,
    IncorrectNumberParametersReturned,
}

impl From<UartHandlerError> for RadioControlProtocolError {
    fn from(error: UartHandlerError) -> Self {
        RadioControlProtocolError::Uart(error)
    }
}
