use embedded_io_async::{Read, Write};
use mock_uart::MockUart;

#[tokio::test]
async fn test_station() {
    let mut uart = MockUart::new();

    let well_formed_message = b"STA:4;";

    uart.write(well_formed_message).await.unwrap();

    let mut buf = [0u8; 40];

    let response_len = uart.read(&mut buf).await.unwrap();

    let s = str::from_utf8(&buf).unwrap();
    assert_eq!(&s[..response_len], "ACK:Bayern Eins;")
}

#[tokio::test]
async fn test_cfg() {
    let mut uart = MockUart::new();

    let well_formed_message = b"CFG;";

    uart.write(well_formed_message).await.unwrap();

    let mut buf = [0u8; 40];

    let response_len = uart.read(&mut buf).await.unwrap();

    let s = str::from_utf8(&buf).unwrap();
    assert_eq!(&s[..response_len], "ACK:7,2,4,6,1;")
}

#[tokio::test]
#[should_panic]
async fn test_large_message() {
    let mut uart = MockUart::new();

    let large_message = r#"Partition Table:\r\nI (48) boot: ## Label            
    Usage          Type ST Offset   Length\r\nI (54) boot:  0 nvs              
    WiFi data        01 02 00009000 00006000\r\nI (61) boot:  1 phy_init         
    RF data          01 01 0000f000 00001000\r\nI (67) boot:  2 factory          
    factory app      00 00 00010000 003f0000\r\nI (74) boot: End of partition 
    table\r\nI (77) esp_image: segment 0: paddr=00010020 vaddr=3c000020 
    size=053fch ( 21500) map\r\nI (89) esp_image: segment 1: paddr=00015424 
    vaddr=3fc818d8 size=00c24h (  3108) load\r\nI (93) esp_image: segment 2: 
    paddr=00016050 vaddr=40380000 size=018d4h (  6356) load\r\nI (101) 
    esp_image: segment 3: paddr=0001792c vaddr=00000000 size=086ech ( 34540) 
    \r\nI (114) esp_image: segment 4: paddr=00020020 vaddr=42010020 size=144cch 
    ( 83148) map\r\nI (134) boot: Loaded app from partition at offset 0x10000\r\nI (134) 
     boot: Disabling RNG early entropy source...STA:4;"#;
    let large_message = large_message.as_bytes();

    uart.write(large_message).await.unwrap();

    let mut buf = [0u8; 40];

    let response_len = uart.read(&mut buf).await.unwrap();

    let s = str::from_utf8(&buf).unwrap();
    assert_eq!(&s[..response_len], "ACK:Bayern Eins;")
}

#[tokio::test]
async fn test_fragmented_message() {
    let mut uart = MockUart::new();

    let message_part1 = b"STA";
    let message_part2 = b":";
    let message_part3 = b"5";
    let message_part4 = b";";

    uart.write(message_part1).await.unwrap();
    uart.write(message_part2).await.unwrap();
    uart.write(message_part3).await.unwrap();
    uart.write(message_part4).await.unwrap();

    let mut buf = [0u8; 40];

    let response_len = uart.read(&mut buf).await.unwrap();

    let s = str::from_utf8(&buf).unwrap();
    assert_eq!(&s[..response_len], "ACK:Classic;")
}

#[tokio::test]
async fn test_write_all() {
    let mut uart = MockUart::new();

    let message = b"CFG:;";

    uart.write_all(message).await.unwrap();

    let mut buf = [0u8; 40];

    let response_len = uart.read(&mut buf).await.unwrap();

    let s = str::from_utf8(&buf).unwrap();
    assert_eq!(&s[..response_len], "ACK:7,2,4,6,1;")
}

#[tokio::test]
async fn test_buf_larger_than_frame_buffer() {
    let mut uart = MockUart::new();

    let message_81: [u8; 81] = core::array::from_fn(|i| {
        let b = b'!' + i as u8;
        if b >= b';' { b + 1 } else { b }
    });

    assert!(!message_81.contains(&b';'));
    assert_eq!(message_81.len(), 81);

    let n = uart.write(&message_81).await.unwrap();

    assert_eq!(n, 80);
}

#[tokio::test]
async fn test_multiple_reads() {
    let mut uart = MockUart::new();

    let mut buf = [0u8; 4];

    let msg = b"CFG:;";

    uart.write_all(msg).await.unwrap();

    uart.read(&mut buf).await.unwrap();

    // b"ACK:7,2,4,6,1;";

    assert_eq!(&buf, b"ACK:");

    let mut buf = [0u8; 10];

    uart.read(&mut buf);

    assert_eq!(&buf, b"7,2,4,6,1;");
}
