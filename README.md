# Gonk

An ESP32-S3 environmental monitoring device built with Rust and [esp-hal](https://github.com/esp-rs/esp-hal). This project explores embedded Rust development while creating a practical display device that shows real-time environmental data: clock, temperature, humidity, and weather information from OpenWeather API.

**Status:** Work in progress. Features are being added incrementally.

## Hardware

- **Microcontroller**: ESP32-S3
- **Display**: I2C display (SSD1306)
- **Sensors**: BMP280 (temperature and pressure)
- **Connectivity**: WiFi for API access

Custom enclosure created with FreeCAD will be shared in the `/models` directory.

## Development Setup

### Prerequisites

- Rust toolchain with `xtensa-esp32s3-none-elf` target
- [espflash](https://github.com/esp-rs/espflash) for flashing

### Configuration

Copy `.env.example` to `.env` and set your WiFi credentials:

```bash
cp .env.example .env
```

### Building and Flashing

```bash
make help              # See all available commands
make build             # Build the project
make flash             # Flash to device
make flash BIN=<name>  # Flash specific binary
```

## Key Technologies

- [esp-hal](https://github.com/esp-rs/esp-hal) - Hardware Abstraction Layer for Espressif chips
- [embassy](https://embassy.dev/) - Async executor for embedded systems
- [embedded-hal](https://github.com/rust-embedded/embedded-hal) - Hardware abstraction traits

## Roadmap

- [x] Basic project setup
- [x] Display integration (SSD1306)
- [x] Temperature sensor (BMP280)
- [x] WiFi connectivity
- [ ] Real-time clock
- [ ] OpenWeather API integration
- [ ] Humidity sensor
- [ ] Web interface for configuration
- [ ] 3D printed enclosure
- [ ] Battery power management

## License

This project is a personal learning exercise and is shared as-is for educational purposes.

## Contributing

While this is primarily a learning project, suggestions and feedback are welcome! Feel free to open issues or discussions.