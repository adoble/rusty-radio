use core::ops::BitOr;

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum Register {
    Mode = 0x00,        // Mode control
    Status = 0x01,      // Status of VS1053b
    Bass = 0x02,        // Built-in bass/treble control
    Clockf = 0x03,      // Clock frequency + multiplier
    Decodetime = 0x04,  // Decode time in seconds
    AudioData = 0x05,   // Misc. audio data
    Wram = 0x06,        // RAM write/read
    Wramaddr = 0x07,    // Base address for RAM write/read
    Hdat0 = 0x08,       // Stream header data 0
    Hdat1 = 0x09,       // Stream header data 1
    AaiAddr = 0x0A,     // Start address of appplicaion
    Volume = 0x0B,      // Volume control
    AppControl0 = 0x0C, // Application control 0
    AppControl1 = 0x0D, // Application control 1
    AppControl2 = 0x0E, // Application control 2
    AppControl3 = 0x0F, // Application control 4
    GpioDdr = 0xC017,   // Direction
    GpioIdata = 0xC018, // Values read from pins
    GpioOdata = 0xC019, // Values set to the pins
    IntEnable = 0xC01A, // Interrupt enable
}

impl From<Register> for u8 {
    fn from(value: Register) -> Self {
        value as u8
    }
}

/// Mode constants
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum Mode {
    Diff = 0x0001,     // Differential, 0: normal in-phase audio, 1: left channel inverted
    Layer12 = 0x0002,  // Allow MPEG layers I & II
    Reset = 0x0004,    // Soft reset
    Cancel = 0x0008,   // Cancel decoding current file
    Earspklo = 0x0010, // EarSpeaker low setting
    Tests = 0x0020,    // Allow SDI tests
    Stream = 0x0040,   // Stream mode
    SdiNew = 0x0800,   // VS1002 native SPI modes
    Adpcm = 0x1000,    // PCM/ADPCM recording active
    Line1 = 0x4000,    // MIC/LINE1 selector, 0: MICP, 1: LINE1
    Clkrange = 0x8000, // Input clock range, 0: 12..13 MHz, 1: 24..26 MHz
}

impl From<Mode> for u16 {
    fn from(value: Mode) -> Self {
        value as u16
    }
}

impl BitOr for Mode {
    type Output = u16;

    fn bitor(self, rhs: Self) -> Self::Output {
        self as u16 | rhs as u16
    }
}

// TODO No idea yet where these are used?
//
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum OtherConstants {
    Vs1053SciAiaddr = 0x0A, // Indicates the start address of the application code written earlier ,  // with SCI_WRAMADDR and SCI_WRAM registers.
    Vs1053SciAictrl0 = 0x0C, // SCI_AICTRL register 0. Used to access the user's application program
    Vs1053SciAictrl1 = 0x0D, // SCI_AICTRL register 1. Used to access the user's application program
    Vs1053SciAictrl2 = 0x0E, // SCI_AICTRL register 2. Used to access the user's application program
    Vs1053SciAictrl3 = 0x0F, // SCI_AICTRL register 3. Used to access the user's application program
    Vs1053SciWram = 0x06,   // RAM write/read
    Vs1053SciWramaddr = 0x07, // Base address for RAM write/read
    Vs1053ParaPlayspeed = 0x1E04, // 0,1 = normal speed, 2 = 2x, 3 = 3x etc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_conversion_test() {
        let val: u8 = Register::Status.into();
        assert_eq!(val, 0x01);
    }

    #[test]
    fn simple_or_test() {
        let v = Mode::Diff | Mode::SdiNew;

        assert_eq!(v, 0x0801);
        // TODO this doe not work. Maybe start using bitflags crate if this
        // chaining is required
        // let v = Mode::Diff | Mode::SdiNew | Mode::Stream;
        // assert_eq!(v, 0x0841);
    }
}
