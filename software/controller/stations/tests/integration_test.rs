use stations::Stations;

const MAX_STATION_NAME_LEN: usize = 32;
const MAX_STATION_URL_LEN: usize = 256;
const NUMBER_PRESETS: usize = 4;

#[test]
fn test_load() {
    let data = include_bytes!("resources/stations.txt");

    let stations =
        Stations::<MAX_STATION_NAME_LEN, MAX_STATION_URL_LEN, NUMBER_PRESETS>::load(data).unwrap();
    let station = stations.get_station(5).unwrap();

    // BBC Radio 3,http://stream.live.vc.bbcmedia.co.uk/bbc_radio_three,UK,Classical
    assert_eq!("BBC Radio 3", station.name());
    assert_eq!(
        "http://stream.live.vc.bbcmedia.co.uk/bbc_radio_three",
        station.url()
    );
}

#[test]
fn test_name_error() {
    let data = "Antenne,http://mp3channels.webradio.antenne.de/antenne,Pop".as_bytes();

    // Small name length
    let stations = Stations::<4, MAX_STATION_URL_LEN, NUMBER_PRESETS>::load(data);

    assert!(stations.is_err());

    // Name just at limit
    let stations = Stations::<7, MAX_STATION_URL_LEN, NUMBER_PRESETS>::load(data);

    assert!(stations.is_ok());
}

#[test]
fn test_url_error() {
    let data = "Antenne,http://ir.de/m.mp3,Pop".as_bytes();

    // Small name length
    let stations = Stations::<64, 17, NUMBER_PRESETS>::load(data);

    assert!(stations.is_err());

    // Name just at limit
    let stations = Stations::<7, 18, NUMBER_PRESETS>::load(data);

    assert!(stations.is_ok());
}
