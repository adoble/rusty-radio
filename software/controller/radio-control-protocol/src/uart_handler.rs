use embedded_io_async::{Read, Write}; // Import the Write trait
use heapless::{String, Vec};

#[allow(unused_imports)]
use log::{debug, error, info};

pub mod command;
pub use command::Command;

mod error;
pub use error::UartHandlerError;

pub struct UartHandler<'a, S, const MAX_PARAMETER_LEN: usize, const MAX_NUMBER_PARAMETERS: usize>
where
    S: Write + Read,
{
    serial: &'a mut S,
}

impl<'a, S, const MAX_PARAMETER_LEN: usize, const MAX_NUMBER_PARAMETERS: usize>
    UartHandler<'a, S, MAX_PARAMETER_LEN, MAX_NUMBER_PARAMETERS>
where
    S: Write + Read,
{
    pub fn new(serial: &'a mut S) -> Self {
        Self { serial }
    }

    pub async fn send_command(
        &mut self,
        command: Command,
        parameters: Vec<String<MAX_PARAMETER_LEN>, MAX_NUMBER_PARAMETERS>,
    ) -> Result<(), S::Error> {
        let cmd: [u8; 3] = (&command).into();
        self.serial.write_all(&cmd).await?;

        self.serial.write_all(b":").await?;

        for (index, param) in parameters.iter().enumerate() {
            self.serial.write_all(param.as_bytes()).await?;

            if index < parameters.len() - 1 {
                // Not at end so add a comma
                self.serial.write_all(b",").await?
            };
        }

        // Terminate
        self.serial.write_all(b";").await?;

        self.serial.flush().await?;

        Ok(())
    }

    pub async fn receive_response(
        &mut self,
        parameters: &mut Vec<String<MAX_PARAMETER_LEN>, MAX_NUMBER_PARAMETERS>,
    ) -> Result<(), UartHandlerError> {
        //let mut rx_buffer = String::<4>::new();
        // TODO what happens if less than 4 characters are sent back?
        let mut rx_cmd_buffer = [0u8; 4];
        self.serial.read_exact(&mut rx_cmd_buffer).await?;

        // TODO what should be the max size of a parameter? Use compile time contraints,
        let mut param = String::<MAX_PARAMETER_LEN>::new();

        let mut error_code: String<3> = String::new();

        let mut char_buf = [0u8; 1];

        match str::from_utf8(&rx_cmd_buffer)? {
            "ACK:" => {
                // Load the parameters returned
                loop {
                    self.serial.read_exact(&mut char_buf).await?;

                    // SAFETY: A char is always avaialable.
                    let c = str::from_utf8(&char_buf)?.chars().next().unwrap();
                    if c != ';' {
                        if c != ',' {
                            param.push(c)?;
                        } else {
                            parameters
                                .push(param)
                                .map_err(|_| UartHandlerError::TooManyParameters)?;
                            param = String::new();
                        }
                    } else {
                        // Terminator found
                        if !param.is_empty() {
                            parameters
                                .push(param)
                                .map_err(|_| UartHandlerError::TooManyParameters)?
                        }
                        break;
                    }
                }
            }

            "ERR:" => {
                let error_code = loop {
                    self.serial.read_exact(&mut char_buf).await?;
                    let c = str::from_utf8(&char_buf)?.chars().next().unwrap();
                    if c != ';' {
                        error_code.push(c)?;
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
