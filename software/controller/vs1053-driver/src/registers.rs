#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum Registers {
    Vs1053RegMode = 0x00,       // Mode control
    Vs1053RegStatus = 0x01,     // Status of VS1053b
    Vs1053RegBass = 0x02,       // Built-in bass/treble control
    Vs1053RegClockf = 0x03,     // Clock frequency + multiplier
    Vs1053RegDecodetime = 0x04, // Decode time in seconds
    Vs1053RegAudata = 0x05,     // Misc. audio data
    Vs1053RegWram = 0x06,       // RAM write/read
    Vs1053RegWramaddr = 0x07,   // Base address for RAM write/read
    Vs1053RegHdat0 = 0x08,      // Stream header data 0
    Vs1053RegHdat1 = 0x09,      // Stream header data 1
    Vs1053RegVolume = 0x0B,     // Volume control
    Vs1053GpioDdr = 0xC017,     // Direction
    Vs1053GpioIdata = 0xC018,   // Values read from pins
    Vs1053GpioOdata = 0xC019,   // Values set to the pins
    Vs1053IntEnable = 0xC01A,   // Interrupt enable
}
// TODO No idea yet where these are used?
//
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum OtherConstants {
    Vs1053ModeSmDiff = 0x0001, // Differential, 0: normal in-phase audio, 1: left channel inverted
    Vs1053ModeSmLayer12 = 0x0002, // Allow MPEG layers I & II
    Vs1053ModeSmReset = 0x0004, // Soft reset
    Vs1053ModeSmCancel = 0x0008, // Cancel decoding current file
    Vs1053ModeSmEarspklo = 0x0010, // EarSpeaker low setting
    Vs1053ModeSmTests = 0x0020, // Allow SDI tests
    Vs1053ModeSmStream = 0x0040, // Stream mode
    Vs1053ModeSmSdinew = 0x0800, // VS1002 native SPI modes
    Vs1053ModeSmAdpcm = 0x1000, // PCM/ADPCM recording active
    Vs1053ModeSmLine1 = 0x4000, // MIC/LINE1 selector, 0: MICP, 1: LINE1
    Vs1053ModeSmClkrange = 0x8000, // Input clock range, 0: 12..13 MHz, 1: 24..26 MHz
    Vs1053SciAiaddr = 0x0A, // Indicates the start address of the application code written earlier ,  // with SCI_WRAMADDR and SCI_WRAM registers.
    Vs1053SciAictrl0 = 0x0C, // SCI_AICTRL register 0. Used to access the user's application program
    Vs1053SciAictrl1 = 0x0D, // SCI_AICTRL register 1. Used to access the user's application program
    Vs1053SciAictrl2 = 0x0E, // SCI_AICTRL register 2. Used to access the user's application program
    Vs1053SciAictrl3 = 0x0F, // SCI_AICTRL register 3. Used to access the user's application program
    Vs1053SciWram = 0x06,   // RAM write/read
    Vs1053SciWramaddr = 0x07, // Base address for RAM write/read
    Vs1053ParaPlayspeed = 0x1E04, // 0,1 = normal speed, 2 = 2x, 3 = 3x etc
}
