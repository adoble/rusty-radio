use crate::task::sync::MUSIC_PIPE;

#[embassy_executor::task]
pub async fn read_test_music() {
    // Some mp3 music for testing
    let test_music: &[u8; 55302] = include_bytes!("../../../../resources/music-16b-2c-8000hz.mp3");
    //let test_music: [u8; 1] = [0; 1];
    //let mut music_iter = test_music.iter().cycle();
    let mut music_iter = test_music.chunks(32).cycle();

    loop {
        if let Some(music_chunk) = music_iter.next() {
            MUSIC_PIPE.write(music_chunk).await;
        }
    }
}
