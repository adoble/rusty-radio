use log::*;

use crate::task::sync::TunerEvents;

use radio_control_protocol::{
    RadioControlProtocol, RadioControlProtocolError, StationConfig,
    station_config::{RadioStationId, RadioStationName},
};

use crate::task::sync::TunerEventsChannelSubscriber;

#[embassy_executor::task]
pub async fn display(
    station_config: &'static StationConfig,
    // tuner_event_subscriber: &'static mut TunerEventsChannelSubscriber,
    mut tuner_event_subscriber: TunerEventsChannelSubscriber,
) {
    loop {
        let event = tuner_event_subscriber.next_message_pure().await;
        match event {
            TunerEvents::TunerMoved(direction) => debug!("Tuner moved {:?}", direction),
            TunerEvents::StationSelected(name) => info!("Station selected: {}", name),
            TunerEvents::Detune => debug!("Detune"),
            TunerEvents::Error(error) => error!("{:?}", error),

            _ => (),
        }
    }
}
