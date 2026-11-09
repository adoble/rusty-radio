#![cfg_attr(not(test), no_std)]

pub mod uart_handler;
pub use uart_handler::{UartHandler, UartHandlerError, command::Command};

pub mod radio_control_protocol;
pub use radio_control_protocol::{
    MAX_NUMBER_PARAMETERS, MAX_PARAMETER_LEN, Parameter, Parameters, RadioCommand,
    RadioControlProtocol, RadioControlProtocolError, RadioStationId, RadioStationName,
    TunerDirection,
};

pub mod station_config;
pub use station_config::StationConfig;
