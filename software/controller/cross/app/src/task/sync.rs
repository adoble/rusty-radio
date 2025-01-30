use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex},
    channel::Channel,
    mutex::Mutex,
    signal,
};

/// Synchronisation between the different tasks.
///

// Signal that the web should be accessed
pub static ACCESS_WEB_SIGNAL: signal::Signal<CriticalSectionRawMutex, bool> = signal::Signal::new();

// Test channel
pub static TEST_CHANNEL: Channel<CriticalSectionRawMutex, [u8; 32], 64> = Channel::new();
