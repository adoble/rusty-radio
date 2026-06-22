use esp_hal::{Async, DriverMode};

use core::fmt;
use embedded_io_async::{Read, Write};
use heapless::Vec;

const NUMBER_STATIONS: usize = 7;

const STATIONS: [&str; NUMBER_STATIONS] = [
    "SWR3",
    "RPR1",
    "BBC1",
    "Radio Antenna",
    "Bayern  Eins",
    "Classic",
    "Deutschland Funk",
];

//#[deprecated]
pub struct MockUart {
    state: State,
    frame_buf: heapless::Vec<u8, 80>,
}

impl MockUart {
    pub fn new() -> Self {
        MockUart {
            state: State::Initial,
            frame_buf: Vec::new(),
        }
    }
}

impl embedded_io_async::Read for MockUart {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, MockUartError> {
        todo!()
    }
}

impl embedded_io_async::Write for MockUart {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        todo!()
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        todo!()
    }
}

enum State {
    Initial,
    Config,
    StationSelected,
    Detuned,
}

#[derive(Debug)]
pub enum MockUartError {
    Utf8Error,
    BufferFull,
}

impl fmt::Display for MockUartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MockUartError::Utf8Error => write!(f, "UTF-8 decode error"),
            MockUartError::BufferFull => write!(f, "response buffer full"),
        }
    }
}

impl embedded_io::Error for MockUartError {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::InvalidData
    }
}

impl embedded_io::ErrorType for MockUart {
    type Error = MockUartError;
}

impl core::error::Error for MockUartError {}
