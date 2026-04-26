//! セーブステート機能
//!
//! エミュレータの状態を保存・復元する

use serde::{Deserialize, Serialize};

/// CPUレジスタの状態（セーブ用）
#[derive(Serialize, Deserialize, Clone)]
pub struct CpuState {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub pc: u16,
    pub status: u8,
    pub total_cycles: u64,
    pub irq_pending: bool,
    pub nmi_pending: bool,
}

/// メモリの状態（セーブ用）
#[derive(Serialize, Deserialize, Clone)]
pub struct MemoryState {
    pub ram: Vec<u8>,    // メインRAM (64KB)
    pub bank1: Vec<u8>,  // ランゲージカード Bank1 (4KB)
    pub bank2: Vec<u8>,  // ランゲージカード Bank2 (4KB)
    pub lc_ram: Vec<u8>, // ランゲージカード RAM (8KB)
    #[serde(default)]
    pub aux_bank2: Vec<u8>, // Aux ランゲージカード Bank2 (4KB)
    #[serde(default)]
    pub aux_lc_ram: Vec<u8>, // Aux ランゲージカード RAM (16KB)

    // ソフトスイッチ
    pub lc_read_enable: bool,
    pub lc_write_enable: bool,
    pub lc_bank2: bool,
    pub lc_prewrite: bool,

    // ビデオモード
    pub text_mode: bool,
    pub mixed_mode: bool,
    pub page2: bool,
    pub hires_mode: bool,
    pub col80: bool,
    pub altchar: bool,

    // キーボード
    pub keyboard_latch: u8,
}

/// ディスクドライブの状態（セーブ用）
#[derive(Serialize, Deserialize, Clone)]
pub struct DiskDriveState {
    pub disk_loaded: bool,
    pub write_protected: bool,
    pub data: Vec<u8>, // ディスクデータ
    pub dsk_data: Option<Vec<u8>>,
    pub format: Option<String>,
    pub filename: Option<String>,
    pub modified: bool,
    pub track_image_dirty: bool,
    pub byte_position: usize,
    pub nibbles: usize,
    pub track_base: usize,
    pub phase: i32, // 現在のフェーズ
    pub phase_precise: f32,
    pub spinning: u32,
    pub write_light: u32,
    pub last_stepper_cycle: u64,
}

/// Disk IIコントローラの状態（セーブ用）
#[derive(Serialize, Deserialize, Clone)]
pub struct DiskState {
    pub curr_drive: usize,
    pub motor_off_scheduled_cycle: u64,
    pub drives: [DiskDriveState; 2],
    pub latch: u8,
    pub write_mode: bool,
    pub motor_on: bool,
    pub magnet_states: u8,
    pub load_mode: bool,
    pub shift_reg: u8,
    pub last_read_latch_cycle: u64,
    pub cumulative_cycles: u64,
    pub raw_bit_cycle_accumulator: u8,
    #[serde(default)]
    pub woz_bit_cycle_accumulator: u32,
    pub raw_bit_counter: u64,
    pub nibble_bit_count: u8,
    pub nibble_shift_register: u8,
    pub nibble_byte_ready: bool,
    pub controller_state: String,
    pub controller_edge_count: u64,
    pub last_controller_edge_cycle: u64,
    pub sync_bit_window: u16,
    pub pending_write_latch: Option<u8>,
    pub sequencer_mode: String,
    pub weak_bits_enabled: bool,
    pub write_splice_enabled: bool,
    pub disk_debug_logging: bool,
    pub last_sync_detect_cycle: u64,
}

/// ビデオの状態（セーブ用）
#[derive(Serialize, Deserialize, Clone)]
pub struct VideoState {
    pub flash_state: bool,
    pub frame_count: u64,
    pub scanner_frame: u64,
    pub scanner_scanline: u16,
    pub scanner_scanline_cycle: u8,
}

/// 完全なエミュレータ状態
#[derive(Serialize, Deserialize, Clone)]
pub struct SaveState {
    pub version: u32, // セーブフォーマットのバージョン
    pub cpu: CpuState,
    pub memory: MemoryState,
    pub disk: DiskState,
    pub video: VideoState,
    pub total_cycles: u64,
    pub frame_count: u64,
    pub pending_cpu_cycles: u32,
    pub floating_bus_value: u8,
    pub floating_bus_address: u16,
}

impl SaveState {
    pub const CURRENT_VERSION: u32 = 2;
}
