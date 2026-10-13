use embedded_io_async_mock::{Mock as SerialMock, Transaction as SerialTransaction};

use radio_control_protocol::{
    RadioCommand, RadioControlProtocol, StationConfig,
    radio_control_protocol::RadioControlProtocolError, uart_handler::UartHandlerError,
};

#[tokio::test]
async fn test_set_station() {
    // Configure expectations
    let tx_message = "STA:5;";
    let rx_message = "ACK:SWR3;";
    let expectations = [
        SerialTransaction::write_many(tx_message.as_bytes()),
        SerialTransaction::flush(),
        SerialTransaction::read_many(rx_message.as_bytes()),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut radio_control_protocol = RadioControlProtocol::new(&mut serial);

    let station_name = radio_control_protocol
        .set_station(5)
        .await
        .expect("Error in setting station");

    assert_eq!("SWR3", station_name);

    serial.done();
}

#[tokio::test]
async fn test_set_station_with_error() {
    // Configure expectations
    let tx_message = "STA:5;";
    let rx_message = "ERR:001;";
    let expectations = [
        SerialTransaction::write_many(tx_message.as_bytes()),
        SerialTransaction::flush(),
        SerialTransaction::read_many(rx_message.as_bytes()),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut radio_control_protocol = RadioControlProtocol::new(&mut serial);

    let result = radio_control_protocol.set_station(5).await;

    assert_eq!(
        Err(RadioControlProtocolError::Uart(
            UartHandlerError::ClientCannotHandleCommand
        )),
        result
    );

    serial.done();
}

#[tokio::test]
async fn test_set_preset() {
    // Configure expectations
    let tx_message = "PRE:2;";
    let rx_message = "ACK:RPR1;";
    let expectations = [
        SerialTransaction::write_many(tx_message.as_bytes()),
        SerialTransaction::flush(),
        SerialTransaction::read_many(rx_message.as_bytes()),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut radio_control_protocol = RadioControlProtocol::new(&mut serial);

    let station_name = radio_control_protocol
        .set_preset(2)
        .await
        .expect("Error in setting preset");

    assert_eq!("RPR1", station_name);

    serial.done();
}

#[tokio::test]
async fn test_query_config() {
    // Configure expectations
    let tx_message = "CFG:;";
    let rx_message = "ACK:7,1,2,3,5;";
    let expectations = [
        SerialTransaction::write_many(tx_message.as_bytes()),
        SerialTransaction::flush(),
        SerialTransaction::read_many(rx_message.as_bytes()),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut radio_control_protocol = RadioControlProtocol::new(&mut serial);

    let station_config = radio_control_protocol
        .query_config()
        .await
        .expect("Error in setting config");

    let expected_station_config = StationConfig::new()
        .with_number_stations(7)
        .with_presets([1, 2, 3, 5]);
    assert_eq!(station_config, expected_station_config);

    serial.done();
}

#[tokio::test]
async fn test_query_config_with_no_presets() {
    // Configure expectations
    let tx_message = "CFG:;";
    let rx_message = "ACK:40;";
    let expectations = [
        SerialTransaction::write_many(tx_message.as_bytes()),
        SerialTransaction::flush(),
        SerialTransaction::read_many(rx_message.as_bytes()),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut radio_control_protocol = RadioControlProtocol::new(&mut serial);

    let station_config = radio_control_protocol
        .query_config()
        .await
        .expect("Error in setting config");

    let expected_station_config = StationConfig::new().with_number_stations(40);

    assert_eq!(station_config, expected_station_config);

    serial.done();
}

#[tokio::test]
async fn test_query_config_with_parameter_error() {
    // Configure expectations
    let tx_message = b"CFG:;";
    let rx_message = b"ACK:;";
    let expectations = [
        SerialTransaction::write_many(tx_message),
        SerialTransaction::flush(),
        SerialTransaction::read_many(rx_message),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut radio_control_protocol = RadioControlProtocol::new(&mut serial);

    let r = radio_control_protocol.query_config().await;

    assert_eq!(
        r,
        Err(RadioControlProtocolError::IncorrectNumberParametersReturned)
    );

    serial.done();
}

#[tokio::test]
async fn test_query_config_with_parse_error() {
    // Configure expectations
    let tx_message = "CFG:;";
    let rx_message = "ACK:XXX;";
    let expectations = [
        SerialTransaction::write_many(tx_message.as_bytes()),
        SerialTransaction::flush(),
        SerialTransaction::read_many(rx_message.as_bytes()),
    ];

    let mut serial = SerialMock::new(&expectations);

    let mut radio_control_protocol = RadioControlProtocol::new(&mut serial);

    let r = radio_control_protocol.query_config().await;

    assert_eq!(r, Err(RadioControlProtocolError::ParseParameter));

    serial.done();
}

//  pub async fn receive_command(&mut self) -> Result<RadioCommand, RadioControlProtocolError> {
#[tokio::test]
async fn test_receive_command() {
    // Configure expectations
    let rx_message = "STA:7;";
    let expectations = [SerialTransaction::read_many(rx_message.as_bytes())];

    let mut serial = SerialMock::new(&expectations);

    let mut radio_control_protocol = RadioControlProtocol::new(&mut serial);

    let command = radio_control_protocol.receive_command().await.unwrap();

    assert_eq!(command, RadioCommand::Station(7));

    serial.done();
}
