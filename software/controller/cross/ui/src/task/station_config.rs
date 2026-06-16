use embassy_sync::signal::Signal;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use esp_hal::{Async, uart::Uart};

use radio_control_protocol::{
    RadioControlProtocol, StationConfig, radio_control_protocol::RadioControlProtocolError,
};

use log::error;

const NUMBER_PRESETS: usize = 4;

pub type RadioStationId = usize;

/// This signal is set once the config  has been loaded
// TODO Usually the synchronisation primitive have been set in the task::sync. Setting it
// close to the actual structure is a departure from this style. Is this a good idea?
pub static STATION_CONFIG: Signal<CriticalSectionRawMutex, StationConfig> = Signal::new();

// use core::ops::DerefMut;

/// Load the station config from the radio
#[embassy_executor::task]
pub async fn load_station_config(
    uart: &'static Mutex<CriticalSectionRawMutex, esp_hal::uart::Uart<'static, Async>>,
) {
    let mut uart = uart.lock().await;

    if let Ok(station_config) = load_station_config_from_radio(&mut uart).await {
        STATION_CONFIG.signal(station_config);
    } else {
        error!("Could not load station from radio")
    };
}

async fn load_station_config_from_radio(
    uart: &mut Uart<'static, Async>,
) -> Result<StationConfig, RadioControlProtocolError> {
    let mut radio_control_protocol = RadioControlProtocol::new(uart);

    //    pub async fn query_config(&mut self) -> Result<usize, RadioControlProtocolError>
    let station_config = radio_control_protocol.query_config().await?;

    Ok(station_config)
}

enum Error {} //TODO
