#![cfg_attr(not(test), no_std)]

use embedded_hal_nb::serial::{Read, Write}; // Import the Write trait
use heapless::{String, Vec};
use itoa::Buffer;
use nb::block; // Import the block! macro to wait for operations

#[deprecated(note = "Make this generic")]
pub const MAX_STATION_NAME_LEN: usize = 40;

// Assumes 'serial' is a pre-configured UART instance (e.g., from a device HAL)
// e.g., let mut serial = stm32f4xx_hal::serial::Serial::new(...);

#[deprecated(note = "Only using this as reference code. Remove later")]
pub fn send_hello<S>(serial: &mut S) -> Result<(), S::Error>
where
    S: Write<u8>, // S must implement Serial Write trait for u8
{
    let message = b"Hello";
    for &byte in message {
        // block! waits until the UART is ready to send the byte
        block!(serial.write(byte))?;
    }
    // Optional: wait for transmission to finish
    block!(serial.flush())?;
    Ok(())
}

pub fn set_station<S>(
    serial: &mut S,
    station_id: u8,
) -> Result<String<MAX_STATION_NAME_LEN>, S::Error>
where
    S: Write<u8> + Read<u8>, // S must implement Serial Write trait for u8
{
    let command = b"STA:";
    for &byte in command {
        block!(serial.write(byte))?;
    }
    let mut buffer = Buffer::new();
    let station_id_str = buffer.format(station_id).as_bytes();
    for &byte in station_id_str {
        block!(serial.write(byte))?;
    }
    block!(serial.write(b';'))?;

    block!(serial.flush())?;

    // TODO change this into a Vec to save lots of conversions
    const MAX_RESPONSE_LEN: usize = MAX_STATION_NAME_LEN + 5;
    let mut rx_bytes = Vec::<u8, MAX_RESPONSE_LEN>::new();
    loop {
        let c = block!(serial.read())?;
        rx_bytes.push(c).expect("TODO: proper error handling");
        if c == b';' {
            break;
        }

        // Optional: Add a timeout or max length check to prevent infinite loop
        // TODO parse the response
    }

    let response =
        String::<MAX_RESPONSE_LEN>::from_utf8(rx_bytes).expect("TODO: Some propoer error handling");

    // Parse the response
    let mut station_name = String::<MAX_STATION_NAME_LEN>::new();
    match response[0..4].as_bytes() {
        b"ACK:" => {
            let terminator_pos = response.find(';').expect("TODO proper error handling");

            station_name
                .push_str(&response[4..terminator_pos])
                .expect("TODO: Proper error handling");
        }
        b"ERR:" => {
            todo!("ERR")
        }
        _ => todo!("proper error handling"),
    };

    Ok(station_name)
}
