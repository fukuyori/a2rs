## [0.4.0] - 2026-03-09

### Added
- Phase 8 groundwork for experimental Disk II sequencer modes (`safe`, `transitional`, `strict`)
- Experimental config block for weak bits, write splice, and disk debug logging
- Diagnostics snapshot helpers for disk controller state

### Changed
- Version bumped to 0.4.0
- Runtime banner updated to report v0.4.2
- README and sample config updated for experimental disk options

## [0.3.0] - 2026-03-09

### Added
- Phase 3-7 timing core integration as the new stable baseline
- `VideoScanner` timing and fetch-phase aware floating bus behavior
- Disk II bit timing, nibble timing scaffolding, and controller state machine scaffolding
- Architecture and phase notes for Phase 4-7 development
- Regression-oriented release documentation

### Changed
- Version bumped to 0.3.0
- Runtime banner updated to report v0.3.0
- README updated to describe the current stable timing architecture
- Configuration handling remains read-only at runtime

### Notes
- Disk nibble timing currently uses a safe-read compatibility path to preserve boot reliability
- This release is intended as the stable base before stricter sequencer / copy-protection work


## Phase 6 – Disk II nibble timing

- Add bit-to-nibble timing in `Disk2InterfaceCard::tick_disk_bit()`
- Latch one nibble every 8 disk bits in Accurate mode
- Stop advancing disk byte position on every I/O read in Accurate mode
- Reflect writes on 8-bit boundaries during Accurate timing

# Changelog

## [0.4.1] - 2026-03-09

### Fixed
- Runtime banner now reports the package version from `Cargo.toml` via `env!("CARGO_PKG_VERSION")`.
- Phase 8 groundwork build now presents a consistent `v0.4.2` version surface.

### Added
- Transitional Disk II sequencer mode now advances internal bit-to-nibble state on `tick_disk_bit()`.
- Transitional mode prefers tick-prepared nibble timing while keeping safe-read fallback for boot compatibility.
- Optional transitional debug logging for Disk II sequencing.

## 2026-03-09 Phase5 completed floating bus
- Latch floating bus only during active video fetch cycles
- Preserve previous bus value during HBL/VBL
- Add fetch-phase-aware VideoScanner helpers
- Support 80-column text bus sampling from aux/main memory alternation


## 2026-03-09 (system tick foundation)

- Added `src/system.rs` with Apple II timing constants
- Added `Apple2::tick()` where 1 tick = 1 CPU cycle
- Changed `run_cycles()` to advance via repeated system ticks
- Changed `run_frame()` to run until the next frame boundary
- `memory.scanline` now keeps the full 0..261 NTSC scanline range
- Removed forced `scanline=192` during video rendering
- Config file remains read-only


## Unreleased

### Phase 3.5 - Tick-driven video scanner
- Added `VideoScanner` with `scanline_cycle`, `scanline`, and `frame` counters.
- `Apple2::tick()` now advances video timing every CPU cycle.
- `memory.scanline`, `memory.scanline_cycle`, and `memory.frame_counter` are updated from the scanner.
- Added `floating_bus()` groundwork based on current scan position.
- `run_frame()` now advances to the next scanner frame boundary instead of assuming a fixed cycle bucket.


- Config file is now treated as read-only at runtime; A2RS no longer writes settings back on edit or exit.
- `rom_dir` and `disk_dir` remain configurable via `apple2_config.json`.

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Optional `gamepad` section in `apple2_config.json` for Windows/Linux button mapping and Linux raw button fallback codes
- Config flag to show or hide the Linux gamepad debug overlay
- `--print-gamepad-codes` option to print gamepad button/axis events and raw input codes to standard output

### Changed
- Auto quality now drops quickly on low FPS and only restores after sustained high FPS
- Gamepad initialization now respects `gamepad.enabled` and falls back to built-in defaults if the config file is missing
- Built-in text glyphs now derive from the project's compact UI font instead of the previous ROM-like bitmap set

## [0.2.0] - 2025-01-07

### Added
- Volume slider in toolbar for adjusting audio level
- `--config` option to specify custom configuration file path
- `--home` option to specify A2RS home directory (base for all relative paths)
- Clipboard paste support (Ctrl+V) in text input fields
- Disk menu now displays up to 60 characters of filename
- Disk list is now sorted alphabetically by filename (case-insensitive)
- Configuration file search priority:
  1. `--config` specified path
  2. `<home>/apple2_config.json` if `--home` specified
  3. Executable directory `apple2_config.json`

### Changed
- **Breaking**: Keyboard shortcuts reorganized to avoid Apple II key conflicts
  - F1: Settings menu (was ESC)
  - F2: Speed control (was F1)
  - F11: Debugger panel (was Tab)
  - Tab: Debugger tab switching (was Left/Right arrows)
- Fast disk mode is now always enabled (removed toggle)
- ESC key now passes through to Apple II when no menu is open
- Settings menu title shows "F1" instead of "ESC"
- Removed fullscreen toggle feature (F11 repurposed for debugger)

### Removed
- Fast Disk toggle from settings menu (always ON now)
- Fullscreen mode toggle (rarely used)
- F2 fast disk shortcut

### Fixed
- ESC and Tab keys now work correctly in Apple II programs
- Arrow keys work in games when debugger is hidden

## [0.1.0] - 2024-12-01

### Added
- Initial release
- Apple II, II+, IIe, IIe Enhanced model support
- 6502 and 65C02 CPU emulation
- Passes Klaus2m5 6502 functional test suite
- Disk II controller emulation
  - DSK, DO, PO, NIB format support
  - Fast disk acceleration (SafeFast)
- Video modes: Text, Lo-Res, Hi-Res, Double Hi-Res
- Speaker audio emulation
- Gamepad/joystick support for paddle emulation
- Save states with 10 slots
- Built-in debugger with CPU, memory, and disk tabs
- Performance profiler
- Screenshot capture
- Configurable quality levels with auto-adjustment
- Boot speed boost for faster startup

### Technical Details
- Written in Rust for safety and performance
- Cross-platform: Windows, macOS, Linux
- ~200 MHz equivalent speed in release build
- Cycle-accurate CPU emulation

[0.2.0]: https://github.com/user/a2rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/user/a2rs/releases/tag/v0.1.0


## Phase 4 Hybrid

- CPU micro-state trace (`CpuMicroState`) を追加
- `tick()` ごとに CPU microcycle / VideoScanner / Disk raw bit timing を同期
- floating bus を scanline / cycle ベースで表示メモリ参照へ改善
- Disk II に `4 CPU cycles = 1 disk bit` の基礎分周器を追加

## Phase 5 (partial): floating bus soft-switch reads

- Read access to write-only video soft switches (`$C050-$C05F`) now returns the current floating bus byte while still applying the soft-switch side effect.
- Speaker soft-switch reads (`$C030-$C03F`) also return floating bus after toggling the speaker latch.
- Paddle trigger reads (`$C070-$C07D`) now behave as floating-bus style reads while preserving trigger side effects.


## Phase7
- Added explicit Disk II controller state machine scaffold with Q6/Q7-derived states.
- Added sync bit window tracking and pending write latch without regressing Phase6 boot compatibility.
[0.4.0]: https://github.com/user/a2rs/compare/v0.3.0...v0.4.2
[0.3.0]: https://github.com/user/a2rs/compare/v0.2.0...v0.3.0
