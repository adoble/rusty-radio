#[derive(Debug, PartialEq, Clone, Copy)]
pub struct DumpRegisters {
    pub mode: u16,
    pub status: u16,
    pub clock_f: u16,
    pub volume: u16,
    pub audio_data: u16,
}
