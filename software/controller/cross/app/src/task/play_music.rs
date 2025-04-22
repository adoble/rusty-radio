use crate::task::sync::{CODEC_DRIVER, MUSIC_CHANNEL, START_PLAYING};

#[embassy_executor::task]
pub async fn play_music() {
    esp_println::println!("DEBUG: Task play_music started");
    let mut buffer: [u8; 32] = [0; 32];

    loop {
        let start_playing = START_PLAYING.wait().await;
        if start_playing {
            break;
        }
    }
    esp_println::println!("DEBUG: Task play_music started playing");

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
