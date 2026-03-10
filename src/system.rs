//! System-wide timing constants and phase markers for A2RS.
//!
//! Phase 3 introduces a system tick model where one `tick()` equals one CPU cycle.

/// Apple II master timing expressed in CPU cycles.
pub const CPU_HZ: f64 = 1_023_000.0;
pub const CYCLES_PER_SCANLINE: u64 = 65;
pub const SCANLINES_PER_FRAME: u64 = 262;
pub const CYCLES_PER_FRAME: u64 = CYCLES_PER_SCANLINE * SCANLINES_PER_FRAME;

/// Disk II nominal relationship: 4 CPU cycles per raw disk bit.
pub const CPU_CYCLES_PER_DISK_BIT: u64 = 4;
