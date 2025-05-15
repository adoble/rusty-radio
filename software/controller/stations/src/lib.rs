#![cfg_attr(not(test), no_std)]

use heapless::{String, Vec};
use nourl::{Error, Url};

// TODO All the hard coded stations have to be made variable.
// NOTE: This station does a number of redirects by setting the response header "location". Note that it does
// not give a return code 3xx which is strange.
// Anaylsed with Google HAR analyser https://toolbox.googleapps.com/apps/har_analyzer/
// For a description of the location field see: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Location
const STATION_URL: &str = "http://liveradio.swr.de/sw282p3/swr3/play.mp3";

// NOTE: This station doesn't seem to have redirects (as of now) so used to test the basic functionality
//const STATION_URL: &str = "http://listen.181fm.com/181-classical_128k.mp3";

// Local server for testing
//const STATION_URL: &str = "http://192.168.2.107:8080/music/2"; // Hijo de la Luna. 128 kb/s

const STATIONS: &[(&str, &str)] = &[
    ("SWR1", "https://liveradio.swr.de/sw282p3/swr1rp/"),
    ("SWR4", "https://liveradio.swr.de/sw282p3/swr4bw/"),
    (
        "181 FM Classic",
        "http://listen.181fm.com/181-classical_128k.mp3",
    ),
    (
        "Absolut Oldie Classics",
        "https://absolut-oldieclassics.live-sm.absolutradio.de/absolut-oldieclassics/stream/mp3",
    ),
];
const MAX_URL_LEN: usize = 512;
const MAX_NUMBER_STATIONS: usize = 256;
const MAX_STATION_NAME_LEN: usize = 24;
pub struct Station<'a> {
    name: String<MAX_STATION_NAME_LEN>,
    url: Url<'a>,
}

impl<'a> Station<'a> {
    // Purposely private as creation  is done in Stations
    fn new(station_name: &'a str, station_url: &'a str) -> Result<Self, StationError> {
        let mut name = String::new();
        name.push_str(station_name)
            .map_err(|_| StationError::NameTooLong)?;

        let url = Url::parse(station_url).map_err(|_| StationError::UrlIncorrect)?;

        Ok(Station { name, url })
    }

    /// Get the URL of the station
    pub fn url(&self) -> Url {
        self.url
    }

    /// Get the name of the station
    pub fn name(&self) -> String<MAX_STATION_NAME_LEN> {
        self.name.clone()
    }
}

pub struct Stations<'a>(Vec<Station<'a>, MAX_NUMBER_STATIONS>);

impl<'a> Stations<'a> {
    pub fn new() -> Stations<'a> {
        Stations(Vec::new())
    }

    /// Load up the stations
    /// TODO read the stations from another source
    pub fn load_stations(&mut self) {
        STATIONS
            .iter()
            .for_each(|s| self.add_station(s.0, s.1).unwrap());
    }

    pub fn iter(&self) -> impl Iterator<Item = &Station> {
        self.0.iter()
    }

    pub fn add_station(&mut self, name: &'a str, url: &'a str) -> Result<(), StationError> {
        let station = Station::new(name, url)?;

        self.0
            .push(station)
            .map_err(|_| StationError::TooManyStations)?;

        Ok(())
    }

    pub fn number_stations(&self) -> usize {
        self.0.len()
    }

    pub fn get_station(&self, index: usize) -> Option<&Station> {
        self.0.get(index)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StationError {
    UrlIncorrect,
    UrlTooLong,
    NameTooLong,
    TooManyStations,
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), StationError>;

    #[test]
    fn new_station() {
        let station_url_str = "http://liveradio.swr.de/sw282p3/swr3/play.mp3";
        let result = Station::new("SWR3", station_url_str);
        assert!(result.is_ok());

        let station = result.unwrap();

        assert_eq!(station.name, "SWR3");
        assert_eq!(station.url.host(), "liveradio.swr.de");

        let very_long_station_name =
            "A very long station name that no one would really use in real life (unless it was some kind of gimmic)";

        assert!(very_long_station_name.len() > MAX_STATION_NAME_LEN);

        assert!(Station::new(very_long_station_name, station_url_str).is_err());
    }

    #[test]
    fn add_and_get_station() -> TestResult {
        let mut stations = Stations::new();

        stations.add_station("SWR3", "http://liveradio.swr.de/sw282p3/swr3/play.mp3")?;

        stations.add_station("Home", "http://home.io/home2/play.mp3")?;

        stations.add_station("Test", "http://cricket.co.uk/matches/stream.mp3")?;

        assert!(stations.number_stations() == 3);

        assert_eq!(stations.get_station(1).unwrap().name, "Home");

        Ok(())
    }

    #[test]
    fn iterate() -> TestResult {
        let mut stations = Stations::new();

        stations.add_station("SWR3", "http://liveradio.swr.de/sw282p3/swr3/play.mp3")?;
        stations.add_station("Home", "http://home.io/home2/play.mp3")?;
        stations.add_station("Test", "http://cricket.co.uk/matches/stream.mp3")?;

        let results: std::vec::Vec<std::string::String> =
            stations.iter().map(|s| s.name.to_string()).collect();

        assert!(results.contains(&"SWR3".to_string()));
        assert!(results.contains(&"Home".to_string()));
        assert!(results.contains(&"Test".to_string()));

        Ok(())
    }
}
