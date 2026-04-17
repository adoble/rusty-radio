use embedded_hal_nb::serial::{Read, Write}; // Import the Write trait
use heapless::{String, Vec};
use nb::block; // Import the block! macro to wait for operations
use static_assertions::{self, const_assert};

pub mod command;
pub use command::Command;

mod error;
pub use error::UartHandlerError;

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
