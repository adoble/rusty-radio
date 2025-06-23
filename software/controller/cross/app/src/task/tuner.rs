use crate::task::sync::STATION_CHANGE_WATCH;

const DEBOUNCE_DURATION: u64 = 100; // Milliseconds  TODO use fugit?

use esp_hal::gpio::Input;

use embassy_time::{Duration, Timer};

use stations::Stations;

#[embassy_executor::task]
#[deprecated]
pub async fn tuner(mut pin: Input<'static>) {
    //Set up the list of stations
    let mut stations = Stations::new();

    let station_change_sender = STATION_CHANGE_WATCH.sender();

    // Send the inital station
    let initial_station = stations
        .get_station(0)
        .expect("ERROR: Could not set intial station (0)");
    station_change_sender.send(initial_station);

    loop {
        pin.wait_for_falling_edge().await;

        // Debounce
        // TODO see also https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/debounce.rs
        Timer::after(Duration::from_millis(DEBOUNCE_DURATION)).await;

        if pin.is_low() {
            // Pin is still low so acknowledge

            let mut station = stations.next();
            if station.is_none() {
                station = stations.get_station(0);
            }

            esp_println::println!("\n\nINFO: Playing: {}\n\n", station.clone().unwrap().name());

            station_change_sender.send(station.unwrap().clone());
        }
    }
}
