#![cfg_attr(not(test), no_std)]

use embedded_hal_nb::serial::Write; // Import the Write trait
use nb::block; // Import the block! macro to wait for operations

// Assumes 'serial' is a pre-configured UART instance (e.g., from a device HAL)
// e.g., let mut serial = stm32f4xx_hal::serial::Serial::new(...);

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
