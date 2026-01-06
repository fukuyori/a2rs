//! Apple II エミュレータ
//! 
//! CPU、メモリ、ビデオ、ディスクを統合

use crate::cpu::{Cpu, CpuType, MemoryBus};
use crate::memory::{AppleModel, Memory};
use crate::video::Video;
use crate::disk::{Disk2InterfaceCard, DiskFormat, DSK_SIZE, NIB_SIZE};
use crate::savestate::{SaveState, CpuState, MemoryState, DiskState, DiskDriveState, VideoState};

/// Apple IIエミュレータのメイン構造体
pub struct Apple2 {
    /// 6502/65C02 CPU
    pub cpu: Cpu,
    /// メモリシステム
    pub memory: Memory,
    /// ビデオサブシステム
    pub video: Video,
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
}

/// メモリバスの実装（Disk II I/Oを含む）
impl MemoryBus for Apple2 {
    fn read(&mut self, address: u16) -> u8 {
        match address {
            // スピーカー ($C030-$C03F)
            0xC030..=0xC03F => {
                self.speaker_clicks.push(self.total_cycles);
                self.memory.read(address)
            }
            // パドル/ジョイスティック ($C064-$C06F, $C070-$C07F)
            0xC064..=0xC07F => {
                // パドル読み取り時にサイクル情報を渡す
                self.memory.paddle_read_cycle = self.total_cycles;
                self.memory.read(address)
            }
            // Disk II ブートROM (スロット6: $C600-$C6FF)
            0xC600..=0xC6FF => {
                // VBRモード: ROMがロードされていない場合
                if !self.disk.is_rom_loaded() {
                    self.vbr_mode = true;
                    // VBR: 0x00を返す（NOPとして実行される）
                    // 実際のブートはrun_cycle内でPCを監視して行う
                    0x00
                } else {
                    self.disk.read_rom((address & 0xFF) as u8)
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
        // Apple IIe (非Enhanced) も 6502 を使用
        let cpu_type = match model {
            AppleModel::AppleIIeEnhanced => CpuType::Cpu65C02,
            _ => CpuType::Cpu6502,
        };

        Apple2 {
            cpu: Cpu::new(cpu_type),
            memory: Memory::new(model),
            video: Video::new(),
            disk: Disk2InterfaceCard::new(),
            total_cycles: 0,
            frame_count: 0,
            running: true,
            speaker_clicks: Vec::with_capacity(4096),
            vbr_mode: false,
            vbr_boot_done: false,
        }
    }

    /// ROMサイズからモデルを自動検出
    pub fn detect_model_from_rom(rom_data: &[u8]) -> AppleModel {
        match rom_data.len() {
            20480 => AppleModel::AppleIIPlus,  // 20KB = Apple II Plus
            32768 => AppleModel::AppleIIe,      // 32KB = Apple IIe
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
        log::info!("Loaded external Disk II Boot ROM");
        Ok(())
    }

    /// ディスク高速化を設定
    pub fn set_fast_disk(&mut self, fast: bool) {
        self.disk.enhance_disk = fast;
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
        
        // ディスクがロードされている場合、直接スロット6ブートを開始
        // (Monitor ROMのAutostart機能を使わず、直接$C600にジャンプ)
        if self.disk.drives[0].disk.disk_loaded {
            // P5 PROMは $C625 で LDA $0100,X (X=SP) を実行してスロット番号を取得
            // SP=$FC の時、TSXでX=$FCになり、$01FCを読む
            // $01FC にスロット番号($06)を書き込む
            self.memory.main_ram[0x01FC] = 0x06;  // スロット番号
            self.cpu.regs.sp = 0xFC;
            
            self.cpu.regs.pc = 0xC600;
            println!("Direct boot from slot 6 ($C600) - disk_loaded=true");
        } else {
            println!("No disk boot - disk_loaded=false");
        }
    }

    /// 1命令を実行
    pub fn step(&mut self) -> u32 {
        // CPUを一時的に取り出して実行
        let mut cpu = std::mem::take(&mut self.cpu);
        let cycles = cpu.step(self);
        self.cpu = cpu;
        self.total_cycles += cycles as u64;
        
        cycles
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

    /// 指定サイクル数だけ実行
    pub fn run_cycles(&mut self, target_cycles: u64) {
        let start = self.total_cycles;
        // CPUを一時的に取り出して実行
        let mut cpu = std::mem::take(&mut self.cpu);
        while self.running && (self.total_cycles - start) < target_cycles {
            let cycles = cpu.step(self);
            self.total_cycles += cycles as u64;
        }
        self.cpu = cpu;
    }

    /// 1フレーム分（約17030サイクル、60Hz）を実行
    pub fn run_frame(&mut self) {
        // VBRモード: $C600にジャンプしようとしている場合
        if self.vbr_mode && !self.vbr_boot_done {
            // PCが$C600-$C6FF範囲にあればVBRブートを実行
            if self.cpu.regs.pc >= 0xC600 && self.cpu.regs.pc <= 0xC6FF {
                if self.vbr_boot() {
                    println!("VBR: Virtual Boot ROM - booting from sector 0");
                } else {
                    println!("VBR: Boot failed - no disk or incompatible format");
                    self.vbr_boot_done = true; // 無限ループ防止
                }
            }
        }
        
        // NTSC: 1.023 MHz、60 Hz → 約17030サイクル/フレーム
        // 262スキャンライン × 65サイクル/ライン = 17030
        const CYCLES_PER_FRAME: u64 = 17030;
        const CYCLES_PER_SCANLINE: u64 = 65;
        
        let target = self.total_cycles + CYCLES_PER_FRAME;
        let frame_start = self.total_cycles;
        
        // CPUを一時的に取り出して実行
        let mut cpu = std::mem::take(&mut self.cpu);
        while self.running && self.total_cycles < target {
            // スキャンラインを更新（VBL検出用）
            let frame_cycles = self.total_cycles - frame_start;
            self.memory.scanline = (frame_cycles / CYCLES_PER_SCANLINE) as u16;
            
            // SafeFast: CPUのPCとメモリを観測（IOB検証付き）
            self.disk.observe_pc_with_memory(cpu.regs.pc, &self.memory.main_ram[..]);
            
            let cycles = cpu.step(self);
            self.total_cycles += cycles as u64;
        }
        self.cpu = cpu;
        
        // フレーム終了後はVBL期間
        self.memory.scanline = 192;
        
        self.frame_count += 1;
        
        // ビデオを更新
        self.video.render(&self.memory);
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
                drives: [
                    DiskDriveState {
                        disk_loaded: self.disk.drives[0].disk.disk_loaded,
                        write_protected: self.disk.drives[0].disk.write_protected,
                        data: self.disk.drives[0].disk.data.to_vec(),
                        byte_position: self.disk.drives[0].disk.byte_position,
                        phase: self.disk.drives[0].phase,
                    },
                    DiskDriveState {
                        disk_loaded: self.disk.drives[1].disk.disk_loaded,
                        write_protected: self.disk.drives[1].disk.write_protected,
                        data: self.disk.drives[1].disk.data.to_vec(),
                        byte_position: self.disk.drives[1].disk.byte_position,
                        phase: self.disk.drives[1].phase,
                    },
                ],
                latch: self.disk.latch,
                write_mode: self.disk.write_mode,
                motor_on: self.disk.motor_on,
            },
            video: VideoState {
                flash_state: self.video.flash_state,
                frame_count: self.video.flash_counter as u64,
            },
            total_cycles: self.total_cycles,
            frame_count: self.frame_count,
        }
    }
    
    /// セーブステートから状態を復元
    pub fn load_state(&mut self, state: &SaveState) -> Result<(), &'static str> {
        if state.version != SaveState::CURRENT_VERSION {
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
        if state.memory.bank1.len() == self.memory.lc_ram_bank2.len() {
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
        self.disk.curr_drive = state.disk.curr_drive;
        self.disk.latch = state.disk.latch;
        self.disk.write_mode = state.disk.write_mode;
        self.disk.motor_on = state.disk.motor_on;
        
        for i in 0..2 {
            self.disk.drives[i].disk.disk_loaded = state.disk.drives[i].disk_loaded;
            self.disk.drives[i].disk.write_protected = state.disk.drives[i].write_protected;
            if state.disk.drives[i].data.len() == self.disk.drives[i].disk.data.len() {
                self.disk.drives[i].disk.data.copy_from_slice(&state.disk.drives[i].data);
            }
            self.disk.drives[i].disk.byte_position = state.disk.drives[i].byte_position;
            self.disk.drives[i].phase = state.disk.drives[i].phase;
        }
        
        // ビデオ状態を復元
        self.video.flash_state = state.video.flash_state;
        self.video.flash_counter = state.video.frame_count as u32;
        
        // グローバル状態を復元
        self.total_cycles = state.total_cycles;
        self.frame_count = state.frame_count;
        
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
    
    // リセットベクター ($FFFC-$FFFD) -> $F000
    rom[0x2FFC] = 0x00; // Low byte
    rom[0x2FFD] = 0xF0; // High byte
    
    // NMI/IRQベクターも$F000に
    rom[0x2FFA] = 0x00;
    rom[0x2FFB] = 0xF0;
    rom[0x2FFE] = 0x00;
    rom[0x2FFF] = 0xF0;
    
    rom
}
