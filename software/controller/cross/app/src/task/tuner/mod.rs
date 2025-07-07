use crate::{front_panel::Buttons, task::sync::STATION_CHANGE_WATCH, FrontPanel, RadioStations};

// [ ] Set the LED to show that a statipon has been tuned. Requires a new task.
// [ ] Extract TiningScale into a different file. This means that tuner becomes a module
// [ ]  Seem to have to move the rotary encoder many turns before a new station is selected.
//      See if this makes sense when the display is there.

use esp_hal::gpio::Input;

use embassy_time::{Duration, Timer};

use periodic_map::PeriodicMap;

mod tuning_scale;

use tuning_scale::TuningScale;

const VALID_WINDOW: usize = 5;
const INVALID_WINDOW: usize = 10;

// TODO Currently using the global static MULTIPLEXER_DRIVER. Change this later to a parameter

// DESIGN NOTE: This does not debouce the buttons in the tradtional way, but this seems to work just fine.
#[embassy_executor::task]
pub async fn tuner(
    stations: &'static mut RadioStations,
    front_panel: &'static FrontPanel,
    mut _interrupt_pin: Input<'static>,
) {
    //Set up the list of stations
    //let mut stations = Stations::new();

    let station_change_sender = STATION_CHANGE_WATCH.sender();

    // Determine the initial station from:
    // 1. The last set station - TODO
    // 2. The first preset stations if set
    // 3. The first station in the station list
    let initial_station = stations.preset(0).or_else(|| stations.get_station(0)); //.expect("No initial station found");

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

            let selected_station = match button_pressed {
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

            match selected_station {
                Some(ref station) => {
                    esp_println::println!(
                        "\n\nINFO: Playing preset station: {}\n\n",
                        station.name()
                    );

                    station_change_sender.send(selected_station);
                }
                None => {
                    esp_println::println!("INFO: No preset for button {:?}", button_pressed)
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
                        esp_println::println!("\n\nINFO: Playing station: {:?}\n\n", station);
                        // TODO assuming that the following will work.
                        stations.set_current_station(id).unwrap();
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
