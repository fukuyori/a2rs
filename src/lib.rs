//! A2RS - Apple II Emulator in Rust
//!
//! A cycle-accurate Apple II emulator supporting:
//! - Apple II, II+, IIe, IIe Enhanced
//! - Disk II with DSK/NIB format support
//! - SafeFast disk acceleration
//! - Text, Lo-Res, Hi-Res graphics

pub mod cpu;
pub mod memory;
pub mod video;
pub mod disk;
pub mod disk_log;
pub mod apple2;
pub mod savestate;
pub mod sound;
pub mod gamepad;
pub mod config;
pub mod gui;
pub mod profiler;
