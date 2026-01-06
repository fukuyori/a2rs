# A2RS - Apple II Emulator in Rust 🍎

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/github/actions/workflow/status/user/a2rs/ci.yml?branch=main)](https://github.com/user/a2rs/actions)

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
| 💾 **Disk II Emulation** | DSK/DO/PO/NIB formats with SafeFast acceleration |
| 🎨 **Accurate Video** | Text, Lo-Res, Hi-Res, Double Hi-Res modes |
| 🎮 **Gamepad Support** | Joystick/gamepad input for paddle emulation |
| 🔊 **Audio Emulation** | Speaker click emulation |
| 📊 **Built-in Profiler** | Performance analysis and boot timing |
| 🐛 **Debugger UI** | Real-time CPU/memory/disk monitoring |
| 💾 **Save States** | Quick save/load with 10 slots |

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
        --fast-disk          Enable fast disk mode
        --size <WxH>         Window size [default: 640x480]
        --fullscreen         Start in fullscreen mode
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
| `ESC` | Settings overlay |
| `Tab` | Toggle debugger panel |
| `F1` | Cycle speed (×1 → ×2 → ×5 → ×10 → MAX) |
| `F2` | Toggle fast disk |
| `F3` | Cycle quality level |
| `F4` | Toggle auto quality |
| `F5` | Quick save |
| `F9` | Quick load |
| `F10` | Screenshot |
| `F11` | Toggle fullscreen |
| `F12` | Reset |

### Debugger Controls

| Key | Function |
|:---:|----------|
| `F6` | Step instruction |
| `F7` | Continue |
| `F8` | Break/Pause |
| `←` `→` | Switch debugger tabs |
| `↑` `↓` | Scroll memory view |

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

## 🛠️ Development

```bash
# Run with debug logging
RUST_LOG=debug cargo run -- -r roms/apple2e.rom -1 dos33.dsk

# Run with disk activity logging
cargo run -- --disk-log flow+state -1 dos33.dsk

# Run with boot boost logging
cargo run -- --boost-log -1 dos33.dsk
```

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
- [AppleWin](https://github.com/AppleWin/AppleWin) — Reference for SafeFast disk acceleration
- [MAME](https://github.com/mamedev/mame) — Apple II driver reference

---

# 日本語

## A2RS - Rust製 Apple IIエミュレータ 🍎

**A2RS**は、Rustで書かれた高精度なApple IIエミュレータです。既存のエミュレータのコードをコピーするのではなく、仕様書に基づいた実装を重視し、Apple IIのハードウェアアーキテクチャを深く理解することに焦点を当てています。

### 主な特徴

- 🖥️ **複数モデル対応** — Apple II, II+, IIe, IIe Enhanced
- ⚡ **高性能** — リリースビルドで200MHz以上の等価速度
- 🎯 **サイクル精度** — Klaus2m5 6502機能テストに合格
- 💾 **Disk IIエミュレーション** — DSK/DO/PO/NIB形式、SafeFast高速化対応
- 🎨 **正確なビデオ出力** — テキスト、Lo-Res、Hi-Res、Double Hi-Resモード
- 🎮 **ゲームパッド対応** — パドルエミュレーション用ジョイスティック入力
- 🔊 **オーディオエミュレーション** — スピーカークリック音
- 📊 **内蔵プロファイラ** — パフォーマンス分析とブート時間測定
- 🐛 **デバッガUI** — リアルタイムCPU/メモリ/ディスク監視
- 💾 **セーブステート** — 10スロットのクイックセーブ/ロード

### クイックスタート

```bash
# クローンとビルド
git clone https://github.com/user/a2rs.git
cd a2rs
cargo build --release

# ディスクイメージで起動
./target/release/a2rs -r roms/apple2e.rom -1 disks/dos33.dsk
```

### 必要なROMファイル

> ⚠️ 著作権の関係上、ROMファイルは含まれていません。ご自身でご用意ください。

- **20KB** (20,480バイト) — Apple II Plus ROM
- **32KB** (32,768バイト) — Apple IIe ROM
- **256バイト** — Disk II Boot ROM（オプション）

### キーボード操作

| キー | 機能 |
|:---:|------|
| `ESC` | 設定オーバーレイ |
| `Tab` | デバッガパネル切り替え |
| `F1` | 速度切り替え（×1 → ×2 → ×5 → ×10 → MAX）|
| `F2` | 高速ディスク切り替え |
| `F5` | クイックセーブ |
| `F9` | クイックロード |
| `F10` | スクリーンショット |
| `F11` | フルスクリーン切り替え |
| `F12` | リセット |

### 開発

```bash
# デバッグログ付きで実行
RUST_LOG=debug cargo run -- -r roms/apple2e.rom -1 dos33.dsk

# ディスクログ付きで実行
cargo run -- --disk-log flow+state -1 dos33.dsk
```

---

<p align="center">
  Made with ❤️ and Rust
</p>
