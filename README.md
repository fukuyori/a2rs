# A2RS — Apple II Emulator in Rust

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.4.2-green.svg)](https://github.com/user/a2rs/releases)

[日本語版 README](README_ja.md)

**A2RS** is a high-accuracy Apple II emulator written in Rust. It focuses on specification-based implementation with emphasis on understanding Apple II hardware architecture at a deep level, rather than copying existing emulator code.

<p align="center">
  <img src="docs/screenshot.png" alt="A2RS Screenshot" width="640">
</p>

## Features

| Feature | Description |
|---------|-------------|
| **Multi-Model Support** | Apple II, II+, IIe, IIe Enhanced |
| **High Performance** | 200+ MHz equivalent speed (release build) |
| **Cycle Accurate** | Passes Klaus2m5 6502 functional test suite |
| **Disk II Emulation** | DSK / DO / PO / NIB / WOZ formats with fast-disk acceleration |
| **Accurate Video** | Text, Lo-Res, Hi-Res, Double Hi-Res modes |
| **Gamepad Support** | Joystick / gamepad input for paddle emulation, JSON-configurable |
| **Audio Emulation** | 1-bit speaker click emulation with volume control |
| **Built-in Profiler** | Performance analysis and boot timing |
| **Debugger UI** | Real-time CPU / memory / disk monitoring |
| **Save States** | Quick save / load with 10 slots |
| **Flexible Configuration** | JSON configuration file with per-platform default paths |

---

## Quick Start

```bash
# Clone and build
git clone https://github.com/user/a2rs.git
cd a2rs
cargo build --release

# Run with a disk image
./target/release/a2rs -r roms/apple2e.rom -1 disks/dos33.dsk
```

---

## Requirements

### Rust

Rust 1.70 or later.

### System Dependencies

<details>
<summary><b>Linux (Debian / Ubuntu)</b></summary>

```bash
# Required
sudo apt-get install libxkbcommon-dev libwayland-dev

# Required for clipboard support
sudo apt-get install libxcb-xfixes0-dev

# Optional: audio support
sudo apt-get install libasound2-dev

# Optional: gamepad support
sudo apt-get install libudev-dev
```
</details>

<details>
<summary><b>Linux (Fedora)</b></summary>

```bash
# Required
sudo dnf install libxkbcommon-devel wayland-devel

# Optional: audio support
sudo dnf install alsa-lib-devel

# Optional: gamepad support
sudo dnf install systemd-devel
```
</details>

<details>
<summary><b>macOS / Windows</b></summary>

No additional system dependencies required.
</details>

---

## Building

```bash
# Standard build (audio + gamepad enabled by default)
cargo build --release

# Build without optional features
cargo build --release --no-default-features

# Build with all features
cargo build --release --features full

# Show options
./target/release/a2rs --help
```

---

## Usage

### Basic Examples

```bash
# Boot DOS 3.3 with Apple IIe ROM
a2rs -r roms/apple2e.rom -1 disks/dos33.dsk

# Auto-detect model from ROM size
a2rs -r roms/apple2plus.rom -1 disks/game.dsk

# Specify model explicitly
a2rs -m iie -r roms/apple2e.rom -1 disks/prodos.dsk

# Two disk drives
a2rs -r roms/apple2e.rom -1 disk1.dsk -2 disk2.dsk

# Use a custom configuration file
a2rs --config /path/to/config.json -1 game.dsk

# Use a custom home directory
a2rs --home ~/Apple2 -1 dos33.dsk
```

### Command-Line Options

| Option | Description |
|--------|-------------|
| `-1, --disk1 <FILE>` | Disk image for Drive 1 |
| `-2, --disk2 <FILE>` | Disk image for Drive 2 |
| `-r, --rom <FILE>` | Apple II ROM file |
| `-m, --model <MODEL>` | `auto` \| `ii` \| `ii+` \| `iie` \| `iie-enhanced` (default: `auto`) |
| `--disk-rom <FILE>` | Disk II Boot ROM (256 bytes) |
| `--speed <N>` | Speed multiplier: `1` = normal, `0` = maximum (default: `1`) |
| `--size <WxH>` | Window size (default: `640x480`) |
| `-c, --config <FILE>` | Path to configuration file |
| `--home <PATH>` | A2RS home directory (base for all relative paths) |
| `--headless` | Run without GUI |
| `--cycles <N>` | Number of cycles to run in headless mode |
| `--profile` | Enable the built-in profiler |
| `--disk-log <LEVEL>` | Disk log verbosity: `none` \| `flow` \| `state` \| `decide` \| `all` |

---

## Keyboard Controls

### Emulator

| Key | Function |
|:---:|----------|
| `F1` | Settings menu |
| `F2` | Cycle speed (×1 → ×2 → ×5 → ×10 → MAX) |
| `F3` | Cycle rendering quality level |
| `F4` | Toggle auto quality adjustment |
| `F5` | Quick save |
| `F6` | Toggle sound ON / OFF |
| `F8` | Cycle save slot (0–9) |
| `F9` | Quick load |
| `F10` | Screenshot |
| `F11` | Toggle debugger panel |
| `F12` | Reset |
| `ESC` | Close menu (passes through to Apple II when no menu is open) |
| `Ctrl+0–9` | Select save slot directly |

### Debugger (when panel is visible)

| Key | Function |
|:---:|----------|
| `Tab` | Switch debugger tabs |
| `F6` | Step instruction |
| `F7` | Continue execution |
| `F8` | Break / Pause |
| `↑` `↓` | Scroll memory view |
| `PageUp/Down` | Fast-scroll memory view |

### Toolbar Buttons

| Button | Function |
|:------:|----------|
| ▶ / ⏸ | Play / Pause |
| ⟳ | Reset |
| 💾1 | Drive 1 disk menu |
| 💾2 | Drive 2 disk menu |
| ⇄ | Swap disks between drives |
| 💾 | Save state |
| 📂 | Load state |
| 📷 | Screenshot |

A volume slider is available on the right side of the toolbar.

---

## Supported Models

| Model | CPU | RAM | ROM Size | Notes |
|-------|:---:|:---:|:--------:|-------|
| Apple II | 6502 | 48 KB | — | Original Apple II |
| Apple II+ | 6502 | 64 KB | 20 KB | Autostart ROM |
| Apple IIe | 6502 | 128 KB | 32 KB | Extended 80-column card |
| Apple IIe Enhanced | 65C02 | 128 KB | 32 KB | MouseText support |

---

## Disk Formats

| Format | Extension | Size | Description |
|--------|:---------:|:----:|-------------|
| DSK | `.dsk` `.do` | 140 KB | Standard disk image (DOS 3.3 sector order) |
| PO | `.po` | 140 KB | ProDOS-ordered disk image |
| NIB | `.nib` | 227 KB | Nibblized disk image (raw bitstream) |
| WOZ 1.0 | `.woz` | varies | Applesauce WOZ format v1 (read-only) |
| WOZ 2.0 | `.woz` | varies | Applesauce WOZ format v2 (read-only) |

> WOZ images are automatically detected by their magic bytes and converted to the internal NIB format on load. Writing back to WOZ is not supported.

---

## ROM Files

> **ROM files are not included due to copyright. You must provide your own.**

| ROM | Size | Notes |
|-----|:----:|-------|
| Apple II Plus ROM | 20,480 bytes (20 KB) | |
| Apple IIe ROM | 32,768 bytes (32 KB) | |
| Disk II Boot ROM | 256 bytes | Optional; filename `disk2.rom` |

Place ROM files in the `roms/` directory or specify with `--rom` / `--disk-rom`.

---

## Directory Structure

By default A2RS stores all files under a platform-specific home directory:

| Platform | Default Home |
|----------|-------------|
| Windows | `%LOCALAPPDATA%\a2rs\` |
| macOS | `~/Library/Application Support/a2rs/` |
| Linux | `~/.local/share/a2rs/` |

```
a2rs_home/
├── roms/               # ROM files
│   ├── apple2e.rom
│   └── disk2.rom
├── disks/              # Disk images
│   ├── dos33.dsk
│   └── games/
├── saves/              # Save states (quicksave.json, save_slot_1.json, …)
└── screenshots/        # PNG screenshots
```

The configuration file is stored separately:

| Platform | Config Path |
|----------|-------------|
| Windows | `%APPDATA%\a2rs\config.json` |
| macOS | `~/Library/Application Support/a2rs/config.json` |
| Linux | `~/.config/a2rs/config.json` |

---

## Configuration File

A2RS reads `config.json` on startup but **never writes GUI changes back to disk** — all in-session changes are temporary.

### Full Example

```json
{
  "a2rs_home": "",
  "rom_dir": "roms",
  "disk_dir": "disks",
  "screenshot_dir": "screenshots",
  "save_dir": "saves",
  "speed": 1,
  "fast_disk": true,
  "sound_enabled": true,
  "volume": 0.5,
  "quality_level": 4,
  "auto_quality": true,
  "window_width": 560,
  "window_height": 384,
  "current_slot": 0,
  "gamepad": {
    "enabled": true,
    "deadzone": 0.15,
    "show_debug_overlay": false,
    "button_a_names": ["South", "C"],
    "button_b_names": ["East"],
    "button_x_names": ["West"],
    "button_y_names": ["North"],
    "button_lb_names": ["LeftTrigger", "LeftTrigger2"],
    "button_rb_names": ["RightTrigger", "RightTrigger2"],
    "button_start_names": ["Start", "Mode"],
    "button_select_names": ["Select"],
    "raw_button_a_codes": [289, 297],
    "raw_button_b_codes": [288, 296],
    "raw_axis_x_code": 0,
    "raw_axis_y_code": 1,
    "raw_hat_x_code": 16,
    "raw_hat_y_code": 17
  },
  "experimental": {
    "disk_sequencer_mode": "safe",
    "weak_bits": false,
    "write_splice": false,
    "disk_debug_logging": false
  }
}
```

All keys are optional — missing keys fall back to the defaults shown above.

---

### Path Settings

#### `a2rs_home`
**Type:** string — **Default:** `""` (platform default)

The base directory from which all other relative paths (`rom_dir`, `disk_dir`, etc.) are resolved.

- `""` (empty) — use the platform default home directory (see table above)
- Absolute path — used as-is
- Relative path — resolved relative to the config file's directory
- `~` prefix — expanded to the user's home directory

```json
// Use a custom location on any platform
{ "a2rs_home": "/Users/alice/retro/apple2" }

// Windows example
{ "a2rs_home": "D:/Games/Apple2" }

// Tilde expansion
{ "a2rs_home": "~/retro/apple2" }
```

Can also be overridden at runtime with `--home <PATH>`.

---

#### `rom_dir`
**Type:** string — **Default:** `"roms"`

Directory where ROM files are searched. Relative paths are resolved from `a2rs_home`.

```json
{ "rom_dir": "roms" }            // → <a2rs_home>/roms/
{ "rom_dir": "/opt/apple2/roms" } // absolute path
```

---

#### `disk_dir`
**Type:** string — **Default:** `"disks"`

Directory where disk images are listed in the disk menu. A2RS recursively scans this directory (up to 3 levels deep) for `.dsk`, `.do`, `.po`, `.nib`, and `.woz` files.

```json
{ "disk_dir": "disks" }           // → <a2rs_home>/disks/
{ "disk_dir": "/mnt/nas/apple2" }  // absolute path
```

---

#### `screenshot_dir`
**Type:** string — **Default:** `""` (→ `<a2rs_home>/screenshots/`)

Directory where screenshots (PNG) are saved. If empty, defaults to `<a2rs_home>/screenshots/`.

```json
{ "screenshot_dir": "screenshots" }
{ "screenshot_dir": "~/Desktop/apple2-screenshots" }
```

---

#### `save_dir`
**Type:** string — **Default:** `"saves"`

Directory where save state files are stored. Save slots are named `quicksave.json` (slot 0) and `save_slot_1.json` through `save_slot_9.json`.

```json
{ "save_dir": "saves" }           // → <a2rs_home>/saves/
```

---

### Emulation Settings

#### `speed`
**Type:** integer — **Default:** `1`

CPU speed multiplier.

| Value | Description |
|:-----:|-------------|
| `1` | Normal speed (≈ 1 MHz, real Apple II speed) |
| `2` | 2× speed |
| `5` | 5× speed |
| `10` | 10× speed |
| `0` | Maximum speed (no throttle) — also enables boot boost |

Can be changed at runtime with `F2`.

---

#### `fast_disk`
**Type:** boolean — **Default:** `true`

Enables SafeFast disk acceleration. When active, A2RS detects standard DOS 3.3 RWTS routines and applies fast-read optimizations, falling back to accurate nibble-by-nibble emulation automatically if non-standard access patterns are detected.

Disabling this will make disk access very slow (real Apple II speed) but maximally accurate.

---

#### `sound_enabled`
**Type:** boolean — **Default:** `true`

Enables or disables the audio output. Can be toggled at runtime with `F6`.

---

#### `volume`
**Type:** float (0.0 – 1.0) — **Default:** `0.5`

Master volume level. `0.0` is silent, `1.0` is maximum. Adjustable with the toolbar slider.

---

#### `quality_level`
**Type:** integer (0–4) — **Default:** `4`

Rendering quality / frame-rate target.

| Value | Description |
|:-----:|-------------|
| `0` | Minimum quality — fastest |
| `1–3` | Intermediate levels |
| `4` | Full quality — matches Apple II frame rate |

Cycle with `F3`. When `auto_quality` is on, this value is adjusted automatically.

---

#### `auto_quality`
**Type:** boolean — **Default:** `true`

When enabled, quality level is reduced quickly on FPS loss and restored slowly after sustained recovery, keeping the emulation smooth on slower machines. Toggle with `F4`.

---

#### `window_width` / `window_height`
**Type:** integer — **Default:** `560` × `384`

Initial window size in pixels. The Apple II display is 280×192; the default 560×384 is 2× with toolbar and status bar.

---

#### `current_slot`
**Type:** integer (0–9) — **Default:** `0`

The active save slot used by quick save (`F5`) and quick load (`F9`).

- `0` → `quicksave.json`
- `1`–`9` → `save_slot_1.json` … `save_slot_9.json`

Cycle with `F8` or select directly with `Ctrl+0`–`Ctrl+9`.

---

### `gamepad` Section

Controls gamepad / joystick input for Apple II paddle emulation.

#### `enabled`
**Type:** boolean — **Default:** `true`

Enables or disables gamepad input entirely.

---

#### `deadzone`
**Type:** float (0.0 – 1.0) — **Default:** `0.15`

Analog stick deadzone. Stick movements smaller than this value are treated as centered (zero). Increase if the cursor drifts when the stick is at rest.

---

#### `show_debug_overlay`
**Type:** boolean — **Default:** `false`

Displays a real-time gamepad debug overlay in the top-left corner of the window (Linux only). Shows axis values, D-pad state, button states, and the last gilrs event. Useful for identifying button mappings on unfamiliar controllers.

---

#### `button_a_names` / `button_b_names`
**Type:** string array — **Defaults:** `["South","C"]` / `["East"]`

Named button identifiers (as reported by `gilrs`) mapped to Apple II **Button 0** and **Button 1** respectively.

Common gilrs names: `South`, `East`, `North`, `West`, `C`, `Z`, `LeftTrigger`, `RightTrigger`, `Start`, `Select`, `Mode`.

---

#### `button_x_names` / `button_y_names`
**Type:** string array — **Defaults:** `["West"]` / `["North"]`

Additional named buttons. Used for secondary functions (not standard Apple II buttons).

---

#### `button_lb_names` / `button_rb_names`
**Type:** string array — **Defaults:** `["LeftTrigger","LeftTrigger2"]` / `["RightTrigger","RightTrigger2"]`

Shoulder / trigger buttons.

---

#### `button_start_names` / `button_select_names`
**Type:** string array — **Defaults:** `["Start","Mode"]` / `["Select"]`

Start and Select buttons.

---

#### `raw_button_a_codes` / `raw_button_b_codes`
**Type:** integer array — **Defaults:** `[289,297]` / `[288,296]`

Linux fallback: raw evdev button codes for controllers that appear as `Unknown` in gilrs. Use `show_debug_overlay: true` to find the correct codes for your controller.

---

#### `raw_axis_x_code` / `raw_axis_y_code`
**Type:** integer — **Defaults:** `0` / `1`

Raw evdev axis codes for the left analog stick (Linux fallback).

---

#### `raw_hat_x_code` / `raw_hat_y_code`
**Type:** integer — **Defaults:** `16` / `17`

Raw evdev axis codes for the D-pad hat (Linux fallback).

---

### `experimental` Section

Advanced options. All default to the safest / most compatible values. **Do not change these unless you understand the implications.**

#### `disk_sequencer_mode`
**Type:** string — **Default:** `"safe"`

Controls the low-level Disk II sequencer implementation.

| Value | Description |
|:-----:|-------------|
| `"safe"` | Standard emulation; maximum software compatibility |
| `"transitional"` | Intermediate mode for testing |
| `"strict"` | Hardware-accurate sequencer; may break some software |

---

#### `weak_bits`
**Type:** boolean — **Default:** `false`

Enables simulation of weak-bit / unstable-bit phenomena found on some copy-protected disks. Not yet fully implemented.

---

#### `write_splice`
**Type:** boolean — **Default:** `false`

Enables write-splice simulation (the gap between the end of one write and the beginning of the next). Not yet fully implemented.

---

#### `disk_debug_logging`
**Type:** boolean — **Default:** `false`

Enables detailed low-level disk sequencer logging to stdout. Very verbose; use together with `--disk-log all` on the command line for maximum detail.

---

## Project Structure

```
a2rs/
├── src/
│   ├── main.rs          # Entry point, GUI, main loop
│   ├── lib.rs           # Library exports
│   ├── apple2.rs        # Emulator orchestration
│   ├── cpu/
│   │   ├── mod.rs       # 6502/65C02 CPU core
│   │   ├── addressing.rs
│   │   ├── opcodes.rs
│   │   └── opcodes2.rs  # 65C02 extended opcodes
│   ├── memory.rs        # Memory map, soft switches
│   ├── video.rs         # Video rendering
│   ├── disk.rs          # Disk II controller
│   ├── disk_log.rs      # Disk activity logging
│   ├── woz.rs           # WOZ 1.0 / 2.0 parser
│   ├── sound.rs         # Audio output
│   ├── gamepad.rs       # Gamepad / joystick support
│   ├── gui.rs           # UI overlay and menus
│   ├── profiler.rs      # Performance profiler
│   ├── config.rs        # Configuration management
│   └── savestate.rs     # Save state serialization
├── Cargo.toml
├── README.md            # This file (English)
└── README_ja.md         # Japanese README
```

---

## Testing

```bash
# Run Klaus2m5 6502 functional test
cargo run --bin cpu_test

# Run with debug logging
RUST_LOG=debug cargo run -- -r roms/apple2e.rom -1 dos33.dsk

# Run with disk activity logging
cargo run -- --disk-log flow -1 dos33.dsk
```

---

## Building Installers

### Windows MSI

```bash
cargo install cargo-wix
cargo wix
# Output: target/wix/a2rs-0.4.2-x86_64.msi
```

### Linux DEB

```bash
cargo install cargo-deb
cargo deb
```

### Linux RPM

```bash
cargo install cargo-generate-rpm
cargo generate-rpm
```

---

## Contributing

Contributions are welcome. Please open an issue or pull request.

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Commit your changes
4. Push and open a Pull Request

---

## License

MIT License — see [LICENSE](LICENSE) for details.

---

## Acknowledgments

- [Beneath Apple DOS](https://archive.org/details/Beneath_Apple_DOS) — Essential Disk II documentation
- [Understanding the Apple II](https://archive.org/details/understanding_the_apple_ii) — Hardware reference
- [Klaus2m5 6502 Test Suite](https://github.com/Klaus2m5/6502_65C02_functional_tests) — CPU validation
- [AppleWin](https://github.com/AppleWin/AppleWin) — Reference implementation
- [Applesauce WOZ Specification](https://applesaucefdc.com/woz/) — WOZ format reference

---

<p align="center">Made with Rust</p>
