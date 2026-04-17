use embedded_hal_nb::serial::{Error, Read, Write}; // Import the Write trait
use heapless::{String, Vec};
use itoa::Buffer;
use static_assertions::{self, const_assert};

use crate::uart_handler::{UartHandler, UartHandlerError, command::Command};

const MAX_NUMBER_PARAMETERS: usize = 5;

#[deprecated(note = "TODO: Make this generic")]
pub const MAX_STATION_NAME_LEN: usize = 40;

pub const MAX_PARAMETER_LEN: usize = 40;

const_assert!(MAX_PARAMETER_LEN >= MAX_STATION_NAME_LEN);

// Assumes 'serial' is a pre-configured UART instance (e.g., from a device HAL)
// e.g., let mut serial = stm32f4xx_hal::serial::Serial::new(...);

pub struct RadioControlProtocol<'a, S>
where
    S: Write<u8> + Read<u8>,
{
    uart_handler: UartHandler<'a, S>,
}

impl<'a, S> RadioControlProtocol<'a, S>
where
    S: Write<u8> + Read<u8>,
{
    pub fn new(serial: &'a mut S) -> Self {
        let uart_handler = UartHandler::new(serial);
        Self { uart_handler }
    }

    pub fn set_station(
        &mut self,
        station_id: u8,
    ) -> Result<String<MAX_STATION_NAME_LEN>, RadioControlProtocolError> {
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
}

#[derive(PartialEq, Debug)]
pub enum RadioControlProtocolError {
    Uart(UartHandlerError),
    StationNameNotReceived,
}

impl From<UartHandlerError> for RadioControlProtocolError {
    fn from(error: UartHandlerError) -> Self {
        RadioControlProtocolError::Uart(error)
    }
}
