/// Synchronisation between the different tasks.
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, pipe::Pipe, signal,
};

use crate::Vs1053DriverType;

// Signal that the web should be accessed
pub static ACCESS_WEB_SIGNAL: signal::Signal<CriticalSectionRawMutex, bool> = signal::Signal::new();

// Test channel
//pub static TEST_CHANNEL: Channel<CriticalSectionRawMutex, [u8; 32], 64> = Channel::new();

// Channel to stream internet radio content to the mp3 codec
// TODO adjust to type [u8;32] and adjust N accordingly
//pub static MUSIC_CHANNEL: Channel<CriticalSectionRawMutex, u8, 130000> = Channel::new();
// pub const MUSIC_CHANNEL_MESSAGE_LEN: usize = 64; // Previously 32;
// pub const MUSIC_CHANNEL_CAPACITY: usize = 2048;
// pub static MUSIC_CHANNEL: Channel<
//     CriticalSectionRawMutex,
//     [u8; MUSIC_CHANNEL_MESSAGE_LEN],
//     MUSIC_CHANNEL_CAPACITY,
// > = Channel::new();

// Pipe to stream internet radio content to the mp3 codec
pub const MUSIC_PIPE_LEN: usize = 130_000;
pub static MUSIC_PIPE: Pipe<CriticalSectionRawMutex, MUSIC_PIPE_LEN> = Pipe::new();

// Signals that the music can start playing
pub static START_PLAYING: signal::Signal<CriticalSectionRawMutex, bool> = signal::Signal::new();

// We need to share the VS1053 driver between tasks so put it in a static mutex
type CodecDriverType = Mutex<CriticalSectionRawMutex, Option<Vs1053DriverType<'static>>>;
pub static CODEC_DRIVER: CodecDriverType = Mutex::new(None);
