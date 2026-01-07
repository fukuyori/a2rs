# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
