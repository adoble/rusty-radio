use crate::task::sync::MUSIC_CHANNEL;

#[embassy_executor::task]
pub async fn read_test_music() {
    // Some mp3 music for testing
    let test_music: &[u8; 55302] = include_bytes!("../../../../resources/music-16b-2c-8000hz.mp3");
    let mut music_iter = test_music.iter().cycle();

    loop {
        if let Some(music_byte) = music_iter.next() {
            MUSIC_CHANNEL.send(*music_byte).await;
        }
    }
}
