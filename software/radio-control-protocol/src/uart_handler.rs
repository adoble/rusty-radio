//! Handles the low level communication over the UART connection between
//! the ui processor and the radio processor.
//!
//! ## Protocol Grammer
//! It effectively handles the following framing grammer for the protocol.
//!
//!
//!```ebnf
//! <send_command>   ::= <command> ":" <parameter_list> ";"
//! <parameter_list> ::= ( <parameter> ( <separator>  <parameter> )* )?
//! <command>        ::= <capital_letter> <capital_letter> <capital_letter>
//! <parameter>      ::= <utf8_string>
//! <capital_letter> ::= [A-Z]
//! <separator>      ::= ","
//! <positive_integer> ::= <digit>+
//! <digit>          ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
//!
//!
//! <response>  ::= <acknowledge_response>  | <error_response>
//!
//! <acknowledge_response> ::= "ACK:"  <parameter_list> ";"
//!
//! <error_response> ::= "ERR:" <error_code> ";"
//!
//! <error_code> ::= <positive_integer>
//!
//!
//!
//! <utf8_string>   ::=  <utf8_char>*
//!
//! <utf8_char> ::= /* A set of UTF8_1 characters */
//!```

use embedded_io_async::{Read, Write}; // Import the Write trait
use heapless::{String, Vec};

#[allow(unused_imports)]
use log::{debug, error, info};

pub mod command;
pub use command::Command;

mod error;
pub use error::UartHandlerError;

/// Sending and receiving of commands and reposnses using the framing grammer.
///
/// Instead of using a specific UART periperhal it uses the traits `embedded_io_async::{Read, Write}`.
/// In this way it can be used tested in a `std` environment,
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
    /// Crate a new uart handler using the serial interface specified.
    ///
    pub fn new(serial: &'a mut S) -> Self {
        Self { serial }
    }

    /// Send a command over the serial interface to the receiver.
    ///
    /// This is used from the sender side of the interface.
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

    /// Receive a response over the serial interfaces
    /// Any parameters contained in the response are placed in `parameters`.
    ///
    /// This is used from the sender side of the interface.
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

    /// Send an acknowledge back to the sender with any parameters.
    ///
    /// This is used from the receiver side of the interface
    pub async fn send_acknowledge(
        &mut self,
        parameters: &Vec<String<MAX_PARAMETER_LEN>, MAX_NUMBER_PARAMETERS>,
    ) -> Result<(), S::Error> {
        self.serial.write_all(b"ACK:").await?;

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

    /// Send an error back from the receiver with the error number
    pub async fn send_error(&mut self, error_number: usize) -> Result<(), S::Error> {
        self.serial.write_all(b"ERR:").await?;

        let mut s: String<20> = String::new();
        core::fmt::Write::write_fmt(&mut s, format_args!("{}", error_number)).ok();
        self.serial.write_all(s.as_bytes()).await?;

        self.serial.write_all(b";").await?;

        self.serial.flush().await?;

        Ok(())
    }
}
