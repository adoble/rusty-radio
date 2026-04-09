use crate::RadioStationId;

const NUMBER_PRESETS: usize = 4;

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
