# HW Tools

**Hardware diagnostic Swiss army knife for Precursor**

A tabbed hardware diagnostic utility that combines System Monitor, GPIO Tester, and UART Monitor into a single app for the [Precursor](https://www.crowdsupply.com/sutajio-kosagi/precursor) hardware platform running Xous OS.

## Features

### Tab 1: System Monitor

Real-time hardware telemetry and system information:

| Category | Data Points |
|----------|-------------|
| **Battery** | Voltage, charge %, current draw, remaining capacity |
| **Charging** | Status indicator with visual progress bar |
| **EC Info** | Uptime, git revision, firmware version |
| **SoC Info** | Git revision, unique DNA identifier |
| **ADC** | VBUS voltage, die temperature, VccInt, VccAux |
| **Sensors** | Gyroscope/accelerometer X/Y/Z values |

### Tab 2: GPIO Tester

Monitor and control the 8 GPIO pins exposed in the battery compartment:

| Pin | ADC | Description |
|-----|-----|-------------|
| 0 | No | General purpose I/O |
| 1 | No | General purpose I/O |
| 2 | **Yes** | ADC capable - voltage measurement |
| 3 | No | General purpose I/O |
| 4 | No | General purpose I/O |
| 5 | **Yes** | ADC capable - voltage measurement |
| 6 | No | General purpose I/O |
| 7 | No | General purpose I/O |

Features:
- Direction control (input/output) for all 8 pins
- Analog voltage measurement for pins 2 and 5
- Visual selection cursor and pin state display
- Settings persistence across sessions

### Tab 3: UART Monitor

Serial interface monitoring and control:

- **Mux Selector**: Switch between Kernel, Log, and Application UARTs
- **Traffic Log**: Timestamped message display with scroll support
- **TX Input**: Text input field for sending data
- **Pause/Resume**: Control log capture

## Keyboard Controls

### Global Keys

| Key | Action |
|-----|--------|
| `1` / `2` / `3` | Switch to tab 1/2/3 |
| `←` / `→` | Previous/Next tab |
| `q` | Quit application |

### System Tab

| Key | Action |
|-----|--------|
| `r` | Force refresh all stats |

### GPIO Tab

| Key | Action |
|-----|--------|
| `0`-`7` | Toggle GPIO pin (or select & set to output) |
| `i` | Set selected pin to input mode |
| `o` | Set selected pin to output mode |
| `↑` / `↓` | Select pin (cursor) |
| `Space` | Toggle selected output pin |
| `a` | Show ADC value (pins 2 & 5 only) |

### UART Tab

| Key | Action |
|-----|--------|
| `c` | Clear log buffer |
| `p` | Pause/Resume capture |
| `m` | Cycle UART mux (Kernel → Log → App) |
| `↑` / `↓` | Scroll log |
| `Enter` | Send TX buffer |
| `Backspace` | Delete character |
| *Type* | Add characters to TX buffer |

## Installation

### Prerequisites

- Rust toolchain with riscv32 target
- [xous-core](https://github.com/betrusted-io/xous-core) repository

### Setup

1. **Clone into xous-core:**
   ```bash
   cd xous-core/apps
   git clone https://github.com/tbcolby/precursor-hwtools.git hwtools
   ```

2. **Add to workspace** (`xous-core/Cargo.toml`):
   ```toml
   # In the [workspace] members array, add:
   "apps/hwtools",
   ```

3. **Add manifest entry** (`xous-core/apps/manifest.json`):
   ```json
   "hwtools": {
       "context_name": "HW Tools",
       "menu_name": {
           "appmenu.hwtools": {
               "en": "HW Tools",
               "en-tts": "Hardware Tools"
           }
       }
   }
   ```

### Build Commands

```bash
cd xous-core

# Build for Renode emulator
cargo xtask renode-image hwtools

# Build for real Precursor hardware
cargo xtask app-image hwtools

# Build with XIP (execute-in-place, saves RAM)
cargo xtask app-image-xip hwtools
```

### Running

After flashing, the app appears in **Menu → Switch to App → HW Tools**.

## Hardware Notes

### UART Mux Modes

| Mode | Baud | Description |
|------|------|-------------|
| Kernel | 115200 | Debug output from kernel |
| Log | 115200 | Log server output (default) |
| Application | 115200 | Application UART (power-sensitive) |

### ADC Specifications

| Channel | Range | Resolution |
|---------|-------|------------|
| VBUS | 0-5V | 12-bit |
| Temperature | ~0-100°C | Approximate |
| VccInt | 0-3V | 12-bit |
| VccAux | 0-3V | 12-bit |
| GPIO 2/5 | 0-3.3V | 12-bit |

## Known Limitations

| Feature | Status | Notes |
|---------|--------|-------|
| GPIO Output | Direction only | `gpio_data_out()` not exposed in llio API |
| GPIO Input | Pins 2 & 5 only | Other pins require direct register access |
| UART RX | Placeholder | Bidirectional serial requires kernel support |
| Temperature | Approximate | Raw ADC conversion, not calibrated |
| Gyroscope | Raw values | Accelerometer readings, not calibrated |

## Settings Persistence

Settings are stored in PDDB under `hwtools.settings`:

- Last active tab
- GPIO pin directions
- GPIO output values
- UART mux selection

---

## Author

Made by Tyler Colby — [Colby's Data Movers, LLC](https://colbysdatamovers.com)

Contact: [tyler@colbysdatamovers.com](mailto:tyler@colbysdatamovers.com) | [GitHub Issues](https://github.com/tbcolby/precursor-hwtools/issues)

## License

Licensed under the Apache License, Version 2.0.
