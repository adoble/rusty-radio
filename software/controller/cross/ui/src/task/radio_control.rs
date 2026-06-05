use esp_hal::{Async, uart::Uart};

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

//use embedded_io_async::{Read, Write}; // Import the Write trait

use log::*;
use radio_control_protocol::RadioControlProtocol;

use crate::station_config::RadioStationId;
use crate::task::sync::{TunerEvents, TunerEventsChannelSubscriber};

// TODO the message structure has been extended.Change to use this.

#[embassy_executor::task]
pub async fn radio_control(
    uart: &'static mut Mutex<CriticalSectionRawMutex, Uart<'static, Async>>,
    tuner_event_channel: &'static mut TunerEventsChannelSubscriber,
) {
    // TODO set up the serial interface
    // DO we need to do this here?

    // let uart = uart.lock().await;
    let uart = uart.get_mut();

    let mut radio_control_protocol = RadioControlProtocol::new(uart);

    loop {
        let event = tuner_event_channel.next_message_pure().await;
        debug!("tuner event {:?}", event);
        match event {
            TunerEvents::StationChanged(station_id) => {
                //DEBUG
                //read_something(&uart);
                station_changed(&mut radio_control_protocol, station_id).await
            }

            TunerEvents::TunerMoved(_) => {}
        }
    }
}

/// Signal the radio that it should either change the station or detune
async fn station_changed(
    radio_control_protocol: &mut RadioControlProtocol<'_, Uart<'static, Async>>,
    station_id: Option<RadioStationId>,
) {
    match station_id {
        Some(id) => {
            let result = radio_control_protocol
                .set_station(id.try_into().expect("station id exceeded u8 range"))
                .await;

            match result {
                Ok(station_name) => info!("Selected station {}", station_name),
                Err(e) => error!("Set station error {:?}", e),
            }
        }
        None => todo!(), // radio_control_protocol.detune(),
    }
}
