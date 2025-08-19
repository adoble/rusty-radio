use embassy_time::{Duration, Timer};

use crate::{front_panel::FrontPanel, task::sync::STATION_CHANGE_WATCH};

#[embassy_executor::task]
pub async fn station_indicator(front_panel: &'static FrontPanel) {
    esp_println::println!("DEBUG: Entered station_indicator task");
    let Some(mut station_change_watch_rcx) = STATION_CHANGE_WATCH.receiver() else {
        panic!("Cannot get station change watch receiver in task:station_indicator")
    };

    loop {
        let station = station_change_watch_rcx.changed().await;

        match station {
            Some(_) => front_panel.set_led_high().await.unwrap(),
            None => front_panel.set_led_low().await.unwrap(),
        }

        Timer::after(Duration::from_millis(5)).await;
    }
}
