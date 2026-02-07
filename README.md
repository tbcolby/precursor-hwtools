# HW Tools

**A stethoscope for an open-source heart.**

```
"You can't trust what you can't measure."
```

---

## What This Is

HW Tools is a hardware diagnostic utility for the [Precursor](https://precursor.dev) platform running Xous OS. It combines a System Monitor, GPIO Tester, and UART Monitor into a single tabbed application that gives you real-time visibility into what your hardware is actually doing. Battery voltage, charge current, die temperature, GPIO pin states, serial traffic -- all live, all on-device, no external tools required.

This is not a benchmark. It is not a stress test. It is an instrument panel for a machine whose blueprints are public but whose runtime behavior is opaque without something to observe it. HW Tools turns the invisible into the legible.

---

## Why This Project

bunnie Huang built Precursor so you could audit the hardware. The schematics are open. The FPGA gateware is open. The bootloader, the kernel, the operating system -- all open, all auditable. His book [*The Hardware Hacker*](https://www.penguinrandomhouse.com/books/534773/the-hardware-hacker-by-andrew-bunnie-huang/) makes the case that open hardware is a prerequisite for trust: if you cannot inspect the system, you cannot know what it is doing.

But open hardware means nothing if you cannot observe it running.

Schematics tell you what the hardware *should* do. Firmware tells you what the software *intends* to do. Only measurement tells you what the system *is* doing. HW Tools extends the audit principle from design-time to runtime. You can read the schematic for the battery charge controller, but can you see the charge current right now? You can review the GPIO pin mux logic in the FPGA source, but can you confirm which pins are actually asserting? You can trace the UART path through the kernel, but can you see what bytes are crossing the wire?

Open hardware demands open diagnostics. HW Tools is Precursor's instrument cluster -- the runtime half of the trust equation.

---

## Why Precursor

Precursor's constraints shaped this app in ways that matter.

The **COM service** mediates all communication with the embedded controller (EC), which owns the battery gas gauge, power management, and sensor subsystems. There is no direct register access from the SoC -- every measurement is an IPC round-trip through the COM server, which polls the EC over an internal SPI bus. The telemetry refresh rate is bounded by this pipeline, not by display speed.

The **llio API** exposes GPIO direction control but not output data registers directly. Setting a pin to output mode is straightforward; driving a specific logic level requires working within the API's current surface. The GPIO tester maps what is possible today and documents what is not yet exposed.

**Renode versus real hardware** creates a persistent gap. The emulator faithfully models the SoC and can run the app, but COM service responses for battery data, sensor readings, and ADC values are either stubbed or absent. HW Tools is most useful on real Precursor hardware, where the measurements correspond to physical reality. In Renode, the UI renders but the numbers are placeholders.

These are not complaints. They are the boundaries of the system, and boundaries are what make engineering possible.

---

## How It Works

### Tab 1: System Monitor

Real-time hardware telemetry and system information, refreshed every 2 seconds via a pump thread:

| Category | Data Points |
|----------|-------------|
| **Battery** | Voltage, charge %, current draw, remaining capacity |
| **Charging** | Status indicator with visual progress bar |
| **EC Info** | Uptime, git revision, firmware version |
| **SoC Info** | Git revision, unique DNA identifier |
| **ADC** | VBUS voltage, die temperature, VccInt, VccAux |
| **Sensors** | Gyroscope/accelerometer X/Y/Z values |

**Keyboard:**

| Key | Action |
|-----|--------|
| `r` | Force refresh all stats |

### Tab 2: GPIO Tester

Monitor and control the 8 GPIO pins exposed in the battery compartment:

| Pin | ADC | Description |
|-----|-----|-------------|
| 0 | No | General purpose I/O |
| 1 | No | General purpose I/O |
| 2 | **Yes** | ADC capable -- voltage measurement |
| 3 | No | General purpose I/O |
| 4 | No | General purpose I/O |
| 5 | **Yes** | ADC capable -- voltage measurement |
| 6 | No | General purpose I/O |
| 7 | No | General purpose I/O |

Features:
- Direction control (input/output) for all 8 pins
- Analog voltage measurement for pins 2 and 5
- Visual selection cursor and pin state display
- Settings persistence across sessions

**Keyboard:**

| Key | Action |
|-----|--------|
| `0`-`7` | Toggle GPIO pin (or select and set to output) |
| `i` | Set selected pin to input mode |
| `o` | Set selected pin to output mode |
| `↑` / `↓` | Select pin (cursor) |
| `Space` | Toggle selected output pin |
| `a` | Show ADC value (pins 2 and 5 only) |

### Tab 3: UART Monitor

Serial interface monitoring and control:

- **Mux Selector**: Switch between Kernel, Log, and Application UARTs
- **Traffic Log**: Timestamped message display with scroll support
- **TX Input**: Text input field for sending data
- **Pause/Resume**: Control log capture

**Keyboard:**

| Key | Action |
|-----|--------|
| `c` | Clear log buffer |
| `p` | Pause/Resume capture |
| `m` | Cycle UART mux (Kernel, Log, App) |
| `↑` / `↓` | Scroll log |
| `Enter` | Send TX buffer |
| `Backspace` | Delete character |
| *Type* | Add characters to TX buffer |

### Global Keys

| Key | Action |
|-----|--------|
| `1` / `2` / `3` | Switch to tab 1/2/3 |
| `←` / `→` | Previous/Next tab |
| `q` | Quit application |

---

## Technical Architecture

```
apps/hwtools/
├── Cargo.toml       # Dependencies: xous, gam, llio, com, pddb, ticktimer
└── src/
    ├── main.rs      # Entry point, pump thread, event loop, focus management
    ├── app.rs       # Tab state machine, rendering dispatch, key routing
    ├── system_tab.rs # COM/llio telemetry queries, system info display
    ├── gpio_tab.rs  # GPIO direction/state control, ADC readout
    ├── uart_tab.rs  # UART mux switching, RX log buffer, TX input
    ├── storage.rs   # PDDB settings persistence (binary format)
    └── ui.rs        # Shared drawing helpers, screen clear
```

### Design Decisions

**Chat UX** (`UxType::Chat`): Uses the standard managed canvas rather than raw framebuffer. Text-heavy diagnostic output benefits from the GAM's text rendering pipeline. Tab headers use rounded rectangles with inverted text for the active tab.

**Pump thread pattern**: A dedicated thread sends blocking scalar messages to the main loop at 2-second intervals, triggering telemetry refresh. The main loop acknowledges each pump, providing natural backpressure. The pump thread is paused when the app goes to background and resumed on foreground -- no wasted cycles reading hardware that nobody is watching.

**Tab state machine**: Three tabs (`System`, `Gpio`, `Uart`) with a simple enum-based state machine. Each tab module owns its own data, drawing, and key handling. The `app.rs` coordinator dispatches to the active tab and manages transitions. Tab switching triggers a full redraw; periodic refreshes only redraw the content area.

**Binary settings format**: Rather than JSON or any text serialization, settings are stored as a compact 9-byte binary blob: `[last_tab, gpio_directions, gpio_outputs, uart_mux, auto_refresh, refresh_interval_ms(4)]`. This minimizes PDDB overhead and parse complexity.

### Hardware Specifications

**UART Mux Modes:**

| Mode | Baud | Description |
|------|------|-------------|
| Kernel | 115200 | Debug output from kernel |
| Log | 115200 | Log server output (default) |
| Application | 115200 | Application UART (power-sensitive) |

**ADC Specifications:**

| Channel | Range | Resolution |
|---------|-------|------------|
| VBUS | 0-5V | 12-bit |
| Temperature | ~0-100C | Approximate |
| VccInt | 0-3V | 12-bit |
| VccAux | 0-3V | 12-bit |
| GPIO 2/5 | 0-3.3V | 12-bit |

### PDDB Storage Layout

All settings stored under dictionary `hwtools.settings`, key `config`:

| Field | Byte Offset | Size | Persistence |
|-------|-------------|------|-------------|
| `last_tab` | 0 | 1 byte | Per session |
| `gpio_directions` | 1 | 1 byte (bitmask) | Across sessions |
| `gpio_outputs` | 2 | 1 byte (bitmask) | Across sessions |
| `uart_mux` | 3 | 1 byte | Across sessions |
| `auto_refresh` | 4 | 1 byte (bool) | Across sessions |
| `refresh_interval_ms` | 5-8 | 4 bytes (u32 LE) | Across sessions |

Total footprint: 9 bytes.

### Known Limitations

| Feature | Status | Notes |
|---------|--------|-------|
| GPIO Output | Direction only | `gpio_data_out()` not exposed in llio API |
| GPIO Input | Pins 2 and 5 only | Other pins require direct register access |
| UART RX | Placeholder | Bidirectional serial requires kernel support |
| Temperature | Approximate | Raw ADC conversion, not calibrated |
| Gyroscope | Raw values | Accelerometer readings, not calibrated |

---

## Building

HW Tools is a Xous app. It builds as part of the [xous-core](https://github.com/betrusted-io/xous-core) workspace.

### Integration

1. Clone into xous-core:
   ```bash
   cd xous-core/apps
   git clone https://github.com/tbcolby/precursor-hwtools.git hwtools
   ```

2. Add to workspace `Cargo.toml`:
   ```toml
   "apps/hwtools",
   ```

3. Add to `apps/manifest.json`:
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

4. Build for Renode emulator:
   ```bash
   cargo xtask renode-image hwtools
   ```

5. Build for hardware:
   ```bash
   cargo xtask app-image hwtools
   ```

6. Build with XIP (execute-in-place, saves RAM):
   ```bash
   cargo xtask app-image-xip hwtools
   ```

After flashing, the app appears in **Menu -> Switch to App -> HW Tools**.

### Dependencies

```toml
[dependencies]
xous = "0.9.69"
xous-ipc = "0.10.9"
gam = { path = "../../services/gam" }
llio = { path = "../../services/llio" }
com = { path = "../../services/com" }
pddb = { path = "../../services/pddb" }
ticktimer-server = { package = "xous-api-ticktimer", version = "0.9.68" }
log-server = { package = "xous-api-log", version = "0.1.68" }
xous-names = { package = "xous-api-names", version = "0.9.70" }
xous-semver = "0.1.5"
num-derive = { version = "0.4.2", default-features = false }
num-traits = { version = "0.2.14", default-features = false }
log = "0.4.14"
```

---

## Screenshots

Screenshots will be added after Renode capture.

---

## Development
---

This app was developed using the methodology described in [xous-dev-toolkit](https://github.com/tbcolby/xous-dev-toolkit) -- an LLM-assisted approach to Precursor app development on macOS ARM64.

## Author
---

Made by Tyler Colby -- [Colby's Data Movers, LLC](https://colbysdatamovers.com)

Contact: [tyler@colbysdatamovers.com](mailto:tyler@colbysdatamovers.com) | [GitHub Issues](https://github.com/tbcolby/precursor-hwtools/issues)

## License
---

Licensed under the Apache License, Version 2.0.

See [LICENSE](LICENSE) for the full text.
