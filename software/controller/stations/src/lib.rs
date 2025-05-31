#![cfg_attr(not(test), no_std)]

// TODO All the hard coded stations have to be made variable.
// NOTE: This station does a number of redirects by setting the response header "location". Note that it does
// not give a return code 3xx which is strange.
// Analysed with Google HAR analyser https://toolbox.googleapps.com/apps/har_analyzer/
// For a description of the location field see: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Location
//const STATION_URL: &str = "http://liveradio.swr.de/sw282p3/swr3/play.mp3";

// NOTE: This station doesn't seem to have redirects (as of now) so used to test the basic functionality
//const STATION_URL: &str = "http://listen.181fm.com/181-classical_128k.mp3";

// Local server for testing
//const STATION_URL: &str = "http://192.168.2.107:8080/music/2"; // Hijo de la Luna. 128 kb/s

// const STATIONS: &[(&str, &str)] = &[
//     ("SWR3", "http://liveradio.swr.de/sw282p3/swr3/play.mp3"),
//     ("SWR4", "https://liveradio.swr.de/sw282p3/swr4bw/"),
//     (
//         "181 FM Classic",
//         "http://listen.181fm.com/181-classical_128k.mp3",
//     ),
//     (
//         "Absolut Oldie Classics",
//         "https://absolut-oldieclassics.live-sm.absolutradio.de/absolut-oldieclassics/stream/mp3",
//     ),
// ];

static STATION_DATA: &[(&str, &str)] = &[
    ("SWR3", "http://liveradio.swr.de/sw282p3/swr3/play.mp3"),
    ("SWR4", "http://liveradio.swr.de/sw282p3/swr4bw/"),
    (
        "181 FM Classic",
        "http://listen.181fm.com/181-classical_128k.mp3",
    ),
    (
        "Absolut Oldie Classics",
        "http://absolut-oldieclassics.live-sm.absolutradio.de/absolut-oldieclassics/stream/mp3",
    ),
];

#[derive(Clone, PartialEq)]
pub struct Station {
    index: usize, // Index into STATION_DATA
}

impl Station {
    pub fn new(index: usize) -> Option<Self> {
        if index < STATION_DATA.len() {
            Some(Station { index })
        } else {
            None
        }
    }

    pub fn name(&self) -> &'static str {
        STATION_DATA[self.index].0
    }

    pub fn url(&self) -> &'static str {
        STATION_DATA[self.index].1
    }
}

pub struct Stations {
    current_station: usize,
}

impl Stations {
    pub fn new() -> Stations {
        Stations { current_station: 0 }
    }

    pub fn number_stations(&self) -> usize {
        STATION_DATA.len()
    }

    pub fn get_station(&mut self, index: usize) -> Option<Station> {
        self.current_station = index;
        Station::new(index)
    }

    pub fn reset(&mut self) {
        self.current_station = 0;
    }
}

impl Iterator for Stations {
    type Item = Station;

    fn next(&mut self) -> Option<Station> {
        let station = Station::new(self.current_station);
        self.current_station += 1;
        station
    }
}
// #[derive(Debug, Clone, Copy)]
// pub enum StationError {
//     UrlIncorrect,
//     UrlTooLong,
//     NameTooLong,
//     TooManyStations,
// }

#[cfg(test)]
mod tests {
    use super::*;

    //type TestResult = Result<(), StationError>;

    #[test]
    fn get_station() {
        let stations = Stations::new();

        assert!(stations.number_stations() == STATION_DATA.len());

        assert_eq!(stations.get_station(1).unwrap().name(), "SWR4");
        assert!(stations.get_station(4).is_none());
    }

    #[test]
    fn iterate() {
        let stations = Stations::new();

        let results: std::vec::Vec<std::string::String> =
            stations.map(|s| s.name().to_string()).collect();

        assert!(results.contains(&"SWR3".to_string()));
        assert!(results.contains(&"SWR4".to_string()));
        assert!(results.contains(&"181 FM Classic".to_string()));
        assert!(results.contains(&"Absolut Oldie Classics".to_string()));
    }
}
