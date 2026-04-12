#![cfg_attr(not(test), no_std)]

use embedded_hal_nb::serial::{Read, Write}; // Import the Write trait
use itoa::Buffer;
use nb::block; // Import the block! macro to wait for operations

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

pub fn set_station<S>(serial: &mut S, station_id: u8) -> Result<(), S::Error>
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

    let mut response = [0; 8];
    let mut i = 0;
    loop {
        let c = block!(serial.read())?;
        if c == b';' {
            break;
        }
        response[i] = c;
        i += 1;

        // Optional: Add a timeout or max length check to prevent infinite loop
        // TODO parse the response
    }

    // let mut c: u8 = b' ';

    // while c != b';' {
    //     c = block!(serial.read())?;
    //     // TODO parse the response
    // }

    Ok(())
}
