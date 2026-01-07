# A2RS - Apple II Emulator in Rust 🍎

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.2.0-green.svg)](https://github.com/user/a2rs/releases)

[English](#features) | [日本語](#日本語)

**A2RS** is a high-accuracy Apple II emulator written in Rust. It focuses on specification-based implementation with emphasis on understanding Apple II hardware architecture at a deep level.

<p align="center">
  <img src="docs/screenshot.png" alt="A2RS Screenshot" width="640">
</p>

## ✨ Features

| Feature | Description |
|---------|-------------|
| 🖥️ **Multi-Model Support** | Apple II, II+, IIe, IIe Enhanced |
| ⚡ **High Performance** | 200+ MHz equivalent speed (release build) |
| 🎯 **Cycle Accurate** | Passes Klaus2m5 6502 functional test suite |
| 💾 **Disk II Emulation** | DSK/DO/PO/NIB formats with fast disk acceleration |
| 🎨 **Accurate Video** | Text, Lo-Res, Hi-Res, Double Hi-Res modes |
| 🎮 **Gamepad Support** | Joystick/gamepad input for paddle emulation |
| 🔊 **Audio Emulation** | Speaker click emulation with volume control |
| 📊 **Built-in Profiler** | Performance analysis and boot timing |
| 🐛 **Debugger UI** | Real-time CPU/memory/disk monitoring |
| 💾 **Save States** | Quick save/load with 10 slots |
| 🔧 **Flexible Configuration** | Customizable paths and settings |

## 🚀 Quick Start

```bash
# Clone and build
git clone https://github.com/user/a2rs.git
cd a2rs
cargo build --release

# Run with a disk image
./target/release/a2rs -r roms/apple2e.rom -1 disks/dos33.dsk
```

## 📋 Requirements

### Rust
- Rust 1.70 or later

### System Dependencies

<details>
<summary><b>Linux (Debian/Ubuntu)</b></summary>

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

## 🔧 Building

```bash
# Basic build
cargo build --release

# Build with all features (audio + gamepad)
cargo build --release --features full

# Run
./target/release/a2rs --help
```

## 📖 Usage

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

# Use custom configuration file
a2rs --config /path/to/config.json -1 game.dsk

# Use custom home directory for all paths
a2rs --home D:/Games/Apple2 -1 dos33.dsk
```

### Command Line Options

```
OPTIONS:
    -1, --disk1 <FILE>       Disk image for Drive 1
    -2, --disk2 <FILE>       Disk image for Drive 2
    -r, --rom <FILE>         Apple II ROM file
    -m, --model <MODEL>      Model: auto, ii, ii+, iie, iie-enhanced [default: auto]
        --disk-rom <FILE>    Disk II Boot ROM (256 bytes)
        --speed <N>          Speed multiplier (1=normal, 0=max) [default: 1]
        --size <WxH>         Window size [default: 640x480]
    -c, --config <FILE>      Configuration file path
        --home <PATH>        A2RS home directory (base for relative paths)
        --headless           Run without GUI
        --cycles <N>         Cycles to run in headless mode
        --profile            Enable profiler
        --disk-log <LEVEL>   Disk log: none, flow, state, decide, all
    -h, --help               Print help
    -V, --version            Print version
```

## ⌨️ Keyboard Controls

### Emulator Controls

| Key | Function |
|:---:|----------|
| `F1` | Settings menu |
| `F2` | Cycle speed (×1 → ×2 → ×5 → ×10 → MAX) |
| `F3` | Cycle quality level |
| `F4` | Toggle auto quality |
| `F5` | Quick save |
| `F6` | Toggle sound ON/OFF |
| `F8` | Cycle save slot (0-9) |
| `F9` | Quick load |
| `F10` | Screenshot |
| `F11` | Toggle debugger panel |
| `F12` | Reset |
| `ESC` | Close menu (when open) |
| `Ctrl+0-9` | Select save slot directly |

### Debugger Controls (when debugger is visible)

| Key | Function |
|:---:|----------|
| `Tab` | Switch debugger tabs |
| `F6` | Step instruction |
| `F7` | Continue execution |
| `F8` | Break/Pause |
| `↑` `↓` | Scroll memory view |
| `PageUp/Down` | Fast scroll memory view |

### Toolbar Buttons

The toolbar provides mouse-clickable buttons:

| Button | Function |
|:------:|----------|
| ▶/⏸ | Play/Pause |
| ⟳ | Reset |
| 💾1 | Drive 1 disk menu |
| 💾2 | Drive 2 disk menu |
| ⇄ | Swap disks |
| 💾 | Save state |
| 📂 | Load state |
| 📷 | Screenshot |

Volume slider is available on the right side of the toolbar.

## 🎮 Supported Models

| Model | CPU | RAM | ROM Size | Notes |
|-------|:---:|:---:|:--------:|-------|
| Apple II | 6502 | 48KB | - | Original Apple II |
| Apple II+ | 6502 | 64KB | 20KB | Autostart ROM |
| Apple IIe | 6502 | 128KB | 32KB | Extended 80-column |
| Apple IIe Enhanced | 65C02 | 128KB | 32KB | MouseText support |

## 💾 Disk Formats

| Format | Extension | Size | Description |
|--------|:---------:|:----:|-------------|
| DSK | `.dsk` | 140KB | Standard disk image (DOS order) |
| DO | `.do` | 140KB | DOS-ordered disk image |
| PO | `.po` | 140KB | ProDOS-ordered disk image |
| NIB | `.nib` | 232KB | Nibblized disk image (raw) |

## 📁 Directory Structure

A2RS uses a flexible directory structure. By default, all paths are relative to the executable directory:

```
a2rs/
├── a2rs.exe              # Executable
├── apple2_config.json    # Configuration file
├── roms/                 # ROM files
│   ├── apple2e.rom
│   └── disk2.rom
├── disks/                # Disk images
│   ├── dos33.dsk
│   └── games/
├── saves/                # Save states
└── screenshots/          # Screenshots
```

### Configuration File (apple2_config.json)

```json
{
  "a2rs_home": "",
  "rom_dir": "roms",
  "disk_dir": "disks",
  "screenshot_dir": "screenshots",
  "save_dir": "saves",
  "speed": 1,
  "sound_enabled": true,
  "volume": 0.5,
  "quality_level": 4,
  "auto_quality": true
}
```

- `a2rs_home`: Base directory for all relative paths (empty = exe directory)
- All directory paths are relative to `a2rs_home`

### Custom Home Directory

You can specify a custom home directory:

```bash
# Via command line
a2rs --home D:/Games/Apple2 -1 dos33.dsk

# Via config file
{
  "a2rs_home": "D:/Games/Apple2",
  "disk_dir": "disks"
}
# Result: disks are loaded from D:/Games/Apple2/disks/
```

## 📁 ROM Files

> ⚠️ ROM files are not included due to copyright. You must provide your own.

**Expected ROM sizes:**
- **20KB** (20,480 bytes) — Apple II Plus ROM
- **32KB** (32,768 bytes) — Apple IIe ROM
- **256 bytes** — Disk II Boot ROM (optional, `disk2.rom`)

Place ROM files in `roms/` directory or specify with `--rom` and `--disk-rom` options.

## 🏗️ Project Structure

```
a2rs/
├── src/
│   ├── main.rs          # Entry point, GUI, main loop
│   ├── lib.rs           # Library exports
│   ├── apple2.rs        # Main emulator orchestration
│   ├── cpu/
│   │   ├── mod.rs       # 6502/65C02 CPU core
│   │   ├── addressing.rs # Addressing modes
│   │   ├── opcodes.rs   # Opcode implementations
│   │   └── opcodes2.rs  # 65C02 extended opcodes
│   ├── memory.rs        # Memory map, soft switches
│   ├── video.rs         # Video rendering (Text/Lo-Res/Hi-Res)
│   ├── disk.rs          # Disk II controller emulation
│   ├── disk_log.rs      # Disk activity logging
│   ├── sound.rs         # Audio output
│   ├── gamepad.rs       # Gamepad/joystick support
│   ├── gui.rs           # UI overlay and menus
│   ├── profiler.rs      # Performance profiler
│   ├── config.rs        # Configuration management
│   └── savestate.rs     # Save state serialization
├── wix/                 # Windows installer files
├── scripts/             # Build scripts
├── Cargo.toml
└── README.md
```

## 🧪 Testing

```bash
# Run Klaus2m5 6502 functional test
a2rs --test-cpu

# Run 65C02 extended opcode test
a2rs --test-65c02

# Quick CPU tests
a2rs --quick-test
```

## 📊 Profiling

```bash
# Enable profiler with JSON output
a2rs --profile --profile-output profile.json -1 dos33.dsk

# Profile boot sequence only (exits after boot)
a2rs --profile --profile-boot -1 dos33.dsk

# Profile with CSV output
a2rs --profile --profile-output profile.csv -1 dos33.dsk
```

## 📦 Building Installers

### Windows MSI Installer

```bash
# Install WiX Toolset and cargo-wix
cargo install cargo-wix

# Build installer
cargo wix
# Output: target/wix/a2rs-0.2.0-x86_64.msi
```

See `wix/README.md` for details.

### Linux DEB Package

```bash
# Install cargo-deb
cargo install cargo-deb

# Build package
cargo deb
```

## 🛠️ Development

```bash
# Run with debug logging
RUST_LOG=debug cargo run -- -r roms/apple2e.rom -1 dos33.dsk

# Run with disk activity logging
cargo run -- --disk-log flow+state -1 dos33.dsk

# Run with boot boost logging
cargo run -- --boost-log -1 dos33.dsk
```

## 📝 Changelog

### Version 0.2.0

- **New Features**
  - Volume slider in toolbar
  - Configurable home directory (`--home` option)
  - Custom config file path (`--config` option)
  - Clipboard paste support (Ctrl+V) in text input fields
  - Disk menu shows 60 characters filename, sorted alphabetically
  
- **Changes**
  - Fast disk mode is now always enabled
  - Settings menu moved to F1 (was ESC)
  - Debugger panel moved to F11 (was Tab)
  - Debugger tab switching now uses Tab key
  - ESC key now passes through to Apple II when no menu is open
  - Removed fullscreen toggle (F11 now opens debugger)

- **Key Mapping (v0.2.0)**
  - F1: Settings menu
  - F2: Speed control
  - F3: Quality level
  - F4: Auto quality
  - F5: Save state
  - F6: Sound toggle / Step (debugger)
  - F7: Continue (debugger)
  - F8: Slot select / Break (debugger)
  - F9: Load state
  - F10: Screenshot
  - F11: Debugger panel
  - F12: Reset

### Version 0.1.0

- Initial release
- Apple II/II+/IIe/IIe Enhanced support
- Disk II emulation
- Save states
- Gamepad support

## 🤝 Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Beneath Apple DOS](https://archive.org/details/Beneath_Apple_DOS) — Essential Disk II documentation
- [Understanding the Apple II](https://archive.org/details/understanding_the_apple_ii) — Hardware reference
- [Klaus2m5 6502 Test Suite](https://github.com/Klaus2m5/6502_65C02_functional_tests) — CPU validation
- [AppleWin](https://github.com/AppleWin/AppleWin) — Reference implementation
- [MAME](https://github.com/mamedev/mame) — Apple II driver reference

---

# 日本語

## A2RS - Rust製 Apple IIエミュレータ 🍎

**A2RS**は、Rustで書かれた高精度なApple IIエミュレータです。既存のエミュレータのコードをコピーするのではなく、仕様書に基づいた実装を重視し、Apple IIのハードウェアアーキテクチャを深く理解することに焦点を当てています。

### 主な特徴

- 🖥️ **複数モデル対応** — Apple II, II+, IIe, IIe Enhanced
- ⚡ **高性能** — リリースビルドで200MHz以上の等価速度
- 🎯 **サイクル精度** — Klaus2m5 6502機能テストに合格
- 💾 **Disk IIエミュレーション** — DSK/DO/PO/NIB形式、高速ディスク対応
- 🎨 **正確なビデオ出力** — テキスト、Lo-Res、Hi-Res、Double Hi-Resモード
- 🎮 **ゲームパッド対応** — パドルエミュレーション用ジョイスティック入力
- 🔊 **オーディオエミュレーション** — スピーカークリック音、音量調整
- 📊 **内蔵プロファイラ** — パフォーマンス分析とブート時間測定
- 🐛 **デバッガUI** — リアルタイムCPU/メモリ/ディスク監視
- 💾 **セーブステート** — 10スロットのクイックセーブ/ロード
- 🔧 **柔軟な設定** — カスタマイズ可能なパスと設定

### クイックスタート

```bash
# クローンとビルド
git clone https://github.com/user/a2rs.git
cd a2rs
cargo build --release

# ディスクイメージで起動
./target/release/a2rs -r roms/apple2e.rom -1 disks/dos33.dsk

# カスタムホームディレクトリを使用
./target/release/a2rs --home D:/Games/Apple2 -1 dos33.dsk
```

### 必要なROMファイル

> ⚠️ 著作権の関係上、ROMファイルは含まれていません。ご自身でご用意ください。

- **20KB** (20,480バイト) — Apple II Plus ROM
- **32KB** (32,768バイト) — Apple IIe ROM
- **256バイト** — Disk II Boot ROM（オプション）

### キーボード操作

| キー | 機能 |
|:---:|------|
| `F1` | 設定メニュー |
| `F2` | 速度切り替え（×1 → ×2 → ×5 → ×10 → MAX）|
| `F3` | 品質レベル切り替え |
| `F4` | 自動品質切り替え |
| `F5` | クイックセーブ |
| `F6` | サウンドON/OFF |
| `F8` | セーブスロット選択 (0-9) |
| `F9` | クイックロード |
| `F10` | スクリーンショット |
| `F11` | デバッガパネル |
| `F12` | リセット |
| `ESC` | メニューを閉じる |
| `Ctrl+0-9` | スロット直接選択 |

### デバッガ操作（デバッガ表示中）

| キー | 機能 |
|:---:|------|
| `Tab` | タブ切り替え |
| `F6` | ステップ実行 |
| `F7` | 継続 |
| `F8` | ブレーク |
| `↑` `↓` | メモリビュースクロール |

### バージョン 0.2.0 の変更点

- **新機能**
  - ツールバーに音量スライダー追加
  - ホームディレクトリ指定オプション（`--home`）
  - 設定ファイルパス指定オプション（`--config`）
  - テキスト入力でのクリップボード貼り付け（Ctrl+V）
  - ディスクメニューで60文字表示、ファイル名ソート

- **変更**
  - 高速ディスクモードは常にON
  - 設定メニューをF1に変更（旧ESC）
  - デバッガパネルをF11に変更（旧Tab）
  - デバッガのタブ切り替えをTabキーに変更
  - ESCキーはメニューが開いていない時はApple IIに送信
  - 全画面モード削除

---

<p align="center">
  Made with ❤️ and Rust
</p>
