use crate::{front_panel::Buttons, task::sync::STATION_CHANGE_WATCH, FrontPanel};

//const DEBOUNCE_DURATION: u64 = 100; // Milliseconds  TODO use fugit?

use esp_hal::gpio::Input;

use embassy_time::{Duration, Timer};

use stations::Stations;

// type FrontPanelDriverMutextType =
//     Mutex<CriticalSectionRawMutex, Option<FrontPanelDriverType<'static>>>;

// TODO Currently using the global static MULTIPLEXER_DRIVER. Change this later to a parameter

// DESIGN NOTE: This does not debouce the buttons in the tradtional way, but this seems to work just fine.
#[embassy_executor::task]
pub async fn tuner2(front_panel: &'static FrontPanel, mut _interrupt_pin: Input<'static>) {
    //Set up the list of stations
    let mut stations = Stations::new();

    let station_change_sender = STATION_CHANGE_WATCH.sender();

    // Send the inital station
    let initial_station = stations
        .get_station(0)
        .expect("ERROR: Could not set intial station (0)");
    station_change_sender.send(initial_station);

    // TODO
    // Initially just try the press buttons. Set up the rotary encoder later.

    let mut last_button_pressed = Buttons::None;

    loop {
        // Default configuration is active low
        //interrupt_pin.wait_for_falling_edge().await;
        //interrupt_pin.wait_for_rising_edge().await;
        //esp_println::println!("DEBUG: Interrupt detected");

        let button_pressed = front_panel.button_pressed().await.unwrap();
        if button_pressed != last_button_pressed {
            esp_println::println!("DEBUG: Button pressed = {:?}", button_pressed);
            last_button_pressed = button_pressed.clone();

            let station_index: Option<usize> = match button_pressed {
                Buttons::RotaryEncoderSwitch => {
                    esp_println::println!("INFO: Rotary Switch pressed");
                    None
                }
                Buttons::Button1 => Some(0),
                Buttons::Button2 => Some(1),
                Buttons::Button3 => Some(2),
                Buttons::Button4 => Some(3),
                Buttons::None => None, // No button pressed so keep waiting
                Buttons::Unknown => panic!("ERROR: Unknown button pressed"),
            };
            esp_println::println!("DEBUG station index: {:?}", station_index);

            if let Some(station_index) = station_index {
                let mut station = stations.get_station(station_index);

                if station.is_none() {
                    station = stations.get_station(0); // The first station is the index
                }

                esp_println::println!("\n\nINFO: Playing: {}\n\n", station.clone().unwrap().name());

                station_change_sender.send(station.unwrap().clone());
            };
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
