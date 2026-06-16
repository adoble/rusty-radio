use embedded_io_async::{Read, Write}; // Import the Write trait
use heapless::{CapacityError, String, Vec};
use itoa::Buffer;
use static_assertions::const_assert;

#[allow(unused_imports)]
use log::{debug, error, info};

use crate::station_config::StationConfig;
use crate::uart_handler::{UartHandler, UartHandlerError, command::Command};

const MAX_NUMBER_PARAMETERS: usize = 5;
const_assert!(MAX_NUMBER_PARAMETERS > 0);

pub const MAX_PARAMETER_LEN: usize = 40;

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

        // let mut buffer = Buffer::new();
        // let preset_id_str = buffer.format(preset_id);

        // SAFETY - Only ever pushing one parameter.
        // tx_parameters.push(preset_id_str).unwrap();

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
            debug!("param {}", param);
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
}

#[derive(PartialEq, Debug)]
pub enum RadioControlProtocolError {
    Uart(UartHandlerError),
    StationNameNotReceived,
    ParseParameter,
    IncorrectNumberParametersReturned,
    ParameterLengthTooSmall,
    CannotLoadStationConfig,
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
