use embassy_net::Stack;
use static_cell::StaticCell;
use stations::StationError;

use crate::RadioStations;

static RADIO_STATIONS: StaticCell<RadioStations> = StaticCell::new();

pub async fn read_stations(
    _stack: Stack<'static>,
    _stations_url: &str,
) -> Result<&'static mut RadioStations, StationError> {
    let stations_data = include_bytes!("../../../resources/rr-stations.txt");

    let stations = RadioStations::load(stations_data).expect("ERROR: Cannot load stations");

    Ok(RADIO_STATIONS.init(stations))
}
