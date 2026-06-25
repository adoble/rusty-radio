use core::fmt;
use embedded_io_async::{Read, Write};
use heapless::Vec;
use log::*;

const NUMBER_STATIONS: usize = 7;

const STATIONS: [&str; NUMBER_STATIONS] = [
    "SWR3",
    "RPR1",
    "BBC1",
    "Radio Antenna",
    "Bayern Eins",
    "Classic",
    "Deutschland Funk",
];

// const PRESETS: [u8; 4] = [2, 4, 6, 1];
const CFG_MSG = b"ACK:7,2,4,6,1;";

//#[deprecated]
pub struct MockUart {
    state: State,
    frame_buf: heapless::Vec<u8, 80>,
    cfg_msg_index: usize, 
}

impl MockUart {
    pub fn new() -> Self {
        MockUart {
            state: State::Initial,
            frame_buf: Vec::new(),
            cfg_msg_index : 0, 
        }
    }

    fn parse_frame_buffer(&mut self) {
        debug!("Entered parse_frame_buffer");
        match &self.frame_buf[0..3] {
            b"STA" => {
                let station_id: &[u8] = &self.frame_buf[4..];
                let station_id: &str = core::str::from_utf8(station_id).unwrap();
                let station_id: usize = station_id.parse().unwrap();
                self.state = State::StationSelected(station_id)
            }

            b"CFG" => self.state = State::Config,

            b"DTN" => self.state = State::Detuned,
            _ => (),
        }
    }
}

impl Default for MockUart {
    fn default() -> Self {
        Self::new()
    }
}

impl embedded_io_async::Read for MockUart {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, MockUartError> {
        let mut len = 0;
        let convert = itoa::Buffer::new();
        debug!("embedded_io_async::Read");

        match self.state {
            State::Initial => panic!("Reading before sending a command"),
            State::Config => {
                debug!("State Config");
                let msg = b"ACK:7,2,4,6,1;";
                debug!("msg : {:?})", msg);
                buf[..msg.len()].copy_from_slice(msg); // Hardcoded for now 
                debug!("out buf : {:?})", buf);
                len = msg.len();
                self.state = State::Initial;
            }
            State::StationSelected(id) => {
                let command = b"ACK:";
                buf[0..4].copy_from_slice(command);
                let station_name_len = STATIONS[id].len();
                buf[4..4 + station_name_len].copy_from_slice(STATIONS[id].as_bytes());
                len = command.len() + station_name_len + 1;
                buf[len - 1] = b';';
                self.state = State::Initial;
            }
            State::Detuned => todo!(),
        }

        Ok(len)
    }
}

impl embedded_io_async::Write for MockUart {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        debug!("Write {:?}", buf);
        let mut len = 0;
        for (i, byte) in buf.iter().enumerate() {
            debug!("Write bye: {}", *byte);
            if *byte != b';' {
                if let Err(missed_byte) = self.frame_buf.push(*byte) {
                    debug!("Returning with Ok({})", i);
                    return Ok(i);
                }
                len += 1;
            } else {
                self.parse_frame_buffer();
                len += 1;
            }
        }

        debug!("Wrote {} bytes", len);

        Ok(len)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.frame_buf.clear();
        Ok(())
    }
}

enum State {
    Initial,
    Config,
    StationSelected(usize),
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
