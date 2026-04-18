#[derive(PartialEq, Debug)]
pub enum Command {
    Station,
    Preset,
    Config,
    Undefined,
}

// TODO this should be moved to the  radio_control_protocol module.
// Make stringify a trait or look into using deref
// impl Command {
//     pub fn stringify(&self) -> String<3> {
//         let mut command_string = String::<3>::new();

//         let s = match self {
//             Self::Station => "STA",
//             Self::Preset => "PRE",
//             Self::Config => "CFG",
//             Self::Undefined => "",
//         };

//         command_string.push_str(s).unwrap();
//         command_string
//     }
// }

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
