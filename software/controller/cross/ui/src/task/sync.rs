/// Synchronisation between the different tasks.
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Publisher, Subscriber},
};

use radio_control_protocol::{
    RadioControlProtocolError,
    station_config::{RadioStationId, RadioStationName},
};

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub enum UiCommand {
//     /// Indicates if the wifi is connected
//     WiFiConnected(bool),

//     /// The selected station id
//     StationSelect(usize),

//     /// If the user has turned the tuner knob
//     TunerMoved(TunerDirection),
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TunerDirection {
    Clockwise,
    CounterClockwise,
}

const TUNER_EVENTS_CHANNEL_CAP: usize = 8;
const TUNER_EVENTS_CHANNEL_SUBS: usize = 2;
const TUNER_EVENTS_CHANNEL_PUBS: usize = 2;

pub static TUNER_EVENTS_CHANNEL: PubSubChannel<
    CriticalSectionRawMutex,
    TunerEvents,
    TUNER_EVENTS_CHANNEL_CAP,
    TUNER_EVENTS_CHANNEL_SUBS,
    TUNER_EVENTS_CHANNEL_PUBS,
> = PubSubChannel::new();

pub type TunerEventsChannelPublisher = Publisher<
    'static,
    CriticalSectionRawMutex,
    TunerEvents,
    TUNER_EVENTS_CHANNEL_CAP,
    TUNER_EVENTS_CHANNEL_SUBS,
    TUNER_EVENTS_CHANNEL_PUBS,
>;
pub type TunerEventsChannelSubscriber = Subscriber<
    'static,
    CriticalSectionRawMutex,
    TunerEvents,
    TUNER_EVENTS_CHANNEL_CAP,
    TUNER_EVENTS_CHANNEL_SUBS,
    TUNER_EVENTS_CHANNEL_PUBS,
>;

#[derive(Clone, Debug)]
pub enum TunerEvents {
    /// The station has been changed to either a station or is in between stations.
    StationChanged(Option<RadioStationId>),

    /// The tuner knob  has been moved
    TunerMoved(TunerDirection),

    /// A station has been selected and the name obtained
    StationSelected(RadioStationName),

    /// Detune between stations
    Detune,

    /// An error has been detected
    Error(RadioControlProtocolError),
}
