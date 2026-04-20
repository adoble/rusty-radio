#[derive(PartialEq, Debug)]
pub enum Command {
    Station,
    Preset,
    Config,
    Undefined,
}

impl From<&Command> for [u8; 3] {
    fn from(cmd: &Command) -> [u8; 3] {
        match cmd {
            Command::Station => *b"STA",
            Command::Preset => *b"PRE",
            Command::Config => *b"CFG",
            Command::Undefined => *b"UND",
        }
    }
}
