use crate::{
    front_panel::Buttons, task::radio_stations::RadioStations, task::sync::STATION_CHANGE_WATCH,
    FrontPanel,
};

use embassy_time::{Duration, Timer};

use periodic_map::PeriodicMap;

mod tuning_scale;

use tuning_scale::TuningScale;

const VALID_WINDOW: usize = 5;
const INVALID_WINDOW: usize = 10;

// DESIGN NOTE: This does not debouce the buttons in the traditional way,
// but this polling technique  seems to work just fine.
#[embassy_executor::task]
pub async fn tuner(stations: &'static mut RadioStations, front_panel: &'static FrontPanel) {
    //Set up the list of stations
    //let mut stations = Stations::new();

    let station_change_sender = STATION_CHANGE_WATCH.sender();

    // 1. The last set station - TODO
    // 2. The first preset stations if set
    // 3. The first station in the station list

    let initial_station = stations
        .preset(0)
        .map(|s| s.1) // Get the preset station from the tuple
        .or_else(|| stations.get_station(0)); //.expect("No initial station found");

    // Send the inital station
    station_change_sender.send(initial_station);

    let mut last_button_pressed = Buttons::None;

    let mut last_station_id = None;

    // Intrepretating rotary encoder movement to as a tuning scale as used in an old analog radio
    let mut tuning_scale =
        TuningScale::new(stations.number_stations() * (VALID_WINDOW + INVALID_WINDOW));
    // Used for mapping the rotary encoder to stations. Is safe as long as VALID_WINDOW and
    // INVALID_WINDOW are more than 0.
    let periodic_map = PeriodicMap::new(VALID_WINDOW, INVALID_WINDOW).unwrap();

    loop {
        let button_pressed = match front_panel.button_pressed().await {
            Ok(button) => button,
            // Sometimes the multiplexer misfires. If so ignore this.
            Err(_) => continue,
        };

        if button_pressed != last_button_pressed {
            last_button_pressed = button_pressed.clone();

            let selection = match button_pressed {
                Buttons::RotaryEncoderSwitch => {
                    esp_println::println!("INFO: Rotary Switch pressed");
                    None
                }
                Buttons::Button1 => stations.preset(0),
                Buttons::Button2 => stations.preset(1),
                Buttons::Button3 => stations.preset(2),
                Buttons::Button4 => stations.preset(3),
                Buttons::None => None, // No button pressed so keep waiting
                Buttons::Unknown => panic!("ERROR: Unknown button pressed"),
            };

            // TODO should the station_id be in Station?
            // [ ] test what happens when no preset is set!

            match selection {
                Some((station_id, station)) => {
                    esp_println::println!(
                        "\n\nINFO: Playing preset station: {}\n\n",
                        station.name()
                    );

                    // Adjust the tuner scale so that any later movement is from the selected preset station
                    let scale_value = periodic_map.inverse_map(station_id);
                    tuning_scale.set(scale_value);

                    // Signal that the station has changed
                    station_change_sender.send(Some(station));
                }
                None => {
                    () //esp_println::println!("INFO: No preset for button {:?}", button_pressed)
                }
            }
        }

        // Now read the rotary encoder.
        let mut rotary_encoder_movement = false;
        let direction = front_panel.decode_rotary_encoder().await.unwrap();
        match direction {
            crate::front_panel::Direction::Clockwise => {
                rotary_encoder_movement = true;
                tuning_scale.increment();
            }
            crate::front_panel::Direction::CounterClockwise => {
                rotary_encoder_movement = true;
                tuning_scale.decrement();
            }
            crate::front_panel::Direction::None => (),
        }

        if rotary_encoder_movement {
            let station_id = periodic_map.map(tuning_scale.get());

            if station_id != last_station_id {
                match station_id {
                    Some(id) => {
                        let station = stations.get_station(id);
                        // TODO change this so that we only print out the station name.
                        esp_println::println!("\n\nINFO: Playing station: {:?}\n\n", station);
                        // TODO assuming that the following will work.
                        //stations.set_current_station(id).unwrap();
                        last_station_id = station_id;
                        station_change_sender.send(station);
                    }
                    None => {
                        // TODO should the current stations be set to None?
                        //stations.set_current_station(None);
                        last_station_id = station_id;
                        station_change_sender.send(None);
                    }
                }
            }
        }

        Timer::after(Duration::from_millis(5)).await;
    }
}
