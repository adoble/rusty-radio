use embassy_time::Timer;
use esp_hal::gpio::Output;

use crate::task::sync::WIFI_CONNECTED_SIGNAL;

/// Task to check  if the wifi is connected and, if so, light the led

#[embassy_executor::task]
pub async fn wifi_connected_indicator(mut led: Output<'static>) {
    esp_println::println!("DEBUG: task wifi_connected_indicator started");
    loop {
        let connected = WIFI_CONNECTED_SIGNAL.wait().await;
        if connected {
            led.set_high();
        } else {
            led.set_low();
        }

        Timer::after_millis(1000).await;
    }
}
