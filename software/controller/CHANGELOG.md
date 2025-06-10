# Changelog
All notable changes to the Rusty Radio Controller project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.5]

### Added 

- User agent derived from manifest file. 

## [0.0.4]

### Added

- Significant code changes to allow more than one station to be tuned.


## [0.0.3]

Proof of concept version. 

### Changed
- Switch target board from ESP32-C3-DevKit_M to Seeed XIAO ESP32C3 to generic ESP32-C3
- Update esp-hal dependencies to v0.23.1
- Fix esp-alloc dependency to use stable version

### Performance
- Optimize WiFi configuration:
  - RX queue size increased to 20
  - TX queue size set to 5
  - Dynamic RX/TX buffer numbers set to 16
  - Enable AMPDU for both RX and TX
  - Set RX BA window to 6
  - Set max burst size to 8
- Build optimizations:
  - Enable LTO (Link Time Optimization)
  - Set optimization level to 3 for speed

### Added
- Configuration for WiFi buffer tuning
- Performance monitoring for streaming

### Fixed
- Download speed was too slow for internet radio
- ESP-alloc dependency issues
- Build configuration for release mode

### Known Issues
- This currently only plays one station and nothing much else!

## [0.0.2] - 2025-05-12

### Added
- Initial implementation of internet radio streaming
- Basic VS1053 codec driver support
- WiFi connectivity
- Audio streaming pipeline

[Unreleased]: https://github.com/adoble/rusty-radio/compare/v0.0.2...HEAD
[0.0.2]: https://github.com/adoble/rusty-radio/releases/tag/v0.0.2