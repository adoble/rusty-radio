use embassy_net::Stack;
use static_cell::StaticCell;

use stations::{Station, StationError, Stations};

pub const MAX_STATION_NAME_LEN: usize = 40;
pub const MAX_STATION_URL_LEN: usize = 256;
pub const NUMBER_PRESETS: usize = 4;

pub type RadioStation = Station<MAX_STATION_NAME_LEN, MAX_STATION_URL_LEN>;
pub type RadioStations = Stations<MAX_STATION_NAME_LEN, MAX_STATION_URL_LEN, NUMBER_PRESETS>;

static RADIO_STATIONS: StaticCell<RadioStations> = StaticCell::new();

// pub async fn read_stations(
//     _stack: Stack<'static>,
//     _stations_url: &str,
// ) -> Result<&'static mut RadioStations, StationError> {
//     let stations_data = include_bytes!("../../../resources/rr-stations.txt");

//     let stations = RadioStations::load(stations_data).expect("ERROR: Cannot load stations");

//     Ok(RADIO_STATIONS.init(stations))
// }

pub async fn read_stations(
    _stack: Stack<'static>,
    _stations_url: &str,
) -> Result<&'static mut RadioStations, RadioStationError> {
    let stations_data = include_bytes!("../../../resources/rr-stations.txt");

    let stations =
        RadioStations::load(stations_data).map_err(RadioStationError::StationConstruction)?;

    Ok(RADIO_STATIONS.init(stations))
}

#[derive(Debug)]
pub enum RadioStationError {
    StationConstruction(StationError),
    Connection,
}
