//! Apple II エミュレータ
//! 
//! CPU、メモリ、ビデオ、ディスクを統合

use crate::cpu::{Cpu, CpuType, MemoryBus};
use crate::memory::{AppleModel, Memory};
use crate::video::{Video, VideoFetchPhase, VideoPosition, VideoScanner};
use crate::disk::{Disk2InterfaceCard, DiskFormat, DiskSequencerMode, DiskControllerState, DSK_SIZE, NIB_SIZE};
use crate::savestate::{SaveState, CpuState, MemoryState, DiskState, DiskDriveState, VideoState};
use crate::system::{CYCLES_PER_FRAME as SYSTEM_CYCLES_PER_FRAME, CYCLES_PER_SCANLINE as SYSTEM_CYCLES_PER_SCANLINE};

/// Apple IIエミュレータのメイン構造体
pub struct Apple2 {
    /// 6502/65C02 CPU
    pub cpu: Cpu,
    /// メモリシステム
    pub memory: Memory,
    /// ビデオサブシステム
    pub video: Video,
    /// tick駆動のビデオスキャナ
    pub video_scanner: VideoScanner,
    /// Disk IIインターフェースカード
    pub disk: Disk2InterfaceCard,
    /// 累積サイクル数
    pub total_cycles: u64,
    /// フレームカウンター
    pub frame_count: u64,
    /// エミュレーション実行中フラグ
    pub running: bool,
    /// スピーカークリックのサイクルリスト
    pub speaker_clicks: Vec<u64>,
    /// 仮想ブートROM（VBR）モードが有効か
    /// Disk II Boot ROMがロードされていない場合にtrueになる
    pub vbr_mode: bool,
    /// VBRブートが実行済みか
    vbr_boot_done: bool,
    /// Monitor ROM スタブモード（本物のROMがない場合に使用）
    pub monitor_stub_mode: bool,
    /// カーソル位置（CH: 水平, CV: 垂直）
    cursor_h: u8,
    cursor_v: u8,
    /// 起動ブースト: PCヒストリ（直近のPC値）
    pc_history: [u16; 256],
    /// 起動ブースト: PCヒストリ書き込み位置
    pc_history_idx: u8,
    /// 起動ブースト: 安定ループ検出フラグ
    pub stable_loop_detected: bool,
    /// 起動ブースト: 最後のループ検出サイクル
    last_loop_check_cycle: u64,
    /// 起動ブースト: ログ出力フラグ
    pub boost_log: bool,
    /// 起動ブースト: 前回のPC-ZONE (0=zero, 1=user, 2=disk_rom, 3=main_rom)
    last_pc_zone: u8,
    /// 起動ブースト: User RAMに入ったことがあるか
    user_ram_entered: bool,
    /// 起動ブースト: Disk ROMを離れたサイクル
    disk_rom_left_cycle: u64,
    /// 現在実行中の命令の残りCPUサイクル数（Phase 3 system tick 用）
    pending_cpu_cycles: u32,
    /// 直近の浮遊バス値（Phase 4 の scanline / cycle ベース実装）
    floating_bus_value: u8,
    /// Phase 4: 直近の floating bus が参照したアドレス
    floating_bus_address: u16,
}

/// メモリバスの実装（Disk II I/Oを含む）
impl MemoryBus for Apple2 {
    fn read(&mut self, address: u16) -> u8 {
        match address {
            // スピーカー ($C030-$C03F)
            // Apple II では write-only soft switch の読み出しはしばしば floating bus を返す。
            0xC030..=0xC03F => {
                self.speaker_clicks.push(self.total_cycles);
                let _ = self.memory.read(address);
                self.floating_bus()
            }
            // パドル/ジョイスティック ($C064-$C06F, $C070-$C07F)
            0xC064..=0xC06F => {
                // パドル読み取り時にサイクル情報を渡す
                self.memory.paddle_read_cycle = self.total_cycles;
                self.memory.read(address)
            }
            // $C070-$C07F は多くが trigger/status で、一部は floating bus を返す側に近い。
            0xC070..=0xC07D => {
                self.memory.paddle_read_cycle = self.total_cycles;
                let _ = self.memory.read(address);
                self.floating_bus()
            }
            0xC07E..=0xC07F => {
                self.memory.paddle_read_cycle = self.total_cycles;
                self.memory.read(address)
            }
            // ビデオ soft switch ($C050-$C05F) は read でも side effect が発生し、
            // バス値は現在の video fetch byte を返す。
            0xC050..=0xC05F => {
                let _ = self.memory.read(address);
                self.floating_bus()
            }
            // Disk II ブートROM (スロット6: $C600-$C6FF)
            0xC600..=0xC6FF => {
                // VBRモード: ROMがロードされていない場合
                if !self.disk.is_rom_loaded() {
                    // VBR: 直接ブート処理を実行
                    if !self.vbr_boot_done && address == 0xC600 {
                        self.vbr_mode = true;
                        // VBRブートを即座に実行
                        if self.vbr_boot() {
                            // ブート成功: PCが$0801に設定されているので
                            // RTSのように、$0801の最初の命令を返す
                            return self.memory.read(0x0801);
                        }
                    }
                    // VBR: BRK (0x00)を返す
                    0x00
                } else {
                    // $C600-$C6FF → disk.boot_rom[0x00-0xFF]
                    self.disk.read_rom((address - 0xC600) as u8)
                }
            }
            // Disk II I/O (スロット6: $C0E0-$C0EF)
            0xC0E0..=0xC0EF => {
                // サイクル数を更新してからI/Oを実行
                self.disk.cumulative_cycles = self.total_cycles;
                self.disk.io_read((address & 0x0F) as u8)
            }
            // Monitor ROMサブルーチンは実際のROMを使用（スタブなし）
            // 他のアドレスはメモリシステムに委譲
            _ => self.memory.read(address),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            // スピーカー ($C030)
            0xC030..=0xC03F => {
                self.speaker_clicks.push(self.total_cycles);
                self.memory.write(address, value);
            }
            // Disk II I/O (スロット6: $C0E0-$C0EF)
            0xC0E0..=0xC0EF => {
                self.disk.cumulative_cycles = self.total_cycles;
                self.disk.io_write((address & 0x0F) as u8, value);
            }
            // 他のアドレスはメモリシステムに委譲
            _ => self.memory.write(address, value),
        }
    }
}

impl Apple2 {
    /// 新しいエミュレータインスタンスを作成
    pub fn new(model: AppleModel) -> Self {
        // Apple IIe Enhanced は 65C02、それ以外は 6502
        let cpu_type = match model {
            AppleModel::AppleIIeEnhanced => CpuType::Cpu65C02,
            _ => CpuType::Cpu6502,
        };

        let disk = Disk2InterfaceCard::new();

        Apple2 {
            cpu: Cpu::new(cpu_type),
            memory: Memory::new(model),
            video: Video::new(),
            video_scanner: VideoScanner::new(),
            disk,
            total_cycles: 0,
            frame_count: 0,
            running: true,
            speaker_clicks: Vec::with_capacity(4096),
            vbr_mode: false,
            vbr_boot_done: false,
            monitor_stub_mode: false,
            cursor_h: 0,
            cursor_v: 0,
            pc_history: [0; 256],
            pc_history_idx: 0,
            stable_loop_detected: false,
            last_loop_check_cycle: 0,
            boost_log: false,
            last_pc_zone: 0,
            user_ram_entered: false,
            disk_rom_left_cycle: 0,
            pending_cpu_cycles: 0,
            floating_bus_value: 0,
            floating_bus_address: 0,
        }
    }

    /// ROMサイズからモデルを自動検出
    pub fn detect_model_from_rom(rom_data: &[u8]) -> AppleModel {
        // 32KB ROMの場合、Apple IIe を判別
        if rom_data.len() == 32768 {
            return AppleModel::AppleIIe;
        }
        
        match rom_data.len() {
            20480 => AppleModel::AppleIIPlus,  // 20KB = Apple II Plus
            12288 => AppleModel::AppleIIPlus,   // 12KB = Apple II/II+ (Autostart ROM only)
            16384 => AppleModel::AppleIIe,      // 16KB = Generic full ROM
            _ => AppleModel::AppleIIPlus,       // デフォルト
        }
    }

    /// ROMをロード
    pub fn load_rom(&mut self, rom_data: &[u8]) {
        self.memory.load_rom(rom_data);
        
        // ROMファイルからDisk II Boot ROMを抽出してDiskControllerに設定
        // 20KB ROM (Apple II Plus) または 32KB ROM (Apple IIe) の場合
        // オフセット $0600-$06FF にDisk II P5 Boot ROMがある
        if rom_data.len() >= 0x0700 {
            let mut boot_rom = [0u8; 256];
            for i in 0..256 {
                boot_rom[i] = rom_data[0x0600 + i];
            }
            // 有効なDisk II ROMかチェック（先頭が$A2 $20で始まる）
            if boot_rom[0] == 0xA2 && boot_rom[1] == 0x20 {
                self.disk.boot_rom = boot_rom;
                log::info!("Loaded Disk II Boot ROM from ROM file");
            }
        }
        
        // Apple IIe 32KB ROMの場合、文字ROMを抽出
        if rom_data.len() == 32768 {
            self.video.load_char_rom_from_iie_rom(rom_data);
        }
    }

    /// 外部Disk II Boot ROMをロード
    pub fn load_disk_rom(&mut self, rom_data: &[u8]) -> Result<(), &'static str> {
        if rom_data.len() != 256 {
            return Err("Disk II ROM must be 256 bytes");
        }
        // 有効なDisk II ROMかチェック（先頭が$A2 $20で始まる）
        if rom_data[0] != 0xA2 || rom_data[1] != 0x20 {
            return Err("Invalid Disk II ROM (should start with A2 20)");
        }
        self.disk.boot_rom.copy_from_slice(rom_data);
        // メモリの$C600-$C6FFにもコピー（CPUから直接読めるようにする）
        self.memory.copy_disk_boot_rom(rom_data);
        log::info!("Loaded external Disk II Boot ROM");
        Ok(())
    }

    /// ディスク高速化を設定
    pub fn set_fast_disk(&mut self, fast: bool) {
        self.disk.enhance_disk = fast;
    }

    /// FastDisk が実効的に有効か
    pub fn is_fast_disk_enabled(&self) -> bool {
        self.disk.is_fastdisk_effective()
    }

    pub fn configure_experimental_disk(
        &mut self,
        sequencer_mode: DiskSequencerMode,
        weak_bits_enabled: bool,
        write_splice_enabled: bool,
        disk_debug_logging: bool,
    ) {
        self.disk.set_experimental_options(
            sequencer_mode,
            weak_bits_enabled,
            write_splice_enabled,
            disk_debug_logging,
        );
    }

    /// ディスクイメージをロード
    pub fn load_disk(&mut self, drive: usize, data: &[u8]) -> Result<(), &'static str> {
        if drive > 1 {
            return Err("Invalid drive number");
        }
        
        // ファイルサイズでフォーマットを判定
        let format = match data.len() {
            DSK_SIZE => DiskFormat::Dsk,  // 143360 bytes
            NIB_SIZE => DiskFormat::Nib,  // 232960 bytes
            _ => return Err("Unknown disk format"),
        };
        
        self.disk.insert_disk(drive, data, format)
    }

    /// エミュレータをリセット
    pub fn reset(&mut self) {
        // ソフトスイッチをリセット（テキストモードで起動）
        self.memory.switches = crate::memory::SoftSwitches::default();
        
        // テキストRAMを$A0（スペース）で初期化（実機のPower-on状態を模倣）
        for addr in 0x0400..=0x07FF {
            self.memory.main_ram[addr] = 0xA0;
        }
        
        // ディスクコントローラーをリセット
        self.disk.reset();
        
        // ディスクブート用のゼロページ初期化
        // P5 PROMはこれらの値を使用してブートセクタを読み込む
        // 標準的なMonitor ROMはこれらを初期化するが、直接ブートする場合は手動で設定
        if self.disk.drives[0].disk.disk_loaded {
            // $26/$27: データポインタ ($0800)
            self.memory.main_ram[0x26] = 0x00;
            self.memory.main_ram[0x27] = 0x08;
            // $3D: 目標トラック (0)
            self.memory.main_ram[0x3D] = 0x00;
            // $41: 目標セクター (0)
            self.memory.main_ram[0x41] = 0x00;
            
            // P5 PROMの$C652-$C657のSTA命令をNOPに置き換え
            // これらの命令は$56を格納するが、既に正しい値を設定済み
            self.disk.boot_rom[0x52] = 0xEA; // NOP
            self.disk.boot_rom[0x53] = 0xEA; // NOP
            self.disk.boot_rom[0x54] = 0xEA; // NOP
            self.disk.boot_rom[0x55] = 0xEA; // NOP
            self.disk.boot_rom[0x56] = 0xEA; // NOP
            self.disk.boot_rom[0x57] = 0xEA; // NOP
            
            // $C64C: JSR $FCA8 (WAIT) → NOP x 3 (待機は不要)
            self.disk.boot_rom[0x4C] = 0xEA;
            self.disk.boot_rom[0x4D] = 0xEA;
            self.disk.boot_rom[0x4E] = 0xEA;
            // 注意: HOME ($C621) はパッチしない - 画面初期化に必要
        }
        
        // CPUを一時的に取り出してリセット
        let mut cpu = std::mem::take(&mut self.cpu);
        cpu.reset(self);
        self.cpu = cpu;
        self.total_cycles = 0;
        self.pending_cpu_cycles = 0;
        self.video_scanner = VideoScanner::new();
        self.memory.scanline = 0;
        self.memory.scanline_cycle = 0;
        self.memory.frame_counter = 0;
        self.memory.hblank = false;
        self.memory.vblank = false;
        self.floating_bus_value = 0;
        self.floating_bus_address = 0;
        
        // ディスクがロードされている場合、直接スロット6ブートを開始
        // (Monitor ROMのAutostart機能を使わず、直接$C600にジャンプ)
        if self.disk.drives[0].disk.disk_loaded {
            // P5 PROMは $C625 で LDA $0100,X (X=SP) を実行してスロット番号を取得
            // SP=$FC の時、TSXでX=$FCになり、$01FCを読む
            // $01FC にスロット番号($06)を書き込む
            self.memory.main_ram[0x01FC] = 0x06;  // スロット番号
            self.cpu.regs.sp = 0xFC;
            
            self.cpu.regs.pc = 0xC600;
            // Boot status is logged by main.rs
        }
        // No message when no disk - normal operation
    }

    /// 1命令を実行
    pub fn step(&mut self) -> u32 {
        let pc = self.cpu.regs.pc;
        
        // Monitor ROMスタブモード: PCがMonitor ROM領域に入ったらスタブを実行
        if self.monitor_stub_mode {
            // $E000 - Applesoft BASIC cold start
            if pc == 0xE000 {
                // BASICがないので、DOSプロンプトを表示してキー入力待ち
                self.stub_basic_prompt();
                // 無限ループに入る（テストROMの$F054: JMP $F037に相当）
                // ただし行0のアニメーションは避けるため$F04A（ウェイト）にジャンプ
                self.cpu.regs.pc = 0xF04A;
                return 6;
            }
            
            // $F800-$FFFF の全Monitor ROM領域をカバー
            if pc >= 0xF800 {
                // Monitor ROMスタブをチェック
                if let Some(cycles) = self.execute_monitor_stub(pc) {
                    self.total_cycles += cycles as u64;
                    return cycles;
                }
            }
        }
        
        // 起動ブースト: PC記録
        self.record_pc(self.cpu.regs.pc);
        
        // SafeFast: CPUのPCとメモリを観測（IOB検証付き）
        self.disk.observe_pc_with_memory(self.cpu.regs.pc, &self.memory.main_ram[..]);
        
        // CPUを一時的に取り出して実行
        let mut cpu = std::mem::take(&mut self.cpu);
        let cycles = cpu.step(self);
        self.cpu = cpu;
        self.total_cycles += cycles as u64;
        
        cycles
    }
    
    /// PC値を記録（起動ブースト用）
    #[inline]
    fn record_pc(&mut self, pc: u16) {
        self.pc_history[self.pc_history_idx as usize] = pc;
        self.pc_history_idx = self.pc_history_idx.wrapping_add(1);
        
        // PC-ZONE遷移を検出
        let zone = match pc {
            0x0000..=0x07FF => 0,          // Zero/Stack page
            0x0800..=0xBFFF => 1,          // User RAM
            0xC600..=0xC6FF => 2,          // Disk II Boot ROM
            0xC000..=0xFFFF => 3,          // Main ROM (Monitor/Applesoft)
        };
        
        if zone != self.last_pc_zone {
            let zone_name = |z: u8| match z {
                0 => "zero",
                1 => "user",
                2 => "disk_rom",
                3 => "main_rom",
                _ => "unknown",
            };
            
            if self.boost_log {
                println!("[PC-ZONE] {} -> {} at ${:04X} t={:.2}M", 
                    zone_name(self.last_pc_zone), zone_name(zone), pc,
                    self.total_cycles as f64 / 1_000_000.0);
            }
            
            // User RAMに入ったことを記録
            if zone == 1 {
                self.user_ram_entered = true;
            }
            
            // Disk ROMを離れたサイクルを記録
            if self.last_pc_zone == 2 && zone != 2 {
                self.disk_rom_left_cycle = self.total_cycles;
            }
            
            self.last_pc_zone = zone;
        }
    }
    
    /// 安定ループを検出（起動ブースト終了判定）
    /// 条件：
    /// 1. User RAMに入ったことがある
    /// 2. Disk ROM領域（$C600-$C6FF）を離れてから一定時間経過
    /// 3. 直近のPC値がユーザー空間に収束
    pub fn check_stable_loop(&mut self) -> bool {
        if self.stable_loop_detected {
            return true;
        }
        
        // 最低500万サイクル経過するまでチェックしない
        if self.total_cycles < 5_000_000 {
            return false;
        }
        
        // 100,000サイクルごとにチェック
        if self.total_cycles.saturating_sub(self.last_loop_check_cycle) < 100_000 {
            return false;
        }
        self.last_loop_check_cycle = self.total_cycles;
        
        // PC値の分析
        let mut pc_counts: std::collections::HashMap<u16, u32> = std::collections::HashMap::new();
        let mut user_space_count = 0;  // $0800-$BFFF
        let mut disk_rom_count = 0;     // $C600-$C6FF (Disk II Boot ROM)
        let mut main_rom_count = 0;     // $C000-$FFFF (excluding Disk ROM)
        let mut min_pc: u16 = 0xFFFF;
        let mut max_pc: u16 = 0x0000;
        let mut valid_count = 0;
        
        for &pc in &self.pc_history {
            if pc == 0 {
                continue;
            }
            valid_count += 1;
            
            *pc_counts.entry(pc).or_insert(0) += 1;
            
            if pc < min_pc { min_pc = pc; }
            if pc > max_pc { max_pc = pc; }
            
            match pc {
                0x0800..=0xBFFF => user_space_count += 1,
                0xC600..=0xC6FF => disk_rom_count += 1,
                0xC000..=0xFFFF => main_rom_count += 1,
                _ => {}
            }
        }
        
        let unique_count = pc_counts.len();
        let user_ratio = if valid_count > 0 { user_space_count as f32 / valid_count as f32 } else { 0.0 };
        let disk_rom_ratio = if valid_count > 0 { disk_rom_count as f32 / valid_count as f32 } else { 0.0 };
        let pc_range = if max_pc >= min_pc { max_pc - min_pc } else { 0 };
        
        // ログ出力
        let cycles_m = self.total_cycles as f64 / 1_000_000.0;
        
        // 新しい解除条件:
        // 1. User RAMに入ったことがある (user_ram_entered)
        // 2. 現在のゾーンがuser (last_pc_zone == 1)
        // 3. 直近256命令の80%以上がユーザー空間 (user_ratio >= 0.8)
        // 4. 直近256命令でDisk ROMにいない (disk_rom_count == 0)
        // 5. Disk ROMを離れてから100万サイクル以上経過
        // 6. 最後のディスクI/Oから50万サイクル以上経過
        let disk_rom_left_time = if self.disk_rom_left_cycle > 0 {
            self.total_cycles.saturating_sub(self.disk_rom_left_cycle)
        } else {
            0
        };
        
        let disk_io_silence = self.total_cycles.saturating_sub(self.disk.last_disk_io_cycle);
        
        let current_zone_is_user = self.last_pc_zone == 1;
        let is_candidate = self.user_ram_entered 
            && current_zone_is_user
            && user_ratio >= 0.8
            && disk_rom_count == 0
            && disk_rom_left_time > 1_000_000
            && disk_io_silence > 500_000;
        
        // boost_logが有効な場合、100万サイクルごとにPC安定性ログを出力
        if self.boost_log && (self.total_cycles % 1_000_000 < 100_000 || is_candidate) {
            // 上位5つのPCを取得
            let mut top_pcs: Vec<_> = pc_counts.iter().collect();
            top_pcs.sort_by(|a, b| b.1.cmp(a.1));
            let top5: Vec<_> = top_pcs.iter().take(5).collect();
            
            println!("[PC-STABLE] t={:.1}M unique={} range=${:04X}-${:04X} (${:X}) user={:.0}% disk_rom={:.0}% main_rom={}",
                cycles_m, unique_count, min_pc, max_pc, pc_range, user_ratio * 100.0, disk_rom_ratio * 100.0, main_rom_count);
            println!("[BOOST-STATE] user_entered={} zone={} disk_left={:.2}M io_silence={:.2}M motor={}",
                self.user_ram_entered, 
                match self.last_pc_zone { 0 => "zero", 1 => "user", 2 => "disk_rom", 3 => "main_rom", _ => "?" },
                disk_rom_left_time as f64 / 1_000_000.0,
                disk_io_silence as f64 / 1_000_000.0,
                self.disk.motor_on);
            
            if !top5.is_empty() {
                print!("[PC-HIST] ");
                for (pc, count) in &top5 {
                    print!("${:04X}:{} ", pc, count);
                }
                println!();
            }
            
            if is_candidate {
                println!("[BOOST-CANDIDATE] all conditions met");
            }
        }
        
        // 安定ループ判定
        if is_candidate {
            self.stable_loop_detected = true;
            if self.boost_log {
                println!("[BOOST-OFF] reason=stable_user_loop user={:.0}% disk_left={:.2}M io_silence={:.2}M cycles={:.1}M",
                    user_ratio * 100.0, disk_rom_left_time as f64 / 1_000_000.0, 
                    disk_io_silence as f64 / 1_000_000.0, cycles_m);
            }
            return true;
        }
        
        false
    }
    
    /// Monitor ROMスタブを実行
    /// 戻り値: Some(cycles) = スタブを実行した、None = 通常実行
    fn execute_monitor_stub(&mut self, pc: u16) -> Option<u32> {
        match pc {
            // $FC58 - HOME: 画面クリア
            0xFC58 => {
                #[cfg(debug_assertions)]
                eprintln!("STUB: HOME called");
                self.stub_home();
                self.do_rts();
                Some(6)
            }
            // $FCA8 - WAIT: 時間待ち（即リターン）
            0xFCA8 => {
                self.do_rts();
                Some(6)
            }
            // $FDED - COUT: 文字出力
            0xFDED => {
                #[cfg(debug_assertions)]
                {
                    let ch = self.cpu.regs.a;
                    if ch >= 0xA0 {
                        eprint!("{}", (ch & 0x7F) as char);
                    } else if ch == 0x8D {
                        eprintln!(); // CR
                    }
                }
                self.stub_cout();
                self.do_rts();
                Some(6)
            }
            // $FD8E - CROUT: 改行
            0xFD8E => {
                #[cfg(debug_assertions)]
                eprintln!(); // CR
                self.stub_crout();
                self.do_rts();
                Some(6)
            }
            // $FDDA - PRBYTE: 16進数出力
            0xFDDA => {
                #[cfg(debug_assertions)]
                eprint!("{:02X}", self.cpu.regs.a);
                self.stub_prbyte();
                self.do_rts();
                Some(6)
            }
            // $FF58 - スロット番号トリック（RTSのみ）
            0xFF58 => {
                self.do_rts();
                Some(6)
            }
            // $FB2F - TEXT2COPY/INIT
            0xFB2F => {
                #[cfg(debug_assertions)]
                eprintln!("STUB: TEXT2COPY/INIT called");
                self.do_rts();
                Some(6)
            }
            // $FE89 - SETKBD
            0xFE89 => {
                self.do_rts();
                Some(6)
            }
            // $FE93 - SETVID
            0xFE93 => {
                self.do_rts();
                Some(6)
            }
            // $FB39 - SETTXT - テキストモード設定
            0xFB39 => {
                self.memory.switches.text_mode = true;
                self.memory.switches.hires = false;
                self.do_rts();
                Some(6)
            }
            // $FB40 - SETGR - グラフィックモード設定
            0xFB40 => {
                self.memory.switches.text_mode = false;
                self.memory.switches.hires = false;
                self.do_rts();
                Some(6)
            }
            // $FBE4 - BELL2 - ベル音
            0xFBE4 => {
                // ベル音（今は無視）
                self.do_rts();
                Some(6)
            }
            // $FC70 - SETNORM - 通常テキストモード
            0xFC70 => {
                self.do_rts();
                Some(6)
            }
            // $FE84 - SETNORM (alternate)
            0xFE84 => {
                self.do_rts();
                Some(6)
            }
            // $FCFC-$FD18 - 内部ルーチン（PRINTやVTAB等）
            0xFC22 => {
                // VTAB - 垂直タブ（Aで行指定）
                self.cursor_v = self.cpu.regs.a.min(23);
                self.do_rts();
                Some(6)
            }
            _ => None, // 通常のROM実行
        }
    }
    
    /// RTS相当の処理
    fn do_rts(&mut self) {
        // スタックからリターンアドレスをポップ
        let sp = self.cpu.regs.sp;
        let lo = self.memory.main_ram[0x0100 + sp.wrapping_add(1) as usize];
        let hi = self.memory.main_ram[0x0100 + sp.wrapping_add(2) as usize];
        self.cpu.regs.sp = sp.wrapping_add(2);
        let ret_addr = ((hi as u16) << 8) | (lo as u16);
        self.cpu.regs.pc = ret_addr.wrapping_add(1);
    }
    
    /// Applesoft BASIC プロンプト表示（スタブ）
    fn stub_basic_prompt(&mut self) {
        // 画面をクリア
        self.stub_home();
        
        // "DOS 3.3 LOADED" のメッセージを表示
        let msg = b"DOS 3.3 LOADED";
        for (i, &ch) in msg.iter().enumerate() {
            let addr = self.get_text_address(i as u8, 0);
            self.memory.main_ram[addr as usize] = ch | 0x80; // high bit set
        }
        
        // "]" プロンプトを次の行に
        let addr = self.get_text_address(0, 2);
        self.memory.main_ram[addr as usize] = 0xDD; // ']' with high bit
        
        // カーソル位置を更新
        self.cursor_h = 1;
        self.cursor_v = 2;
    }
    
    /// HOME - 画面クリア
    fn stub_home(&mut self) {
        // テキストページ1 ($0400-$07FF) を$A0（スペース）でクリア
        for addr in 0x0400..=0x07FF {
            self.memory.main_ram[addr] = 0xA0;
        }
        // カーソルを左上に
        self.cursor_h = 0;
        self.cursor_v = 0;
        // Zero page の BARONE ($24) と CV ($25) を更新
        self.memory.main_ram[0x24] = 0;  // CH (horizontal)
        self.memory.main_ram[0x25] = 0;  // CV (vertical)
    }
    
    /// COUT - 文字出力
    fn stub_cout(&mut self) {
        let ch = self.cpu.regs.a;
        
        // 制御文字の処理
        match ch {
            0x8D => {
                // CR - 改行
                self.stub_crout();
                return;
            }
            0xA0..=0xDF | 0x80..=0x9F => {
                // 通常文字（Apple II形式: high bit set）
            }
            _ => {
                // その他の制御文字は無視
                return;
            }
        }
        
        // 文字をテキストRAMに書き込み
        let addr = self.get_text_address(self.cursor_h, self.cursor_v);
        self.memory.main_ram[addr as usize] = ch;
        
        // カーソル進める
        self.cursor_h += 1;
        if self.cursor_h >= 40 {
            self.stub_crout();
        }
        
        // Zero page更新
        self.memory.main_ram[0x24] = self.cursor_h;
    }
    
    /// CROUT - 改行
    fn stub_crout(&mut self) {
        self.cursor_h = 0;
        self.cursor_v += 1;
        if self.cursor_v >= 24 {
            // スクロール
            self.scroll_text();
            self.cursor_v = 23;
        }
        // Zero page更新
        self.memory.main_ram[0x24] = self.cursor_h;
        self.memory.main_ram[0x25] = self.cursor_v;
    }
    
    /// PRBYTE - Aレジスタを16進数2桁で出力
    fn stub_prbyte(&mut self) {
        let val = self.cpu.regs.a;
        let hi = (val >> 4) & 0x0F;
        let lo = val & 0x0F;
        
        // 上位ニブル
        self.cpu.regs.a = if hi < 10 { 0xB0 + hi } else { 0xC1 + hi - 10 };
        self.stub_cout();
        
        // 下位ニブル
        self.cpu.regs.a = if lo < 10 { 0xB0 + lo } else { 0xC1 + lo - 10 };
        self.stub_cout();
        
        // Aを元に戻す
        self.cpu.regs.a = val;
    }
    
    /// テキストアドレスを計算（Apple II独自のメモリマップ）
    fn get_text_address(&self, x: u8, y: u8) -> u16 {
        // Apple II テキスト画面のアドレス計算
        // 行0-7: $0400, $0480, $0500, $0580, $0600, $0680, $0700, $0780
        // 行8-15: $0428, $04A8, $0528, $05A8, $0628, $06A8, $0728, $07A8
        // 行16-23: $0450, $04D0, $0550, $05D0, $0650, $06D0, $0750, $07D0
        let y = y as u16;
        let x = x as u16;
        
        let base = match y {
            0..=7 => 0x0400 + (y & 7) * 0x80,
            8..=15 => 0x0428 + ((y - 8) & 7) * 0x80,
            16..=23 => 0x0450 + ((y - 16) & 7) * 0x80,
            _ => 0x0400,
        };
        
        base + x
    }
    
    /// テキスト画面をスクロール
    fn scroll_text(&mut self) {
        // 各行を1行上にコピー
        for y in 0..23 {
            let src = self.get_text_address(0, y + 1);
            let dst = self.get_text_address(0, y);
            for x in 0..40 {
                self.memory.main_ram[(dst + x) as usize] = 
                    self.memory.main_ram[(src + x) as usize];
            }
        }
        // 最終行をクリア
        let last_line = self.get_text_address(0, 23);
        for x in 0..40 {
            self.memory.main_ram[(last_line + x) as usize] = 0xA0;
        }
    }

    /// 仮想ブートROM（VBR）によるブート処理
    /// 
    /// Disk II Boot ROMがロードされていない場合、この関数でブート処理を
    /// エミュレートします。Boot ROMが行う処理:
    /// 1. モーターON、ドライブ選択
    /// 2. トラック0、セクタ0を読み込み
    /// 3. $0800にロード
    /// 4. JMP $0801
    fn vbr_boot(&mut self) -> bool {
        // ディスクが挿入されているか確認
        if !self.disk.drives[0].disk.disk_loaded {
            return false;
        }
        
        // DSKデータから直接セクタ0を読み込む
        let sector_data = if let Some(ref dsk_data) = self.disk.drives[0].disk.dsk_data {
            // DSK形式: トラック0、セクタ0は先頭256バイト
            if dsk_data.len() >= 256 {
                dsk_data[0..256].to_vec()
            } else {
                return false;
            }
        } else {
            // NIB形式の場合は通常のブートROMが必要
            return false;
        };
        
        // $0800にセクタ0をロード
        for (i, &byte) in sector_data.iter().enumerate() {
            self.memory.main_ram[0x0800 + i] = byte;
        }
        
        // モーターをONに設定
        self.disk.motor_on = true;
        
        // デコードテーブルを生成（$0356-$03FF）
        // これはBoot ROMが最初に行う処理
        self.generate_decode_table();
        
        // PCを$0801に設定（ブートセクタの実行開始点）
        // $0800の最初のバイトは通常ジャンプ命令のオペランド
        self.cpu.regs.pc = 0x0801;
        
        // スタックポインタを初期化
        self.cpu.regs.sp = 0xFF;
        
        // VBRブート完了
        self.vbr_boot_done = true;
        
        true
    }
    
    /// 6-and-2デコードテーブルを生成（$0356-$03FF）
    /// Boot ROMが最初に行う初期化処理
    fn generate_decode_table(&mut self) {
        // 6-and-2エンコーディングのデコードテーブル
        // Apple II DOS 3.3マニュアル参照
        let decode_table: [u8; 64] = [
            0x00, 0x01, 0xFF, 0xFF, 0x02, 0x03, 0xFF, 0x04,
            0x05, 0x06, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0x07, 0x08, 0xFF, 0xFF, 0xFF, 0x09, 0x0A, 0x0B,
            0x0C, 0x0D, 0xFF, 0xFF, 0x0E, 0x0F, 0x10, 0x11,
            0x12, 0x13, 0xFF, 0x14, 0x15, 0x16, 0x17, 0x18,
            0x19, 0x1A, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x1B, 0xFF, 0x1C,
            0x1D, 0x1E, 0xFF, 0xFF, 0xFF, 0x1F, 0xFF, 0xFF,
        ];
        
        // $0396-$03FFにマッピング（$96-$FFの範囲）
        for i in 0..64 {
            let addr = 0x0300 + 0x56 + i; // $0356 + i
            if addr < 0x0400 {
                // 逆引きテーブル: エンコード値 → デコード値
                self.memory.main_ram[addr] = decode_table[i];
            }
        }
    }

    // NTSC: 1.023 MHz、60 Hz → 約17030サイクル/フレーム
    // 262スキャンライン × 65サイクル/ライン = 17030
    pub const CYCLES_PER_FRAME: u64 = SYSTEM_CYCLES_PER_FRAME;
    pub const CYCLES_PER_SCANLINE: u64 = SYSTEM_CYCLES_PER_SCANLINE;

    #[inline]
    fn update_video_timing_state(&mut self) {
        let pos = self.video_scanner.position();
        self.memory.scanline = pos.scanline;
        self.memory.scanline_cycle = pos.scanline_cycle;
        self.memory.frame_counter = pos.frame;
        self.memory.hblank = pos.hblank;
        self.memory.vblank = pos.vblank;
        self.disk.cumulative_cycles = self.total_cycles;
        if let Some((addr, value)) = self.sample_floating_bus() {
            self.floating_bus_address = addr;
            self.floating_bus_value = value;
        }
    }

    #[inline]
    fn text_address_for_scanline(&self, row: usize, col: usize) -> u16 {
        let base: usize = if self.memory.switches.page2 { 0x0800 } else { 0x0400 };
        let char_row = (row / 8).min(23);
        let addr = base
            + ((char_row & 0x07) << 7)
            + ((char_row >> 3) * 40)
            + col.min(39);
        addr as u16
    }

    #[inline]
    fn hires_address_for_scanline(&self, row: usize, byte_col: usize) -> u16 {
        let base: usize = if self.memory.switches.page2 { 0x4000 } else { 0x2000 };
        let addr = base
            + ((row & 0x07) << 10)
            + (((row >> 3) & 0x07) << 7)
            + ((row >> 6) * 40)
            + byte_col.min(39);
        addr as u16
    }

    #[inline]
    fn lores_address_for_scanline(&self, row: usize, col: usize) -> u16 {
        self.text_address_for_scanline(row, col)
    }

    #[inline]
    fn sample_floating_bus(&self) -> Option<(u16, u8)> {
        // Phase 9-A: floating bus follows the current tick-driven video beam
        // position. It is only driven while the video sequencer is fetching a
        // visible display byte; during HBL/VBL the previously latched value
        // remains on the bus.
        let pos = self.video_scanner.position();
        let row = pos.visible_row?;
        let column = match pos.fetch_phase {
            VideoFetchPhase::VisibleByte { column } => column as usize,
            VideoFetchPhase::HBlank | VideoFetchPhase::VBlank => return None,
        };

        let mixed_text_zone = self.memory.switches.mixed_mode && row >= 160;
        let use_text = self.memory.switches.text_mode || mixed_text_zone;
        let use_hires = self.memory.switches.hires && !use_text;

        let addr = if use_hires {
            self.hires_address_for_scanline(row, column)
        } else if !self.memory.switches.text_mode && !self.memory.switches.hires {
            self.lores_address_for_scanline(row / 8, column)
        } else {
            self.text_address_for_scanline(row, column)
        };

        let value = match (self.memory.switches.col_80, addr) {
            // 80-column text on IIe alternates between main/aux memory. For now,
            // only active text fetches are mirrored to the floating bus.
            (true, 0x0400..=0x07FF) | (true, 0x0800..=0x0BFF) => {
                if (column & 1) == 0 {
                    self.memory.aux_ram[addr as usize]
                } else {
                    self.memory.main_ram[addr as usize]
                }
            }
            _ => self.memory.main_ram[addr as usize],
        };
        Some((addr, value))
    }

    #[inline]
    fn maybe_do_vbr_boot(&mut self) {
        if self.vbr_mode && !self.vbr_boot_done {
            if self.cpu.regs.pc >= 0xC600 && self.cpu.regs.pc <= 0xC6FF {
                if self.vbr_boot() {
                    log::debug!("VBR: Virtual Boot ROM - booting from sector 0");
                } else {
                    log::warn!("VBR: Boot failed - no disk or incompatible format");
                    self.vbr_boot_done = true;
                }
            }
        }
    }

    /// Phase 3 system tick. 1 tick = 1 CPU cycle.
    ///
    /// まだCPU自体は命令単位実装だが、システム全体の時間軸は1サイクル単位で前進する。
    pub fn tick(&mut self) {
        if !self.running {
            return;
        }

        self.maybe_do_vbr_boot();

        // 命令境界で次の命令を発行する。
        if self.pending_cpu_cycles == 0 {
            self.record_pc(self.cpu.regs.pc);
            self.disk.observe_pc_with_memory(self.cpu.regs.pc, &self.memory.main_ram[..]);

            let mut cpu = std::mem::take(&mut self.cpu);
            let cycles = cpu.step(self);
            self.cpu = cpu;
            self.pending_cpu_cycles = cycles.max(1);
        }

        self.total_cycles = self.total_cycles.saturating_add(1);
        self.pending_cpu_cycles = self.pending_cpu_cycles.saturating_sub(1);
        self.cpu.advance_microcycle();
        self.disk.tick_disk_bit();

        let new_frame = self.video_scanner.tick();
        self.update_video_timing_state();
        if new_frame {
            self.frame_count = self.video_scanner.frame;
        }
    }

    /// 一命令だけ実行し、周辺回路も同じクロック軸で前進させる。
    pub fn step_instruction(&mut self) -> u64 {
        if !self.running {
            return 0;
        }

        let start_cycles = self.total_cycles;
        let mut started = false;
        let mut guard = 0u32;

        while self.running {
            let pending_before = self.pending_cpu_cycles;
            self.tick();
            guard = guard.saturating_add(1);

            if !started && pending_before == 0 {
                started = true;
            }

            if started && self.pending_cpu_cycles == 0 {
                break;
            }

            if guard > 64 {
                break;
            }
        }

        self.total_cycles.saturating_sub(start_cycles)
    }

    /// 指定サイクル数だけ実行（ビデオ/ディスクと同じクロック軸で進める）
    pub fn run_cycles(&mut self, target_cycles: u64) {
        let target = self.total_cycles.saturating_add(target_cycles);
        while self.running && self.total_cycles < target {
            self.tick();
        }
    }

    /// 現在のメモリ状態からビデオを描画
    pub fn render_video(&mut self) {
        self.video.render(&self.memory);
        self.frame_count = self.video_scanner.frame;
    }

    /// 1フレーム境界まで実行して描画する。
    pub fn run_frame(&mut self) {
        let start_frame = self.video_scanner.frame;
        while self.running && self.video_scanner.frame == start_frame {
            self.tick();
        }
        self.render_video();
    }

    /// 現在VBL期間中かどうか。
    #[inline]
    pub fn in_vblank(&self) -> bool {
        self.video_scanner.in_vblank()
    }

    /// 現在のfloating bus近似値を返す。
    #[inline]
    pub fn floating_bus(&self) -> u8 {
        self.floating_bus_value
    }

    /// 直近の floating bus が参照した表示メモリアドレス。
    #[inline]
    pub fn floating_bus_address(&self) -> u16 {
        self.floating_bus_address
    }

    /// 現在のビデオ走査位置。
    #[inline]
    pub fn video_position(&self) -> VideoPosition {
        self.video_scanner.position()
    }

    /// キー入力を処理
    pub fn key_down(&mut self, key: u8) {
        self.memory.set_key(key);
    }

    /// キーストローブが有効かどうかを確認
    #[allow(dead_code)]
    pub fn has_key_strobe(&self) -> bool {
        self.memory.has_key_strobe()
    }

    /// フレームバッファを取得
    pub fn get_framebuffer(&self) -> &[u32] {
        &self.video.framebuffer
    }
    
    /// スピーカークリックを取得してクリア
    pub fn take_speaker_clicks(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.speaker_clicks)
    }
    
    /// 現在の状態をセーブステートとして取得
    pub fn save_state(&self) -> SaveState {
        SaveState {
            version: SaveState::CURRENT_VERSION,
            cpu: CpuState {
                a: self.cpu.regs.a,
                x: self.cpu.regs.x,
                y: self.cpu.regs.y,
                sp: self.cpu.regs.sp,
                pc: self.cpu.regs.pc,
                status: self.cpu.regs.status,
                total_cycles: self.cpu.total_cycles,
                irq_pending: self.cpu.irq_pending,
                nmi_pending: self.cpu.nmi_pending,
            },
            memory: MemoryState {
                ram: self.memory.main_ram.to_vec(),
                bank1: self.memory.lc_ram_bank2.to_vec(),
                bank2: self.memory.lc_ram_bank2.to_vec(),
                lc_ram: self.memory.lc_ram.to_vec(),
                lc_read_enable: self.memory.switches.lc_read_enable,
                lc_write_enable: self.memory.switches.lc_write_enable,
                lc_bank2: self.memory.switches.lc_bank2,
                lc_prewrite: self.memory.switches.lc_prewrite,
                text_mode: self.memory.switches.text_mode,
                mixed_mode: self.memory.switches.mixed_mode,
                page2: self.memory.switches.page2,
                hires_mode: self.memory.switches.hires,
                col80: self.memory.switches.col_80,
                altchar: self.memory.switches.alt_char,
                keyboard_latch: self.memory.switches.keyboard_strobe,
            },
            disk: DiskState {
                curr_drive: self.disk.curr_drive,
                motor_off_scheduled_cycle: self.disk.motor_off_scheduled_cycle(),
                drives: [
                    DiskDriveState {
                        disk_loaded: self.disk.drives[0].disk.disk_loaded,
                        write_protected: self.disk.drives[0].disk.write_protected,
                        data: self.disk.drives[0].disk.data.to_vec(),
                        dsk_data: self.disk.drives[0].disk.dsk_data.clone(),
                        format: self.disk.drives[0].disk.format.map(|f| format!("{:?}", f)),
                        filename: self.disk.drives[0].disk.filename.clone(),
                        modified: self.disk.drives[0].disk.modified,
                        track_image_dirty: self.disk.drives[0].disk.track_image_dirty,
                        byte_position: self.disk.drives[0].disk.byte_position,
                        nibbles: self.disk.drives[0].disk.nibbles,
                        track_base: self.disk.drives[0].disk.track_base,
                        phase: self.disk.drives[0].phase,
                        phase_precise: self.disk.drives[0].phase_precise,
                        spinning: self.disk.drives[0].spinning,
                        write_light: self.disk.drives[0].write_light,
                        last_stepper_cycle: self.disk.drives[0].last_stepper_cycle,
                    },
                    DiskDriveState {
                        disk_loaded: self.disk.drives[1].disk.disk_loaded,
                        write_protected: self.disk.drives[1].disk.write_protected,
                        data: self.disk.drives[1].disk.data.to_vec(),
                        dsk_data: self.disk.drives[1].disk.dsk_data.clone(),
                        format: self.disk.drives[1].disk.format.map(|f| format!("{:?}", f)),
                        filename: self.disk.drives[1].disk.filename.clone(),
                        modified: self.disk.drives[1].disk.modified,
                        track_image_dirty: self.disk.drives[1].disk.track_image_dirty,
                        byte_position: self.disk.drives[1].disk.byte_position,
                        nibbles: self.disk.drives[1].disk.nibbles,
                        track_base: self.disk.drives[1].disk.track_base,
                        phase: self.disk.drives[1].phase,
                        phase_precise: self.disk.drives[1].phase_precise,
                        spinning: self.disk.drives[1].spinning,
                        write_light: self.disk.drives[1].write_light,
                        last_stepper_cycle: self.disk.drives[1].last_stepper_cycle,
                    },
                ],
                latch: self.disk.latch,
                write_mode: self.disk.write_mode,
                motor_on: self.disk.motor_on,
                magnet_states: self.disk.magnet_states,
                load_mode: self.disk.load_mode,
                shift_reg: self.disk.shift_reg,
                last_read_latch_cycle: self.disk.last_read_latch_cycle,
                cumulative_cycles: self.disk.cumulative_cycles,
                raw_bit_cycle_accumulator: self.disk.raw_bit_cycle_accumulator,
                raw_bit_counter: self.disk.raw_bit_counter,
                nibble_bit_count: self.disk.nibble_bit_count,
                nibble_shift_register: self.disk.nibble_shift_register,
                nibble_byte_ready: self.disk.nibble_byte_ready,
                controller_state: format!("{:?}", self.disk.controller_state),
                controller_edge_count: self.disk.controller_edge_count,
                last_controller_edge_cycle: self.disk.last_controller_edge_cycle,
                sync_bit_window: self.disk.sync_bit_window,
                pending_write_latch: self.disk.pending_write_latch,
                sequencer_mode: format!("{:?}", self.disk.sequencer_mode),
                weak_bits_enabled: self.disk.weak_bits_enabled,
                write_splice_enabled: self.disk.write_splice_enabled,
                disk_debug_logging: self.disk.disk_debug_logging,
                last_sync_detect_cycle: self.disk.last_sync_detect_cycle,
            },
            video: VideoState {
                flash_state: self.video.flash_state,
                frame_count: self.video.flash_counter as u64,
                scanner_frame: self.video_scanner.frame,
                scanner_scanline: self.video_scanner.scanline,
                scanner_scanline_cycle: self.video_scanner.scanline_cycle,
            },
            total_cycles: self.total_cycles,
            frame_count: self.frame_count,
            pending_cpu_cycles: self.pending_cpu_cycles,
            floating_bus_value: self.floating_bus_value,
            floating_bus_address: self.floating_bus_address,
        }
    }
    
    /// セーブステートから状態を復元
    pub fn load_state(&mut self, state: &SaveState) -> Result<(), &'static str> {
        if state.version == 0 || state.version > SaveState::CURRENT_VERSION {
            return Err("Incompatible save state version");
        }
        
        // CPU状態を復元
        self.cpu.regs.a = state.cpu.a;
        self.cpu.regs.x = state.cpu.x;
        self.cpu.regs.y = state.cpu.y;
        self.cpu.regs.sp = state.cpu.sp;
        self.cpu.regs.pc = state.cpu.pc;
        self.cpu.regs.status = state.cpu.status;
        self.cpu.total_cycles = state.cpu.total_cycles;
        self.cpu.irq_pending = state.cpu.irq_pending;
        self.cpu.nmi_pending = state.cpu.nmi_pending;
        
        // メモリ状態を復元
        if state.memory.ram.len() == self.memory.main_ram.len() {
            self.memory.main_ram.copy_from_slice(&state.memory.ram);
        }
        if state.memory.bank2.len() == self.memory.lc_ram_bank2.len() {
            self.memory.lc_ram_bank2.copy_from_slice(&state.memory.bank2);
        } else if state.memory.bank1.len() == self.memory.lc_ram_bank2.len() {
            self.memory.lc_ram_bank2.copy_from_slice(&state.memory.bank1);
        }
        if state.memory.lc_ram.len() == self.memory.lc_ram.len() {
            self.memory.lc_ram.copy_from_slice(&state.memory.lc_ram);
        }
        
        self.memory.switches.lc_read_enable = state.memory.lc_read_enable;
        self.memory.switches.lc_write_enable = state.memory.lc_write_enable;
        self.memory.switches.lc_bank2 = state.memory.lc_bank2;
        self.memory.switches.lc_prewrite = state.memory.lc_prewrite;
        self.memory.switches.text_mode = state.memory.text_mode;
        self.memory.switches.mixed_mode = state.memory.mixed_mode;
        self.memory.switches.page2 = state.memory.page2;
        self.memory.switches.hires = state.memory.hires_mode;
        self.memory.switches.col_80 = state.memory.col80;
        self.memory.switches.alt_char = state.memory.altchar;
        self.memory.switches.keyboard_strobe = state.memory.keyboard_latch;
        
        // ディスク状態を復元
        self.disk.curr_drive = state.disk.curr_drive.min(1);
        if state.version >= 3 {
            self.disk.set_motor_off_scheduled_cycle_for_restore(state.disk.motor_off_scheduled_cycle);
        } else {
            self.disk.set_motor_off_scheduled_cycle_for_restore(0);
        }
        self.disk.latch = state.disk.latch;
        self.disk.write_mode = state.disk.write_mode;
        self.disk.motor_on = state.disk.motor_on;
        self.disk.magnet_states = state.disk.magnet_states;
        self.disk.load_mode = state.disk.load_mode;
        self.disk.shift_reg = state.disk.shift_reg;
        self.disk.last_read_latch_cycle = state.disk.last_read_latch_cycle;
        self.disk.cumulative_cycles = state.disk.cumulative_cycles;
        self.disk.raw_bit_cycle_accumulator = state.disk.raw_bit_cycle_accumulator;
        self.disk.raw_bit_counter = state.disk.raw_bit_counter;
        self.disk.nibble_bit_count = state.disk.nibble_bit_count;
        self.disk.nibble_shift_register = state.disk.nibble_shift_register;
        self.disk.nibble_byte_ready = state.disk.nibble_byte_ready;
        self.disk.controller_state = match state.disk.controller_state.as_str() {
            "ReadSequencing" => DiskControllerState::ReadSequencing,
            "CheckWriteProtect" => DiskControllerState::CheckWriteProtect,
            "WriteLoad" => DiskControllerState::WriteLoad,
            "WriteShift" => DiskControllerState::WriteShift,
            _ => DiskControllerState::Idle,
        };
        self.disk.controller_edge_count = state.disk.controller_edge_count;
        self.disk.last_controller_edge_cycle = state.disk.last_controller_edge_cycle;
        self.disk.sync_bit_window = state.disk.sync_bit_window;
        self.disk.pending_write_latch = state.disk.pending_write_latch;
        self.disk.sequencer_mode = match state.disk.sequencer_mode.as_str() {
            "Transitional" => DiskSequencerMode::Transitional,
            "Strict" => DiskSequencerMode::Strict,
            _ => DiskSequencerMode::Safe,
        };
        self.disk.weak_bits_enabled = state.disk.weak_bits_enabled;
        self.disk.write_splice_enabled = state.disk.write_splice_enabled;
        self.disk.disk_debug_logging = state.disk.disk_debug_logging;
        self.disk.last_sync_detect_cycle = state.disk.last_sync_detect_cycle;

        for i in 0..2 {
            self.disk.drives[i].disk.disk_loaded = state.disk.drives[i].disk_loaded;
            self.disk.drives[i].disk.write_protected = state.disk.drives[i].write_protected;
            self.disk.drives[i].disk.dsk_data = state.disk.drives[i].dsk_data.clone();
            self.disk.drives[i].disk.format = match state.disk.drives[i].format.as_deref() {
                Some("Nib") => Some(DiskFormat::Nib),
                Some("Po") => Some(DiskFormat::Po),
                Some("Dsk") => Some(DiskFormat::Dsk),
                _ => None,
            };
            self.disk.drives[i].disk.filename = state.disk.drives[i].filename.clone();
            self.disk.drives[i].disk.modified = state.disk.drives[i].modified;
            self.disk.drives[i].disk.track_image_dirty = state.disk.drives[i].track_image_dirty;
            self.disk.drives[i].disk.data = state.disk.drives[i].data.clone();
            self.disk.drives[i].disk.byte_position = state.disk.drives[i].byte_position;
            self.disk.drives[i].disk.nibbles = state.disk.drives[i].nibbles;
            self.disk.drives[i].disk.track_base = state.disk.drives[i].track_base;
            self.disk.drives[i].phase = state.disk.drives[i].phase;
            self.disk.drives[i].phase_precise = state.disk.drives[i].phase_precise;
            self.disk.drives[i].spinning = state.disk.drives[i].spinning;
            self.disk.drives[i].write_light = state.disk.drives[i].write_light;
            self.disk.drives[i].last_stepper_cycle = state.disk.drives[i].last_stepper_cycle;
        }
        
        // ビデオ状態を復元
        self.video.flash_state = state.video.flash_state;
        self.video.flash_counter = state.video.frame_count as u32;
        
        // グローバル状態を復元
        self.total_cycles = state.total_cycles;
        self.frame_count = state.frame_count;
        self.pending_cpu_cycles = state.pending_cpu_cycles;
        self.floating_bus_value = state.floating_bus_value;
        self.floating_bus_address = state.floating_bus_address;

        if state.version >= 2 {
            self.video_scanner.scanline = state.video.scanner_scanline;
            self.video_scanner.scanline_cycle = state.video.scanner_scanline_cycle;
            self.video_scanner.frame = state.video.scanner_frame;
        } else {
            let frame_cycles = self.total_cycles % SYSTEM_CYCLES_PER_FRAME;
            self.video_scanner.scanline = (frame_cycles / SYSTEM_CYCLES_PER_SCANLINE) as u16;
            self.video_scanner.scanline_cycle = (frame_cycles % SYSTEM_CYCLES_PER_SCANLINE) as u8;
            self.video_scanner.frame = self.total_cycles / SYSTEM_CYCLES_PER_FRAME;
            self.pending_cpu_cycles = 0;
            self.floating_bus_address = 0;
            self.floating_bus_value = 0;
        }
        self.speaker_clicks.clear();
        self.update_video_timing_state();

        Ok(())
    }
}

// Note: Disk II Boot ROM must be loaded from external file (roms/disk2.rom)
// Apple's ROM code is copyrighted and cannot be distributed with this software.

/// テスト用ROMを生成（デモプログラム）
pub fn create_test_rom() -> Vec<u8> {
    let mut rom = vec![0xEAu8; 12288]; // $D000-$FFFF (12KB) NOPで埋める
    
    // $F000からのテストプログラム（オフセット = F000 - D000 = 2000）
    let offset = 0x2000usize;
    
    // Apple II テキスト画面の行アドレス（$0400ベース）
    // 行0: $0400, 行1: $0480, 行2: $0500, 行3: $0580
    // 行4: $0600, 行5: $0680, 行6: $0700, 行7: $0780
    // 行8: $0428, 行9: $04A8, 行10: $0528, 行11: $05A8
    // 行12: $0628, 行13: $06A8, 行14: $0728, 行15: $07A8
    // 行16: $0450, 行17: $04D0, 行18: $0550, 行19: $05D0
    // 行20: $0650, 行21: $06D0, 行22: $0750, 行23: $07D0
    
    let program: &[u8] = &[
        // F000: テキストモードに設定
        0xAD, 0x51, 0xC0, // LDA $C051 (TEXT ON)
        0xAD, 0x54, 0xC0, // LDA $C054 (PAGE1)
        
        // F006: 画面全体をスペースでクリア
        // $0400-$07FFの1024バイトをクリア
        0xA9, 0xA0,       // LDA #$A0 (space with high bit)
        0xA2, 0x00,       // LDX #$00
        // F00A:
        0x9D, 0x00, 0x04, // STA $0400,X
        0x9D, 0x00, 0x05, // STA $0500,X
        0x9D, 0x00, 0x06, // STA $0600,X
        0x9D, 0x00, 0x07, // STA $0700,X
        0xE8,             // INX
        0xD0, 0xF1,       // BNE $F00A
        
        // F019: 行12 ($0628) に "APPLE II EMULATOR" を表示
        // 列12から開始 → $0628 + 12 = $0634
        0xA2, 0x00,       // LDX #$00
        // F01B:
        0xBD, 0x70, 0xF0, // LDA $F070,X (メッセージ1) *** 修正 ***
        0xF0, 0x08,       // BEQ +8 (終了)
        0x9D, 0x34, 0x06, // STA $0634,X
        0xE8,             // INX
        0x4C, 0x1B, 0xF0, // JMP $F01B
        
        // F026: 行14 ($0728) に "RUST EDITION" を表示
        // 列14から開始 → $0728 + 14 = $0736
        0xA2, 0x00,       // LDX #$00
        // F028:
        0xBD, 0x82, 0xF0, // LDA $F082,X (メッセージ2) *** 修正 ***
        0xF0, 0x08,       // BEQ +8 (終了)
        0x9D, 0x36, 0x07, // STA $0736,X
        0xE8,             // INX
        0x4C, 0x28, 0xF0, // JMP $F028
        
        // F033: カウンタ初期化
        0xA9, 0x00,       // LDA #$00
        0x85, 0x00,       // STA $00
        
        // F037: メインループ - 行0にアニメーション
        0xE6, 0x00,       // INC $00
        0xA2, 0x00,       // LDX #$00
        // F03B:
        0x8A,             // TXA
        0x18,             // CLC
        0x65, 0x00,       // ADC $00
        0x29, 0x0F,       // AND #$0F (0-15)
        0x09, 0xB0,       // ORA #$B0 (数字 '0'-'?' with high bit)
        0x9D, 0x00, 0x04, // STA $0400,X (行0)
        0xE8,             // INX
        0xE0, 0x28,       // CPX #$28 (40列)
        0xD0, 0xF1,       // BNE $F03B
        
        // F04A: ウェイト
        0xA0, 0x06,       // LDY #$06
        // F04C:
        0xA2, 0x00,       // LDX #$00
        // F04E:
        0xCA,             // DEX
        0xD0, 0xFD,       // BNE $F04E
        0x88,             // DEY
        0xD0, 0xF7,       // BNE $F04C
        
        // F054: ループ
        0x4C, 0x37, 0xF0, // JMP $F037
        
        // パディング（$F057-$F06F = 25バイト → 実際は22バイト必要: 0x70-0x5A=0x16=22）
        0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA,
        0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA,
        0xEA, 0xEA, 0xEA, 0xEA, 0xEA, 0xEA,
        
        // F070: "APPLE II EMULATOR" (17文字 + 終端 = 18バイト)
        // Apple IIでは通常文字は $80-$FF（high bit set）
        0xC1, // 'A' + $80
        0xD0, // 'P' + $80
        0xD0, // 'P' + $80
        0xCC, // 'L' + $80
        0xC5, // 'E' + $80
        0xA0, // ' ' + $80
        0xC9, // 'I' + $80
        0xC9, // 'I' + $80
        0xA0, // ' ' + $80
        0xC5, // 'E' + $80
        0xCD, // 'M' + $80
        0xD5, // 'U' + $80
        0xCC, // 'L' + $80
        0xC1, // 'A' + $80
        0xD4, // 'T' + $80
        0xCF, // 'O' + $80
        0xD2, // 'R' + $80
        0x00, // 終端
        
        // F082: "RUST EDITION" (12文字 + 終端)
        0xD2, // 'R' + $80
        0xD5, // 'U' + $80
        0xD3, // 'S' + $80
        0xD4, // 'T' + $80
        0xA0, // ' ' + $80
        0xC5, // 'E' + $80
        0xC4, // 'D' + $80
        0xC9, // 'I' + $80
        0xD4, // 'T' + $80
        0xC9, // 'I' + $80
        0xCF, // 'O' + $80
        0xCE, // 'N' + $80
        0x00, // 終端
    ];
    
    // プログラムをROMにコピー
    for (i, &byte) in program.iter().enumerate() {
        if offset + i < rom.len() {
            rom[offset + i] = byte;
        }
    }
    
    // Monitor ROMスタブを追加 (DOS 3.3ブートに必要)
    // これらはディスクからブートしない場合のデモ用ROMなので、
    // ディスクブートには実際のApple II ROMが必要
    
    // $FF58 - スロット番号取得用（Boot ROMが使用）
    // 単にRTSするだけ。Boot ROMはJSR後のスタックを読んでスロット番号を計算
    let ff58_offset = 0x2F58; // $FF58 - $D000
    rom[ff58_offset] = 0x60; // RTS
    
    // $FCA8 - WAIT ルーチン
    // A = 待ち時間、即リターンでOK
    let fca8_offset = 0x2CA8; // $FCA8 - $D000
    rom[fca8_offset] = 0x60; // RTS
    
    // $FC58 - HOME（画面クリア）
    // 簡易実装：即リターン
    let fc58_offset = 0x2C58; // $FC58 - $D000
    rom[fc58_offset] = 0x60; // RTS
    
    // $FDED - COUT（文字出力）
    // A = 文字、即リターン
    let fded_offset = 0x2DED; // $FDED - $D000
    rom[fded_offset] = 0x60; // RTS
    
    // $FD8E - CROUT（改行）
    let fd8e_offset = 0x2D8E; // $FD8E - $D000
    rom[fd8e_offset] = 0x60; // RTS
    
    // $FDDA - PRBYTE（16進数出力）
    let fdda_offset = 0x2DDA; // $FDDA - $D000
    rom[fdda_offset] = 0x60; // RTS
    
    // $FB2F - TEXT2COPY/INIT（初期化）
    let fb2f_offset = 0x2B2F; // $FB2F - $D000
    rom[fb2f_offset] = 0x60; // RTS
    
    // $FE89 - SETKBD（キーボード入力設定）
    let fe89_offset = 0x2E89; // $FE89 - $D000
    rom[fe89_offset] = 0x60; // RTS
    
    // $FE93 - SETVID（ビデオ出力設定）
    let fe93_offset = 0x2E93; // $FE93 - $D000
    rom[fe93_offset] = 0x60; // RTS
    
    // リセットベクター ($FFFC-$FFFD) -> $C600 (ディスクブート)
    // ディスクがあればブート、なければ$F000へ
    rom[0x2FFC] = 0x00; // Low byte
    rom[0x2FFD] = 0xC6; // High byte -> $C600
    
    // NMI/IRQベクターも設定
    rom[0x2FFA] = 0x00;
    rom[0x2FFB] = 0xF0;
    rom[0x2FFE] = 0x00;
    rom[0x2FFF] = 0xF0;
    
    rom
}
