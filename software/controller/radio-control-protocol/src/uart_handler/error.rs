// use embedded_io::Read;
use core::str::Utf8Error;
use embedded_io_async::{ErrorKind, ReadExactError};
use heapless::CapacityError;

#[derive(Debug, PartialEq)]
//#[cfg_attr(not(test), derive(defmt::Format))] // Only used when running on target hardware

// TODO  use what we learnt in writing mock-serial-async
pub enum UartHandlerError {
    SerialWrite,
    SerialRead,
    ResponseTooLarge,
    ConnectionAborted,
    NonUTF8(Option<usize>),
    IllFormedReponse,
    ParameterTooLarge,
    TooManyParameters,
    UnknownCommand,

    ClientCannotHandleCommand,
    ClientSentUnknownErrorCode,
    Other(ErrorKind),
}

impl From<Utf8Error> for UartHandlerError {
    fn from(err: Utf8Error) -> Self {
        Self::NonUTF8(err.error_len())
    }
}

// the trait `From<embedded_io_async::ReadExactError<<S as embedded_io_async::ErrorType>::Error>>`
// is not implemented for `uart_handler::error::UartHandlerError`

impl<E: embedded_io_async::Error> From<ReadExactError<E>> for UartHandlerError {
    fn from(value: ReadExactError<E>) -> Self {
        match value {
            ReadExactError::UnexpectedEof => Self::ConnectionAborted,
            ReadExactError::Other(e) => Self::Other(e.kind()),
        }
    }
}

impl From<CapacityError> for UartHandlerError {
    fn from(_: CapacityError) -> Self {
        Self::ResponseTooLarge
    }
}
