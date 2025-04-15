use crate::task::sync::MUSIC_CHANNEL;

use embassy_time::{Duration, Timer};

#[embassy_executor::task]
pub async fn system_monitor() {
    loop {
        let channel_free_capacity = MUSIC_CHANNEL.free_capacity();

        esp_println::println!("MUSIC_CHANNEL free capacity: {channel_free_capacity}");

        Timer::after(Duration::from_millis(1000)).await;
    }
}
