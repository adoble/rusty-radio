use radio_control_protocol::{RadioControlProtocolError, RadioStationId};

// TODO this belongs to radio_control_protocol

/// Error constants
const NOSTATION: u8 = 101;
const TOOLONG: u8 = 102;
const BADCMD: u8 = 103;
const UNSUPPORTED: u8 = 104;

#[derive(Debug)]
pub enum RadioError {
    StationNotFound(RadioStationId),
    ResponseTooLong,
    Receive(RadioControlProtocolError),
    UnsupportedCommand,
}

impl RadioErrorX {
    /// Short fixed codes, so building the error response can't realistically overflow.
    fn code(&self) -> u8 {
        match self {
            RadioError::StationNotFound(_) => NOSTATION,
            RadioError::ResponseTooLong => TOOLONG,
            RadioError::Receive(_) => BADCMD,
            RadioError::UnsupportedCommand => UNSUPPORTED,
        }
    }

    // fn to_response(&self) -> Response {
    //     let mut response = Response::new();
    //     if write!(response, "ERR:{};", self.code()).is_err() {
    //         response.clear();
    //         let _ = response.push_str("ERR;"); // last-resort fallback
    //     }
    //     response
    // }
}

impl From<RadioControlProtocolError> for RadioError {
    fn from(e: RadioControlProtocolError) -> Self {
        RadioError::Receive(e)
    }
}

impl From<core::fmt::Error> for RadioError {
    fn from(_: core::fmt::Error) -> Self {
        RadioError::ResponseTooLong
    }
}
