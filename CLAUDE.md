# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`iio_ambient_brightness` is a Rust daemon that automatically adjusts keyboard and screen brightness based on ambient light sensor readings from Industrial I/O (IIO) devices on Linux systems. The application operates in client-server architecture with Unix socket communication.

## Development Commands

### Building and Running
```bash
# Build the project
cargo build

# Build optimized release version
cargo build --release

# Run the daemon in server mode
cargo run -- --server

# Run client commands
cargo run -- --idle        # Set to idle mode (dimmed)
cargo run -- --active      # Set to active mode (normal)
cargo run -- --increase 5  # Increase brightness offset
cargo run -- --decrease 3  # Decrease brightness offset
```

### Development Environment with Nix
```bash
# Enter development shell with all dependencies
nix develop

# Build using Nix
nix build

# Run the built binary
./result/bin/iio_ambient_brightness --server
```

### Testing and Formatting
```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Check for compilation errors
cargo check
```

## Architecture

### Core Components

- **main.rs**: Entry point with CLI parsing and controller orchestration
- **ambient_brightness.rs**: IIO sensor interface with weighted moving average smoothing
- **screen_brightness.rs**: Screen backlight control via systemd-logind DBus interface
- **kbd_brightness.rs**: Keyboard backlight control via systemd-logind DBus interface
- **control_server.rs**: Unix socket server for IPC commands using mio for async I/O
- **control_client.rs**: Unix socket client for sending brightness commands

### Key Architecture Patterns

1. **Self-Referencing Structs**: Uses `ouroboros` crate for `AmbientBrightnessController` to manage complex borrowing relationships with DBus proxies
2. **Channel-Based Communication**: Crossbeam channels for command passing between server thread and main controller
3. **Event Loop**: Main controller uses `crossbeam::select!` for handling timer ticks, shutdown signals, and commands
4. **Retry Logic**: Network operations use exponential backoff retry patterns
5. **Signal Handling**: Graceful shutdown via Ctrl-C handling with atomic boolean coordination

### Brightness Mapping

- **Ambient Sensor**: Reads raw IIO values, applies log10 transformation, uses weighted moving average smoothing
- **Screen Brightness**: Non-linear mapping from ambient percentage to screen brightness levels (2-50%)
- **Keyboard Brightness**: Discrete levels (0-3) based on ambient light thresholds with inverted relationship (brighter ambient = dimmer keyboard)

### Hardware Integration

- **IIO Device**: Expects ambient light sensor at `/sys/bus/iio/devices/iio:device*` with "als" name
- **Screen Backlight**: Controls via `/sys/class/backlight/intel_backlight/` (configurable)
- **Keyboard Backlight**: Controls via `/sys/class/leds/asus::kbd_backlight/` (configurable)
- **DBus**: Uses systemd-logind session proxy for brightness adjustments

### Communication Protocol

Unix socket at `/tmp/ambient_brightness.sock` with binary protocol:
- `0`: Idle command
- `1`: Active command  
- `2 + i8`: Increase brightness by amount
- `3 + i8`: Decrease brightness by amount

## Dependencies

Key external dependencies:
- `industrial-io`: IIO sensor interface (configured to use libiio v0.24 compatibility)
- `logind-zbus`: systemd-logind DBus integration
- `yata`: Weighted moving average calculations
- `mio`: Async I/O for socket server
- `crossbeam`: Channel-based concurrency
- `ouroboros`: Self-referencing struct generation

### Important Notes

The project uses `industrial-io` with explicit `libiio_v0_24` feature to ensure compatibility with the libiio system library. The default features are disabled to avoid conflicts with newer libiio versions that are not yet supported by the Rust crate.

The project requires libiio system library and is optimized for embedded/resource-constrained environments with aggressive compiler optimizations.