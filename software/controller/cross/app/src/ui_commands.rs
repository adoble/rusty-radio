#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiCommand {
    /// Indicates if the wifi is connected
    WiFiConnected(bool),

    /// The selected station id
    StationSelect(usize),

    /// If the user has turned the tuner knob
    TunerMoved(TunerDirection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TunerDirection {
    Clockwise,
    CounterClockwise,
}
