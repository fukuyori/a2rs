//! Disk II ログシステム（AppleWin型設計）
//!
//! 原則:
//! 1. ログは「現象」ではなく「判断」を記録
//! 2. 状態遷移のみ記録（毎回のI/Oは記録しない）
//! 3. レベル分離: FLOW / STATE / DECIDE / NIBBLE

use std::sync::atomic::{AtomicU32, Ordering};

bitflags::bitflags! {
    /// ログカテゴリ（AppleWin互換）
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct DiskLogLevel: u32 {
        /// L1: 何が起きているか（人間向け）
        const FLOW   = 0b0001;
        /// L2: 状態遷移（開発者向け）
        const STATE  = 0b0010;
        /// L2: 判断（FastDisk等）
        const DECIDE = 0b0100;
        /// L3: 生データ（短時間のみ）
        const NIBBLE = 0b1000;
    }
}

/// FastDisk無効化の理由コード（AppleWin互換）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FastDisableReason {
    /// ニブル単位の読み取り検出
    NibbleRead,
    /// ハーフトラック検出
    HalfTrack,
    /// タイミング観測ループ検出
    TimingLoop,
    /// 連続ラッチアクセス（256回超）
    ExcessiveLatchRead,
    /// 急激なフェーズ変化
    RapidPhaseChange,
    /// 書き込み操作検出
    WriteOperation,
    /// 不明なパターン
    UnknownPattern,
}

impl std::fmt::Display for FastDisableReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FastDisableReason::NibbleRead => write!(f, "NIBBLE_READ"),
            FastDisableReason::HalfTrack => write!(f, "HALF_TRACK"),
            FastDisableReason::TimingLoop => write!(f, "TIMING_LOOP"),
            FastDisableReason::ExcessiveLatchRead => write!(f, "EXCESSIVE_LATCH"),
            FastDisableReason::RapidPhaseChange => write!(f, "RAPID_PHASE"),
            FastDisableReason::WriteOperation => write!(f, "WRITE_OP"),
            FastDisableReason::UnknownPattern => write!(f, "UNKNOWN"),
        }
    }
}

/// グローバルログレベル
static LOG_LEVEL: AtomicU32 = AtomicU32::new(0);

/// ログレベルを設定
pub fn set_log_level(level: DiskLogLevel) {
    LOG_LEVEL.store(level.bits(), Ordering::Relaxed);
}

/// 現在のログレベルを取得
pub fn get_log_level() -> DiskLogLevel {
    DiskLogLevel::from_bits_truncate(LOG_LEVEL.load(Ordering::Relaxed))
}

/// ログレベルが有効かチェック
#[inline]
pub fn is_enabled(flag: DiskLogLevel) -> bool {
    (LOG_LEVEL.load(Ordering::Relaxed) & flag.bits()) != 0
}

/// ニブルリングバッファ（最後のN個を保持）
pub struct NibbleRing {
    buf: Vec<u8>,
    pos: usize,
    capacity: usize,
}

impl NibbleRing {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: vec![0; capacity],
            pos: 0,
            capacity,
        }
    }

    pub fn push(&mut self, nibble: u8) {
        self.buf[self.pos % self.capacity] = nibble;
        self.pos += 1;
    }

    /// 最新からN個を取得（古い順）
    pub fn last_n(&self, n: usize) -> Vec<u8> {
        let n = n.min(self.capacity).min(self.pos);
        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            let idx = (self.pos - n + i) % self.capacity;
            result.push(self.buf[idx]);
        }
        result
    }

    /// ダンプ出力
    pub fn dump(&self, n: usize) {
        if !is_enabled(DiskLogLevel::NIBBLE) {
            return;
        }
        let data = self.last_n(n);
        println!("[DUMP] Last {} nibbles:", data.len());
        for (i, b) in data.iter().enumerate() {
            print!("{:02X} ", b);
            if (i + 1) % 16 == 0 {
                println!();
            }
        }
        if data.len() % 16 != 0 {
            println!();
        }
    }
}

impl Default for NibbleRing {
    fn default() -> Self {
        Self::new(256)
    }
}

// ============================================================
// ログ出力関数（AppleWin的「判断」ベース）
// ============================================================

/// [FLOW] モーターON
pub fn log_motor_on() {
    if is_enabled(DiskLogLevel::FLOW) {
        println!("[DISK] Motor ON");
    }
}

/// [FLOW] モーターOFF
pub fn log_motor_off() {
    if is_enabled(DiskLogLevel::FLOW) {
        println!("[DISK] Motor OFF");
    }
}

/// [STATE] トラック変更
pub fn log_track_change(from: u8, to: u8) {
    if is_enabled(DiskLogLevel::STATE) {
        println!("[STATE] Track {} -> {}", from, to);
    }
}

/// [FLOW] 同期マーク検出
pub fn log_sync_found(marker: &str, track: u8, pos: usize) {
    if is_enabled(DiskLogLevel::FLOW) {
        println!("[DISK] Sync {} at T={} pos={}", marker, track, pos);
    }
}

/// [FLOW] セクタヘッダ検出
pub fn log_sector_header(track: u8, sector: u8, volume: u8) {
    if is_enabled(DiskLogLevel::FLOW) {
        println!("[DISK] Sector header: T={} S={} V={}", track, sector, volume);
    }
}

/// [FLOW] セクタ読み取り完了
pub fn log_sector_read(track: u8, sector: u8) {
    if is_enabled(DiskLogLevel::FLOW) {
        println!("[DISK] Sector read: T={} S={}", track, sector);
    }
}

/// [FLOW] ブートジャンプ
pub fn log_boot_jump(addr: u16) {
    if is_enabled(DiskLogLevel::FLOW) {
        println!("[BOOT] Jump to ${:04X}", addr);
    }
}

/// [DECIDE] FastDisk無効化（文字列版 - 後方互換）
pub fn log_fastdisk_disabled(reason: &str) {
    if is_enabled(DiskLogLevel::DECIDE) {
        println!("[FAST] Disabled: {}", reason);
    }
}

/// [DECIDE] FastDisk無効化（理由コード版）
pub fn log_fastdisk_disabled_reason(reason: FastDisableReason) {
    if is_enabled(DiskLogLevel::DECIDE) {
        println!("[FAST] Disabled: {}", reason);
    }
}

/// FastDisk有効化の理由コード
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FastEnableReason {
    /// RWTS検出
    RwtsDetected,
    /// 正規ブートシーケンス検出
    BootSequence,
    /// 連続正常読み取り
    ConsistentReads,
}

impl std::fmt::Display for FastEnableReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FastEnableReason::RwtsDetected => write!(f, "RWTS_DETECTED"),
            FastEnableReason::BootSequence => write!(f, "BOOT_SEQUENCE"),
            FastEnableReason::ConsistentReads => write!(f, "CONSISTENT_READS"),
        }
    }
}

/// [DECIDE] FastDisk有効化（理由なし - 後方互換）
pub fn log_fastdisk_enabled() {
    if is_enabled(DiskLogLevel::DECIDE) {
        println!("[FAST] Enabled");
    }
}

/// [DECIDE] FastDisk有効化（理由コード版）
pub fn log_fastdisk_enabled_reason(reason: FastEnableReason) {
    if is_enabled(DiskLogLevel::DECIDE) {
        println!("[FAST] Enabled: {}", reason);
    }
}

/// [FLOW] FastDiskセクタ読み取り
pub fn log_fastdisk_read(track: u8, sector: u8, addr: u16) {
    if is_enabled(DiskLogLevel::FLOW) {
        if addr != 0 {
            println!("[FAST] Read T={} S={} -> ${:04X}", track, sector, addr);
        } else {
            println!("[FAST] Read T={} S={}", track, sector);
        }
    }
}

/// [DECIDE] FastDisk実行中に無効化
pub fn log_fastdisk_disabled_midrun(reason: FastDisableReason) {
    if is_enabled(DiskLogLevel::DECIDE) {
        println!("[FAST] Disabled mid-run: {}", reason);
    }
}

/// [DECIDE] 同期探索失敗（1回転後）
pub fn log_sync_not_found(track: u8, rotations: u32) {
    if is_enabled(DiskLogLevel::DECIDE) {
        println!("[DISK] Sync not found after {} rotation(s) (T={})", rotations, track);
    }
}

/// [STATE] 1回転あたりのニブル数
pub fn log_rotation_nibbles(nibbles: usize) {
    if is_enabled(DiskLogLevel::STATE) {
        println!("[STATE] Rotation: {} nibbles", nibbles);
    }
}

/// [STATE] スピニング状態
pub fn log_spinning_state(motor_on: bool, spinning: u32) {
    if is_enabled(DiskLogLevel::STATE) {
        if motor_on && spinning == 0 {
            println!("[STATE] WARNING: motor_on=true but spinning=0");
        }
    }
}

/// [STATE] ドライブ選択
pub fn log_drive_select(drive: usize) {
    if is_enabled(DiskLogLevel::STATE) {
        println!("[STATE] Drive {} selected", drive + 1);
    }
}

/// [DECIDE] RWTS候補検出
pub fn log_rwts_candidate(pc: u16, score: i32) {
    if is_enabled(DiskLogLevel::DECIDE) {
        println!("[RWTS] Candidate PC=${:04X} score={}", pc, score);
    }
}

/// [DECIDE] RWTS外でのディスクアクセス検出
pub fn log_rwts_outside(pc: u16) {
    if is_enabled(DiskLogLevel::DECIDE) {
        println!("[RWTS] Outside range PC=${:04X}", pc);
    }
}

/// [FLOW] RWTS侵入検出
pub fn log_rwts_enter(track: u8, sector: u8, command: u8) {
    if is_enabled(DiskLogLevel::FLOW) {
        let cmd_str = match command {
            1 => "READ",
            2 => "WRITE",
            _ => "UNKNOWN",
        };
        println!("[RWTS] Enter: T={} S={} cmd={}", track, sector, cmd_str);
    }
}

/// [FLOW] RWTS完了
pub fn log_rwts_exit(success: bool) {
    if is_enabled(DiskLogLevel::FLOW) {
        if success {
            println!("[RWTS] Exit: OK");
        } else {
            println!("[RWTS] Exit: ERROR");
        }
    }
}

/// [DECIDE] RWTSセッション開始
pub fn log_rwts_session_start(pc: u16) {
    if is_enabled(DiskLogLevel::DECIDE) {
        println!("[RWTS] Session START at PC=${:04X}", pc);
    }
}

/// [DECIDE] RWTSセッション終了
pub fn log_rwts_session_end(reason: &str, sector_count: u32) {
    if is_enabled(DiskLogLevel::DECIDE) {
        if sector_count > 0 {
            println!("[RWTS] Session END: {} ({} sectors via FastDisk)", reason, sector_count);
        } else {
            println!("[RWTS] Session END: {}", reason);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nibble_ring() {
        let mut ring = NibbleRing::new(8);
        for i in 0..10 {
            ring.push(i as u8);
        }
        let last4 = ring.last_n(4);
        assert_eq!(last4, vec![6, 7, 8, 9]);
    }

    #[test]
    fn test_log_level() {
        set_log_level(DiskLogLevel::FLOW | DiskLogLevel::STATE);
        assert!(is_enabled(DiskLogLevel::FLOW));
        assert!(is_enabled(DiskLogLevel::STATE));
        assert!(!is_enabled(DiskLogLevel::DECIDE));
        assert!(!is_enabled(DiskLogLevel::NIBBLE));
    }
}
