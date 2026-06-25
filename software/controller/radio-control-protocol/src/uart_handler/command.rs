use crate::UartHandlerError;

#[derive(PartialEq, Debug)]
pub enum Command {
    Station,
    Preset,
    Config,
    Detune,

    Undefined,
}

impl From<&Command> for [u8; 3] {
    fn from(cmd: &Command) -> [u8; 3] {
        match cmd {
            Command::Station => *b"STA",
            Command::Preset => *b"PRE",
            Command::Config => *b"CFG",
            Command::Detune => *b"DTN",
            Command::Undefined => *b"   ",
        }
    }
}

impl TryFrom<&[u8; 3]> for Command {
    type Error = UartHandlerError;

    fn try_from(bytes: &[u8; 3]) -> Result<Self, Self::Error> {
        match bytes {
            b"STA" => Ok(Command::Station),
            b"PRE" => Ok(Command::Preset),
            b"CFG" => Ok(Command::Config),
            b"DTN" => Ok(Command::Detune),
            _ => Err(UartHandlerError::UnknownCommand),
        }
    }
}
