/// Synchronisation between the different tasks.
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::Channel,
    watch::{Receiver, Watch},
};

use crate::RadioStationId;

// This watches for changes to the station
const STATION_CHANGE_WATCHERS: usize = 2;
pub static STATION_CHANGE_WATCH: Watch<
    CriticalSectionRawMutex,
    Option<RadioStationId>,
    STATION_CHANGE_WATCHERS,
> = Watch::new();

pub type StationChangeReceiver = Receiver<
    'static,
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    Option<RadioStationId>,
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

const RADIO_COMMANDS_CHANNEL_DEPTH: usize = 3;
/// This channel transports commands to the radio control task
static RADIO_COMMANDS_CHANNEL: Channel<
    CriticalSectionRawMutex,
    RadioCommand,
    RADIO_COMMANDS_CHANNEL_DEPTH,
> = Channel::new();

pub enum RadioCommand {
    /// Query the configuration status
    Config,
    /// Select the station
    Station(RadioStationId),
}
