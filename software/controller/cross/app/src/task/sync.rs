use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex},
    channel::Channel,
    mutex::Mutex,
    signal,
};

use crate::Vs1053DriverType;

/// Synchronisation between the different tasks.
///

// Signal that the web should be accessed
pub static ACCESS_WEB_SIGNAL: signal::Signal<CriticalSectionRawMutex, bool> = signal::Signal::new();

// Test channel
pub static TEST_CHANNEL: Channel<CriticalSectionRawMutex, [u8; 32], 64> = Channel::new();

// Channel to stream internet radio content to the mp3 codec
pub static MUSIC_CHANNEL: Channel<CriticalSectionRawMutex, u8, 130000> = Channel::new();

// We need to share the VS1053 driver between tasks so put it in a static mutex
type CodecDriverType = Mutex<CriticalSectionRawMutex, Option<Vs1053DriverType<'static>>>;
pub static CODEC_DRIVER: CodecDriverType = Mutex::new(None);
