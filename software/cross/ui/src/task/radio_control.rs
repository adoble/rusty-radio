use esp_hal::{Async, uart::Uart};

// use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

//use embedded_io_async::{Read, Write}; // Import the Write trait

use log::*;
use radio_control_protocol::{
    RadioControlProtocol, RadioControlProtocolError, RadioStationId, RadioStationName,
    StationConfig,
};

use crate::task::sync::{TunerEvents, TunerEventsChannelPublisher, TunerEventsChannelSubscriber};

/// Process the events from the tuner
#[embassy_executor::task]
pub async fn radio_control(
    uart: &'static mut Uart<'static, Async>,
    tuner_event_subscriber: TunerEventsChannelSubscriber,
    tuner_event_publisher: TunerEventsChannelPublisher,
) {
    run_radio_control(uart, tuner_event_subscriber, tuner_event_publisher).await;
}

pub async fn run_radio_control<U>(
    uart: &mut U,
    mut tuner_event_subscriber: TunerEventsChannelSubscriber,
    tuner_event_publisher: TunerEventsChannelPublisher,
) where
    U: embedded_io_async::Read + embedded_io_async::Write + 'static,
{
    loop {
        let event = tuner_event_subscriber.next_message_pure().await;

        if let TunerEvents::StationChanged(station_id) = event {
            match station_changed(uart, station_id).await {
                Ok(station_name) => match station_name {
                    Some(station_name) => {
                        tuner_event_publisher
                            .publish(TunerEvents::StationSelected(station_name))
                            .await
                    }
                    None => tuner_event_publisher.publish(TunerEvents::Detune).await,
                },
                Err(e) => tuner_event_publisher.publish(TunerEvents::Error(e)).await,
            };
        }
    }
}

/// Signal the radio that it should either change the station or detune
async fn station_changed<U>(
    // radio_control_protocol: &mut RadioControlProtocol<'_, Uart<'static, Async>>,
    // uart: &mut Uart<'static, Async>,
    uart: &mut U,
    station_id: Option<RadioStationId>,
) -> Result<Option<RadioStationName>, RadioControlProtocolError>
where
    U: embedded_io_async::Read + embedded_io_async::Write + 'static,
{
    let mut radio_control_protocol = RadioControlProtocol::new(uart);

    let station_name = match station_id {
        Some(id) => {
            let station_name = radio_control_protocol
                .set_station(id.try_into().expect("Station id exceeded range"))
                .await?;
            Some(station_name)
        }
        None => {
            radio_control_protocol
                .detune()
                .await
                .expect("Could not detune radio");
            None
        } // radio_control_protocol.detune(),
    };
    Ok(station_name)
}

/// Load the station config from the radio
pub async fn load_station_config<U>(
    //uart: &mut Uart<'static, Async>,
    uart: &mut U,
) -> Result<StationConfig, RadioControlProtocolError>
where
    U: embedded_io_async::Read + embedded_io_async::Write + 'static,
{
    #[cfg(feature = "solo")]
    return Ok(debug_station_config());

    let station_config = match load_station_config_from_radio(uart).await {
        Ok(station_config) => station_config,
        Err(e) => {
            error!("Could not load station from radio, error:{:?}", e);
            return Err(RadioControlProtocolError::CannotLoadStationConfig);
        }
    };

    Ok(station_config)
}

async fn load_station_config_from_radio<U>(
    //uart: &mut Uart<'static, Async>,
    uart: &mut U,
) -> Result<StationConfig, RadioControlProtocolError>
where
    U: embedded_io_async::Read + embedded_io_async::Write + 'static,
{
    let mut radio_control_protocol = RadioControlProtocol::new(uart);

    //    pub async fn query_config(&mut self) -> Result<usize, RadioControlProtocolError>
    let station_config = radio_control_protocol.query_config().await?;

    Ok(station_config)
}

#[allow(dead_code)]
fn debug_station_config() -> StationConfig {
    StationConfig::new()
        .with_number_stations(10)
        .with_presets([2, 4, 6, 8])
}
