use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use esp_hal::{Async, uart::Uart};

// use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

//use embedded_io_async::{Read, Write}; // Import the Write trait

use log::*;
use radio_control_protocol::RadioControlProtocol;

use crate::task::station_config::{RadioStationId, STATION_CONFIG};
use crate::task::sync::{TunerEvents, TunerEventsChannelSubscriber};

// TODO the message structure has been extended.Change to use this.

#[embassy_executor::task]
pub async fn radio_control(
    uart: &'static Mutex<CriticalSectionRawMutex, esp_hal::uart::Uart<'static, Async>>,
    tuner_event_channel: &'static mut TunerEventsChannelSubscriber,
) {
    // TODO set up the serial interface
    // DO we need to do this here?

    // Only continue if the station config has been read.
    let station_config = STATION_CONFIG.wait().await;

    let mut uart = uart.lock().await;

    //let mut radio_control_protocol = RadioControlProtocol::new(&mut uart);

    loop {
        let event = tuner_event_channel.next_message_pure().await;
        debug!("tuner event {:?}", event);
        match event {
            TunerEvents::StationChanged(station_id) => {
                //DEBUG
                //read_something(&uart);
                station_changed(&mut uart, station_id).await
            }

            TunerEvents::TunerMoved(_) => {}
        }
    }
}

/// Signal the radio that it should either change the station or detune
async fn station_changed(
    // radio_control_protocol: &mut RadioControlProtocol<'_, Uart<'static, Async>>,
    uart: &mut Uart<'static, Async>,
    station_id: Option<RadioStationId>,
) {
    let mut radio_control_protocol = RadioControlProtocol::new(uart);

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
