use crate::task::sync::{CODEC_DRIVER, MUSIC_CHANNEL};

#[embassy_executor::task]
pub async fn play_music() {
    let mut buffer: [u8; 32] = [0; 32];
    loop {
        for i in 0..32 {
            let b = MUSIC_CHANNEL.receive().await;
            buffer[i] = b;
        }

        {
            let mut driver_unlocked = CODEC_DRIVER.lock().await;
            if let Some(driver) = driver_unlocked.as_mut() {
                let r = driver.play_data(&buffer).await;
                match r {
                    Ok(_) => continue,
                    Err(err) => {
                        esp_println::println!("Error {:?} in play music", err);
                        break;
                    }
                };
            }
        }
    }
}
