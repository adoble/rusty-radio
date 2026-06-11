use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

const NUMBER_PRESETS: usize = 4;

pub type RadioStationId = usize;
#[derive(Copy, Clone)]
pub struct StationConfig {
    pub number_stations: usize,
    pub presets: Option<[RadioStationId; NUMBER_PRESETS]>,
}

impl StationConfig {
    /// Get the station id from the preset number.
    /// For instance, if the presets were [2, 5, 12, 6]
    /// then `preset(2)` would return the station id `Some(12)`.
    /// If no presets have been set then returns `None``
    /// If the preset number is out of range then return Ǹone`.
    pub fn map_preset(&self, preset_number: usize) -> Option<RadioStationId> {
        self.presets
            .and_then(|presets| presets.get(preset_number).copied())
    }
}

// For test purposes this is set to some values. Later there need to be loaded from the radio
impl Default for StationConfig {
    /// # Deprecated  
    /// Temporary placeholder — replace with a read from the radio processor.
    fn default() -> Self {
        Self {
            number_stations: 21,
            presets: Some([2, 3, 5, 7]),
        }
    }
}

pub type StationConfigMutex = Mutex<CriticalSectionRawMutex, StationConfig>;

pub static STATION_CONFIG: StationConfigMutex = Mutex::new(StationConfig {
    number_stations: 0,
    presets: None,
});
