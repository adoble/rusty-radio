use crate::{FrontPanel, StationConfig, front_panel::Buttons, task::sync::STATION_CHANGE_WATCH};

use embassy_time::{Duration, Timer};

use periodic_map::PeriodicMap;

mod tuning_scale;

use tuning_scale::TuningScale;

const VALID_WINDOW: usize = 5;
const INVALID_WINDOW: usize = 10;

#[deprecated(
    since = "0.2.0",
    note = "Replace  with reading  the number of stations from the radio processor"
)]
pub const NUM_STATIONS: usize = 21; // Corresponds to the current contents of http://andrew-doble.hier-im-netz.de/ir/rr-stations.txt

// DESIGN NOTE: This does not debouce the buttons in the traditional way,
// but this polling technique seems to work just fine.
#[embassy_executor::task]
pub async fn tuner(station_config: &'static StationConfig, front_panel: &'static FrontPanel) {
    //Set up the list of stations
    //let mut stations = Stations::new();

    let station_change_sender = STATION_CHANGE_WATCH.sender();

    // 1. The last set station - TODO
    // 2. The first preset stations if set
    // 3. The first station in the station list

    let initial_station = if let Some(presets) = station_config.presets {
        presets[0]
    } else {
        0
    };

    // Send the inital station
    station_change_sender.send(Some(initial_station));

    let mut last_button_pressed = Buttons::None;

    let mut last_station_id = None;

    // Intrepretating rotary encoder movement to as a tuning scale as used in an old analog radio
    let mut tuning_scale =
        TuningScale::new(station_config.number_stations * (VALID_WINDOW + INVALID_WINDOW));
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
                Buttons::Button1 => station_config.map_preset(0),
                Buttons::Button2 => station_config.map_preset(1),
                Buttons::Button3 => station_config.map_preset(2),
                Buttons::Button4 => station_config.map_preset(3),
                Buttons::None => None, // No button pressed so keep waiting
                Buttons::Unknown => panic!("ERROR: Unknown button pressed"),
            };

            // TODO should the station_id be in Station?
            // [ ] test what happens when no preset is set!

            match selection {
                Some(station_id) => {
                    esp_println::println!("\n\nINFO: Playing preset station: {}\n\n", station_id);

                    // Adjust the tuner scale so that any later movement is from the selected preset station
                    let scale_value = periodic_map.inverse_map(station_id);
                    tuning_scale.set(scale_value);

                    // Signal that the station has changed
                    station_change_sender.send(Some(station_id));
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
                        esp_println::println!("\n\nINFO: Playing station: {:?}\n\n", id);
                        last_station_id = station_id;
                        station_change_sender.send(Some(id));

                        // TODO assuming that the following will work.
                        //stations.set_current_station(id).unwrap();
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
