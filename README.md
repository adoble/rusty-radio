# Rusty Radio

An internet radio written in Rust. 

 [!WARNING]
 Under construction


# System Diagram
![](./hardware/system/System.drawio.svg)


# Notes
- Use Embassy (see [Embassy on ESP: Getting Started ](https://dev.to/theembeddedrustacean/embassy-on-esp-getting-started-27fi))

- ESP Rust installtion set up environment variable.  A file was created at 'C:\Users\T440s\export-esp.ps1' showing the injected environment variables. 

- Generated rust project with `cargo generate esp-rs/esp-template`. See the [GitHub repo](https://github.com/esp-rs/esp-template). Project is called `controller`.

- Embassy code examples taken from https://github.com/esp-rs/esp-hal/tree/main/examples/src/bin

- Using the [ESP32-C3-DevKitM-1](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32c3/esp32-c3-devkitm-1/user_guide.html#) board 

- How to  use Embassy in the [Embassy Book](https://embassy.dev/book/)

- Instead of implementing a buffer, try using an Embassy `Channel`. See [this example](https://dev.to/theembeddedrustacean/sharing-data-among-tasks-in-rust-embassy-synchronization-primitives-59hk)

- HHTP Client: Possible crates to try:
https://docs.embassy.dev/embassy-net/...
https://github.com/smoltcp-rs/smoltcp
https://github.com/drogue-iot/reqwless
https://crates.io/crates/picoserve


- [Using the RGB LED on the dev board](https://github.com/kayhannay/esp32-rgb-led/blob/main/src/main.rs)

- HTTP no-std example: https://github.com/Nereuxofficial/nostd-wifi-lamp/blob/main/src/main.rs

- Useful video on how to do wifi and http connectionwith `reqwless` is [here](https://www.youtube.com/watch?v=AC4nZ67Qj20). Also using his code from [GitHub](https://github.com/flyaruu/esp32-nostd/blob/main/src/main.rs)