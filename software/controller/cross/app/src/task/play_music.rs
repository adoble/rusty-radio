use crate::task::sync::{CODEC_DRIVER, MUSIC_PIPE, START_PLAYING};

#[embassy_executor::task]
pub async fn play_music() {
    esp_println::println!("DEBUG: Task play_music started");
    //let mut buffer: [u8; 32] = [0; 32];

    loop {
        let start_playing = START_PLAYING.wait().await;
        if start_playing {
            break;
        }
    }
    esp_println::println!("DEBUG: Task play_music started playing");

    let mut read_buffer = [0u8; 32]; // 32 bytes of data to read from the pipe

    loop {
        // for i in 0..32 {
        //     let b = MUSIC_CHANNEL.receive().await;
        //     buffer[i] = b;
        // }

        let bytes_read = MUSIC_PIPE.read(&mut read_buffer).await;

        {
            let mut driver_unlocked = CODEC_DRIVER.lock().await;
            if let Some(driver) = driver_unlocked.as_mut() {
                //let r = driver.play_data(&buffer).await;
                let r = driver.play_data(&read_buffer[..bytes_read]).await;
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
