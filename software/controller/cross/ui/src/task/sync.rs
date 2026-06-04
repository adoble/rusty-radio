/// Synchronisation between the different tasks.
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Publisher, Subscriber},
};

use crate::station_config::RadioStationId;

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

const TUNER_EVENTS_CHANNEL_CAP: usize = 8;
const TUNER_EVENTS_CHANNEL_SUBS: usize = 2;
const TUNER_EVENTS_CHANNEL_PUBS: usize = 1;

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

#[derive(Clone, Copy, Debug)]
pub enum TunerEvents {
    /// The station has been changed to either a station or is in between stations.
    StationChanged(Option<RadioStationId>),

    /// The tuner know has been moved
    TunerMoved(TunerDirection),
}
