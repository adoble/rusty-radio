#![cfg_attr(not(test), no_std)]

use embedded_hal_nb::serial::{Error, Read, Write}; // Import the Write trait
use heapless::{String, Vec};
use itoa::Buffer;
use nb::block; // Import the block! macro to wait for operations
use static_assertions::{self, const_assert};

pub mod command;
use command::Command;

mod error;
use error::UartHandlerError;

const MAX_NUMBER_PARAMETERS: usize = 5;

#[deprecated(note = "TODO: Make this generic")]
pub const MAX_STATION_NAME_LEN: usize = 40;

pub const MAX_PARAMETER_LEN: usize = 40;

const_assert!(MAX_PARAMETER_LEN >= MAX_STATION_NAME_LEN);

// Assumes 'serial' is a pre-configured UART instance (e.g., from a device HAL)
// e.g., let mut serial = stm32f4xx_hal::serial::Serial::new(...);

pub struct UartHandler<'a, S>
where
    S: Write<u8> + Read<u8>,
{
    serial: &'a mut S,
}

impl<'a, S> UartHandler<'a, S>
where
    S: Write<u8> + Read<u8>,
{
    pub fn new(serial: &'a mut S) -> Self {
        Self { serial }
    }

    pub fn set_station(
        &mut self,
        station_id: u8,
    ) -> Result<String<MAX_STATION_NAME_LEN>, UartHandlerError> {
        let cmd = Command::Station;
        let mut parameters = Vec::<&str, 5>::new();

        let mut buffer = Buffer::new();
        let station_id_str = buffer.format(station_id);

        // SAFETY - Ony ever pushing one parameter.
        parameters.push(station_id_str).unwrap();

        self.send_command(cmd, parameters)
            .map_err(|e| UartHandlerError::SerialWrite(e.kind()))?;

        // let cmd = b"STA:";
        // for &byte in cmd {
        //     block!(self.serial.write(byte)).map_err(|e| UartHandlerError::SerialWrite(e.kind()))?;
        // }
        // let mut buffer = Buffer::new();
        // let station_id_str = buffer.format(station_id).as_bytes();
        // for &byte in station_id_str {
        //     block!(self.serial.write(byte)).map_err(|e| UartHandlerError::SerialWrite(e.kind()))?;
        // }
        // block!(self.serial.write(b';')).map_err(|e| UartHandlerError::SerialWrite(e.kind()))?;

        // block!(self.serial.flush()).map_err(|e| UartHandlerError::SerialWrite(e.kind()))?;

        // TODO change this into a Vec to save lots of conversions
        const MAX_RESPONSE_LEN: usize = MAX_STATION_NAME_LEN + 5;
        let mut rx_bytes = Vec::<u8, MAX_RESPONSE_LEN>::new();
        loop {
            let c =
                block!(self.serial.read()).map_err(|e| UartHandlerError::SerialRead(e.kind()))?;
            rx_bytes
                .push(c)
                .map_err(|_| UartHandlerError::ResponseTooLarge)?;
            if c == b';' {
                break;
            }

            // Optional: Add a timeout or max length check to prevent infinite loop
            // TODO parse the response
        }

        let response = String::<MAX_RESPONSE_LEN>::from_utf8(rx_bytes)
            .map_err(|_| UartHandlerError::NonUTF8)?;

        // Parse the response
        let mut station_name = String::<MAX_STATION_NAME_LEN>::new();
        match response[0..4].as_bytes() {
            b"ACK:" => {
                let terminator_pos = response
                    .find(';')
                    .ok_or(UartHandlerError::IllFormedReponse)?;

                station_name
                    .push_str(&response[4..terminator_pos])
                    .map_err(|_| UartHandlerError::ParameterTooLarge)?;
            }
            b"ERR:" => {
                todo!("ERR")
            }
            _ => todo!("proper error handling"),
        };

        Ok(station_name)
    }

    pub fn send_command(
        &mut self,
        command: Command,
        parameters: Vec<&str, MAX_NUMBER_PARAMETERS>,
    ) -> Result<(), S::Error> {
        let cmd = command.stringify().into_bytes();
        for byte in cmd {
            block!(self.serial.write(byte))?;
        }
        block!(self.serial.write(b':'))?;

        for (index, param) in parameters.iter().enumerate() {
            for &byte in param.as_bytes() {
                block!(self.serial.write(byte))?;
            }
            if index < parameters.len() - 1 {
                // Not at end so add a comma
                block!(self.serial.write(b','))?;
            };
        }

        // Terminate
        block!(self.serial.write(b';'))?;

        block!(self.serial.flush())?;

        Ok(())
    }

    pub fn receive_response(
        &mut self,
        parameters: &mut Vec<String<MAX_PARAMETER_LEN>, MAX_NUMBER_PARAMETERS>,
    ) -> Result<(), UartHandlerError> {
        let mut rx_buffer = String::<4>::new();

        // TODO what happens if less than 4 characters are sent back?
        for _ in 0..4 {
            let c = self.serial.read().expect("TODO error handling");
            // SAFETY: number of reads and string capacity both set to 4.
            rx_buffer.push(c as char).unwrap();
        }

        // TODO what should be the max size of a parameter? Use compile time contraints,
        let mut param = String::<MAX_PARAMETER_LEN>::new();

        let mut error_code: String<3> = String::new();

        match rx_buffer.as_str() {
            "ACK:" => {
                // Load the parameters returned
                loop {
                    let c = self.serial.read().expect("TODO error handling");
                    if c != b';' {
                        if c != b',' {
                            param.push(c as char).expect("TODO error handling");
                        } else {
                            parameters.push(param).expect("TODO error handling");
                            param = String::new();
                        }
                    } else {
                        // Terminator found
                        if !param.is_empty() {
                            parameters.push(param).expect("TODO error handling");
                        }
                        break;
                    }
                }
            }

            "ERR:" => {
                let error_code = loop {
                    let c = self.serial.read().expect("TODO error handling");
                    if c != b';' {
                        error_code.push(c as char).expect("TODO error handling");
                    } else {
                        // Terminator found
                        break error_code;
                    }
                };

                match error_code.as_str() {
                    "001" => return Err(UartHandlerError::ClientCannotHandleCommand),
                    _ => return Err(UartHandlerError::ClientSentUnknownErrorCode),
                };
            }

            _ => return Err(UartHandlerError::IllFormedReponse),
        }

        Ok(())
    }
}
