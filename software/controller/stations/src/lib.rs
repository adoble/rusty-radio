#![cfg_attr(not(test), no_std)]

// The music type needs to be specified as the extension is not enough to decide.

static STATION_DATA: &[(&str, &str)] = &[
    // Simple M3U stations (no m3u extensions)
    (
        "Hit Radio FFH",
        "http://streams.ffh.de/radioffh/mp3/hqlivestream.m3u",
    ),
    // SWR3 does a number of redirects, 128 kB/s
    ("SWR3", "http://liveradio.swr.de/sw282p3/swr3/play.mp3"),
    ("SWR4", "http://liveradio.swr.de/sw282p3/swr4bw/"),
    // 181 FM Classic does no redirects, 128 kB/s
    (
        "181 FM Classic",
        "http://listen.181fm.com/181-classical_128k.mp3",
    ),
    // Error station
    ("Bad Wolf", "https://aqswdefr.de"),
    (
        "Absolut Oldie Classics",
        "http://absolut-oldieclassics.live-sm.absolutradio.de/absolut-oldieclassics/stream/mp3",
    ),
    // AAC Station
    (
        "bigFM",
        "http://streams.bigfm.de/bigfm-deutschland-128-aac?usid=0-0-H-A-D-30",
    ),
    // Local server for testing, 128 kB/s
    //("Hijo de la Luna", "http://192.168.2.107:8080/music/2"),
];

#[derive(Default, Debug, PartialEq, PartialOrd)]
pub enum MusicType {
    #[default]
    MP3,
    AAC,
    M3U,
    M3U8,
    Unknown,
}

impl From<&str> for MusicType {
    fn from(value: &str) -> Self {
        match value {
            "mp3" => MusicType::MP3,
            "MP3" => MusicType::MP3,
            "aac" => MusicType::AAC,
            "AAC" => MusicType::AAC,
            "m3u" => MusicType::M3U,
            "M3U" => MusicType::M3U,
            "m3u8" => MusicType::M3U8,
            "M3U8" => MusicType::M3U8,
            _ => MusicType::Unknown,
        }
    }
}

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

impl Default for Stations {
    fn default() -> Self {
        Self::new()
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

    #[test]
    fn test_get_station() {
        let mut stations = Stations::new();

        let number_stations = stations.number_stations();

        assert!(number_stations == STATION_DATA.len());

        let station_id = 1;
        assert!(station_id < STATION_DATA.len());
        assert_eq!(stations.get_station(station_id).unwrap().name(), "SWR4");
    }

    #[test]
    fn test_number_stations() {
        let stations = Stations::new();

        let number_stations = stations.number_stations();

        assert!(number_stations == STATION_DATA.len());
    }

    #[test]
    fn test_no_station() {
        let mut stations = Stations::new();

        let number_stations = stations.number_stations();

        assert!(stations.get_station(number_stations).is_none());
    }

    #[test]
    fn test_music_type() {
        let mut stations = Stations::new();
        assert_eq!(
            stations.get_station(0).unwrap().music_type(),
            MusicType::MP3
        );

        assert_eq!(
            stations.get_station(4).unwrap().music_type(),
            MusicType::AAC
        );
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
