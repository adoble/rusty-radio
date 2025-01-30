use core::str::from_utf8;

use crate::task::sync::TEST_CHANNEL;

#[embassy_executor::task]
pub async fn display_web_content() {
    loop {
        let data = TEST_CHANNEL.receive().await;

        let content = from_utf8(&data).unwrap();
        esp_println::print!("{content}");
    }
}
