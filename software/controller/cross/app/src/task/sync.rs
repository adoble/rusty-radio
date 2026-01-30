/// Synchronisation between the different tasks.
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::Channel,
    mutex::Mutex,
    pipe::Pipe,
    signal,
    watch::{Receiver, Watch},
};

use crate::task::radio_stations::RadioStation;
use crate::Vs1053DriverType;

use crate::sendable_multiplexer_driver::SendableMultiplexerDriver;

// Signal that the web should be accessed
//pub static ACCESS_WEB_SIGNAL: signal::Signal<CriticalSectionRawMutex, bool> = signal::Signal::new();

// Signal that the wifi has been connected and is operational
pub static WIFI_CONNECTED_SIGNAL: signal::Signal<CriticalSectionRawMutex, bool> =
    signal::Signal::new();

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

// Audio buffer size = 16,000 bytes/sec × 0.375 sec = 6,000 bytes (6 KB)
// The size of the pipe cannot be smaller than the size of the audio buffer used in
// stream task.
pub const AUDIO_BUFFER_SIZE: usize = 6_000;
// pub const AUDIO_BUFFER_SIZE: usize = 4000; // This still works if more space required

// Pipe to stream internet radio content to the mp3 codec
// pub const MUSIC_PIPE_LEN: usize = 130_000;   // Old value
// pub const MUSIC_PIPE_LEN: usize = 6_000;  // Reduced value still works
//pub const MUSIC_PIPE_LEN: usize = 4_000; //
pub static MUSIC_PIPE: Pipe<CriticalSectionRawMutex, AUDIO_BUFFER_SIZE> = Pipe::new();

// Signals that the music can start playing
pub static START_PLAYING: signal::Signal<CriticalSectionRawMutex, bool> = signal::Signal::new();

// We need to share the VS1053 driver between tasks so put it in a static mutex
type CodecDriverType = Mutex<CriticalSectionRawMutex, Option<Vs1053DriverType<'static>>>;
pub static CODEC_DRIVER: CodecDriverType = Mutex::new(None);

// We need to share the front panel driver between tasks so put it in a static mutex
// pub static MULTIPLEXER_DRIVER: Mutex<
//     CriticalSectionRawMutex,
//     Option<MultiplexerDriverType<'static>>,
// > = Mutex::new(None);

pub static MULTIPLEXER_DRIVER: Mutex<CriticalSectionRawMutex, Option<SendableMultiplexerDriver>> =
    Mutex::new(None);

// This signal is used to indicate the current station

//pub static STATION_SELECTED: signal::Signal<CriticalSectionRawMutex, Station> =
//    signal::Signal::new();

// This watches for changes to the station
const STATION_CHANGE_WATCHERS: usize = 2;
pub static STATION_CHANGE_WATCH: Watch<
    CriticalSectionRawMutex,
    Option<RadioStation>,
    STATION_CHANGE_WATCHERS,
> = Watch::new();

pub type StationChangeReceiver = Receiver<
    'static,
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    Option<RadioStation>,
    STATION_CHANGE_WATCHERS,
>;

// This channel transports commands to the display.
const UI_COMMAND_BUFFER_DEPTH: usize = 3;
pub static UI_COMMANDS_CHANNEL: Channel<
    CriticalSectionRawMutex,
    UiCommand,
    UI_COMMAND_BUFFER_DEPTH,
> = Channel::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiCommand {
    /// Indicates if the wifi is connected
    WiFiConnected(bool),

    /// The selected station id
    StationSelect(usize),

    /// If the user has turned the tuner knob
    TunerMoved(TunerDirection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TunerDirection {
    Clockwise,
    CounterClockwise,
}
