//! セーブステート機能
//! 
//! エミュレータの状態を保存・復元する

use serde::{Serialize, Deserialize};

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
    pub ram: Vec<u8>,           // メインRAM (64KB)
    pub bank1: Vec<u8>,         // ランゲージカード Bank1 (4KB)
    pub bank2: Vec<u8>,         // ランゲージカード Bank2 (4KB)
    pub lc_ram: Vec<u8>,        // ランゲージカード RAM (8KB)
    
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
    pub data: Vec<u8>,          // ディスクデータ
    pub byte_position: usize,
    pub phase: i32,             // 現在のフェーズ
}

/// Disk IIコントローラの状態（セーブ用）
#[derive(Serialize, Deserialize, Clone)]
pub struct DiskState {
    pub curr_drive: usize,
    pub drives: [DiskDriveState; 2],
    pub latch: u8,
    pub write_mode: bool,
    pub motor_on: bool,
}

/// ビデオの状態（セーブ用）
#[derive(Serialize, Deserialize, Clone)]
pub struct VideoState {
    pub flash_state: bool,
    pub frame_count: u64,
}

/// 完全なエミュレータ状態
#[derive(Serialize, Deserialize, Clone)]
pub struct SaveState {
    pub version: u32,           // セーブフォーマットのバージョン
    pub cpu: CpuState,
    pub memory: MemoryState,
    pub disk: DiskState,
    pub video: VideoState,
    pub total_cycles: u64,
    pub frame_count: u64,
}

impl SaveState {
    pub const CURRENT_VERSION: u32 = 1;
}
