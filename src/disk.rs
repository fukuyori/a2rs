//! Apple II Disk II ドライブエミュレーション
//! 
//! Disk II hardware emulation based on "Beneath Apple DOS" documentation
//! DSK/NIB形式のディスクイメージをサポート
//! SafeFast: DOSのRWTSルーチン検出時のみ高速化、怪しい挙動で即Accurateに戻る
//! RWTSキャッシュ: 読み取り完了セクタをキャッシュして高速化

// AppleWin型ログシステム
use crate::disk_log::{
    log_motor_on, log_motor_off, log_track_change,
    log_sync_found, log_fastdisk_disabled,
    log_fastdisk_enabled_reason, log_fastdisk_disabled_midrun,
    log_sector_read, log_sector_header, log_fastdisk_read,
    log_rwts_candidate, log_rwts_outside,
    log_rwts_session_start, log_rwts_session_end,
    FastEnableReason, FastDisableReason,
};

use std::collections::HashMap;

/// ディスクの定数
pub const TRACKS: usize = 35;
pub const SECTORS_PER_TRACK: usize = 16;
pub const BYTES_PER_SECTOR: usize = 256;
pub const BYTES_PER_TRACK: usize = SECTORS_PER_TRACK * BYTES_PER_SECTOR;
pub const DSK_SIZE: usize = TRACKS * BYTES_PER_TRACK; // 143360 bytes

/// NIBフォーマットの定数
pub const NIB_TRACK_SIZE: usize = 6656;
pub const NIB_SIZE: usize = TRACKS * NIB_TRACK_SIZE;

/// WOZ フォーマット用ハーフトラック定数
/// WOZ TMAP は 160クォータートラック(0〜159)を持つが、
/// エミュレータのステッパーモーターはハーフトラック単位(phase 0〜79)で動くため
/// 偶数クォータートラック位置(QT = phase * 2)のみを使う。
pub const WOZ_HALF_TRACKS: usize = 80;
pub const WOZ_NIB_SIZE: usize = WOZ_HALF_TRACKS * NIB_TRACK_SIZE;

/// RWTSセクタキャッシュ
/// 読み取り完了したセクタデータをキャッシュして高速化
#[derive(Clone)]
pub struct SectorCache {
    /// キャッシュデータ: (track, sector) -> 256バイト
    data: HashMap<(u8, u8), [u8; BYTES_PER_SECTOR]>,
    /// キャッシュが有効か
    pub enabled: bool,
    /// キャッシュヒット数（統計用）
    pub hits: u64,
    /// キャッシュミス数（統計用）
    pub misses: u64,
}

impl Default for SectorCache {
    fn default() -> Self {
        Self::new()
    }
}

impl SectorCache {
    pub fn new() -> Self {
        SectorCache {
            data: HashMap::new(),
            enabled: true,
            hits: 0,
            misses: 0,
        }
    }
    
    /// キャッシュをクリア
    pub fn clear(&mut self) {
        self.data.clear();
        self.hits = 0;
        self.misses = 0;
    }
    
    /// セクタをキャッシュに追加
    pub fn insert(&mut self, track: u8, sector: u8, data: &[u8]) {
        if !self.enabled || data.len() != BYTES_PER_SECTOR {
            return;
        }
        let mut buf = [0u8; BYTES_PER_SECTOR];
        buf.copy_from_slice(data);
        self.data.insert((track, sector), buf);
    }
    
    /// キャッシュからセクタを取得
    pub fn get(&mut self, track: u8, sector: u8) -> Option<&[u8; BYTES_PER_SECTOR]> {
        if !self.enabled {
            return None;
        }
        if let Some(data) = self.data.get(&(track, sector)) {
            self.hits += 1;
            Some(data)
        } else {
            self.misses += 1;
            None
        }
    }
    
    /// 特定セクタを無効化（書き込み時）
    pub fn invalidate(&mut self, track: u8, sector: u8) {
        self.data.remove(&(track, sector));
    }
    
    /// キャッシュサイズを取得
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// キャッシュが空かどうか
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// スピニング停止までのサイクル数（約1秒）
const SPINNING_CYCLES: u32 = 1_000_000;

/// SafeFast: Fastモードの有効期限（サイクル数）- ラッチ方式では使用しない
#[allow(dead_code)]
const FAST_MODE_TTL: u64 = 100_000;

/// SafeFast: ロックアウト期間（サイクル数）- 使用しない（ラッチ方式）
#[allow(dead_code)]
const LOCKOUT_DURATION: u64 = 500_000;

/// SafeFast: Candidateからの昇格に必要なスコア
const CANDIDATE_THRESHOLD: i32 = 2;

/// SafeFast: 同一トラックでの連続読み取り上限（標準1周の約2倍）
const MAX_CONSECUTIVE_READS: u32 = 14000;

/// SafeFast: 短時間フェーズ変化の閾値
const RAPID_PHASE_THRESHOLD: u32 = 8;
const RAPID_PHASE_CYCLES: u64 = 5000;

/// SafeFast: ディスク速度モード（観測用）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiskSpeedMode {
    /// 常にニブル単位（正確モード）
    Accurate,
    /// 正規I/Oの可能性を観測中
    Candidate { score: i32 },
    /// Fastモード適用中
    Fast,
}

impl Default for DiskSpeedMode {
    fn default() -> Self {
        DiskSpeedMode::Accurate
    }
}

/// 6-and-2エンコーディングテーブル
const WRITE_TABLE: [u8; 64] = [
    0x96, 0x97, 0x9A, 0x9B, 0x9D, 0x9E, 0x9F, 0xA6,
    0xA7, 0xAB, 0xAC, 0xAD, 0xAE, 0xAF, 0xB2, 0xB3,
    0xB4, 0xB5, 0xB6, 0xB7, 0xB9, 0xBA, 0xBB, 0xBC,
    0xBD, 0xBE, 0xBF, 0xCB, 0xCD, 0xCE, 0xCF, 0xD3,
    0xD6, 0xD7, 0xD9, 0xDA, 0xDB, 0xDC, 0xDD, 0xDE,
    0xDF, 0xE5, 0xE6, 0xE7, 0xE9, 0xEA, 0xEB, 0xEC,
    0xED, 0xEE, 0xEF, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6,
    0xF7, 0xF9, 0xFA, 0xFB, 0xFC, 0xFD, 0xFE, 0xFF,
];

/// DOS 3.3セクターインターリーブ
const DOS_SECTOR_ORDER: [usize; 16] = [0, 7, 14, 6, 13, 5, 12, 4, 11, 3, 10, 2, 9, 1, 8, 15];

/// ProDOSセクターオーダー
const PRODOS_SECTOR_ORDER: [usize; 16] = [0, 8, 1, 9, 2, 10, 3, 11, 4, 12, 5, 13, 6, 14, 7, 15];

/// ディスクイメージ形式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiskFormat {
    Dsk,
    Nib,
    #[allow(dead_code)]
    Po,
    /// WOZ 1.0 / 2.0 形式（読み取り専用、NIBに変換済み）
    Woz,
}

/// フロッピーディスクの状態
#[derive(Clone)]
pub struct FloppyDisk {
    /// ディスクデータ（NIB形式で保持）
    pub data: Vec<u8>,
    /// 元のDSKデータ（セクタ直接読み取り用、Fast Disk高速化）
    pub dsk_data: Option<Vec<u8>>,
    /// 元のディスクフォーマット
    pub format: Option<DiskFormat>,
    /// 書き込みプロテクト
    pub write_protected: bool,
    /// ディスクがロードされているか
    pub disk_loaded: bool,
    /// ディスクファイル名（パス）
    pub filename: Option<String>,
    /// 変更されたか
    pub modified: bool,
    /// トラック内のバイト位置
    pub byte_position: usize,
    /// トラック内のニブル数（デフォルト = NIB_TRACK_SIZE）
    pub nibbles: usize,
    /// WOZ: ハーフトラックごとの実ニブル数（回転タイミング再現用）
    /// None = 全トラック同一（NIB_TRACK_SIZE）
    pub track_nibble_counts: Option<Vec<usize>>,
    /// トラックイメージがダーティか
    pub track_image_dirty: bool,
    /// トラック開始位置キャッシュ（高速化用）
    pub track_base: usize,
    /// WOZ: ハーフトラックごとの生ビットストリーム（パック済み、MSB first）
    /// ビットレベルシーケンサエミュレーション用
    pub woz_bitstreams: Option<Vec<Vec<u8>>>,
    /// WOZ: ハーフトラックごとのビット数
    pub woz_bit_counts: Option<Vec<usize>>,
}

impl Default for FloppyDisk {
    fn default() -> Self {
        Self::new()
    }
}

impl FloppyDisk {
    pub fn new() -> Self {
        FloppyDisk {
            data: vec![0; NIB_SIZE],
            dsk_data: None,
            format: None,
            write_protected: false,
            disk_loaded: false,
            filename: None,
            modified: false,
            byte_position: 0,
            nibbles: NIB_TRACK_SIZE,
            track_nibble_counts: None,
            track_image_dirty: false,
            track_base: 0,
            woz_bitstreams: None,
            woz_bit_counts: None,
        }
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.data = vec![0; NIB_SIZE];
        self.dsk_data = None;
        self.format = None;
        self.write_protected = false;
        self.disk_loaded = false;
        self.filename = None;
        self.modified = false;
        self.byte_position = 0;
        self.nibbles = NIB_TRACK_SIZE;
        self.track_nibble_counts = None;
        self.track_image_dirty = false;
        self.track_base = 0;
        self.woz_bitstreams = None;
        self.woz_bit_counts = None;
    }
    
    /// トラックベース位置を更新
    #[inline(always)]
    pub fn update_track_base(&mut self, track: usize) {
        self.track_base = track * NIB_TRACK_SIZE;
    }
    
    /// セクタを直接読み取り（Fast Disk用、将来の拡張用）
    /// 成功時は256バイトのセクタデータを返す
    #[inline]
    #[allow(dead_code)]
    pub fn read_sector_direct(&self, track: usize, sector: usize) -> Option<&[u8]> {
        if let Some(ref dsk) = self.dsk_data {
            if track < TRACKS && sector < SECTORS_PER_TRACK {
                let offset = track * BYTES_PER_TRACK + sector * BYTES_PER_SECTOR;
                if offset + BYTES_PER_SECTOR <= dsk.len() {
                    return Some(&dsk[offset..offset + BYTES_PER_SECTOR]);
                }
            }
        }
        None
    }
}

/// フロッピードライブの状態
#[derive(Clone)]
pub struct FloppyDrive {
    /// ディスク
    pub disk: FloppyDisk,
    /// 接続されているか
    #[allow(dead_code)]
    pub is_connected: bool,
    /// 現在のフェーズ（0-79、ハーフトラック単位）
    pub phase: i32,
    /// 精密なフェーズ（クォータートラック対応）
    pub phase_precise: f32,
    /// スピニングカウンタ
    pub spinning: u32,
    /// 書き込みライト
    pub write_light: u32,
    /// 最後のステッパーサイクル
    pub last_stepper_cycle: u64,
    /// キャッシュされたトラック番号（トラック変更検出用）
    cached_track: usize,
    /// WOZ ビットレベルシーケンサ: 現在のビット位置（トラック内）
    pub woz_bit_position: usize,
    /// WOZ ビットレベルシーケンサ: シフトレジスタ自己同期状態
    /// OpenEmulator の sequencerState に対応
    pub woz_sequencer_state: bool,
}

impl Default for FloppyDrive {
    fn default() -> Self {
        Self::new()
    }
}

impl FloppyDrive {
    pub fn new() -> Self {
        FloppyDrive {
            disk: FloppyDisk::new(),
            is_connected: true,
            phase: 0,
            phase_precise: 0.0,
            spinning: 0,
            write_light: 0,
            last_stepper_cycle: 0,
            cached_track: 0,
            woz_bit_position: 0,
            woz_sequencer_state: false,
        }
    }

    /// 現在のトラック番号を取得（0-34）
    #[inline(always)]
    pub fn current_track(&self) -> usize {
        ((self.phase / 2) as usize).min(TRACKS - 1)
    }

    /// 現在のハーフトラック位置を取得する。
    ///
    /// main 側のデバッグ表示や旧コードとの互換用。
    /// 0..=68 の範囲にクランプして返す。
    #[inline(always)]
    pub fn half_track_position(&self) -> i32 {
        self.phase.clamp(0, ((TRACKS - 1) * 2) as i32)
    }
    
    /// トラックベースを更新（トラック変更時のみ）
    ///
    /// WOZ フォーマット時はハーフトラック(phase)を直接インデックスとして使い、
    /// コピープロテクトで使われるハーフトラック上のデータを正しく参照する。
    #[inline(always)]
    pub fn update_track_base_if_needed(&mut self) {
        if matches!(self.disk.format, Some(DiskFormat::Woz)) {
            let half_track = (self.phase as usize).min(WOZ_HALF_TRACKS - 1);
            if half_track != self.cached_track {
                self.cached_track = half_track;
                self.disk.update_track_base(half_track);
                // WOZ: トラックごとの実ニブル数に更新（回転タイミング再現）
                if let Some(ref counts) = self.disk.track_nibble_counts {
                    if half_track < counts.len() && counts[half_track] > 0 {
                        self.disk.nibbles = counts[half_track];
                    } else {
                        self.disk.nibbles = NIB_TRACK_SIZE;
                    }
                }
            }
        } else {
            let track = self.current_track();
            if track != self.cached_track {
                self.cached_track = track;
                self.disk.update_track_base(track);
            }
        }
    }
}

/// シーケンサー機能
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SequencerFunction {
    ReadSequencing,
    DataShiftWrite,
    CheckWriteProtAndInitWrite,
    DataLoadWrite,
}

/// Phase 7: Disk II コントローラ状態機械
///
/// Q6/Q7 の線状態から導出される内部状態を明示的に持つ。
/// これにより、将来の nibble write flush / sync detect / copy protection 対応の
/// 足場を作りつつ、現状のブート互換性は維持する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskControllerState {
    Idle,
    ReadSequencing,
    CheckWriteProtect,
    WriteLoad,
    WriteShift,
}

/// Phase 8: 実験的シーケンサ厳密度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskSequencerMode {
    Safe,
    Transitional,
    Strict,
}

/// Disk IIインターフェースカード
#[derive(Clone)]
pub struct Disk2InterfaceCard {
    /// ドライブ0と1
    pub drives: [FloppyDrive; 2],
    /// 選択されているドライブ (0 or 1)
    pub curr_drive: usize,
    /// データラッチ
    pub latch: u8,
    /// モーターオン
    pub motor_on: bool,
    /// マグネット状態（フェーズ0-3）
    pub magnet_states: u8,
    /// Q6状態（false=L, true=H）
    q6: bool,
    /// Q7状態（false=L, true=H）
    q7: bool,
    /// 書き込みモード（Q7から派生）
    pub write_mode: bool,
    /// ロードモード（Q6から派生）
    pub load_mode: bool,
    /// シーケンサー機能
    pub seq_func: SequencerFunction,
    /// シフトレジスタ
    pub shift_reg: u8,
    /// 最後のサイクル
    #[allow(dead_code)]
    pub last_cycle: u64,
    /// 最後の読み取りラッチサイクル
    pub last_read_latch_cycle: u64,
    /// エンハンスディスクモード（高速化）
    pub enhance_disk: bool,
    /// Apple IIc (IWM) モード
    pub iwm_mode: bool,
    /// ブートROM
    pub boot_rom: [u8; 256],
    /// パッチ前のブートROM（WOZ/NIB用にリストア可能）
    pub original_boot_rom: [u8; 256],
    /// 累積サイクル
    pub cumulative_cycles: u64,
    /// Phase 4: 4 CPU cycles ごとの raw bit 分周器
    pub raw_bit_cycle_accumulator: u8,
    /// Phase 4: 回転中に進んだ raw bit 総数
    pub raw_bit_counter: u64,
    /// Phase 6: ニブル読み込み用のビットカウンタ (0..7)
    pub nibble_bit_count: u8,
    /// Phase 6: 進行中のニブル組み立てレジスタ
    pub nibble_shift_register: u8,
    /// Phase 6: 新しいニブルがラッチ済みか
    pub nibble_byte_ready: bool,
    /// Phase 7: 明示的なコントローラ状態
    pub controller_state: DiskControllerState,
    /// Phase 7: 状態遷移回数（診断用）
    pub controller_edge_count: u64,
    /// Phase 7: 最後の状態遷移サイクル
    pub last_controller_edge_cycle: u64,
    /// Phase 7: 読み取り同期検出のための 16bit 窓
    pub sync_bit_window: u16,
    /// Phase 7: 書き込みデータをシフトへ反映する保留ラッチ
    pub pending_write_latch: Option<u8>,
    /// Phase 8: 実験的シーケンサ厳密度
    pub sequencer_mode: DiskSequencerMode,
    /// Phase 8: weak bits 実験
    pub weak_bits_enabled: bool,
    /// Phase 8: write splice 実験
    pub write_splice_enabled: bool,
    /// Phase 8: 診断ログを有効化
    pub disk_debug_logging: bool,
    /// Phase 8: 最後の同期検出サイクル
    pub last_sync_detect_cycle: u64,
    /// セクタバイパスバッファ（高速読み取り用）
    #[allow(dead_code)]
    sector_buffer: [u8; BYTES_PER_SECTOR],
    /// セクタバッファ内の位置
    #[allow(dead_code)]
    sector_buffer_pos: usize,
    /// セクタバッファが有効か
    sector_buffer_valid: bool,
    /// バッファされているセクタ情報
    #[allow(dead_code)]
    buffered_track: usize,
    #[allow(dead_code)]
    buffered_sector: usize,
    /// SafeFast: 現在の速度モード（観測用）
    pub speed_mode: DiskSpeedMode,
    /// SafeFast: ラッチOFF（危険検知後、自動では戻さない）
    /// 解除条件: ディスク交換、Cold Reset のみ
    fastdisk_latched_off: bool,
    /// SafeFast: FastDiskが有効か（ログ用、二重ログ防止）
    fast_enabled: bool,
    /// SafeFast: 最後にFastDiskを無効化した理由（ログ表示用）
    last_fastdisk_disable_reason: Option<String>,
    /// SafeFast: 同一トラックでの連続読み取りカウント
    consecutive_reads: u32,
    /// SafeFast: 前回のトラック（異常なフェーズ変化検出用）
    last_track: usize,
    /// SafeFast: フェーズ変化回数（短時間での異常な変化を検出）
    phase_change_count: u32,
    /// SafeFast: 最後のフェーズ変化サイクル
    last_phase_change_cycle: u64,
    /// SafeFast: 連続ラッチアクセスカウント（タイミング観測検出）
    consecutive_latch_reads: u32,
    /// SafeFast: 最後のラッチアクセスサイクル
    last_latch_cycle: u64,
    /// ログ用: 同期検出バッファ（D5 AA 96/AD検出）
    sync_buf: [u8; 3],
    /// ログ用: 同期検出済みフラグ（連続ログ防止）
    sync_logged: bool,
    /// ログ用: 現在のセクタ読み取り状態
    sector_read_state: SectorReadState,
    /// ログ用: 現在読み取り中のセクタ情報
    current_sector_info: Option<SectorInfo>,
    /// SafeFast: RWTSセッション状態
    rwts_session: RwtsSession,
    /// SafeFast: セッション中のセクタ読み取りカウント
    session_sector_count: u32,
    /// SafeFast: RWTS範囲外に出たサイクル（セッション継続判定用）
    rwts_outside_cycle: u64,
    /// SafeFast: 最後のRWTSセッション終了サイクル（再入検知用）
    last_rwts_end_cycle: u64,
    /// SafeFast: motor-off予約サイクル（0=予約なし）
    motor_off_scheduled_cycle: u64,
    /// 起動ブースト: 最後のディスクI/Oサイクル
    pub last_disk_io_cycle: u64,
    /// 起動ブースト: ディスクI/O静寂検出フラグ
    pub disk_quiet: bool,
    /// 起動ブースト: I/Oカウント（累積）
    disk_io_count: u64,
    /// 起動ブースト: 前回チェック時のI/Oカウント
    disk_io_count_prev: u64,
    /// 起動ブースト: 前回I/O頻度チェックサイクル
    disk_io_check_cycle: u64,
}

/// motor-offディレイ（サイクル数）
/// 約500ms相当（1MHz想定）- AppleWin互換
const MOTOR_OFF_DELAY_CYCLES: u64 = 500_000;

/// RWTSセッション間隔閾値（サイクル数）
/// この時間内に次のRWTSが来たらmotor-onを維持
const RWTS_GAP_THRESHOLD: u64 = 200_000;

/// ディスクI/O頻度閾値（回/秒）
/// この値以下になったら「起動完了」とみなす
const DISK_IO_QUIET_THRESHOLD: u64 = 100;

/// RWTSセッション状態
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RwtsSession {
    /// セッション外（通常状態）
    Inactive,
    /// セッション中（FastDisk有効）
    Active {
        /// セッション開始時のPC
        start_pc: u16,
        /// セッション開始時のサイクル
        start_cycle: u64,
    },
}

/// セクタ読み取り状態（FLOWログ用）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SectorReadState {
    /// 待機中
    Idle,
    /// アドレスフィールド読み取り中（D5 AA 96検出後）
    ReadingAddress,
    /// データフィールド読み取り中（D5 AA AD検出後）
    ReadingData,
}

/// セクタ情報（ログ用）
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct SectorInfo {
    track: u8,
    sector: u8,
    volume: u8,
}

impl Default for Disk2InterfaceCard {
    fn default() -> Self {
        Self::new()
    }
}

impl Disk2InterfaceCard {
    pub fn new() -> Self {
        Disk2InterfaceCard {
            drives: [FloppyDrive::new(), FloppyDrive::new()],
            curr_drive: 0,
            latch: 0,
            motor_on: false,
            magnet_states: 0,
            q6: false,
            q7: false,
            write_mode: false,
            load_mode: false,
            seq_func: SequencerFunction::ReadSequencing,
            shift_reg: 0,
            last_cycle: 0,
            last_read_latch_cycle: 0,
            enhance_disk: true,
            iwm_mode: false,
            boot_rom: Self::create_boot_rom(),
            original_boot_rom: Self::create_boot_rom(),
            cumulative_cycles: 0,
            raw_bit_cycle_accumulator: 0,
            raw_bit_counter: 0,
            nibble_bit_count: 0,
            nibble_shift_register: 0,
            nibble_byte_ready: false,
            controller_state: DiskControllerState::Idle,
            controller_edge_count: 0,
            last_controller_edge_cycle: 0,
            sync_bit_window: 0,
            pending_write_latch: None,
            sequencer_mode: DiskSequencerMode::Safe,
            weak_bits_enabled: false,
            write_splice_enabled: false,
            disk_debug_logging: false,
            last_sync_detect_cycle: 0,
            sector_buffer: [0; BYTES_PER_SECTOR],
            sector_buffer_pos: 0,
            sector_buffer_valid: false,
            buffered_track: 0,
            buffered_sector: 0,
            speed_mode: DiskSpeedMode::Accurate,
            fastdisk_latched_off: false,
            fast_enabled: false,
            last_fastdisk_disable_reason: None,
            consecutive_reads: 0,
            last_track: 0,
            phase_change_count: 0,
            last_phase_change_cycle: 0,
            consecutive_latch_reads: 0,
            last_latch_cycle: 0,
            sync_buf: [0; 3],
            sync_logged: false,
            sector_read_state: SectorReadState::Idle,
            current_sector_info: None,
            rwts_session: RwtsSession::Inactive,
            session_sector_count: 0,
            rwts_outside_cycle: 0,
            last_rwts_end_cycle: 0,
            motor_off_scheduled_cycle: 0,
            last_disk_io_cycle: 0,
            disk_quiet: false,
            disk_io_count: 0,
            disk_io_count_prev: 0,
            disk_io_check_cycle: 0,
        }
    }

    /// リセット（Cold Reset: ラッチOFFも解除）
    pub fn reset(&mut self) {
        self.latch = 0;
        self.motor_on = false;
        self.magnet_states = 0;
        self.q6 = false;
        self.q7 = false;
        self.write_mode = false;
        self.load_mode = false;
        self.seq_func = SequencerFunction::ReadSequencing;
        self.shift_reg = 0;
        self.curr_drive = 0;
        self.cumulative_cycles = 0;
        self.raw_bit_cycle_accumulator = 0;
        self.raw_bit_counter = 0;
        self.nibble_bit_count = 0;
        self.nibble_shift_register = 0;
        self.nibble_byte_ready = false;
        self.controller_state = DiskControllerState::Idle;
        self.controller_edge_count = 0;
        self.last_controller_edge_cycle = 0;
        self.sync_bit_window = 0;
        self.pending_write_latch = None;
        self.last_read_latch_cycle = 0;
        self.sector_buffer_valid = false;
        self.sector_buffer_pos = 0;
        self.speed_mode = DiskSpeedMode::Accurate;
        // Cold Reset時のみラッチOFFを解除
        self.fastdisk_latched_off = false;
        self.fast_enabled = false;
        self.last_fastdisk_disable_reason = None;
        self.consecutive_reads = 0;
        self.last_track = 0;
        self.phase_change_count = 0;
        self.last_phase_change_cycle = 0;
        self.consecutive_latch_reads = 0;
        self.last_latch_cycle = 0;
        self.last_rwts_end_cycle = 0;
        self.motor_off_scheduled_cycle = 0;
        self.last_disk_io_cycle = 0;
        self.disk_quiet = false;
        self.disk_io_count = 0;
        self.disk_io_count_prev = 0;
        self.disk_io_check_cycle = 0;
        // 注意: boot_romはリセットしない（外部からロードされたROMを維持）
        // ドライブの状態をリセット
        for drive in &mut self.drives {
            drive.phase = 0;
            drive.phase_precise = 0.0;
            drive.spinning = 0;
            drive.write_light = 0;
            drive.disk.byte_position = 0;
            drive.disk.track_base = 0;
            drive.woz_bit_position = 0;
            drive.woz_sequencer_state = false;
        }
    }
    
    /// ディスク1と2を入れ替え
    pub fn swap_disks(&mut self) {
        self.drives.swap(0, 1);
        log::info!("Disks swapped: Drive1 <-> Drive2");
    }

    /// サイクル更新
    #[allow(dead_code)]
    pub fn update(&mut self, cycles: u64) {
        self.cumulative_cycles = cycles;
        
        // motor-off予約の処理
        self.check_scheduled_motor_off();
        
        // スピニング状態をチェック
        for drive in &mut self.drives {
            if drive.spinning > 0 {
                drive.spinning = drive.spinning.saturating_sub(1);
            }
            if drive.write_light > 0 {
                drive.write_light = drive.write_light.saturating_sub(1);
            }
        }
    }
    
    /// motor-off予約をチェックして実行
    fn check_scheduled_motor_off(&mut self) {
        if self.motor_off_scheduled_cycle > 0 
           && self.cumulative_cycles >= self.motor_off_scheduled_cycle 
        {
            // 予約時間に達した
            self.motor_off_scheduled_cycle = 0;
            if self.motor_on {
                self.motor_on = false;
                log_motor_off();
            }
        }
    }
    
    /// motor-offを予約（即座にOFFしない）
    fn schedule_motor_off(&mut self) {
        self.motor_off_scheduled_cycle = self.cumulative_cycles + MOTOR_OFF_DELAY_CYCLES;
    }
    
    /// motor-off予約をキャンセル（RWTS再入時）
    fn cancel_motor_off(&mut self) {
        self.motor_off_scheduled_cycle = 0;
    }
    
    /// ディスクI/O発生を記録（起動ブースト用）
    #[inline]
    fn record_disk_io(&mut self) {
        self.last_disk_io_cycle = self.cumulative_cycles;
        self.disk_io_count += 1;
    }
    
    /// ディスクI/O頻度をチェック（起動ブースト終了判定）
    /// 条件：I/O頻度が低下した（1秒あたりのI/O回数が閾値以下）
    pub fn check_disk_quiet(&mut self) -> bool {
        if self.disk_quiet {
            return true;
        }
        
        // 1秒ごとにI/O頻度を計算
        let elapsed = self.cumulative_cycles.saturating_sub(self.disk_io_check_cycle);
        if elapsed >= 1_000_000 {
            // 1秒間のI/O回数
            let io_in_last_second = self.disk_io_count.saturating_sub(self.disk_io_count_prev);
            
            // 閾値以下ならdisk_quiet
            if io_in_last_second < DISK_IO_QUIET_THRESHOLD {
                self.disk_quiet = true;
                return true;
            }
            
            // 次の計測期間へ
            self.disk_io_count_prev = self.disk_io_count;
            self.disk_io_check_cycle = self.cumulative_cycles;
        }
        
        false
    }

    /// ディスクをロード
    pub fn insert_disk(&mut self, drive: usize, data: &[u8], format: DiskFormat) -> Result<(), &'static str> {
        self.insert_disk_with_name(drive, data, format, None)
    }
    
    /// ファイル名付きでディスクを挿入
    pub fn insert_disk_with_name(&mut self, drive: usize, data: &[u8], format: DiskFormat, filename: Option<String>) -> Result<(), &'static str> {
        if drive > 1 {
            return Err("Invalid drive number");
        }

        let floppy = &mut self.drives[drive].disk;

        match format {
            DiskFormat::Dsk => {
                if data.len() != DSK_SIZE {
                    return Err("Invalid DSK file size");
                }
                floppy.data = Self::dsk_to_nib(data, &DOS_SECTOR_ORDER);
                // セクタ直接読み取り用にDSKデータも保持
                floppy.dsk_data = Some(data.to_vec());
                floppy.format = Some(format);
            }
            DiskFormat::Po => {
                if data.len() != DSK_SIZE {
                    return Err("Invalid PO file size");
                }
                floppy.data = Self::dsk_to_nib(data, &PRODOS_SECTOR_ORDER);
                // ProDOS用にセクタ順序を変換して保持
                floppy.dsk_data = Some(Self::reorder_sectors(data, &PRODOS_SECTOR_ORDER));
                floppy.format = Some(format);
            }
            DiskFormat::Nib => {
                if data.len() != NIB_SIZE {
                    return Err("Invalid NIB file size");
                }
                floppy.data = data.to_vec();
                // NIB形式はセクタ直接読み取り非対応
                floppy.dsk_data = None;
                floppy.format = Some(DiskFormat::Nib);
            }
            DiskFormat::Woz => {
                // WOZは事前にNIB変換済みのデータを受け取る（80ハーフトラック分）
                if data.len() != WOZ_NIB_SIZE {
                    return Err("Invalid WOZ-converted NIB size");
                }
                floppy.data = data.to_vec();
                floppy.dsk_data = None;
                floppy.format = Some(DiskFormat::Woz);
                // WOZフォーマットは書き込み非対応（書き込み保護）
                floppy.write_protected = true;
            }
        }

        floppy.disk_loaded = true;
        floppy.filename = filename;
        floppy.modified = false;
        floppy.byte_position = 0;
        floppy.nibbles = NIB_TRACK_SIZE;
        floppy.track_base = 0;

        // ディスク交換時: ラッチOFFを解除（新しいディスクに対してFast再試行）
        self.fastdisk_latched_off = false;
        self.last_fastdisk_disable_reason = None;
        self.speed_mode = DiskSpeedMode::Accurate;
        self.consecutive_reads = 0;
        self.phase_change_count = 0;

        // WOZ/NIB: コピープロテクトが回転タイミングに依存するため
        // Strictシーケンサモードへの切り替えを推奨（ensure_woz_sequencer_mode() で適用）

        Ok(())
    }
    
    /// セクタ順序を変換
    fn reorder_sectors(data: &[u8], sector_order: &[usize; 16]) -> Vec<u8> {
        let mut result = vec![0u8; DSK_SIZE];
        for track in 0..TRACKS {
            for logical_sector in 0..SECTORS_PER_TRACK {
                let physical_sector = sector_order[logical_sector];
                let src_offset = track * BYTES_PER_TRACK + physical_sector * BYTES_PER_SECTOR;
                let dst_offset = track * BYTES_PER_TRACK + logical_sector * BYTES_PER_SECTOR;
                result[dst_offset..dst_offset + BYTES_PER_SECTOR]
                    .copy_from_slice(&data[src_offset..src_offset + BYTES_PER_SECTOR]);
            }
        }
        result
    }

    /// ディスクをイジェクト
    pub fn eject_disk(&mut self, drive: usize) {
        if drive <= 1 {
            self.drives[drive].disk.clear();
        }
    }

    /// Disk IIブートROMを作成（16セクター版 P5A）
    /// デフォルトブートROMを作成（未ロード状態）
    /// 
    /// 実際のDisk II Boot ROMはAppleの著作物であり、外部ファイルから
    /// ロードする必要があります。ROMがロードされていない場合は、
    /// 仮想ブートROM（VBR）モードで起動を試みます。
    fn create_boot_rom() -> [u8; 256] {
        // 未ロード状態を示す特殊パターン
        // 最初のバイトが0x00（LDA #$20ではない）の場合、VBRモードとして検出
        [0u8; 256]
    }
    
    /// 外部ファイルからブートROMをロード
    pub fn load_boot_rom(&mut self, data: &[u8]) -> Result<(), &'static str> {
        if data.len() != 256 {
            return Err("Boot ROM must be exactly 256 bytes");
        }
        // 簡易検証: Disk II ROMは 0xA2 0x20 で始まる (LDX #$20)
        if data[0] != 0xA2 || data[1] != 0x20 {
            return Err("Invalid Disk II ROM signature");
        }
        self.boot_rom.copy_from_slice(data);
        Ok(())
    }
    
    /// ROMがロードされているかチェック
    pub fn is_rom_loaded(&self) -> bool {
        // Disk II ROMは 0xA2 0x20 (LDX #$20) で始まる
        self.boot_rom[0] == 0xA2 && self.boot_rom[1] == 0x20
    }

    /// ブートROMからの読み取り
    pub fn read_rom(&self, address: u8) -> u8 {
        self.boot_rom[address as usize]
    }

    /// シーケンサー機能を更新（アドレスの下位ビットから）
    fn update_sequencer_function(&mut self, address: u8) {
        // Q6: $C0xC (Q6L) / $C0xD (Q6H)
        // Q7: $C0xE (Q7L) / $C0xF (Q7H)
        match address & 0x03 {
            0x00 => self.q6 = false,  // Q6L
            0x01 => self.q6 = true,   // Q6H
            0x02 => self.q7 = false,  // Q7L
            0x03 => self.q7 = true,   // Q7H
            _ => {}
        }
        
        // write_mode = Q7, load_mode = Q6
        self.write_mode = self.q7;
        self.load_mode = self.q6;

        self.seq_func = match (self.write_mode, self.load_mode) {
            (false, false) => SequencerFunction::ReadSequencing,
            (false, true) => SequencerFunction::CheckWriteProtAndInitWrite,
            (true, false) => SequencerFunction::DataShiftWrite,
            (true, true) => SequencerFunction::DataLoadWrite,
        };

        self.update_controller_state();
    }

    #[inline]
    fn update_controller_state(&mut self) {
        let new_state = match (self.write_mode, self.load_mode) {
            (false, false) => DiskControllerState::ReadSequencing,
            (false, true) => DiskControllerState::CheckWriteProtect,
            (true, false) => DiskControllerState::WriteShift,
            (true, true) => DiskControllerState::WriteLoad,
        };

        if self.controller_state != new_state {
            self.controller_state = new_state;
            self.controller_edge_count = self.controller_edge_count.saturating_add(1);
            self.last_controller_edge_cycle = self.cumulative_cycles;

            match new_state {
                DiskControllerState::WriteLoad => {
                    self.pending_write_latch = Some(self.latch);
                }
                DiskControllerState::WriteShift => {
                    if self.pending_write_latch.is_none() {
                        self.pending_write_latch = Some(self.latch);
                    }
                }
                DiskControllerState::ReadSequencing | DiskControllerState::CheckWriteProtect => {
                    self.pending_write_latch = None;
                }
                DiskControllerState::Idle => {}
            }
        }
    }

    // ========================================
    // SafeFast: 安全な高速化モード
    // 核心: 「ON条件」より「OFF条件」を多く・早く・確実に
    // ラッチ方式: 一度危険検知したら自動では戻さない
    // ========================================
    
    /// SafeFast: 実効的な高速化が有効か
    /// enhance_disk（ユーザー設定）AND NOT fastdisk_latched_off
    #[inline]
    pub fn is_fastdisk_effective(&self) -> bool {
        self.enhance_disk && !self.fastdisk_latched_off
    }

    /// 現在のディスク速度モード名
    #[inline]
    pub fn speed_mode_name(&self) -> &'static str {
        match self.speed_mode {
            DiskSpeedMode::Accurate => "Accurate",
            DiskSpeedMode::Candidate { .. } => "Candidate",
            DiskSpeedMode::Fast => "Fast",
        }
    }

    /// RWTSセッションがアクティブか
    #[inline]
    pub fn rwts_session_active(&self) -> bool {
        matches!(self.rwts_session, RwtsSession::Active { .. })
    }

    /// Phase 2: AccurateBoost 用の推奨ブースト倍率
    /// CPUとディスクは同じ emu.run_frame() の中で進むため、
    /// フレーム単位の追加実行で安全寄りに高速化する。
    #[inline]
    pub fn suggested_accurate_boost_multiplier(&self) -> u32 {
        if !self.motor_on {
            return 1;
        }

        // NIB / WOZ は物理再現優先のため控えめにする
        if matches!(self.drives[self.curr_drive].disk.format, Some(DiskFormat::Nib) | Some(DiskFormat::Woz)) {
            return 2;
        }

        match self.speed_mode {
            DiskSpeedMode::Fast => 8,
            DiskSpeedMode::Candidate { .. } => 6,
            DiskSpeedMode::Accurate => {
                if self.rwts_session_active() || self.is_fastdisk_effective() {
                    4
                } else {
                    2
                }
            }
        }
    }
    
    pub fn set_experimental_options(
        &mut self,
        sequencer_mode: DiskSequencerMode,
        weak_bits_enabled: bool,
        write_splice_enabled: bool,
        disk_debug_logging: bool,
    ) {
        self.sequencer_mode = sequencer_mode;
        self.weak_bits_enabled = weak_bits_enabled;
        self.write_splice_enabled = write_splice_enabled;
        self.disk_debug_logging = disk_debug_logging;
        self.nibble_bit_count = 0;
        self.nibble_shift_register = 0;
        self.nibble_byte_ready = false;
        self.sync_bit_window = 0;
    }

    /// WOZ/NIB ディスクが挿入されている場合、コピープロテクトの回転タイミングを
    /// 正確に再現するため Strict シーケンサモードに自動切り替えする。
    /// config ロード後（set_experimental_options 後）に呼ぶこと。
    pub fn ensure_woz_sequencer_mode(&mut self) {
        for drive in &self.drives {
            if matches!(drive.disk.format, Some(DiskFormat::Woz) | Some(DiskFormat::Nib)) {
                if self.sequencer_mode == DiskSequencerMode::Safe {
                    self.sequencer_mode = DiskSequencerMode::Strict;
                    self.nibble_bit_count = 0;
                    self.nibble_shift_register = 0;
                    self.nibble_byte_ready = false;
                    self.sync_bit_window = 0;
                    log::info!("WOZ/NIB disk detected: auto-enabled Strict sequencer mode");
                }
                return;
            }
        }
    }

    pub fn diagnostics_line(&self) -> String {
        format!(
            "mode={} weak_bits={} write_splice={} ctrl={} edges={} sync_window={:04X}",
            self.sequencer_mode_name(),
            self.weak_bits_enabled,
            self.write_splice_enabled,
            self.controller_state_name(),
            self.controller_edge_count,
            self.sync_bit_window,
        )
    }

    #[inline]
    pub fn motor_off_scheduled_cycle(&self) -> u64 {
        self.motor_off_scheduled_cycle
    }

    #[inline]
    pub fn set_motor_off_scheduled_cycle_for_restore(&mut self, cycle: u64) {
        self.motor_off_scheduled_cycle = cycle;
    }

    #[inline]
    pub fn motor_off_delay_remaining_cycles(&self) -> u64 {
        self.motor_off_scheduled_cycle.saturating_sub(self.cumulative_cycles)
    }

    pub fn realism_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!(
            "Disk realism: mode={} ctrl={} motor={} pending_off={} cyc q6={} q7={} load={} write={} drive=D{}",
            self.sequencer_mode_name(),
            self.controller_state_name(),
            if self.motor_on { "ON" } else { "OFF" },
            self.motor_off_delay_remaining_cycles(),
            self.q6 as u8,
            self.q7 as u8,
            self.load_mode as u8,
            self.write_mode as u8,
            self.curr_drive + 1,
        ));

        lines.push(format!(
            "RWTS={} speed={} latch={:02X} shift={:02X} sync={:04X} pending_write={}",
            self.rwts_session_active(),
            self.speed_mode_name(),
            self.latch,
            self.shift_reg,
            self.sync_bit_window,
            self.pending_write_latch
                .map(|v| format!("{:02X}", v))
                .unwrap_or_else(|| "--".to_string()),
        ));

        for (idx, drive) in self.drives.iter().enumerate() {
            let loaded = if drive.disk.disk_loaded { "loaded" } else { "empty" };
            let ro = if drive.disk.write_protected { "RO" } else { "RW" };
            let dirty = if drive.disk.modified || drive.disk.track_image_dirty { "dirty" } else { "clean" };
            let active = if idx == self.curr_drive { "*" } else { " " };
            let half_track = drive.phase / 2;
            let half_step = (drive.phase & 1) != 0;
            lines.push(format!(
                "{}D{} {} {} {} track={} half={} phase={} phase_prec={:.1} spin={} wr={} pos={} base={}",
                active,
                idx + 1,
                loaded,
                ro,
                dirty,
                drive.current_track(),
                if half_step { format!("{}.5", half_track) } else { format!("{}", half_track) },
                drive.phase,
                drive.phase_precise,
                drive.spinning,
                drive.write_light,
                drive.disk.byte_position,
                drive.disk.track_base,
            ));
        }

        lines
    }

    /// Phase 7: 現在のコントローラ状態名
    #[inline]
    pub fn controller_state_name(&self) -> &'static str {
        match self.controller_state {
            DiskControllerState::Idle => "Idle",
            DiskControllerState::ReadSequencing => "ReadSequencing",
            DiskControllerState::CheckWriteProtect => "CheckWriteProtect",
            DiskControllerState::WriteLoad => "WriteLoad",
            DiskControllerState::WriteShift => "WriteShift",
        }
    }

    #[inline]
    fn update_sync_window_with_bit(&mut self, bit: u8) {
        self.sync_bit_window = (self.sync_bit_window << 1) | ((bit as u16) & 1);
    }

    #[inline]
    fn update_sync_window_with_byte(&mut self, value: u8) {
        self.sync_bit_window = ((self.sync_bit_window << 8) | value as u16) & 0xFFFF;
    }

    /// SafeFast: CPUのPCとメモリを観測して正規DOS/ProDOS I/Oを検出
    /// RWTSセッション単位でFastDiskを管理
    pub fn observe_pc_with_memory(&mut self, pc: u16, _memory: &[u8]) {
        // ラッチOFF済み or ユーザー設定OFF -> 何もしない
        if self.fastdisk_latched_off || !self.enhance_disk {
            return;
        }
        
        // NIB / WOZ フォーマットは常にAccurate（物理構造が本体）
        if matches!(self.drives[self.curr_drive].disk.format, Some(DiskFormat::Nib) | Some(DiskFormat::Woz)) {
            self.speed_mode = DiskSpeedMode::Accurate;
            return;
        }

        // PC範囲チェック: RWTS/MLIは複数の位置にある可能性
        let in_rwts_range = (pc >= 0x3D00 && pc < 0x4000)  // DOS 3.3 初期位置
                         || (pc >= 0x9D00 && pc < 0xA000)  // リロケート後
                         || (pc >= 0xB700 && pc < 0xC000); // 最終位置
        
        // RWTSセッション管理
        match self.rwts_session {
            RwtsSession::Inactive => {
                // セッション外：RWTS進入を検出
                if in_rwts_range && self.motor_on {
                    // スコアリングでRWTS進入を確認
                    match self.speed_mode {
                        DiskSpeedMode::Accurate => {
                            self.speed_mode = DiskSpeedMode::Candidate { score: 1 };
                            log_rwts_candidate(pc, 1);
                        }
                        DiskSpeedMode::Candidate { score } => {
                            let new_score = score + 1;
                            log_rwts_candidate(pc, new_score);
                            if new_score >= CANDIDATE_THRESHOLD {
                                // RWTSセッション開始
                                self.start_rwts_session(pc);
                            } else {
                                self.speed_mode = DiskSpeedMode::Candidate { score: new_score };
                            }
                        }
                        DiskSpeedMode::Fast => {
                            // すでにFast（通常はここには来ない）
                        }
                    }
                } else if !in_rwts_range {
                    // RWTS外 -> Candidateをリセット
                    if matches!(self.speed_mode, DiskSpeedMode::Candidate { .. }) {
                        log_rwts_outside(pc);
                        self.speed_mode = DiskSpeedMode::Accurate;
                    }
                }
            }
            RwtsSession::Active { start_cycle, .. } => {
                // セッション中：継続または終了を判定
                let _session_cycles = self.cumulative_cycles.saturating_sub(start_cycle);
                
                // モーター状態の判定（予約中はON扱い）
                let motor_effectively_on = self.motor_on || self.motor_off_scheduled_cycle > 0;
                
                // AppleWin互換：セッション継続を最優先
                // motor OFFが確定するまでセッションを維持
                if !motor_effectively_on {
                    // モーターOFF（予約も完了） -> セッション終了
                    self.end_rwts_session("motor off");
                } else if in_rwts_range {
                    // RWTS範囲内に戻った -> セッション継続
                    self.rwts_outside_cycle = 0;
                }
                // RWTS範囲外でもmotor ONならセッション継続（DOS的な挙動）
            }
        }
    }
    
    /// RWTSセッション開始
    fn start_rwts_session(&mut self, pc: u16) {
        // RWTS再入検知：前回セッション終了から短時間なら motor-off 予約をキャンセル
        let gap = self.cumulative_cycles.saturating_sub(self.last_rwts_end_cycle);
        if gap < RWTS_GAP_THRESHOLD && self.motor_off_scheduled_cycle > 0 {
            self.cancel_motor_off();
            // motor は ON のまま維持
        }
        
        self.rwts_session = RwtsSession::Active {
            start_pc: pc,
            start_cycle: self.cumulative_cycles,
        };
        self.session_sector_count = 0;
        self.speed_mode = DiskSpeedMode::Fast;
        self.try_enable_fastdisk(FastEnableReason::RwtsDetected);
        self.consecutive_reads = 0;
        self.phase_change_count = 0;
        
        // DECIDEログ
        log_rwts_session_start(pc);
    }
    
    /// RWTSセッション終了
    fn end_rwts_session(&mut self, reason: &str) {
        if let RwtsSession::Active { .. } = self.rwts_session {
            // DECIDEログ
            log_rwts_session_end(reason, self.session_sector_count);
            
            // 最終セッション終了サイクルを記録（RWTS再入検知用）
            self.last_rwts_end_cycle = self.cumulative_cycles;
            
            self.rwts_session = RwtsSession::Inactive;
            self.fast_enabled = false;
            self.speed_mode = DiskSpeedMode::Accurate;
            self.session_sector_count = 0;
        }
    }
    
    /// SafeFast: 旧API互換（メモリなしバージョン）
    #[allow(dead_code)]
    pub fn observe_pc(&mut self, pc: u16) {
        if self.fastdisk_latched_off || !self.enhance_disk {
            return;
        }
        
        // NIB / WOZ フォーマットは常にAccurate
        if matches!(self.drives[self.curr_drive].disk.format, Some(DiskFormat::Nib) | Some(DiskFormat::Woz)) {
            self.speed_mode = DiskSpeedMode::Accurate;
            return;
        }

        // PC範囲チェック: 複数位置対応
        let in_rwts_range = (pc >= 0x3D00 && pc < 0x4000)
                         || (pc >= 0x9D00 && pc < 0xA000)
                         || (pc >= 0xB700 && pc < 0xC000);
        
        match self.speed_mode {
            DiskSpeedMode::Accurate => {
                if in_rwts_range && self.motor_on {
                    self.speed_mode = DiskSpeedMode::Candidate { score: 1 };
                    log_rwts_candidate(pc, 1);
                }
            }
            DiskSpeedMode::Candidate { score } => {
                if in_rwts_range && self.motor_on {
                    let new_score = score + 1;
                    log_rwts_candidate(pc, new_score);
                    if new_score >= CANDIDATE_THRESHOLD {
                        self.speed_mode = DiskSpeedMode::Fast;
                        self.try_enable_fastdisk(FastEnableReason::ConsistentReads);
                        self.consecutive_reads = 0;
                        self.phase_change_count = 0;
                    } else {
                        self.speed_mode = DiskSpeedMode::Candidate { score: new_score };
                    }
                } else if self.motor_on && !in_rwts_range {
                    log_rwts_outside(pc);
                    self.speed_mode = DiskSpeedMode::Accurate;
                }
            }
            DiskSpeedMode::Fast => {
                if self.motor_on && !in_rwts_range {
                    self.latch_off_reason(FastDisableReason::UnknownPattern);
                }
            }
        }
    }
    
    /// SafeFast: DOS 3.3 IOB（I/O Control Block）の妥当性チェック
    /// IOBは通常$B7E8付近にある
    /// レイアウト:
    ///   +0: 操作コード (1=READ, 2=WRITE)
    ///   +1: スロット番号 * 16 (例: $60 = slot 6)
    ///   +2: ドライブ番号 (1 or 2)
    ///   +3: ボリューム番号
    ///   +4: トラック (0-34)
    ///   +5: セクター (0-15)
    ///   +6,7: バッファアドレス (lo, hi)
    #[allow(dead_code)]
    fn looks_like_dos_iob(&self, memory: &[u8]) -> bool {
        // IOBの典型的なアドレス
        const IOB_ADDR: usize = 0xB7E8;
        self.check_iob_at(memory, IOB_ADDR)
    }
    
    /// SafeFast: 初期DOSのIOBチェック（$3E00付近）
    #[allow(dead_code)]
    fn looks_like_early_dos_iob(&self, memory: &[u8]) -> bool {
        // ブート初期のIOBアドレス候補
        const EARLY_IOB_ADDRS: [usize; 3] = [0x3E00, 0x47E8, 0x37E8];
        for &addr in &EARLY_IOB_ADDRS {
            if self.check_iob_at(memory, addr) {
                return true;
            }
        }
        false
    }
    
    /// 指定アドレスのIOB妥当性チェック
    #[allow(dead_code)]
    fn check_iob_at(&self, memory: &[u8], iob_addr: usize) -> bool {
        if memory.len() <= iob_addr + 8 {
            return false;
        }
        
        let op_code = memory[iob_addr];
        let slot = memory[iob_addr + 1];
        let drive = memory[iob_addr + 2];
        let track = memory[iob_addr + 4];
        let sector = memory[iob_addr + 5];
        let buf_lo = memory[iob_addr + 6];
        let buf_hi = memory[iob_addr + 7];
        
        // 操作コード: 1=READ, 2=WRITE
        let valid_op = op_code == 1 || op_code == 2;
        
        // スロット: $60 = slot 6 (Disk II標準)
        let valid_slot = slot == 0x60;
        
        // ドライブ: 1 or 2
        let valid_drive = drive == 1 || drive == 2;
        
        // トラック: 0-34
        let valid_track = track <= 34;
        
        // セクター: 0-15
        let valid_sector = sector <= 15;
        
        // バッファ: RAM領域 ($0000-$BFFF)
        let buf_addr = (buf_hi as u16) << 8 | buf_lo as u16;
        let valid_buffer = buf_addr < 0xC000;
        
        valid_op && valid_slot && valid_drive && valid_track && valid_sector && valid_buffer
    }
    
    /// SafeFast: 怪しい挙動を検出したらラッチOFF
    /// OFF条件を多く・早く・確実に
    fn detect_suspicious_behavior(&mut self) {
        // RWTSセッション中は全てのチェックをスキップ（AppleWin互換）
        // RWTS中の連続読み取りは正常な動作
        if matches!(self.rwts_session, RwtsSession::Active { .. }) {
            return;
        }
        
        // ① 半トラック検出（コピーガードの王道）
        let current_phase = self.drives[self.curr_drive].phase;
        if current_phase % 2 != 0 {
            self.latch_off("half-track position detected");
            return;
        }
        
        // ② 同一トラックでの異常な連続読み取り（セクタ数を大幅に超える）
        // 16セクタ × 約400ニブル/セクタ ≒ 6400、余裕を見て上限設定
        if self.consecutive_reads > MAX_CONSECUTIVE_READS {
            self.latch_off("excessive nibble reads on same track");
            return;
        }
        
        // ③ 短時間での異常なフェーズ変化（回転位相測定の兆候）
        let cycle_diff = self.cumulative_cycles.saturating_sub(self.last_phase_change_cycle);
        if self.phase_change_count > RAPID_PHASE_THRESHOLD && cycle_diff < RAPID_PHASE_CYCLES {
            self.latch_off("rapid phase changes (timing check?)");
            return;
        }
        
        // ④ トラック番号が異常（非DOS）
        let track = self.drives[self.curr_drive].current_track();
        if track > 34 {
            self.latch_off("invalid track number");
            return;
        }
    }
    
    fn remember_fastdisk_disable_reason(&mut self, reason: impl Into<String>) {
        self.last_fastdisk_disable_reason = Some(reason.into());
    }

    fn clear_fastdisk_disable_reason(&mut self) {
        self.last_fastdisk_disable_reason = None;
    }

    pub fn last_fastdisk_disable_reason(&self) -> Option<&str> {
        self.last_fastdisk_disable_reason.as_deref()
    }

    /// SafeFast: 書き込み発生時のラッチOFF
    /// 書き込みはコピー保護や自己書換えRWTSの温床
    /// ただしRWTSセッション中はセッション終了まで待つ
    fn latch_off_on_write(&mut self) {
        if self.is_fastdisk_effective() && matches!(self.speed_mode, DiskSpeedMode::Fast) {
            // RWTSセッション中はWRITE_OPを完全に無視（ログも出さない）
            // AppleWin互換：RWTS中のWRITEは位相制御/ラッチ操作であり、実際の書き込みではない
            if matches!(self.rwts_session, RwtsSession::Active { .. }) {
                // セッション中：何もしない（FastDiskを維持）
                return;
            }
            // セッション外：通常の処理
            self.latch_off_reason(FastDisableReason::WriteOperation);
        }
    }
    
    /// SafeFast: ラッチOFF（自動では戻さない）- 文字列版（後方互換）
    /// 解除条件: ディスク交換、Cold Reset のみ
    fn latch_off(&mut self, reason: &str) {
        self.remember_fastdisk_disable_reason(reason.to_string());
        log::warn!("FastDisk disabled permanently: {}", reason);

        // DECIDEログ: FastDisk無効化の判断
        if self.fast_enabled {
            log_fastdisk_disabled(reason);
        }
        
        // RWTSセッションも終了
        self.rwts_session = RwtsSession::Inactive;
        self.fastdisk_latched_off = true;
        self.fast_enabled = false;
        self.speed_mode = DiskSpeedMode::Accurate;
        self.consecutive_reads = 0;
        self.phase_change_count = 0;
        self.consecutive_latch_reads = 0;
    }
    
    /// SafeFast: ラッチOFF（理由コード版）
    /// コピーガード級の理由では永続的にラッチOFF（セッションも強制終了）
    /// WRITE_OPとUNKNOWN_PATTERNはセッション中は完全に無視
    fn latch_off_reason(&mut self, reason: FastDisableReason) {
        self.remember_fastdisk_disable_reason(reason.to_string());

        // RWTSセッション中の処理
        if matches!(self.rwts_session, RwtsSession::Active { .. }) {
            match reason {
                // セッション中は軽微な異常を完全に無視（ログも出さない）
                // AppleWin互換：RWTS中のWRITE/UNKNOWNは正常な動作
                FastDisableReason::WriteOperation | 
                FastDisableReason::UnknownPattern => {
                    return; // FastDiskを維持してセッション継続
                }
                // コピーガード級はセッションを強制終了
                _ => {
                    self.end_rwts_session("copy protection detected");
                    // 続けて永続ラッチOFF
                }
            }
        }
        
        // DECIDEログ: FastDisk無効化の判断
        if self.fast_enabled {
            log_fastdisk_disabled_midrun(reason);
        }
        
        // 一時的disable vs 永続的ラッチOFFの判断
        match reason {
            // 一時的disable: 次のREAD RWTSで再有効化可能
            FastDisableReason::WriteOperation | 
            FastDisableReason::UnknownPattern => {
                log::warn!("FastDisk disabled temporarily: {}", reason);
                self.fast_enabled = false;
                self.speed_mode = DiskSpeedMode::Accurate;
                self.consecutive_reads = 0;
                self.phase_change_count = 0;
                // fastdisk_latched_off は変更しない
            }
            // コピーガード級: 永続的にラッチOFF
            _ => {
                log::warn!("FastDisk disabled permanently: {}", reason);
                self.rwts_session = RwtsSession::Inactive;
                self.fastdisk_latched_off = true;
                self.fast_enabled = false;
                self.speed_mode = DiskSpeedMode::Accurate;
                self.consecutive_reads = 0;
                self.phase_change_count = 0;
                self.consecutive_latch_reads = 0;
            }
        }
    }
    
    /// SafeFast: FastDisk有効化を試みる（条件を満たした場合のみ）
    fn try_enable_fastdisk(&mut self, reason: FastEnableReason) {
        // 既に有効なら何もしない（二重ログ防止）
        if self.fast_enabled {
            return;
        }
        // ラッチOFFされていたら有効化しない
        if self.fastdisk_latched_off {
            if let Some(reason) = self.last_fastdisk_disable_reason() {
                log::info!("FastDisk remains disabled (latched off): {}", reason);
            } else {
                log::info!("FastDisk remains disabled (latched off)");
            }
            return;
        }
        // enhance_diskが無効なら有効化しない
        if !self.enhance_disk {
            return;
        }
        
        self.fast_enabled = true;
        self.speed_mode = DiskSpeedMode::Fast;
        self.clear_fastdisk_disable_reason();
        
        // DECIDEログ: FastDisk有効化の判断
        log_fastdisk_enabled_reason(reason);
    }
    
    /// SafeFast: 連続ラッチアクセス観測（タイミング観測検出）
    /// コピープロテクトはディスク回転位相を計測するため、
    /// 極端に短いサイクル間隔での連続ラッチアクセスを行う
    fn observe_latch_read(&mut self) {
        // RWTSセッション中はnibble読みカウントを無効化（AppleWin互換）
        // RWTS中の連続ラッチ読みは正常な動作
        if matches!(self.rwts_session, RwtsSession::Active { .. }) {
            return;
        }
        
        let delta = self.cumulative_cycles.saturating_sub(self.last_latch_cycle);
        self.last_latch_cycle = self.cumulative_cycles;
        
        // 4サイクル以内の連続アクセスをカウント
        if delta <= 4 {
            self.consecutive_latch_reads = self.consecutive_latch_reads.saturating_add(1);
        } else {
            self.consecutive_latch_reads = 0;
        }
        
        // Fastモード中に256回を超える連続アクセス → タイミング観測の疑い
        if self.is_fastdisk_effective() && self.consecutive_latch_reads > 256 {
            self.latch_off_reason(FastDisableReason::ExcessiveLatchRead);
        }
    }
    
    /// SafeFast: 現在Fastモードで動作可能か
    #[inline]
    fn is_safe_fast(&self) -> bool {
        self.is_fastdisk_effective() && matches!(self.speed_mode, DiskSpeedMode::Fast)
    }



    /// Phase 4/6: 1 CPU cycle 分だけディスクの bit/nibble timing を進める。
    ///
    /// - 4 CPU cycles = 1 disk bit
    /// - 8 bits = 1 nibble byte
    /// - Accurate モードでは nibble のラッチ更新を tick 側で行う
    ///
    /// FastDisk 中は従来どおり I/O access 側でバイトを進め、ここでは
    /// raw bit counter のみ維持する。
    #[inline]
    pub fn tick_disk_bit(&mut self) {
        if !self.motor_on {
            self.raw_bit_cycle_accumulator = 0;
            self.nibble_bit_count = 0;
            self.nibble_shift_register = 0;
            self.nibble_byte_ready = false;
            self.pending_write_latch = None;
            self.sync_bit_window = 0;
            return;
        }

        self.raw_bit_cycle_accumulator = self.raw_bit_cycle_accumulator.wrapping_add(1);
        if self.raw_bit_cycle_accumulator < 4 {
            return;
        }

        self.raw_bit_cycle_accumulator = 0;
        self.raw_bit_counter = self.raw_bit_counter.saturating_add(1);

        let curr_drive = self.curr_drive;
        if !self.drives[curr_drive].disk.disk_loaded {
            return;
        }

        if !self.drives[curr_drive].disk.disk_loaded {
            return;
        }

        // FastDisk 時は access 側でニブルを進めるため、bit timing のみ進める。
        if self.is_safe_fast() {
            return;
        }

        let spinning = self.drives[curr_drive].spinning;
        if spinning == 0 {
            return;
        }

        self.drives[curr_drive].update_track_base_if_needed();

        // ── WOZ ビットレベルシーケンサ (OpenEmulator 互換) ──────────────
        // WOZ ビットストリームがある場合、実機の Disk II LSS と同じアルゴリズムで
        // 1ビットずつシフトレジスタに送り込む。
        if matches!(self.sequencer_mode, DiskSequencerMode::Strict)
           && self.drives[curr_drive].disk.woz_bitstreams.is_some()
        {
            if !self.write_mode || self.iwm_mode {
                // ── 読み取りモード: READSHIFT ──
                // 現在のハーフトラックからビットを1つ読む
                let phase = (self.drives[curr_drive].phase as usize).min(WOZ_HALF_TRACKS - 1);
                let bit = {
                    let bitstreams = self.drives[curr_drive].disk.woz_bitstreams.as_ref().unwrap();
                    let bit_counts = self.drives[curr_drive].disk.woz_bit_counts.as_ref().unwrap();
                    let bs = &bitstreams[phase];
                    let bc = bit_counts[phase];
                    if bs.is_empty() || bc == 0 {
                        0u8
                    } else {
                        let bit_pos = self.drives[curr_drive].woz_bit_position % bc;
                        let byte_idx = bit_pos >> 3;
                        let bit_idx = 7 - (bit_pos & 7); // MSB first
                        self.drives[curr_drive].woz_bit_position = (bit_pos + 1) % bc;
                        (bs[byte_idx] >> bit_idx) & 1
                    }
                };

                // OpenEmulator の Disk II LSS アルゴリズム:
                // dataRegister (= self.latch) をシフトレジスタとして使う。
                // bit7=1: LOAD 状態（自己同期待ち）
                // bit7=0: SHIFT 状態（ビットをシフトイン中）
                if self.latch & 0x80 != 0 {
                    // LOAD 状態: 最初の 1-bit を待つ
                    if !self.drives[curr_drive].woz_sequencer_state {
                        // まだ最初の 1-bit を見ていない
                        if bit != 0 {
                            self.drives[curr_drive].woz_sequencer_state = true;
                        }
                    } else {
                        // 最初の 1-bit の次のビット → 新バイト開始
                        self.drives[curr_drive].woz_sequencer_state = false;
                        self.latch = 0x02 | bit;
                        self.shift_reg = self.latch;
                    }
                } else {
                    // SHIFT 状態: ビットを左シフトして挿入
                    self.latch = (self.latch << 1) | bit;
                    self.shift_reg = self.latch;
                }
            }
            return;
        }

        // ── 従来の NIB ベースパス (Safe / Transitional / Strict without WOZ bits) ──
        let disk_len = self.drives[curr_drive].disk.data.len();
        let byte_pos = self.drives[curr_drive].disk.byte_position;
        let nibbles = self.drives[curr_drive].disk.nibbles;
        let track_base = self.drives[curr_drive].disk.track_base;
        let offset = track_base + byte_pos;

        if offset >= disk_len || nibbles == 0 {
            return;
        }

        if !self.write_mode || self.iwm_mode {
            let sample = self.drives[curr_drive].disk.data[offset];
            let bit_index = match self.sequencer_mode {
                DiskSequencerMode::Strict => self.nibble_bit_count.min(7),
                _ => (self.raw_bit_counter as u8 & 7) as u8,
            };
            let bit = (sample >> (7 - bit_index)) & 1;
            self.update_sync_window_with_bit(bit);

            if matches!(self.sequencer_mode, DiskSequencerMode::Transitional | DiskSequencerMode::Strict) {
                self.nibble_shift_register = (self.nibble_shift_register << 1) | bit;
                self.nibble_bit_count = self.nibble_bit_count.saturating_add(1);

                if self.nibble_bit_count >= 8 {
                    let completed_nibble = self.nibble_shift_register;
                    self.nibble_bit_count = 0;
                    self.nibble_byte_ready = true;
                    self.latch = completed_nibble;
                    self.shift_reg = completed_nibble;
                    self.last_read_latch_cycle = self.cumulative_cycles;
                    self.last_sync_detect_cycle = self.cumulative_cycles;
                    self.nibble_shift_register = 0;

                    if matches!(self.sequencer_mode, DiskSequencerMode::Strict) {
                        self.update_sync_window_with_byte(completed_nibble);
                        self.check_sync_marker(curr_drive);
                        self.advance_track_byte_position(curr_drive);


                        if self.disk_debug_logging {
                            log::debug!(
                                "[DISK][strict] nibble ready drive={} track={} pos={} cycle={} latch={:02X} window={:04X}",
                                curr_drive + 1,
                                self.drives[curr_drive].current_track(),
                                byte_pos,
                                self.cumulative_cycles,
                                completed_nibble,
                                self.sync_bit_window,
                            );
                        }
                    } else if self.disk_debug_logging {
                        log::debug!(
                            "[DISK][transitional] nibble ready drive={} track={} pos={} cycle={} window={:04X}",
                            curr_drive + 1,
                            self.drives[curr_drive].current_track(),
                            byte_pos,
                            self.cumulative_cycles,
                            self.sync_bit_window,
                        );
                    }
                }
            }
            return;
        } else {
            let write_protected = self.drives[curr_drive].disk.write_protected;
            if write_protected {
                return;
            }

            let source_latch = self.pending_write_latch.unwrap_or(self.latch);
            let bit_index = self.nibble_bit_count.min(7);
            let bit = (source_latch >> (7 - bit_index)) & 1;
            self.nibble_shift_register = (self.nibble_shift_register << 1) | bit;
            self.nibble_bit_count = self.nibble_bit_count.saturating_add(1);

            if self.nibble_bit_count >= 8 {
                self.nibble_bit_count = 0;
                self.nibble_shift_register = source_latch;
                self.drives[curr_drive].disk.data[offset] = self.nibble_shift_register;
                self.drives[curr_drive].disk.track_image_dirty = true;
                self.drives[curr_drive].disk.modified = true;
                self.drives[curr_drive].write_light = SPINNING_CYCLES;
                self.latch = source_latch;
                self.shift_reg = source_latch;

                let next_pos = byte_pos + 1;
                self.drives[curr_drive].disk.byte_position = if next_pos >= nibbles { 0 } else { next_pos };
                self.nibble_shift_register = 0;
                self.pending_write_latch = None;
            }
        }
    }

    /// I/O読み取り ($C0E0-$C0EF)
    #[inline]
    pub fn io_read(&mut self, address: u8) -> u8 {
        // motor-off予約のチェック
        self.check_scheduled_motor_off();
        
        // ディスクI/O発生を記録（起動ブースト用）
        self.record_disk_io();
        
        let reg = address & 0x0F;

        // $C0xC-$C0xFの場合はシーケンサー機能を更新
        if reg >= 0x0C {
            self.update_sequencer_function(reg);
        }

        match reg {
            // Phase 0-3 ステッパーモーター制御
            0x00..=0x07 => {
                self.control_stepper(reg);
            }

            // Motor off
            0x08 => {
                self.control_motor(false);
            }

            // Motor on
            0x09 => {
                self.control_motor(true);
            }

            // Drive 1 select
            0x0A => {
                self.enable_drive(0);
            }

            // Drive 2 select
            0x0B => {
                self.enable_drive(1);
            }

            // Q6L - シフトデータ読み取り
            0x0C => {
                self.read_write_nibble();
            }

            // Q6H - 書き込みプロテクト読み取り / ラッチロード
            0x0D => {
                self.load_write_protect();
            }

            // Q7L - 読み取りモード設定
            0x0E => {
                self.read_write_nibble();
            }

            // Q7H - 書き込みモード設定
            0x0F => {
                // 書き込みモードへ
            }

            _ => {}
        }

        // 偶数アドレスのみラッチを返す
        if (reg & 1) == 0 {
            if self.iwm_mode && reg == 0x0C {
                // IWMモード: $C0ECはステータス/データを返す
                // bit7: データレディ（ニブルのMSBが立っていればready）
                // bit6: SENSE（モーター状態など）
                // Apple IIc ROMはbit6=0を待つループがあるので、常にbit6=0を返す
                self.latch & 0xBF  // bit6をクリア
            } else {
                self.latch
            }
        } else {
            // 奇数アドレスはフローティングバス
            0xFF
        }
    }

    /// I/O書き込み ($C0E0-$C0EF)
    #[inline]
    pub fn io_write(&mut self, address: u8, value: u8) {
        // motor-off予約のチェック
        self.check_scheduled_motor_off();
        
        // ディスクI/O発生を記録（起動ブースト用）
        self.record_disk_io();
        
        let reg = address & 0x0F;

        // $C0xC-$C0xFの場合はシーケンサー機能を更新
        if reg >= 0x0C {
            self.update_sequencer_function(reg);
        }

        match reg {
            0x00..=0x07 => self.control_stepper(reg),
            0x08 => self.control_motor(false),
            0x09 => self.control_motor(true),
            0x0A => self.enable_drive(0),
            0x0B => self.enable_drive(1),
            0x0C => self.read_write_nibble(),
            0x0D => self.load_write_protect(),
            0x0E => self.read_write_nibble(),
            0x0F => {}
            _ => {}
        }

        // データロード書き込みモードならラッチに値を設定
        if self.seq_func == SequencerFunction::DataLoadWrite {
            self.latch = value;
        }
    }

    /// モーター制御
    fn control_motor(&mut self, on: bool) {
        if on {
            // モーターON
            // 予約されていたOFFをキャンセル
            self.cancel_motor_off();
            
            if !self.motor_on {
                self.motor_on = true;
                log_motor_on();
            }
        } else {
            // モーターOFF要求
            // 即座にOFFせず予約（AppleWin互換）
            if self.motor_on && self.motor_off_scheduled_cycle == 0 {
                self.schedule_motor_off();
                // ログは実際にOFFになった時に出す
            }
            // マグネット状態をクリア
            self.magnet_states = 0;
        }

        self.check_spinning(on != self.motor_on);
    }

    /// ドライブ選択
    fn enable_drive(&mut self, drive: usize) {
        let state_changed = drive != self.curr_drive;

        self.curr_drive = drive;

        // 他のドライブのスピニングをクリア
        let other_drive = 1 - drive;
        self.drives[other_drive].spinning = 0;
        self.drives[other_drive].write_light = 0;

        self.check_spinning(state_changed);
    }

    /// ステッパーモーター制御
    fn control_stepper(&mut self, reg: u8) {
        // 借用問題を避けるために先に値をコピー
        let spinning = self.drives[self.curr_drive].spinning;
        
        if !self.motor_on && spinning == 0 {
            return;
        }

        // フェーズとオン/オフを取得
        let phase = (reg >> 1) & 3;
        let phase_bit = 1u8 << phase;

        // マグネット状態を更新
        if (reg & 1) != 0 {
            self.magnet_states |= phase_bit;
        } else {
            self.magnet_states &= !phase_bit;
        }

        // ステッパー移動を計算
        let old_phase = self.drives[self.curr_drive].phase;
        self.control_stepper_move();
        let new_phase = self.drives[self.curr_drive].phase;
        
        // SafeFast: フェーズ変化を追跡
        if old_phase != new_phase {
            self.phase_change_count += 1;
            self.last_phase_change_cycle = self.cumulative_cycles;
        }

        // サイクルを更新
        let cycles = self.cumulative_cycles;
        self.drives[self.curr_drive].last_stepper_cycle = cycles;
    }

    /// ステッパー移動を実行
    fn control_stepper_move(&mut self) {
        let drive = &mut self.drives[self.curr_drive];
        let current_phase = drive.phase & 3;
        let old_track = drive.phase / 2;

        // 移動方向を計算
        let mut direction: i32 = 0;

        // 次のフェーズがオンか
        if (self.magnet_states & (1 << ((current_phase + 1) & 3))) != 0 {
            direction += 1;
        }
        // 前のフェーズがオンか
        if (self.magnet_states & (1 << ((current_phase + 3) & 3))) != 0 {
            direction -= 1;
        }

        // フェーズを更新（0-79の範囲、ハーフトラック）
        let new_phase = (drive.phase + direction).clamp(0, 79);
        if new_phase != drive.phase {
            drive.phase = new_phase;
            drive.phase_precise = new_phase as f32;
            
            // STATEログ: トラック変化（整数トラック単位で）
            let new_track = new_phase / 2;
            if new_track != old_track {
                log_track_change(old_track as u8, new_track as u8);
            }
        }
    }

    /// スピニング状態をチェック
    fn check_spinning(&mut self, state_changed: bool) {
        let drive = &mut self.drives[self.curr_drive];

        if self.motor_on {
            drive.spinning = SPINNING_CYCLES;
        } else if state_changed {
            drive.spinning = SPINNING_CYCLES;
        }
    }
    
    /// 同期マーカー検出（FLOWログ用）
    /// D5 AA 96（アドレス）またはD5 AA AD（データ）を検出してログ
    /// セクタ読み完了時にFastDiskログを出力
    fn check_sync_marker(&mut self, drive: usize) {
        // バッファをシフト
        self.sync_buf[0] = self.sync_buf[1];
        self.sync_buf[1] = self.sync_buf[2];
        self.sync_buf[2] = self.latch;
        
        // D5 AA 96 (アドレスマーカー) 検出
        if self.sync_buf == [0xD5, 0xAA, 0x96] {
            let track = self.drives[drive].current_track();
            let pos = self.drives[drive].disk.byte_position;
            log_sync_found("D5 AA 96 (Address)", track as u8, pos);
            self.sync_logged = true;
            self.sector_read_state = SectorReadState::ReadingAddress;
            // アドレスフィールドを解析（D5 AA 96の直後のデータ）
            self.parse_address_field(drive);
        }
        // D5 AA AD (データマーカー) 検出
        else if self.sync_buf == [0xD5, 0xAA, 0xAD] {
            let track = self.drives[drive].current_track();
            let pos = self.drives[drive].disk.byte_position;
            log_sync_found("D5 AA AD (Data)", track as u8, pos);
            self.sync_logged = true;
            self.sector_read_state = SectorReadState::ReadingData;
        }
        // DE AA (エピローグ) 検出 - セクタ/データ完了
        else if self.sync_buf[1] == 0xDE && self.sync_buf[2] == 0xAA {
            if self.sector_read_state == SectorReadState::ReadingData {
                // セクタ読み完了
                if let Some(info) = self.current_sector_info {
                    // Fastモード中ならFastDiskログを出す
                    if self.fast_enabled && matches!(self.rwts_session, RwtsSession::Active { .. }) {
                        // 宛先アドレスは不明なので0を使用（将来的にはIOBから取得）
                        log_fastdisk_read(info.track, info.sector, 0x0000);
                    } else {
                        // Accurateモードなら通常のセクタ読みログ
                        log_sector_read(info.track, info.sector);
                    }
                    
                    // RWTSセッション中ならカウントをインクリメント
                    if matches!(self.rwts_session, RwtsSession::Active { .. }) {
                        self.session_sector_count += 1;
                    }
                }
                self.sector_read_state = SectorReadState::Idle;
                // current_sector_infoは次のセクタ用に保持
            } else if self.sector_read_state == SectorReadState::ReadingAddress {
                // アドレスフィールド完了
                self.sector_read_state = SectorReadState::Idle;
            }
        }
        // D5で始まる新しいシーケンスの開始
        else if self.latch == 0xD5 {
            self.sync_logged = false;
        }
    }
    
    /// アドレスフィールドからセクタ情報を抽出
    /// 4-and-4エンコードされたアドレスフィールドを解析
    fn parse_address_field(&mut self, drive: usize) {
        // アドレスフィールドのフォーマット:
        // D5 AA 96 [vol1] [vol2] [trk1] [trk2] [sec1] [sec2] [chk1] [chk2] DE AA EB
        // 各バイトは4-and-4エンコード: val = ((byte1 << 1) | 1) & byte2
        
        let disk = &self.drives[drive].disk;
        let pos = disk.byte_position;
        let track_base = disk.track_base;
        let nibbles = disk.nibbles;
        let data = &disk.data;
        
        // 現在位置から6バイト先（vol, trk, sec）を読む
        // D5 AA 96の直後から
        let read_nibble = |offset: usize| -> u8 {
            let idx = track_base + (pos + offset) % nibbles;
            if idx < data.len() { data[idx] } else { 0 }
        };
        
        let vol1 = read_nibble(0);
        let vol2 = read_nibble(1);
        let trk1 = read_nibble(2);
        let trk2 = read_nibble(3);
        let sec1 = read_nibble(4);
        let sec2 = read_nibble(5);
        
        // 4-and-4デコード
        let volume = ((vol1 << 1) | 1) & vol2;
        let track = ((trk1 << 1) | 1) & trk2;
        let sector = ((sec1 << 1) | 1) & sec2;
        
        self.current_sector_info = Some(SectorInfo {
            track,
            sector,
            volume,
        });
        
        // セクタヘッダログ
        log_sector_header(track, sector, volume);
    }

    #[inline]
    fn sequencer_mode_name(&self) -> &'static str {
        match self.sequencer_mode {
            DiskSequencerMode::Safe => "safe",
            DiskSequencerMode::Transitional => "transitional",
            DiskSequencerMode::Strict => "strict",
        }
    }

    #[inline]
    fn read_current_track_byte(&mut self, curr_drive: usize) -> u8 {
        self.drives[curr_drive].update_track_base_if_needed();

        let byte_pos = self.drives[curr_drive].disk.byte_position;
        let track_base = self.drives[curr_drive].disk.track_base;
        let offset = track_base + byte_pos;

        if offset < self.drives[curr_drive].disk.data.len() {
            self.drives[curr_drive].disk.data[offset]
        } else {
            0xFF
        }
    }

    #[inline]
    fn advance_track_byte_position(&mut self, curr_drive: usize) {
        let nibbles = self.drives[curr_drive].disk.nibbles;
        if nibbles != 0 {
            let byte_pos = self.drives[curr_drive].disk.byte_position;
            self.drives[curr_drive].disk.byte_position = (byte_pos + 1) % nibbles;
        }
    }

    /// データ読み書き（ニブル単位）- SafeFast対応版
    #[inline(always)]
    fn read_write_nibble(&mut self) {
        let curr_drive = self.curr_drive;
        
        // 先に必要な値を取得
        let disk_loaded = self.drives[curr_drive].disk.disk_loaded;
        
        if !disk_loaded {
            self.latch = 0xFF;
            return;
        }

        // IWMモードでは write_mode に関係なく常にニブルを読み取る
        // (Apple IIc ROMは$C0EFに書き込んだ後もニブルを読み続ける)
        if !self.write_mode || self.iwm_mode {
            // 読み取りモード（またはIWMモード）
            
            // 連続ラッチアクセス検出（タイミング観測＝コピープロテクト検出）
            self.observe_latch_read();
            
            // 連続読み取りカウント更新
            let current_track = self.drives[curr_drive].current_track();
            if current_track == self.last_track {
                self.consecutive_reads += 1;
            } else {
                self.consecutive_reads = 0;
                self.last_track = current_track;
            }
            
            // SafeFastモード: スピニングチェック省略 + unsafe
            // ラッチOFFの場合は常にAccurate
            let use_fast = self.is_safe_fast();
            
            if use_fast {
                // 怪しい挙動チェック（Fastモード中のみ）
                if self.is_safe_fast() {
                    self.detect_suspicious_behavior();
                }
                
                // トラックベース更新
                self.drives[curr_drive].update_track_base_if_needed();
                
                let byte_pos = self.drives[curr_drive].disk.byte_position;
                let nibbles = self.drives[curr_drive].disk.nibbles;
                let track_base = self.drives[curr_drive].disk.track_base;
                let offset = track_base + byte_pos;

                // unsafeで境界チェック省略
                self.latch = unsafe {
                    *self.drives[curr_drive].disk.data.get_unchecked(offset)
                };
                
                // 1バイトずつ進める（sync marker検出のため）
                let next_pos = byte_pos + 1;
                self.drives[curr_drive].disk.byte_position = if next_pos >= nibbles { 0 } else { next_pos };
                
                self.shift_reg = self.latch;
                self.last_read_latch_cycle = self.cumulative_cycles;
                self.update_sync_window_with_byte(self.latch);
                
                // Fastモードでも同期マーカー検出（セクタカウント用）
                self.check_sync_marker(curr_drive);
            } else {
                // Safe / Transitional accurate path:
                // Safe は従来どおり read ごとに 1 byte 進める。
                // Transitional は tick 側で準備された nibble があればそれを優先し、
                // 空振り時は safe path へフォールバックする。
                let spinning = self.drives[curr_drive].spinning;
                if spinning == 0 {
                    return;
                }

                let use_strict = matches!(self.sequencer_mode, DiskSequencerMode::Strict);
                let use_transitional = matches!(self.sequencer_mode, DiskSequencerMode::Transitional);

                if use_strict {
                    // Strict + WOZ ビットレベルシーケンサ:
                    // tick_disk_bit() でシフトレジスタが常に更新されている。
                    // CPU読み取りは現在の latch 値をそのまま返す（OpenEmulator と同じ）。
                    // latch の bit7 が 0 なら BPL ループで待機、1 なら有効バイト。
                    self.shift_reg = self.latch;
                    self.last_read_latch_cycle = self.cumulative_cycles;
                } else {
                    let transitional_latch = if use_transitional && self.nibble_byte_ready {
                        self.nibble_byte_ready = false;
                        Some(self.latch)
                    } else {
                        None
                    };

                    self.latch = transitional_latch.unwrap_or_else(|| self.read_current_track_byte(curr_drive));
                    self.advance_track_byte_position(curr_drive);

                    self.shift_reg = self.latch;
                    self.last_read_latch_cycle = self.cumulative_cycles;
                    self.update_sync_window_with_byte(self.latch);

                    if self.disk_debug_logging && use_transitional {
                        log::debug!(
                            "[DISK][transitional] read drive={} track={} cycle={} ready_hit={} latch={:02X} pos={} window={:04X}",
                            curr_drive + 1,
                            self.drives[curr_drive].current_track(),
                            self.cumulative_cycles,
                            transitional_latch.is_some(),
                            self.latch,
                            self.drives[curr_drive].disk.byte_position,
                            self.sync_bit_window,
                        );
                    }

                    // 同期マーカー検出ログ
                    self.check_sync_marker(curr_drive);
                }
            }
        } else {
            // 書き込みモード
            // SafeFast: 書き込みが発生したら即ラッチOFF（コピー保護の温床）
            self.latch_off_on_write();
            
            let write_protected = self.drives[curr_drive].disk.write_protected;
            if write_protected {
                return;
            }
            
            let spinning = self.drives[curr_drive].spinning;
            if spinning == 0 {
                return;
            }

            // Accurate timing では実際の媒体書き込みは tick_disk_bit() の
            // 8-bit 境界で反映する。ここではラッチ値の更新のみを行う。
            self.pending_write_latch = Some(self.latch);
            if self.is_safe_fast() {
                self.drives[curr_drive].update_track_base_if_needed();

                let byte_pos = self.drives[curr_drive].disk.byte_position;
                let nibbles = self.drives[curr_drive].disk.nibbles;
                let track_base = self.drives[curr_drive].disk.track_base;
                let offset = track_base + byte_pos;
                let latch = self.latch;

                if offset < self.drives[curr_drive].disk.data.len() {
                    self.drives[curr_drive].disk.data[offset] = latch;
                    self.drives[curr_drive].disk.track_image_dirty = true;
                    self.drives[curr_drive].disk.modified = true;
                }

                self.drives[curr_drive].write_light = SPINNING_CYCLES;
                self.drives[curr_drive].disk.byte_position = (byte_pos + 1) % nibbles;
            }
        }
    }

    /// 書き込みプロテクト状態をロード
    /// $C0xDを読んだ時、ラッチのbit7にwrite protect状態を反映
    /// 注意：既存のラッチデータは保持し、bit7のみを更新
    fn load_write_protect(&mut self) {
        let floppy = &self.drives[self.curr_drive].disk;
        // write protectの場合、ラッチを読むとbit7が1になる
        // ただし、これはシーケンサーの動作状態も含む
        // Q6H + Q7L (読み取りモード) でwrite protect sense
        if floppy.write_protected {
            self.latch |= 0x80;
        }
        // write protectでない場合、bit7はデータのまま（クリアしない）
        // 実際のDisk IIでは、write_protectedでない場合bit7は不定
    }

    /// DSKをNIBに変換
    fn dsk_to_nib(dsk_data: &[u8], sector_order: &[usize; 16]) -> Vec<u8> {
        let mut nib_data = vec![0u8; NIB_SIZE];
        let volume = 254u8;

        for track in 0..TRACKS {
            let mut nib_offset = track * NIB_TRACK_SIZE;

            // GAP1 - トラック先頭の同期バイト（48バイト）
            for _ in 0..48 {
                if nib_offset < (track + 1) * NIB_TRACK_SIZE {
                    nib_data[nib_offset] = 0xFF;
                    nib_offset += 1;
                }
            }

            for sector in 0..SECTORS_PER_TRACK {
                let phys_sector = sector_order[sector];
                let dsk_offset = track * BYTES_PER_TRACK + phys_sector * BYTES_PER_SECTOR;

                // アドレスフィールド
                nib_data[nib_offset] = 0xD5; nib_offset += 1;
                nib_data[nib_offset] = 0xAA; nib_offset += 1;
                nib_data[nib_offset] = 0x96; nib_offset += 1;

                // ボリューム（4-and-4エンコード）
                // byte1 = 上位ビット (D7,D5,D3,D1) + 0xAA
                // byte2 = 下位ビット (D6,D4,D2,D0) + 0xAA
                nib_data[nib_offset] = (volume >> 1) | 0xAA; nib_offset += 1;
                nib_data[nib_offset] = volume | 0xAA; nib_offset += 1;

                // トラック（4-and-4エンコード）
                let t = track as u8;
                nib_data[nib_offset] = (t >> 1) | 0xAA; nib_offset += 1;
                nib_data[nib_offset] = t | 0xAA; nib_offset += 1;

                // セクター（4-and-4エンコード）
                let s = sector as u8;
                nib_data[nib_offset] = (s >> 1) | 0xAA; nib_offset += 1;
                nib_data[nib_offset] = s | 0xAA; nib_offset += 1;

                // チェックサム（4-and-4エンコード）
                let checksum = volume ^ t ^ s;
                nib_data[nib_offset] = (checksum >> 1) | 0xAA; nib_offset += 1;
                nib_data[nib_offset] = checksum | 0xAA; nib_offset += 1;

                // エピローグ
                nib_data[nib_offset] = 0xDE; nib_offset += 1;
                nib_data[nib_offset] = 0xAA; nib_offset += 1;
                nib_data[nib_offset] = 0xEB; nib_offset += 1;

                // GAP2 - 6バイト
                for _ in 0..6 {
                    nib_data[nib_offset] = 0xFF;
                    nib_offset += 1;
                }

                // データフィールド
                nib_data[nib_offset] = 0xD5; nib_offset += 1;
                nib_data[nib_offset] = 0xAA; nib_offset += 1;
                nib_data[nib_offset] = 0xAD; nib_offset += 1;

                // 6-and-2エンコードされたデータ（343バイト）
                let sector_data = &dsk_data[dsk_offset..dsk_offset + BYTES_PER_SECTOR];
                let encoded = Self::encode_6and2(sector_data);
                for byte in &encoded {
                    nib_data[nib_offset] = *byte;
                    nib_offset += 1;
                }

                // エピローグ
                nib_data[nib_offset] = 0xDE; nib_offset += 1;
                nib_data[nib_offset] = 0xAA; nib_offset += 1;
                nib_data[nib_offset] = 0xEB; nib_offset += 1;

                // GAP3 - 27バイト
                for _ in 0..27 {
                    if nib_offset < (track + 1) * NIB_TRACK_SIZE {
                        nib_data[nib_offset] = 0xFF;
                        nib_offset += 1;
                    }
                }
            }
        }

        nib_data
    }

    /// 6-and-2エンコーディング
    fn encode_6and2(data: &[u8]) -> Vec<u8> {
        let mut aux = [0u8; 86];
        let mut nib = [0u8; 256];
        let mut result = Vec::with_capacity(343);

        // 補助バッファを構築（下位2ビットを収集）
        // P5 PROMのデコード: Y=0..255のメインデータと X=85..0の補助データを組み合わせ
        // つまり main[Y] と aux[85-Y] (Y<86の場合) を組み合わせる
        // したがって aux[85-i] に data[i] の下位ビットを格納
        // 
        // さらに、P5 PROMは LSR; ROL; LSR; ROL でデコードするので
        // 最初のLSRでaux.bit0が、次のLSRでaux.bit1(元)がキャリーに入る
        // ROLはキャリーをAのbit0に入れるので、
        // 結果のA.bit1 = 最初のキャリー (aux.bit0)
        // 結果のA.bit0 = 2回目のキャリー (aux.bit1)
        // つまり元データのD1,D0が入れ替わってAに入る
        // したがって aux には (D0 << 1) | D1 を格納
        for i in 0..86 {
            // data[i]の下位2ビットを aux[85-i] に格納
            let aux_idx = 85 - i;
            let a = ((data[i] & 0x01) << 1) | ((data[i] & 0x02) >> 1);  // (D0 << 1) | D1
            let b = if i + 86 < 256 {
                ((data[i + 86] & 0x01) << 3) | ((data[i + 86] & 0x02) << 1)
            } else {
                0
            };
            let c = if i + 172 < 256 {
                ((data[i + 172] & 0x01) << 5) | ((data[i + 172] & 0x02) << 3)
            } else {
                0
            };
            aux[aux_idx] = a | b | c;
        }

        // メインデータ（上位6ビット）
        for i in 0..256 {
            nib[i] = data[i] >> 2;
        }

        // XORチェックサム計算とエンコード
        let mut checksum = 0u8;

        // 補助バッファを逆順でエンコード
        for i in (0..86).rev() {
            let val = aux[i];
            result.push(WRITE_TABLE[(val ^ checksum) as usize & 0x3F]);
            checksum = val;
        }

        // メインデータをエンコード
        for i in 0..256 {
            let val = nib[i];
            result.push(WRITE_TABLE[(val ^ checksum) as usize & 0x3F]);
            checksum = val;
        }

        // 最終チェックサム
        result.push(WRITE_TABLE[checksum as usize & 0x3F]);

        result
    }

    /// フルスピード条件かチェック
    #[allow(dead_code)]
    pub fn is_condition_for_full_speed(&self) -> bool {
        self.enhance_disk && self.motor_on
    }

    /// ドライブの状態を取得
    #[allow(dead_code)]
    pub fn get_drive_status(&self, drive: usize) -> (bool, bool, bool) {
        let d = &self.drives[drive];
        (d.disk.disk_loaded, self.motor_on && self.curr_drive == drive, d.write_light > 0)
    }

    /// 現在のトラックを取得
    #[allow(dead_code)]
    pub fn get_current_track(&self) -> usize {
        self.drives[self.curr_drive].current_track()
    }

    /// 現在のドライブを取得
    #[allow(dead_code)]
    pub fn get_current_drive(&self) -> usize {
        self.curr_drive
    }
    
    fn export_disk_with_order(&self, drive: usize, sector_order: &[usize; 16]) -> Result<Vec<u8>, &'static str> {
        if drive > 1 {
            return Err("Invalid drive number");
        }

        let disk = &self.drives[drive].disk;
        if !disk.disk_loaded {
            return Err("No disk loaded");
        }

        let mut dsk_data = vec![0u8; DSK_SIZE];

        for track in 0..TRACKS {
            let track_offset = track * NIB_TRACK_SIZE;
            let nib_track = &disk.data[track_offset..track_offset + NIB_TRACK_SIZE];

            for logical_sector in 0..SECTORS_PER_TRACK {
                let physical_sector = sector_order[logical_sector];
                if let Some(sector_data) = self.decode_sector(nib_track, physical_sector) {
                    let dsk_offset = (track * SECTORS_PER_TRACK + logical_sector) * BYTES_PER_SECTOR;
                    dsk_data[dsk_offset..dsk_offset + BYTES_PER_SECTOR]
                        .copy_from_slice(&sector_data);
                }
            }
        }

        Ok(dsk_data)
    }

    /// 挿入時の形式を尊重してディスクイメージを書き戻し用にエクスポート
    pub fn export_disk_in_original_format(&self, drive: usize) -> Result<Vec<u8>, &'static str> {
        if drive > 1 {
            return Err("Invalid drive number");
        }

        let disk = &self.drives[drive].disk;
        if !disk.disk_loaded {
            return Err("No disk loaded");
        }

        match disk.format.unwrap_or(DiskFormat::Dsk) {
            DiskFormat::Dsk => self.export_disk_with_order(drive, &DOS_SECTOR_ORDER),
            DiskFormat::Po => self.export_disk_with_order(drive, &PRODOS_SECTOR_ORDER),
            DiskFormat::Nib => Ok(disk.data.clone()),
            DiskFormat::Woz => Err("WOZ format is read-only"),
        }
    }

    /// 書き込み保護状態を設定
    pub fn set_write_protected(&mut self, drive: usize, enabled: bool) -> Result<(), &'static str> {
        if drive > 1 {
            return Err("Invalid drive number");
        }
        self.drives[drive].disk.write_protected = enabled;
        Ok(())
    }

    /// 書き込み保護状態を取得
    pub fn is_write_protected(&self, drive: usize) -> Result<bool, &'static str> {
        if drive > 1 {
            return Err("Invalid drive number");
        }
        Ok(self.drives[drive].disk.write_protected)
    }

    /// 変更済みディスクを元のファイル名へ書き戻す
    pub fn flush_disk_to_filename(&mut self, drive: usize) -> Result<Option<String>, String> {
        use std::fs;

        if drive > 1 {
            return Err("Invalid drive number".to_string());
        }

        let (loaded, modified, write_protected, filename) = {
            let disk = &self.drives[drive].disk;
            (disk.disk_loaded, disk.modified, disk.write_protected, disk.filename.clone())
        };

        if !loaded || !modified {
            return Ok(None);
        }
        if write_protected {
            return Err(format!("Drive {} disk is write-protected", drive + 1));
        }
        let filename = filename.ok_or_else(|| format!("Drive {} has no backing filename", drive + 1))?;
        let bytes = self.export_disk_in_original_format(drive).map_err(|e| e.to_string())?;
        fs::write(&filename, &bytes).map_err(|e| format!("Failed to save {}: {}", filename, e))?;
        self.drives[drive].disk.modified = false;
        if let Some(ref mut dsk) = self.drives[drive].disk.dsk_data {
            match self.drives[drive].disk.format.unwrap_or(DiskFormat::Dsk) {
                DiskFormat::Dsk | DiskFormat::Po => *dsk = bytes.clone(),
                DiskFormat::Nib | DiskFormat::Woz => {}
            }
        }
        Ok(Some(filename))
    }

    /// ディスクをイジェクトし、必要なら書き戻しを行う
    pub fn eject_disk_with_flush(&mut self, drive: usize) -> Result<Option<String>, String> {
        let flushed = self.flush_disk_to_filename(drive)?;
        self.eject_disk(drive);
        Ok(flushed)
    }

    /// ディスクイメージをDSK形式でエクスポート
    #[allow(dead_code)]
    pub fn export_disk(&self, drive: usize) -> Result<Vec<u8>, &'static str> {
        self.export_disk_with_order(drive, &DOS_SECTOR_ORDER)
    }
    
    /// NIBトラックからセクターデータをデコード
    #[allow(dead_code)]
    fn decode_sector(&self, nib_track: &[u8], target_sector: usize) -> Option<[u8; 256]> {
        // 6-and-2デコードテーブルを構築
        let mut decode_table = [0u8; 256];
        for (i, &code) in WRITE_TABLE.iter().enumerate() {
            decode_table[code as usize] = i as u8;
        }
        
        // セクターマーカーを探す
        let mut pos = 0;
        while pos < nib_track.len() - 20 {
            // アドレスフィールドマーカー (D5 AA 96)
            if nib_track[pos] == 0xD5 && 
               pos + 1 < nib_track.len() && nib_track[pos + 1] == 0xAA &&
               pos + 2 < nib_track.len() && nib_track[pos + 2] == 0x96 {
                
                // セクター番号をデコード（4-and-4エンコード）
                if pos + 7 < nib_track.len() {
                    let sector_odd = nib_track[pos + 5];
                    let sector_even = nib_track[pos + 6];
                    let sector = ((sector_odd & 0x55) << 1) | (sector_even & 0x55);
                    
                    if sector as usize == target_sector {
                        // データフィールドマーカー (D5 AA AD) を探す
                        let mut data_pos = pos + 10;
                        while data_pos < nib_track.len() - 350 {
                            if nib_track[data_pos] == 0xD5 &&
                               nib_track[data_pos + 1] == 0xAA &&
                               nib_track[data_pos + 2] == 0xAD {
                                // データをデコード
                                return self.decode_6and2(&nib_track[data_pos + 3..], &decode_table);
                            }
                            data_pos += 1;
                        }
                    }
                }
            }
            pos += 1;
        }
        None
    }
    
    /// 6-and-2エンコードされたデータをデコード
    #[allow(dead_code)]
    fn decode_6and2(&self, encoded: &[u8], decode_table: &[u8; 256]) -> Option<[u8; 256]> {
        if encoded.len() < 343 {
            return None;
        }
        
        let mut aux = [0u8; 86];
        let mut data = [0u8; 256];
        
        // 補助バイト（86バイト）をデコード
        let mut prev = 0u8;
        for i in 0..86 {
            let code = encoded[i];
            if code < 0x96 {
                return None;
            }
            let val = decode_table[code as usize];
            aux[i] = val ^ prev;
            prev = aux[i];
        }
        
        // メインデータ（256バイト）をデコード
        for i in 0..256 {
            let code = encoded[86 + i];
            if code < 0x96 {
                return None;
            }
            let val = decode_table[code as usize];
            data[i] = val ^ prev;
            prev = data[i];
        }
        
        // 補助ビットを結合して完全な8ビットデータを復元
        for i in 0..256 {
            let aux_idx = i % 86;
            let bit_pos = i / 86;
            let aux_bits = (aux[aux_idx] >> (bit_pos * 2)) & 0x03;
            data[i] = (data[i] << 2) | aux_bits;
        }
        
        Some(data)
    }
}

// 後方互換性のための型エイリアス
#[allow(dead_code)]
pub type DiskDrive = FloppyDrive;
#[allow(dead_code)]
pub type DiskController = Disk2InterfaceCard;
