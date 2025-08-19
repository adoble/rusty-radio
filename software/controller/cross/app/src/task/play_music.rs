use crate::task::sync::{CODEC_DRIVER, MUSIC_PIPE, START_PLAYING};

#[embassy_executor::task]
pub async fn play_music() {
    esp_println::println!("DEBUG: Entered play_music task");
    loop {
        let start_playing = START_PLAYING.wait().await;
        if start_playing {
            break;
        }
    }

    let mut read_buffer = [0u8; 32]; // 32 bytes of data to read from the pipe

    loop {
        let bytes_read = MUSIC_PIPE.read(&mut read_buffer).await;

        {
            let mut driver_unlocked = CODEC_DRIVER.lock().await;
            if let Some(driver) = driver_unlocked.as_mut() {
                let r = driver.play_data(&read_buffer[..bytes_read]).await;
                match r {
                    Ok(_) => continue,
                    Err(err) => {
                        esp_println::println!("ERROR: {:?} in play music", err);
                        break;
                    }
                };
            }
        }
    }
}
