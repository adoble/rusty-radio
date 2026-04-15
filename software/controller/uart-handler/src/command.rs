use heapless::String;

#[derive(PartialEq, Debug)]
pub enum Command {
    Station,
    Undefined,
}

impl Command {
    pub fn to_string(&self) -> String<3> {
        let mut command_string = String::<3>::new();

        let s = match self {
            Self::Station => "STA",
            Self::Undefined => "",
        };

        command_string.push_str(s).unwrap();
        command_string
    }
}
