use esp_hal::{Async, uart::Uart};

// use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

//use embedded_io_async::{Read, Write}; // Import the Write trait

use log::*;
use radio_control_protocol::{
    RadioControlProtocol, RadioControlProtocolError, StationConfig, station_config,
};
use station_config::RadioStationId;

use crate::task::sync::{TunerEvents, TunerEventsChannelSubscriber};

/// Process the events from the tuner
#[embassy_executor::task]
pub async fn radio_control(
    uart: &'static mut Uart<'static, Async>,
    station_config: &'static StationConfig,
    tuner_event_channel: &'static mut TunerEventsChannelSubscriber,
) {
    loop {
        let event = tuner_event_channel.next_message_pure().await;

        if let TunerEvents::StationChanged(station_id) = event {
            station_changed(uart, station_id).await
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

/// Load the station config from the radio
pub async fn load_station_config(
    uart: &mut Uart<'static, Async>,
) -> Result<StationConfig, RadioControlProtocolError> {
    let station_config = match load_station_config_from_radio(uart).await {
        Ok(station_config) => station_config,
        Err(e) => {
            error!("Could not load station from radio, error:{:?}", e);
            return Err(RadioControlProtocolError::CannotLoadStationConfig);
        }
    };

    Ok(station_config)
}

async fn load_station_config_from_radio(
    uart: &mut Uart<'static, Async>,
) -> Result<StationConfig, RadioControlProtocolError> {
    let mut radio_control_protocol = RadioControlProtocol::new(uart);

    //    pub async fn query_config(&mut self) -> Result<usize, RadioControlProtocolError>
    let station_config = radio_control_protocol.query_config().await?;

    Ok(station_config)
}
