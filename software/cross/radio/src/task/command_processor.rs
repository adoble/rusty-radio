#![allow(unused_imports)]
#![allow(unused_variables)]
use core::fmt::Write;
/// Receives the serial command and pushes them to the radio command channel
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Sender};
use heapless::String;
use log::*;

use crate::radio_stations::{RadioStation, RadioStations};
use crate::task::sync::{StationChangeSender, STATION_CHANGE_WATCH};
use radio_control_protocol::{
    Parameter, Parameters, RadioCommand, RadioControlProtocol, RadioControlProtocolError,
    RadioStationId, MAX_PARAMETER_LEN,
};
use stations::{Station, Stations};

#[embassy_executor::task]
pub async fn process_commands(
    uart: &'static mut esp_hal::uart::Uart<'static, esp_hal::Async>,
    station_change_sender: StationChangeSender,
    radio_stations: &'static mut RadioStations,
) {
    // TODO think about time outs and retries

    // Clear out any  boot up text from the UI processor from the UART buffer before sending a receiving commands
    uart.flush().unwrap(); // TODO error handling

    let mut radio_control_protocol = RadioControlProtocol::new(uart);

    loop {
        let result: Result<heapless::Vec<String<_>, _>, RadioControlProtocolError> =
            match radio_control_protocol.receive_command().await {
                Ok(command) => {
                    process_command(command, radio_stations, &station_change_sender).await
                }
                Err(e) => {
                    error!("Receive commmand:  {:?}", e);
                    // Continue loop??
                    // radio_control_protocol.flush().await;

                    // Under what error conditions should a retry be done?

                    Err(e)
                }
            };
        debug!("Received command");

        let response = match result {
            Ok(v) => v,
            Err(e) => {
                error!("Command failed: {:?}", e);
                radio_control_protocol.send_error(e.into()).await;
                // Cannot send the error. This is unrecoverable
                // TODO think about time outs and retries
                panic!("RCP Error");
            }
        };

        if let Err(e) = radio_control_protocol.send_acknowledge(&response).await {
            error!("Sending response to UI processor: {:?}", e);
            // This is an unrecoverable error
            panic!("RCP Error");
        }
    }
}

async fn process_command(
    command: RadioCommand,

    radio_stations: &'static RadioStations,
    station_change_sender: &StationChangeSender,
) -> Result<Parameters, RadioControlProtocolError> {
    debug!("Entered process_command");

    let response = match command {
        RadioCommand::Station(id) => {
            let station = radio_stations
                .get_station(id)
                .ok_or(RadioControlProtocolError::StationIdInvalid)?;

            station_change_sender.send(Some(station.clone()));

            let mut response = Parameters::new();
            // SAFETY: Only ever pushing one parameter
            response.push(station.name()).unwrap();

            response
        }
        RadioCommand::Detune => todo!(),
        RadioCommand::Config => {
            // TODO not all presets set
            let mut response = Parameters::new();

            let number_stations_param = int_to_parameter(radio_stations.number_stations())?;

            response
                .push(number_stations_param)
                .expect("Exceeded capacity");

            let mut preset: usize = 0;
            while let Some((station_id, _)) = radio_stations.preset(preset) {
                let preset_param = int_to_parameter(station_id)?;
                response.push(preset_param).expect("Exceeed capacity");
                preset += 1;
            }

            response
        }
    };

    Ok(response)
}

fn int_to_parameter(n: impl core::fmt::Display) -> Result<Parameter, RadioControlProtocolError> {
    let mut p = Parameter::new();
    write!(p, "{}", n).map_err(|_| RadioControlProtocolError::ParseParameter)?;
    Ok(p)
}
