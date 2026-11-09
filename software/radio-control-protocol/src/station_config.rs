use heapless::Vec;

const NUMBER_PRESETS: usize = 4;

// // TODO move these to radio_starion_protocol and check the size
// pub type RadioStationId = usize;
// pub type RadioStationName = String<40>;

use crate::radio_control_protocol::RadioStationId;
#[derive(Clone, Debug, PartialEq)]
pub struct StationConfig {
    pub number_stations: usize,
    // pub presets: Option<[RadioStationId; NUMBER_PRESETS]>,
    pub presets: Vec<RadioStationId, NUMBER_PRESETS>,
}

impl StationConfig {
    pub fn new() -> Self {
        Self {
            number_stations: 0,
            presets: Vec::new(),
        }
    }

    // pub fn with_presets(mut self, presets: &[RadioStationId]) -> Self {
    //     self.presets.copy_from_slice(presets);
    //     self
    // }

    pub fn with_presets(mut self, presets: [RadioStationId; NUMBER_PRESETS]) -> Self {
        self.presets.clear();
        // SAFETY: The parameter presets and the vec length are always the same.
        self.presets.extend_from_slice(&presets).unwrap();
        self
    }

    pub fn with_number_stations(mut self, number_stations: RadioStationId) -> Self {
        self.number_stations = number_stations;
        self
    }

    /// Get the station id from the preset number.
    /// For instance, if the presets were [2, 5, 12, 6]
    /// then `preset(2)` would return the station id `Some(12)`.
    /// If no presets have been set then returns `None``
    /// If the preset number is out of range then return Ǹone`.
    pub fn map_preset(&self, preset_number: usize) -> Option<RadioStationId> {
        self.presets.get(preset_number).copied()
    }
}

impl Default for StationConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_map_preset() {
        let config = StationConfig::new().with_presets([3, 5, 7, 11]);

        assert_eq!(config.map_preset(0), Some(3));
        assert_eq!(config.map_preset(1), Some(5));
        assert_eq!(config.map_preset(2), Some(7));
        assert_eq!(config.map_preset(3), Some(11));
        assert_eq!(config.map_preset(4), None);
    }

    #[test]
    fn test_default() {
        let config = StationConfig::default();

        assert_eq!(config.number_stations, 0);
        assert!(config.presets.len() == 0);
    }
}
