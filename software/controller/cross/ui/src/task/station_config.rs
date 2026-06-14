use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
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

/// Load the station config from the radio
#[embassy_executor::task]
pub async fn load_station_config(
    mut uart: Uart<'static, Async>,
    load_signal: &'static Signal<CriticalSectionRawMutex, StationConfig>,
) {
    if let Ok(station_config) = load_station_config_from_radio(uart).await {
        load_signal.signal(station_config);
    } else {
        error!("Could not load station from radio")
    };
}

async fn load_station_config_from_radio(
    mut uart: Uart<'static, Async>,
) -> Result<StationConfig, RadioControlProtocolError> {
    let mut radio_control_protocol = RadioControlProtocol::new(&mut uart);

    //    pub async fn query_config(&mut self) -> Result<usize, RadioControlProtocolError>
    let station_config = radio_control_protocol.query_config().await?;

    Ok(station_config)
}

enum Error {} //TODO
