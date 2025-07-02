use crate::{front_panel::Buttons, task::sync::STATION_CHANGE_WATCH, FrontPanel, RadioStations};

//const DEBOUNCE_DURATION: u64 = 100; // Milliseconds  TODO use fugit?

use esp_hal::gpio::Input;

use embassy_time::{Duration, Timer};

//use stations::Stations;

// type FrontPanelDriverMutextType =
//     Mutex<CriticalSectionRawMutex, Option<FrontPanelDriverType<'static>>>;

// TODO Currently using the global static MULTIPLEXER_DRIVER. Change this later to a parameter

// DESIGN NOTE: This does not debouce the buttons in the tradtional way, but this seems to work just fine.
#[embassy_executor::task]
pub async fn tuner2(
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
    let initial_station = stations
        .preset(0)
        .ok()
        .flatten()
        .or_else(|| stations.get_station(0).expect("No initial station found"))
        .expect("No initial station found");

    esp_println::println!("DEBUG: Initial station {:?}", initial_station);

    // Send the inital station
    // let initial_station = stations
    //     .get_station(0)
    //     .expect("ERROR: Could not set intial station (0)");
    station_change_sender.send(initial_station);

    // TODO
    // Initially just try the press buttons. Set up the rotary encoder later.

    let mut last_button_pressed = Buttons::None;
    let mut rotary_encoder_transition = false;
    let mut rotary_encoder_movement: i32 = 0;

    loop {
        // Default configuration is active low
        //interrupt_pin.wait_for_falling_edge().await;
        //interrupt_pin.wait_for_rising_edge().await;
        //esp_println::println!("DEBUG: Interrupt detected");

        let button_pressed = front_panel.button_pressed().await.unwrap();
        if button_pressed != last_button_pressed {
            esp_println::println!("DEBUG: Button pressed = {:?}", button_pressed);
            last_button_pressed = button_pressed.clone();

            // let station_index: Option<usize> = match button_pressed {
            //     Buttons::RotaryEncoderSwitch => {
            //         esp_println::println!("INFO: Rotary Switch pressed");
            //         None
            //     }
            //     Buttons::Button1 => Some(0),
            //     Buttons::Button2 => Some(1),
            //     Buttons::Button3 => Some(2),
            //     Buttons::Button4 => Some(3),
            //     Buttons::None => None, // No button pressed so keep waiting
            //     Buttons::Unknown => panic!("ERROR: Unknown button pressed"),
            // };

            let selected_station = match button_pressed {
                Buttons::RotaryEncoderSwitch => {
                    esp_println::println!("INFO: Rotary Switch pressed");
                    Ok(None)
                }
                Buttons::Button1 => stations.preset(0),
                Buttons::Button2 => stations.preset(1),
                Buttons::Button3 => stations.preset(2),
                Buttons::Button4 => stations.preset(3),
                Buttons::None => Ok(None), // No button pressed so keep waiting
                Buttons::Unknown => panic!("ERROR: Unknown button pressed"),
            };

            match selected_station {
                Ok(Some(station)) => {
                    esp_println::println!("\n\nINFO: Playing: {}\n\n", station.name());

                    station_change_sender.send(station.clone());
                }
                Ok(None) => {
                    esp_println::println!("INFO: No preset for button {:?}", button_pressed)
                }
                Err(err) => panic!("ERROR: cannot select station ({:?})", err),
            }
        }

        // Now read the rotary controller. Using this approach means that there can be some spurious
        // direction changes, but the trend is correct.
        let rotary_encoder_state = front_panel.read_rotary_encoder().await.unwrap();

        match rotary_encoder_state {
            (true, true) => (),
            (true, false) => {
                if !rotary_encoder_transition {
                    rotary_encoder_movement += 1;
                    rotary_encoder_transition = true;

                    esp_println::println!("DEBUG Increment");
                }
            }
            (false, true) => {
                if !rotary_encoder_transition {
                    rotary_encoder_movement -= 1;

                    rotary_encoder_transition = true;
                    esp_println::println!("DEBUG Decrement");
                }
            }
            (false, false) => {
                //rotary_encoder_movement = 0;
                rotary_encoder_transition = false;
            }
        }

        // esp_println::println!(
        //     "DEBUG rotary_encoder_movement = {}",
        //     rotary_encoder_movement
        // );

        if rotary_encoder_movement >= 4 {
            stations.increment_current_station();
            let station = stations.current_station().unwrap(); //TODO Error handling

            esp_println::println!("\n\nDEBUG: Playing: {:?}\n\n", station); // TODO unwrap?

            station_change_sender.send(station.unwrap().clone());
            rotary_encoder_movement = 0;
        } else if rotary_encoder_movement <= -4 {
            stations.decrement_current_station();

            let station = stations.current_station().unwrap(); //TODO Error handling
            esp_println::println!("\n\nDEBUG: Playing: {:?}\n\n", station);

            station_change_sender.send(station.unwrap().clone());

            rotary_encoder_movement = 0;
        }

        // Debounce
        // TODO see also https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/debounce.rs
        //Timer::after(Duration::from_millis(DEBOUNCE_DURATION)).await;

        //let button_pressed = front_panel.button_pressed().await.unwrap();

        Timer::after(Duration::from_millis(10)).await;
    }
}

// // Helper function to lock  the front panel driver mutex and get the button pressed
// async fn get_button_pressed(front_panel_driver: &'static FrontPanelDriverMutextType) -> Buttons {
//     let mut button_pressed = Buttons::Unknown;

//     {
//         let mut front_panel_driver_unlocked = front_panel_driver.lock().await;
//         if let Some(front_panel_driver) = front_panel_driver_unlocked.as_mut() {
//             button_pressed = front_panel_driver.button_pressed().await.unwrap();
//         }
//     }

//     button_pressed
// }
