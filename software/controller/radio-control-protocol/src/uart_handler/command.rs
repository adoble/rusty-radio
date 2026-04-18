use heapless::String;

#[derive(PartialEq, Debug)]
pub enum Command {
    Station,
    Preset,
    Config,
    Undefined,
}

// TOOD this shoudl be moved to the  radio_control_protocol module.
// Make stringify a trait or look into using deref
impl Command {
    pub fn stringify(&self) -> String<3> {
        let mut command_string = String::<3>::new();

        let s = match self {
            Self::Station => "STA",
            Self::Preset => "PRE",
            Self::Config => "CFG",
            Self::Undefined => "",
        };

        command_string.push_str(s).unwrap();
        command_string
    }
}
