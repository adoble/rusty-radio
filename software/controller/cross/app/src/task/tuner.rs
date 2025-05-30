use crate::task::sync::STATION_CHANGE_WATCH;

const DEBOUNCE_DURATION: u64 = 100; // Milliseconds  TODO use fugit?

use esp_hal::gpio::Input;

use embassy_time::{Duration, Timer};

use stations::{Station, Stations};

#[embassy_executor::task]
pub async fn tuner(mut pin: Input<'static>) {
    // Set up the list of stations
    // let mut stations = Stations::new();
    // stations
    //     .load_stations()
    //     .expect("Cannot initialise the stations");

    let mut current_sender_id = 0;

    let station_change_sender = STATION_CHANGE_WATCH.sender();

    loop {
        pin.wait_for_falling_edge().await;

        // Debounce
        // TODO see also https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/debounce.rs
        Timer::after(Duration::from_millis(DEBOUNCE_DURATION)).await;

        if pin.is_low() {
            // Pin is still low so acknowledge
            esp_println::println!("Button pressed after debounce!");
            current_sender_id += 1;
            // if current_sender_id >= stations.number_stations() {
            //     current_sender_id = 0;
            // }

            // let station = stations
            //     .get_station(current_sender_id)
            //     .expect("ERROR: Station {current_station_id} not found!");
            // esp_println::println!("\nSTATION: {}\n", station.name());

            let station = Station::new("SWR4", "http://liveradio.swr.de/sw282p3/swr4bw/")
                .expect("ERROR: Could not create station");
            station_change_sender.send(station);
        }
    }
}
