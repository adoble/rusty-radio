use crate::task::sync::ACCESS_WEB_SIGNAL;

const DEBOUNCE_DURATION: u64 = 100; // Milliseconds  TODO use fugit?

use esp_hal::gpio::Input;

use embassy_time::{Duration, Timer};

#[embassy_executor::task]
pub async fn button_monitor(mut pin: Input<'static>) {
    loop {
        pin.wait_for_falling_edge().await;

        // Debounce
        // TODO see also https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/debounce.rs
        Timer::after(Duration::from_millis(DEBOUNCE_DURATION)).await;

        if pin.is_low() {
            // Pin is still low so acknowledge
            esp_println::println!("Button pressed after debounce!");

            // Now access the web by sending a signal

            ACCESS_WEB_SIGNAL.signal(true)
        }
    }
}
