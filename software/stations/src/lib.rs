#![cfg_attr(not(test), no_std)]
//! # Minimal-Memory Station List
//!
//! This crate provides a memory-efficient list of radio stations, each with a name and URL, using fixed-size buffers and a string pool.
//!
//! ## Features
//!
//! - Stores station names and URLs in a compact string pool to minimize memory usage.
//! - Supports a configurable maximum number of stations, name length, URL length, and preset slots via const generics.
//! - Allows adding, retrieving, and assigning stations to preset slots.
//! - Designed for embedded and resource-constrained environments (uses `heapless`).
//!
//! ## Const Generics
//!
//! The main types use const generics for flexibility and efficiency:
//!
//! - `NAME_LEN`: Maximum length of a station name (in bytes).
//! - `URL_LEN`: Maximum length of a station URL (in bytes).
//! - `NUM_PRESETS`: Number of preset slots available.
//!
//! Example type alias for a typical configuration:
//!
//! ```rust
//! # use crate::stations::Stations;
//! type MyStations = Stations<32, 256, 4>;
//! ```
//!
//! I.e. `NAME_LEN` = 32, `URL_LEN` = 256, `NUM_PRESETS` = 4.
//!
//! ## Usage Example
//!
//! ```rust
//! use stations::Stations;
//!
//! const NAME_LEN: usize = 32;
//! const URL_LEN: usize = 256;
//! const NUM_PRESETS: usize = 4;
//!
//! let mut stations = Stations::<NAME_LEN, URL_LEN, NUM_PRESETS>::new();
//! stations.add_station(b"Radio 1", b"http://radio1.example/stream").unwrap();
//! stations.add_station(b"Radio 2", b"http://radio2.example/stream").unwrap();
//!
//! // Set a preset
//! stations.set_preset(1, 0).unwrap();
//!
//! // Retrieve a station by index
//! let  station = stations.get_station(0).unwrap();
//! assert_eq!(station.name(), "Radio 1");
//!
//! // Retrieve a preset
//! let (_id, preset_station) = stations.preset(0).unwrap();
//! assert_eq!(preset_station.name(), "Radio 2");
//! ```
//!
//! ## Loading from CSV
//!
//! You can load stations from a CSV file (as a byte slice):
//!
//! ```rust
//! # use stations::Stations;
//! let csv = b"Radio1,http://radio1.example/stream\nRadio2,http://radio2.example/stream,PRESET:0";
//! let stations = Stations::<32, 256, 4>::load(csv).unwrap();
//! ```
//!
//! ## Error Handling
//!
//! Most operations return a `Result` with a `StationError` describing the failure reason (e.g., field too long, invalid UTF-8, out-of-bounds).
//!
//! ## Crate Features
//!
//! - No-std compatible (when `std` is disabled).
//! - Suitable for embedded and microcontroller projects.

use core::str::Utf8Error;

use heapless::{String, Vec};

use csv_core::{ReadFieldResult, Reader};

const POOL_SIZE: usize = 4096;
const MAX_NUM_STATIONS: usize = 64;

/// A station.
/// This struct is in a form that can be easily used in an application.
///
/// The struct cannot be created alone, but has to be obtained from the
///  `Stations`struct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Station<const NAME_LEN: usize, const URL_LEN: usize> {
    /// The name of the station
    name: String<NAME_LEN>,

    /// The URL of the station
    url: String<URL_LEN>,
}

impl<const NAME_LEN: usize, const URL_LEN: usize> Station<NAME_LEN, URL_LEN> {
    // Private function to create a staion
    fn new() -> Station<NAME_LEN, URL_LEN> {
        Station {
            name: String::new(),
            url: String::new(),
        }
    }

    /// The name of the station
    pub fn name(&self) -> String<NAME_LEN> {
        self.name.clone()
    }

    /// The URL of the station as `&str`
    pub fn url(&self) -> String<URL_LEN> {
        self.url.clone()
    }
}

// The position of the station data in the pool.
// This is only used internally.
#[derive(Clone, PartialEq)]
struct StationPositions {
    // Start and end index of the station name
    name: (usize, usize),
    // Start and end index of the station url string
    url: (usize, usize),
}

/// A list of stations with name and url.
///
///  Assuming that all station data is UTF8.
pub struct Stations<const NAME_LEN: usize, const URL_LEN: usize, const NUM_PRESETS: usize> {
    // To save storage, the station names and urls are stored in a long string pool.
    pool: String<POOL_SIZE>,

    // The positions of each station name and url in the string pool.
    positions: Vec<StationPositions, MAX_NUM_STATIONS>,

    // The preset stations
    preset_slots: [Option<usize>; NUM_PRESETS],
    // // The current station
    // current_station: Option<usize>,
}

impl<const NAME_LEN: usize, const URL_LEN: usize, const NUM_PRESETS: usize>
    Stations<NAME_LEN, URL_LEN, NUM_PRESETS>
{
    /// Creates an empty list of stations.
    ///
    /// # Returns
    ///
    /// A new `Stations` instance with no stations or presets set.
    pub fn new() -> Stations<NAME_LEN, URL_LEN, NUM_PRESETS> {
        Stations {
            pool: String::new(),
            positions: Vec::new(),
            preset_slots: [None; NUM_PRESETS],
            // current_station: None,
        }
    }

    /// Loads a set of stations from a CSV file represented as a byte slice.
    ///
    /// The CSV file must have at least two fields per record:
    /// `{station name},{station_url},...`
    ///
    /// Only the first two fields of each record are used; additional fields are ignored
    /// unless the last field has the form PREFIX:n (n is the preset slot number).
    /// In this case the station is assigned to a preset slot.
    ///
    /// # Arguments
    ///
    /// * `data` - The CSV data as a UTF-8 encoded byte slice.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Stations)` containing all successfully parsed stations.
    /// Returns `Err(StationError)` if any record is invalid or a field is too long.
    ///
    /// # Errors
    ///
    /// * [`StationError::CsvFieldTooLong`] - If a CSV field exceeds the allowed buffer size.
    /// * [`StationError::NameTooLong`] - If a station name exceeds the maximum allowed length.
    /// * [`StationError::UrlTooLong`] - If a station URL exceeds the maximum allowed length.
    /// * [`StationError::NameNotUtf8`] - If a station name is not valid UTF-8.
    /// * [`StationError::UrlNotUtf8`] - If a station URL is not valid UTF-8.
    /// * [`StationError::TooManyStations`] - If the station pool or list is full.
    ///
    pub fn load(data: &[u8]) -> Result<Stations<NAME_LEN, URL_LEN, NUM_PRESETS>, StationError> {
        let mut reader = Reader::new();

        let mut stations = Stations::new();

        let mut out = [0u8; 1024];

        let mut in_bytes = data;
        let mut name = [0u8; NAME_LEN];
        let mut url = [0u8; URL_LEN];
        let mut field_index: usize = 0;
        let mut name_len = 0;
        let mut station_id = 0;
        loop {
            // let (result, nin, nout) = reader.read_field(&in_bytes, &mut out);
            let (result, nin, nout) = reader.read_field(in_bytes, &mut out);

            match result {
                ReadFieldResult::InputEmpty => {}
                ReadFieldResult::OutputFull => Err(StationError::CsvFieldTooLong)?,
                ReadFieldResult::Field { record_end } => match field_index {
                    0 => {
                        if nout > NAME_LEN {
                            Err(StationError::NameTooLong)?;
                        };
                        name[0..nout].copy_from_slice(&out[0..nout]);
                        name_len = nout;
                        field_index += 1;
                    }
                    1 => {
                        if nout > URL_LEN {
                            Err(StationError::UrlTooLong)?;
                        };
                        url[0..nout].copy_from_slice(&out[0..nout]);
                        station_id = stations.add_station(&name[0..name_len], &url[0..nout])?;
                        field_index += 1;
                    }
                    _ => {
                        // Check if this is a preset field
                        let value = str::from_utf8(&out[0..nout])?.trim();
                        if value.starts_with("PRESET:") {
                            let preset_slot = Self::extract_prefix_slot(value)?;
                            stations.set_preset(station_id, preset_slot)?;
                        }
                        if !record_end {
                            field_index += 1;
                        } else {
                            field_index = 0;
                        }
                        in_bytes = &in_bytes[nin..];
                        continue;
                    }
                },
                ReadFieldResult::End => break,
            }
            in_bytes = &in_bytes[nin..];
        }

        //stations.set_current_station(0)?;
        Ok(stations)
    }

    /// Adds a station to the list.
    ///
    /// The station name and URL are provided as byte slices and must be valid UTF-8.
    ///
    /// # Arguments
    ///
    /// * `station_name` - The name of the station as a UTF-8 encoded byte slice.
    /// * `station_url` - The URL of the station as a UTF-8 encoded byte slice.
    ///
    /// # Returns
    ///
    /// Returns `Ok(index)` with the index of the newly added station on success.
    /// Returns `Err(StationError)` if:
    /// - The name or URL is not valid UTF-8.
    /// - The name or URL exceeds the maximum allowed length.
    /// - The pool or station list is full.
    ///
    /// # Errors
    ///
    /// * [`StationError::NameNotUtf8`] - If the station name is not valid UTF-8.
    /// * [`StationError::UrlNotUtf8`] - If the station URL is not valid UTF-8.
    /// * [`StationError::NameTooLong`] - If the station name is too long.
    /// * [`StationError::UrlTooLong`] - If the station URL is too long.
    /// * [`StationError::TooManyStations`] - If the station pool or list is full.
    ///
    pub fn add_station(
        &mut self,
        station_name: &[u8],
        station_url: &[u8],
    ) -> Result<usize, StationError> {
        let name = str::from_utf8(station_name).map_err(|_| StationError::NameNotUtf8)?;
        let url = str::from_utf8(station_url).map_err(|_| StationError::UrlNotUtf8)?;

        if name.len() > NAME_LEN {
            Err(StationError::NameTooLong)?;
        }
        if url.len() > URL_LEN {
            Err(StationError::UrlTooLong)?;
        }

        let name_positions = (self.pool.len(), self.pool.len() + name.len());
        self.pool
            .push_str(name)
            .map_err(|_| StationError::TooManyStations)?;

        let url_positions = (self.pool.len(), self.pool.len() + url.len());
        self.pool
            .push_str(url)
            .map_err(|_| StationError::TooManyStations)?;

        let station_positions = StationPositions {
            name: name_positions,
            url: url_positions,
        };

        self.positions
            .push(station_positions)
            .map_err(|_| StationError::TooManyStations)?;

        let added_station_id = self.positions.len() - 1;
        Ok(added_station_id)
    }

    /// Sets a station as a preset at the specified preset index.
    ///
    /// # Arguments
    ///
    /// * `station_id` - The index of the station to set as a preset.
    /// * `preset_id` - The preset slot to assign the station to.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Station)` with the station set as the preset on success.
    /// Returns `Err(StationError)` if the preset or station index is out of bounds.
    ///
    /// # Errors
    ///
    /// * [`StationError::TooManyPresets`] - If the preset index is out of range.
    /// * [`StationError::StationNonExistent`] - If the station index does not exist.
    ///
    pub fn set_preset(
        &mut self,
        station_id: usize,
        preset_id: usize,
    ) -> Result<Station<NAME_LEN, URL_LEN>, StationError> {
        // Check bounds
        if preset_id >= NUM_PRESETS {
            Err(StationError::TooManyPresets)?;
        }
        if station_id >= self.number_stations() {
            Err(StationError::StationNonExistent)?;
        }

        self.preset_slots[preset_id] = Some(station_id);

        let station = self.get_station(station_id);

        Ok(station.unwrap())
    }

    /// Returns a tuple with the id and the station assigned to the specified preset index
    /// or `None` if the preset is empty, the preset index is out of bounds. or, for some reason,
    /// the station itself does not exist
    pub fn preset(&self, preset_id: usize) -> Option<(usize, Station<NAME_LEN, URL_LEN>)> {
        // Check bounds
        if preset_id >= NUM_PRESETS {
            return None;
        }

        // Get the station preset
        if let Some(station_id) = self.preset_slots[preset_id] {
            self.get_station(station_id)
                .map(|station| (station_id, station))
        } else {
            None
        }
    }

    // /// Retrieves the id of the  station assigned to the specified preset index
    // /// or `None` if the preset is empty or the preset index is out of bounds.
    // pub fn preset_station_id(&self, preset_id: usize) -> Option<usize> {
    //     // Check bounds
    //     if preset_id >= NUM_PRESETS {
    //         return None;
    //     }

    //     // Get the station id p of the  preset
    //     self.preset_slots[preset_id]
    // }

    /// Returns the number of stations that have been added to the station list
    pub fn number_stations(&self) -> usize {
        self.positions.len()
    }

    /// Retrieves a station by its index.
    ///
    /// # Arguments
    ///
    /// * `id` - The index of the station to retrieve.
    ///
    /// # Returns
    ///
    /// Returns `Some(Station)` if the station exists at the given index.
    /// Returns `None` if the index is out of bounds.
    ///
    pub fn get_station(&self, id: usize) -> Option<Station<NAME_LEN, URL_LEN>> {
        let station_index = self.positions.get(id);

        let mut station = Station::<NAME_LEN, URL_LEN>::new();

        match station_index {
            Some(index) => {
                let station_name = &self.pool[index.name.0..index.name.1];
                let station_url = &self.pool[index.url.0..index.url.1];

                // SAFETY: No stations with names or urls that are too long for the Station strings
                // are created (see add_station). As such not too long names are going to be copied
                // into a station
                station.name.push_str(station_name).unwrap();
                station.url.push_str(station_url).unwrap();

                Some(station)
            }
            None => None,
        }
    }

    // / Sets the current station by index.
    // /
    // / # Arguments
    // /
    // / * `station_index` - The index of the station to set as the current station.
    // /
    // / # Returns
    // /
    // / Returns `Ok(())` if the station index is valid and the current station is set.
    // / Returns `Err(StationError::StationNonExistent)` if the index is out of bounds.
    // /
    // / # Errors
    // /
    // / * [`StationError::StationNonExistent`] - If the specified station index does not exist.
    // pub fn set_current_station(&mut self, station_index: usize) -> Result<(), StationError> {
    //     if station_index < self.positions.len() {
    //         self.current_station = Some(station_index);
    //         Ok(())
    //     } else {
    //         Err(StationError::StationNonExistent)
    //     }
    // }

    // pub fn current_station(&self) -> Option<Station<NAME_LEN, URL_LEN>> {
    //     match self.current_station {
    //         Some(station_index) => self.get_station(station_index),
    //         None => None,
    //     }
    // }

    /// Set the current station to the next one in the stations list. If the range is exceeded
    /// the current stations is clamped to the last one in the list.
    // pub fn increment_current_station(&mut self) {
    //     if let Some(station_id) = self.current_station {
    //         let inc_station_id = (station_id + 1).clamp(0, self.positions.len() - 1);

    //         self.current_station = Some(inc_station_id);
    //     }
    // }
    /// Set the current station to the previous one in the stations list. If the range is exceeded
    /// the current stations is clamped to the first one in the list.
    // pub fn decrement_current_station(&mut self) {
    //     if let Some(station_id) = self.current_station {
    //         let dec_station_id = station_id.saturating_sub(1);
    //         //let dec_station_id = (station_id - 1).clamp(0, self.positions.len() - 1);

    //         self.current_station = Some(dec_station_id);
    //     }
    // }

    /// Reset the current station to 0.
    ///
    /// Equivalent to `set_current_station(0)`
    // pub fn reset_current_station(&mut self) {
    //     self.current_station = Some(0);
    // }

    // Helper function to extact the prefix slot number from the CSV field
    fn extract_prefix_slot(field_value: &str) -> Result<usize, StationError> {
        let slot_str = field_value
            .rsplit(":")
            .next()
            .ok_or(StationError::InvalidPreset)?
            .trim();

        let slot = slot_str.parse().map_err(|_| StationError::InvalidPreset)?;

        if slot < NUM_PRESETS {
            Ok(slot)
        } else {
            Err(StationError::InvalidPreset)
        }
    }
}

impl<const NAME_LEN: usize, const URL_LEN: usize, const NUM_PRESETS: usize> Default
    for Stations<NAME_LEN, URL_LEN, NUM_PRESETS>
{
    fn default() -> Stations<NAME_LEN, URL_LEN, NUM_PRESETS> {
        Self::new()
    }
}

impl<const NAME_LEN: usize, const URL_LEN: usize, const NUM_PRESETS: usize> core::fmt::Display
    for Stations<NAME_LEN, URL_LEN, NUM_PRESETS>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut display_string = String::<POOL_SIZE>::new();

        for i in 0..self.number_stations() {
            let station = self.get_station(i);
            if let Some(station) = station {
                display_string.push_str(&station.name()).unwrap();
                display_string.push_str(", ").unwrap();
                display_string.push_str(&station.url()).unwrap();
                display_string.push_str("\n").unwrap();
            }
        }

        write!(f, "{display_string}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StationError {
    /// The station URL added is not in UTF8
    UrlNotUtf8,

    /// The station URL added is too long
    UrlTooLong,

    /// The station name added is not in UTF8
    NameNotUtf8,

    /// The station name added is too long
    NameTooLong,

    /// A field in CSV file is not UTF8,
    CsvFieldNotUtf8,

    /// A field in the stations csv file is too long for a name or url
    CsvFieldTooLong,

    /// Attempt to add too many stations. Poll size is exceeded
    TooManyStations,

    /// Requested station does not exist
    StationNonExistent,

    /// Attempt to set too many presets
    TooManyPresets,

    /// Attempt to access a preset that cannot exist or the preset is
    /// incorrectly specified.
    InvalidPreset,
}

impl From<Utf8Error> for StationError {
    fn from(_err: Utf8Error) -> Self {
        StationError::CsvFieldNotUtf8
    }
}
