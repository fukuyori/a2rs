//! Apple II メモリサブシステム
//! 
//! Apple IIのメモリマップとソフトスイッチを実装

use crate::cpu::MemoryBus;

/// Apple IIのモデル
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppleModel {
    AppleII,
    AppleIIPlus,
    AppleIIe,
    AppleIIeEnhanced,
}

/// ソフトスイッチの状態
#[derive(Debug, Clone)]
pub struct SoftSwitches {
    pub keyboard_strobe: u8,
    pub text_mode: bool,
    pub mixed_mode: bool,
    pub page2: bool,
    pub hires: bool,
    pub store_80: bool,
    pub col_80: bool,
    pub alt_char: bool,
    pub dhires: bool,         // ダブルHi-Resモード
    pub ioudis: bool,         // IOU disable (DHIRESアクセス制御)
    pub lc_bank2: bool,
    pub lc_read_enable: bool,
    pub lc_write_enable: bool,
    pub lc_prewrite: bool,
    pub ramrd: bool,
    pub ramwrt: bool,
    pub altzp: bool,
    pub speaker_click: bool,
    #[allow(dead_code)]
    pub annunciator: [bool; 4],
    
    // ゲームコントローラ
    pub button0: bool,        // $C061 - ジョイスティックボタン0 / Open-Apple
    pub button1: bool,        // $C062 - ジョイスティックボタン1 / Closed-Apple
    pub button2: bool,        // $C063 - ジョイスティックボタン2
    pub paddle0: u8,          // $C064 - パドル0 (X軸) 0-255
    pub paddle1: u8,          // $C065 - パドル1 (Y軸) 0-255
    pub paddle2: u8,          // $C066 - パドル2
    pub paddle3: u8,          // $C067 - パドル3
    pub paddle_trigger_cycle: u64,  // パドルトリガーサイクル
}

impl Default for SoftSwitches {
    fn default() -> Self {
        SoftSwitches {
            keyboard_strobe: 0,
            text_mode: true,      // 起動時はテキストモード
            mixed_mode: false,
            page2: false,
            hires: false,
            store_80: false,
            col_80: false,
            alt_char: false,
            dhires: false,
            ioudis: true,         // デフォルトでIOUは有効
            lc_bank2: false,
            lc_read_enable: false,
            lc_write_enable: false,
            lc_prewrite: false,
            ramrd: false,
            ramwrt: false,
            altzp: false,
            speaker_click: false,
            annunciator: [false; 4],
            
            // ゲームコントローラ（中央位置で初期化）
            button0: false,
            button1: false,
            button2: false,
            paddle0: 128,
            paddle1: 128,
            paddle2: 128,
            paddle3: 128,
            paddle_trigger_cycle: 0,
        }
    }
}

/// Apple IIメモリシステム
#[derive(Clone)]
pub struct Memory {
    pub main_ram: Box<[u8; 65536]>,
    pub aux_ram: Box<[u8; 65536]>,
    pub lc_ram: Box<[u8; 16384]>,
    pub lc_ram_bank2: Box<[u8; 4096]>,
    pub rom: Vec<u8>,
    pub slot_rom: Vec<[u8; 256]>,
    pub model: AppleModel,
    pub switches: SoftSwitches,
    /// パドル読み取り時のCPUサイクル（外部から設定）
    pub paddle_read_cycle: u64,
    /// 現在のスキャンライン（VBL検出用）
    pub scanline: u16,
}

impl Default for Memory {
    fn default() -> Self {
        Memory::new(AppleModel::AppleIIPlus)
    }
}

impl Memory {
    pub fn new(model: AppleModel) -> Self {
        Memory {
            main_ram: Box::new([0; 65536]),
            aux_ram: Box::new([0; 65536]),
            lc_ram: Box::new([0; 16384]),
            lc_ram_bank2: Box::new([0; 4096]),
            rom: Vec::new(),
            slot_rom: vec![[0; 256]; 8],
            model,
            switches: SoftSwitches::default(),
            paddle_read_cycle: 0,
            scanline: 0,
        }
    }

    pub fn load_rom(&mut self, rom_data: &[u8]) {
        // ROMサイズに応じて適切に配置
        // 2KB:  $F800-$FFFF (ミニROM、apple2dead.bin等)
        // 12KB: $D000-$FFFF (Apple II/II+ ROM)
        // 16KB: $C000-$FFFF (フルROM)
        // 20KB: Apple II Plus ROM パッケージ
        //       $0000-$05FF: パディング/ゼロ
        //       $0600-$06FF: Disk II P5 Boot ROM (256B) → $C600
        //       $0700-$15FF: パディング
        //       $1600-$16FF: Disk II P6 ROM (256B) → $C500 (未使用)
        //       $1700-$1FFF: パディング
        //       $2000-$4FFF: Autostart Monitor ROM (12KB) → $D000-$FFFF
        // 32KB: Apple IIe ROM
        //       $0000-$00FF: 未使用/パディング
        //       $0200-$02FF: Self-test ROM
        //       $0600-$06FF: Disk II P5 Boot ROM → $C600
        //       $4000-$7FFF: メインROM (16KB) → $C000-$FFFF
        match rom_data.len() {
            2048 => {
                // 2KB ROM: $F800-$FFFF にマッピング
                self.rom = vec![0xFF; 16384]; // 16KB ($C000-$FFFF)
                let offset = 0x3800; // $F800 - $C000
                for (i, &byte) in rom_data.iter().enumerate() {
                    if offset + i < self.rom.len() {
                        self.rom[offset + i] = byte;
                    }
                }
            }
            12288 => {
                // 12KB ROM: $D000-$FFFF にマッピング
                self.rom = vec![0xFF; 16384]; // 16KB ($C000-$FFFF)
                // $D000-$FFFF = オフセット $1000 から
                for (i, &byte) in rom_data.iter().enumerate() {
                    self.rom[0x1000 + i] = byte;
                }
            }
            16384 => {
                // 16KB ROM: $C000-$FFFF にそのままマッピング
                self.rom = rom_data.to_vec();
            }
            20480 => {
                // 20KB Apple II Plus ROM パッケージ
                // 構造:
                //   $0000-$05FF: パディング
                //   $0600-$06FF: Disk II P5 Boot ROM (256B)
                //   $0700-$1FFF: パディング
                //   $2000-$4FFF: Autostart Monitor ROM (12KB)
                //
                // メモリマッピング:
                //   Disk II ROM → $C600-$C6FF
                //   Monitor ROM → $D000-$FFFF (12KB)
                
                // 16KB ROMスペースを確保（$C000-$FFFF）
                self.rom = vec![0xFF; 16384];
                
                // Disk II P5 Boot ROM ($0600-$06FF) → $C600-$C6FF
                // ROMオフセット $0600
                for i in 0..256 {
                    self.rom[0x0600 + i] = rom_data[0x0600 + i];
                }
                
                // Autostart Monitor ROM ($2000-$4FFF, 12KB) → $D000-$FFFF
                // $D000 = ROMオフセット $1000
                // ファイル$2000 → ROM$1000、ファイル$4FFF → ROM$3FFF
                for i in 0..12288 {
                    self.rom[0x1000 + i] = rom_data[0x2000 + i];
                }
                
                println!("Loaded 20KB Apple II Plus ROM");
                println!("  Disk II Boot ROM: $C600-$C6FF");
                println!("  Autostart ROM: $D000-$FFFF");
                
                // デバッグ：ベクタを確認
                let reset_low = self.rom[0x3FFC];
                let reset_high = self.rom[0x3FFD];
                println!("  Reset vector: ${:02X}{:02X}", reset_high, reset_low);
            }
            32768 => {
                // 32KB Apple IIe ROM
                // 構造:
                //   $0000-$07FF: 文字ROM等
                //   $0600-$06FF: Disk II P5 Boot ROM (256B)
                //   $4000-$7FFF: メインROM (16KB)
                //
                // メモリマッピング:
                //   メインROM ($4000-$7FFF) → $C000-$FFFF
                //   Disk II ROM は後半にも含まれている ($4600-$46FF)
                
                // 後半16KB ($4000-$7FFF) をそのまま使用
                self.rom = rom_data[0x4000..0x8000].to_vec();
                
                println!("Loaded 32KB Apple IIe ROM");
                println!("  Main ROM: $C000-$FFFF (from file offset $4000-$7FFF)");
                
                // デバッグ：ベクタを確認
                let reset_low = self.rom[0x3FFC];
                let reset_high = self.rom[0x3FFD];
                println!("  Reset vector: ${:02X}{:02X}", reset_high, reset_low);
                
                // Disk II Boot ROMが正しい位置にあるか確認
                // $C600 = ROM offset $0600
                if self.rom[0x0600] == 0xA2 && self.rom[0x0601] == 0x20 {
                    println!("  Disk II Boot ROM: OK at $C600");
                } else {
                    // 前半からDisk II ROMをコピー
                    println!("  Copying Disk II Boot ROM from file offset $0600");
                    for i in 0..256 {
                        self.rom[0x0600 + i] = rom_data[0x0600 + i];
                    }
                }
            }
            _ => {
                // その他のサイズはそのまま
                log::warn!("Unknown ROM size: {} bytes", rom_data.len());
                self.rom = rom_data.to_vec();
            }
        }
    }
    
    /// 文字ROMデータを取得（32KB ROMから）
    #[allow(dead_code)]
    pub fn get_char_rom_from_32k(rom_data: &[u8]) -> Option<Vec<u8>> {
        if rom_data.len() == 32768 {
            // 後半16KBの先頭部分が文字ROM
            Some(rom_data[16384..18432].to_vec()) // 2KB
        } else {
            None
        }
    }

    pub fn is_iie(&self) -> bool {
        matches!(self.model, AppleModel::AppleIIe | AppleModel::AppleIIeEnhanced)
    }

    pub fn set_key(&mut self, key: u8) {
        self.switches.keyboard_strobe = key | 0x80;
    }

    /// キーストローブが有効か（bit7がセットされているか）
    #[allow(dead_code)]
    pub fn has_key_strobe(&self) -> bool {
        (self.switches.keyboard_strobe & 0x80) != 0
    }
    
    /// ジョイスティックボタンを設定
    pub fn set_button(&mut self, button: usize, pressed: bool) {
        match button {
            0 => self.switches.button0 = pressed,
            1 => self.switches.button1 = pressed,
            2 => self.switches.button2 = pressed,
            _ => {}
        }
    }
    
    /// パドル値を設定 (0-255, 128が中央)
    pub fn set_paddle(&mut self, paddle: usize, value: u8) {
        match paddle {
            0 => self.switches.paddle0 = value,
            1 => self.switches.paddle1 = value,
            2 => self.switches.paddle2 = value,
            3 => self.switches.paddle3 = value,
            _ => {}
        }
    }
    
    /// ジョイスティック軸を設定 (-1.0 to 1.0 を 0-255 に変換)
    #[allow(dead_code)]
    pub fn set_joystick_axis(&mut self, axis: usize, value: f32) {
        let paddle_value = ((value + 1.0) * 127.5).clamp(0.0, 255.0) as u8;
        self.set_paddle(axis, paddle_value);
    }

    fn read_soft_switch(&mut self, address: u16) -> u8 {
        let addr = address & 0xFF;
        match addr {
            0x00..=0x0F => self.switches.keyboard_strobe,
            0x10 => {
                // $C010: ANY KEY DOWN (キーストローブクリア)
                let result = self.switches.keyboard_strobe;
                self.switches.keyboard_strobe &= 0x7F;
                result
            }
            0x11 if self.is_iie() => {
                // $C011: RDLCBNK2 - LC Bank2 status
                if self.switches.lc_bank2 { 0x80 } else { 0x00 }
            }
            0x12 if self.is_iie() => {
                // $C012: RDLCRAM - LC RAM read enabled
                if self.switches.lc_read_enable { 0x80 } else { 0x00 }
            }
            0x13 if self.is_iie() => {
                // $C013: RDRAMRD - Aux RAM read
                if self.switches.ramrd { 0x80 } else { 0x00 }
            }
            0x14 if self.is_iie() => {
                // $C014: RDRAMWRT - Aux RAM write
                if self.switches.ramwrt { 0x80 } else { 0x00 }
            }
            0x15 if self.is_iie() => {
                // $C015: RDCXROM - Internal ROM
                0x00 // 常に外部スロットROM
            }
            0x16 if self.is_iie() => {
                // $C016: RDALTZP - Alt zero page
                if self.switches.altzp { 0x80 } else { 0x00 }
            }
            0x17 if self.is_iie() => {
                // $C017: RDC3ROM - Slot 3 ROM
                0x00
            }
            0x18 if self.is_iie() => {
                // $C018: RD80STORE
                if self.switches.store_80 { 0x80 } else { 0x00 }
            }
            0x19 if self.is_iie() => {
                // $C019: RDVBL - Vertical blank
                // スキャンライン192-261がVBL期間
                // ビット7: 0=VBL中, 1=表示中（実機と逆に見えるが正しい）
                if self.scanline >= 192 {
                    0x00  // VBL期間中
                } else {
                    0x80  // 表示期間中
                }
            }
            0x1A if self.is_iie() => {
                // $C01A: RDTEXT
                if self.switches.text_mode { 0x80 } else { 0x00 }
            }
            0x1B if self.is_iie() => {
                // $C01B: RDMIXED
                if self.switches.mixed_mode { 0x80 } else { 0x00 }
            }
            0x1C if self.is_iie() => {
                // $C01C: RDPAGE2
                if self.switches.page2 { 0x80 } else { 0x00 }
            }
            0x1D if self.is_iie() => {
                // $C01D: RDHIRES
                if self.switches.hires { 0x80 } else { 0x00 }
            }
            0x1E if self.is_iie() => {
                // $C01E: RDALTCHAR
                if self.switches.alt_char { 0x80 } else { 0x00 }
            }
            0x1F if self.is_iie() => {
                // $C01F: RD80COL
                if self.switches.col_80 { 0x80 } else { 0x00 }
            }
            0x11..=0x1F => {
                // Apple II/II+: キーストローブクリア
                let result = self.switches.keyboard_strobe;
                self.switches.keyboard_strobe &= 0x7F;
                result
            }
            0x20..=0x2F => 0x00, // カセットI/O（未実装）
            0x30..=0x3F => { self.switches.speaker_click = !self.switches.speaker_click; 0x00 }
            0x40..=0x4F => 0x00, // ゲームI/O
            0x50 => { self.switches.text_mode = false; 0x00 }
            0x51 => { self.switches.text_mode = true; 0x00 }
            0x52 => { self.switches.mixed_mode = false; 0x00 }
            0x53 => { self.switches.mixed_mode = true; 0x00 }
            0x54 => { self.switches.page2 = false; 0x00 }
            0x55 => { self.switches.page2 = true; 0x00 }
            0x56 => { self.switches.hires = false; 0x00 }
            0x57 => { self.switches.hires = true; 0x00 }
            // アヌンシエータ $C058-$C05F
            0x58 => { self.switches.annunciator[0] = false; 0x00 }
            0x59 => { self.switches.annunciator[0] = true; 0x00 }
            0x5A => { self.switches.annunciator[1] = false; 0x00 }
            0x5B => { self.switches.annunciator[1] = true; 0x00 }
            0x5C => { self.switches.annunciator[2] = false; 0x00 }
            0x5D => { self.switches.annunciator[2] = true; 0x00 }
            // $C05E/$C05F: Apple IIeではDHIRES制御
            0x5E => {
                if self.is_iie() && !self.switches.ioudis {
                    self.switches.dhires = true;
                } else {
                    self.switches.annunciator[3] = false;
                }
                0x00
            }
            0x5F => {
                if self.is_iie() && !self.switches.ioudis {
                    self.switches.dhires = false;
                } else {
                    self.switches.annunciator[3] = true;
                }
                0x00
            }
            0x60 => 0x00, // カセットI/O
            // ゲームポート: ボタン
            0x61 => if self.switches.button0 { 0x80 } else { 0x00 },
            0x62 => if self.switches.button1 { 0x80 } else { 0x00 },
            0x63 => if self.switches.button2 { 0x80 } else { 0x00 },
            // ゲームポート: パドル（タイマー方式）
            // $C070でトリガー後、パドル値×11サイクル経過するまでHighを返す
            0x64..=0x67 => {
                let paddle_idx = (addr - 0x64) as usize;
                let paddle_val = match paddle_idx {
                    0 => self.switches.paddle0,
                    1 => self.switches.paddle1,
                    2 => self.switches.paddle2,
                    _ => self.switches.paddle3,
                } as u64;
                
                // パドル値×11サイクル経過するまでHighを返す
                // Apple IIでは約2.8ms（=2872サイクル）が最大
                let timeout_cycles = paddle_val * 11;
                let elapsed = self.paddle_read_cycle.saturating_sub(self.switches.paddle_trigger_cycle);
                
                if elapsed < timeout_cycles {
                    0x80 // まだタイムアウトしていない
                } else {
                    0x00 // タイムアウト
                }
            }
            0x70..=0x7D => {
                // $C070: パドルトリガー（読み込みでタイマーリセット）
                self.switches.paddle_trigger_cycle = self.paddle_read_cycle;
                0x00
            }
            0x7E if self.is_iie() => {
                // $C07E: IOUDIS - IOU disable status
                if self.switches.ioudis { 0x80 } else { 0x00 }
            }
            0x7F if self.is_iie() => {
                // $C07F: DHIRES status
                if self.switches.dhires { 0x80 } else { 0x00 }
            }
            0x7E | 0x7F => {
                // Apple II/II+: パドルトリガー
                self.switches.paddle_trigger_cycle = self.paddle_read_cycle;
                0x00
            }
            0x80..=0x8F => self.handle_language_card((addr & 0xFF) as u8),
            _ => 0x00,
        }
    }

    fn write_soft_switch(&mut self, address: u16, _value: u8) {
        let addr = address & 0xFF;
        match addr {
            // $C010-$C01F: キーストローブクリア（書き込みでも）
            0x10..=0x1F => {
                self.switches.keyboard_strobe &= 0x7F;
            }
            // Apple IIe 80列カードスイッチ（書き込みで動作）
            0x00 if self.is_iie() => self.switches.store_80 = false,
            0x01 if self.is_iie() => self.switches.store_80 = true,
            0x02 if self.is_iie() => self.switches.ramrd = false,
            0x03 if self.is_iie() => self.switches.ramrd = true,
            0x04 if self.is_iie() => self.switches.ramwrt = false,
            0x05 if self.is_iie() => self.switches.ramwrt = true,
            0x08 if self.is_iie() => self.switches.altzp = false,
            0x09 if self.is_iie() => self.switches.altzp = true,
            0x0C if self.is_iie() => self.switches.col_80 = false,
            0x0D if self.is_iie() => self.switches.col_80 = true,
            0x0E if self.is_iie() => self.switches.alt_char = false,
            0x0F if self.is_iie() => self.switches.alt_char = true,
            0x30 => self.switches.speaker_click = !self.switches.speaker_click,
            0x50 => self.switches.text_mode = false,
            0x51 => self.switches.text_mode = true,
            0x52 => self.switches.mixed_mode = false,
            0x53 => self.switches.mixed_mode = true,
            0x54 => self.switches.page2 = false,
            0x55 => self.switches.page2 = true,
            0x56 => self.switches.hires = false,
            0x57 => self.switches.hires = true,
            // アナンシエーター / DHIRES制御
            0x5E if self.is_iie() && !self.switches.ioudis => self.switches.dhires = true,
            0x5F if self.is_iie() && !self.switches.ioudis => self.switches.dhires = false,
            // IOUDIS制御
            0x7E if self.is_iie() => self.switches.ioudis = true,
            0x7F if self.is_iie() => self.switches.ioudis = false,
            0x80..=0x8F => { self.handle_language_card((addr & 0xFF) as u8); }
            _ => {}
        }
    }

    fn handle_language_card(&mut self, addr: u8) -> u8 {
        match addr & 0x0F {
            0x0 | 0x4 => {
                self.switches.lc_bank2 = true;
                self.switches.lc_read_enable = true;
                self.switches.lc_write_enable = false;
                self.switches.lc_prewrite = false;
            }
            0x1 | 0x5 => {
                self.switches.lc_bank2 = true;
                self.switches.lc_read_enable = false;
                if self.switches.lc_prewrite { self.switches.lc_write_enable = true; }
                self.switches.lc_prewrite = !self.switches.lc_prewrite;
            }
            0x2 | 0x6 => {
                self.switches.lc_bank2 = true;
                self.switches.lc_read_enable = false;
                self.switches.lc_write_enable = false;
                self.switches.lc_prewrite = false;
            }
            0x3 | 0x7 => {
                self.switches.lc_bank2 = true;
                self.switches.lc_read_enable = true;
                if self.switches.lc_prewrite { self.switches.lc_write_enable = true; }
                self.switches.lc_prewrite = !self.switches.lc_prewrite;
            }
            0x8 | 0xC => {
                self.switches.lc_bank2 = false;
                self.switches.lc_read_enable = true;
                self.switches.lc_write_enable = false;
                self.switches.lc_prewrite = false;
            }
            0x9 | 0xD => {
                self.switches.lc_bank2 = false;
                self.switches.lc_read_enable = false;
                if self.switches.lc_prewrite { self.switches.lc_write_enable = true; }
                self.switches.lc_prewrite = !self.switches.lc_prewrite;
            }
            0xA | 0xE => {
                self.switches.lc_bank2 = false;
                self.switches.lc_read_enable = false;
                self.switches.lc_write_enable = false;
                self.switches.lc_prewrite = false;
            }
            0xB | 0xF => {
                self.switches.lc_bank2 = false;
                self.switches.lc_read_enable = true;
                if self.switches.lc_prewrite { self.switches.lc_write_enable = true; }
                self.switches.lc_prewrite = !self.switches.lc_prewrite;
            }
            _ => {}
        }
        0x00
    }
}

impl MemoryBus for Memory {
    fn read(&mut self, address: u16) -> u8 {
        match address {
            0x0000..=0x01FF => {
                if self.is_iie() && self.switches.altzp {
                    self.aux_ram[address as usize]
                } else {
                    self.main_ram[address as usize]
                }
            }
            0x0200..=0xBFFF => {
                if self.is_iie() && self.switches.ramrd {
                    self.aux_ram[address as usize]
                } else {
                    self.main_ram[address as usize]
                }
            }
            0xC000..=0xC0FF => self.read_soft_switch(address),
            0xC100..=0xC7FF => {
                // スロットROM領域 - ROMから読み取り
                if !self.rom.is_empty() && self.rom.len() >= 16384 {
                    // 16KB ROMの場合、$C100は offset 0x0100
                    let offset = (address - 0xC000) as usize;
                    self.rom[offset]
                } else {
                    // スロット個別のROMを使用
                    let slot = ((address - 0xC100) / 256) as usize;
                    let offset = (address & 0xFF) as usize;
                    self.slot_rom[slot][offset]
                }
            }
            0xC800..=0xCFFF => {
                // 拡張スロットROM領域
                if !self.rom.is_empty() && self.rom.len() >= 16384 {
                    let offset = (address - 0xC000) as usize;
                    self.rom[offset]
                } else {
                    0x00
                }
            }
            0xD000..=0xDFFF => {
                if self.switches.lc_read_enable {
                    if self.switches.lc_bank2 {
                        self.lc_ram_bank2[(address - 0xD000) as usize]
                    } else {
                        self.lc_ram[(address - 0xD000) as usize]
                    }
                } else if !self.rom.is_empty() {
                    let offset = (address - 0xC000) as usize;
                    if offset < self.rom.len() { self.rom[offset] } else { 0xFF }
                } else { 0xFF }
            }
            0xE000..=0xFFFF => {
                if self.switches.lc_read_enable {
                    self.lc_ram[(address - 0xD000) as usize]
                } else if !self.rom.is_empty() {
                    let offset = (address - 0xC000) as usize;
                    if offset < self.rom.len() { self.rom[offset] } else { 0xFF }
                } else { 0xFF }
            }
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x01FF => {
                if self.is_iie() && self.switches.altzp {
                    self.aux_ram[address as usize] = value;
                } else {
                    self.main_ram[address as usize] = value;
                }
            }
            0x0200..=0xBFFF => {
                if self.is_iie() && self.switches.ramwrt {
                    self.aux_ram[address as usize] = value;
                } else {
                    self.main_ram[address as usize] = value;
                }
            }
            0xC000..=0xC0FF => self.write_soft_switch(address, value),
            0xC100..=0xCFFF => {}
            0xD000..=0xDFFF => {
                if self.switches.lc_write_enable {
                    if self.switches.lc_bank2 {
                        self.lc_ram_bank2[(address - 0xD000) as usize] = value;
                    } else {
                        self.lc_ram[(address - 0xD000) as usize] = value;
                    }
                }
            }
            0xE000..=0xFFFF => {
                if self.switches.lc_write_enable {
                    self.lc_ram[(address - 0xD000) as usize] = value;
                }
            }
        }
    }
}
