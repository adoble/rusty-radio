use log::*;

use crate::task::sync::TunerEvent;

use crate::task::sync::TunerEventsChannelSubscriber;

#[embassy_executor::task]
pub async fn display(
    // tuner_event_subscriber: &'static mut TunerEventsChannelSubscriber,
    mut tuner_event_subscriber: TunerEventsChannelSubscriber,
) {
    loop {
        let event = tuner_event_subscriber.next_message_pure().await;
        match event {
            TunerEvent::TunerMoved(direction) => debug!("Tuner moved {:?}", direction),
            TunerEvent::StationSelected(name) => info!("Station selected: {}", name),
            TunerEvent::Detune => debug!("Detune"),
            TunerEvent::Error(error) => error!("{:?}", error),

            _ => (),
        }
    }
}
