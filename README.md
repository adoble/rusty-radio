# Rusty Radio

An embedded internet radio written in Rust.

> **⚠️ Work In Progress**
>
> This project is under active development. Features, hardware, and software may change frequently. 

## Supported Formats

Rusty Radio currently supports:
- MP3
- AAC
- M3U

## System Diagram

![](./hardware/system/System.drawio.svg)

## Hardware

- Uses the Seeed Studio [XIAO ESP32C3](https://wiki.seeedstudio.com/XIAO_ESP32C3_Getting_Started/) for its external antenna, making the radio independent of the enclosure.
    - Initially used the [ESP32-C3-DevKitM-1](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32c3/esp32-c3-devkitm-1/user_guide.html#) board.
- Uses the VS1053 chip to decode the streamed audio.
- Schematics are created using KiCad 9.0.

## Software Architecture

- Built with `esp-hal` and [Embassy](https://dev.to/theembeddedrustacean/embassy-on-esp-getting-started-27fi) for async task scheduling.
- Project scaffolded using `cargo generate esp-rs/esp-template`. See the [GitHub repo](https://github.com/esp-rs/esp-template). The software project root is `controller`.

## Development Notes

- ESP Rust installation requires setting up environment variables. Example: `C:\Users\T440s\export-esp.ps1` contains the injected environment variables.
- Embassy code examples are referenced from [esp-hal examples](https://github.com/esp-rs/esp-hal/tree/main/examples/src/bin).
- For more on Embassy, see the [Embassy Book](https://embassy.dev/book/).

---


