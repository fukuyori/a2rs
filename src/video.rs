//! Apple II ビデオエミュレーション
//! 
//! テキスト、Lo-Res、Hi-Res各モードのレンダリング

use crate::memory::Memory;
use crate::gui::get_char_pattern;

/// 画面サイズ
pub const SCREEN_WIDTH: usize = 560;  // 280 * 2 for double width
pub const SCREEN_HEIGHT: usize = 384; // 192 * 2 for double height

/// Apple IIのカラーパレット（NTSC artifact colors）
/// Based on NTSC color artifact specifications
pub const COLORS: [u32; 16] = [
    0x000000, // 0: Black
    0xDD0033, // 1: Magenta
    0x604EBD, // 2: Dark Blue
    0xFF44FD, // 3: Purple (NTSC artifact)
    0x00A360, // 4: Dark Green
    0x9C9C9C, // 5: Gray 1
    0x14CFFD, // 6: Medium Blue (NTSC artifact - cyan-ish)
    0xD0C3FF, // 7: Light Blue
    0x607203, // 8: Brown
    0xFF6A3C, // 9: Orange (NTSC artifact)
    0x9C9C9C, // 10: Gray 2
    0xFFA0D0, // 11: Pink
    0x14F53C, // 12: Light Green (NTSC artifact)
    0xD0DD8D, // 13: Yellow
    0x72FFD0, // 14: Aqua
    0xFFFFFF, // 15: White
];

/// AppleWin/Feline-style Double Hi-Res palette in lo-res color order.
const DHGR_COLORS: [u32; 16] = [
    0x000000, // 0: Black
    0xAC124C, // 1: Deep red
    0x000783, // 2: Dark blue
    0xAA1AD1, // 3: Magenta
    0x00832F, // 4: Dark green
    0x9F977E, // 5: Dark gray
    0x008AB5, // 6: Blue
    0x9F9EFF, // 7: Light blue
    0x7A5F00, // 8: Brown
    0xFF7247, // 9: Orange
    0x78687F, // 10: Light gray
    0xFF7ACF, // 11: Pink
    0x6FE62C, // 12: Green
    0xFFF67B, // 13: Yellow
    0x6CEEB2, // 14: Aqua
    0xFFFFFF, // 15: White
];

/// Hi-Resカラー（モノクロ緑）
pub const HIRES_GREEN: u32 = 0x33FF33;
#[allow(dead_code)]
pub const HIRES_BLACK: u32 = 0x000000;

fn compact_glyph_6x10_to_7x8(ch: char) -> [u8; 8] {
    if ch == '\u{7f}' {
        return [0x7F; 8];
    }

    let source = get_char_pattern(ch);
    let mut out = [0u8; 8];
    for row in 0..8 {
        out[row] = (source[row] & 0x3F) << 1;
    }
    out
}

/// ビデオモード
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum VideoMode {
    Text40,
    Text80,
    LoRes,
    HiRes,
    DoubleLoRes,
    DoubleHiRes,
}


/// Tick-driven video timing state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoFetchPhase {
    /// Display memory is actively being fetched for the current scanline.
    VisibleByte { column: u8 },
    /// Horizontal blank / housekeeping period between visible fetches.
    HBlank,
    /// Vertical blanking interval.
    VBlank,
}

/// Cycle-accurate video beam position snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoPosition {
    pub frame: u64,
    pub scanline: u16,
    pub scanline_cycle: u8,
    pub hblank: bool,
    pub vblank: bool,
    pub visible_row: Option<usize>,
    pub visible_column: Option<usize>,
    pub fetch_phase: VideoFetchPhase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoTimingDiagnostics {
    pub frame: u64,
    pub scanline: u16,
    pub scanline_cycle: u8,
    pub cycle_in_frame: u32,
    pub hblank: bool,
    pub vblank: bool,
    pub floating_bus_address: u16,
    pub floating_bus_value: u8,
}

impl VideoTimingDiagnostics {
    pub fn from_position(position: VideoPosition, floating_bus_address: u16, floating_bus_value: u8) -> Self {
        Self {
            frame: position.frame,
            scanline: position.scanline,
            scanline_cycle: position.scanline_cycle,
            cycle_in_frame: (position.scanline as u32) * (VideoScanner::CYCLES_PER_SCANLINE as u32)
                + (position.scanline_cycle as u32),
            hblank: position.hblank,
            vblank: position.vblank,
            floating_bus_address,
            floating_bus_value,
        }
    }

    pub fn lines(&self) -> Vec<String> {
        vec![
            "VIDEO TIMING".to_string(),
            format!("frame {:>6}  line {:>3}  cyc {:>2}", self.frame, self.scanline, self.scanline_cycle),
            format!("frame_cycle {:>5}  hblank={}  vblank={}", self.cycle_in_frame, self.hblank as u8, self.vblank as u8),
            format!("floating bus: ${:04X} = ${:02X}", self.floating_bus_address, self.floating_bus_value),
        ]
    }
}

pub struct VideoScanner {
    /// Current cycle position within a scanline (0..64).
    pub scanline_cycle: u8,
    /// Current scanline within a frame (0..261).
    pub scanline: u16,
    /// Completed frame count.
    pub frame: u64,
}

impl Default for VideoScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoScanner {
    pub const CYCLES_PER_SCANLINE: u8 = 65;
    pub const SCANLINES_PER_FRAME: u16 = 262;
    pub const VISIBLE_SCANLINES: u16 = 192;
    pub const VISIBLE_COLUMNS: u8 = 40;
    pub const HBLANK_COLUMNS: u8 = Self::CYCLES_PER_SCANLINE - Self::VISIBLE_COLUMNS;

    pub const fn new() -> Self {
        Self {
            scanline_cycle: 0,
            scanline: 0,
            frame: 0,
        }
    }

    /// Advance the scanner by one CPU cycle.
    /// Returns true when a new frame starts.
    pub fn tick(&mut self) -> bool {
        self.scanline_cycle = self.scanline_cycle.wrapping_add(1);
        if self.scanline_cycle >= Self::CYCLES_PER_SCANLINE {
            self.scanline_cycle = 0;
            self.scanline += 1;
            if self.scanline >= Self::SCANLINES_PER_FRAME {
                self.scanline = 0;
                self.frame = self.frame.saturating_add(1);
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn in_vblank(&self) -> bool {
        self.scanline >= Self::VISIBLE_SCANLINES
    }

    #[inline]
    pub fn visible_row(&self) -> Option<usize> {
        if self.in_vblank() {
            None
        } else {
            Some(self.scanline as usize)
        }
    }

    #[inline]
    pub fn current_fetch_phase(&self) -> VideoFetchPhase {
        if self.in_vblank() {
            VideoFetchPhase::VBlank
        } else if self.scanline_cycle < Self::VISIBLE_COLUMNS {
            VideoFetchPhase::VisibleByte {
                column: self.scanline_cycle,
            }
        } else {
            VideoFetchPhase::HBlank
        }
    }

    #[inline]
    pub fn current_visible_column(&self) -> Option<usize> {
        match self.current_fetch_phase() {
            VideoFetchPhase::VisibleByte { column } => Some(column as usize),
            VideoFetchPhase::HBlank | VideoFetchPhase::VBlank => None,
        }
    }

    #[inline]
    pub fn in_hblank(&self) -> bool {
        !self.in_vblank() && self.scanline_cycle >= Self::VISIBLE_COLUMNS
    }

    #[inline]
    pub fn position(&self) -> VideoPosition {
        let fetch_phase = self.current_fetch_phase();
        VideoPosition {
            frame: self.frame,
            scanline: self.scanline,
            scanline_cycle: self.scanline_cycle,
            hblank: matches!(fetch_phase, VideoFetchPhase::HBlank),
            vblank: matches!(fetch_phase, VideoFetchPhase::VBlank),
            visible_row: self.visible_row(),
            visible_column: self.current_visible_column(),
            fetch_phase,
        }
    }

    #[inline]
    pub fn total_cycles_in_frame(&self) -> u32 {
        (self.scanline as u32) * (Self::CYCLES_PER_SCANLINE as u32) + (self.scanline_cycle as u32)
    }
}

/// ビデオエミュレータ
pub struct Video {
    /// フレームバッファ (ARGB形式)
    pub framebuffer: Vec<u32>,
    /// 文字ROM（フォントデータ）
    pub char_rom: [u8; 2048],
    /// モノクロモード
    pub monochrome: bool,
    /// モノクロ色
    pub mono_color: u32,
    /// 点滅状態
    pub flash_state: bool,
    /// 点滅カウンター
    pub flash_counter: u32,
}

impl Default for Video {
    fn default() -> Self {
        Self::new()
    }
}

impl Video {
    pub fn new() -> Self {
        let mut video = Video {
            framebuffer: vec![0; SCREEN_WIDTH * SCREEN_HEIGHT],
            char_rom: [0; 2048],
            monochrome: false,
            mono_color: HIRES_GREEN,
            flash_state: false,
            flash_counter: 0,
        };
        video.init_char_rom();
        video
    }

    /// 外部文字ROMをロード（Apple IIe 32KB ROMから抽出した場合など）
    #[allow(dead_code)]
    pub fn load_char_rom(&mut self, data: &[u8]) {
        if data.len() >= 2048 {
            for i in 0..2048 {
                self.char_rom[i] = data[i];
            }
            log::info!("Loaded external character ROM");
        }
    }

    /// 32KB Apple IIe ROMから文字ROMを抽出してロード
    /// 注意: 一般的な32KB Apple IIe ROMには文字ROMが含まれていない場合が多い
    /// 文字ROMは別ファイル（char_set.romなど）で提供されることが多い
    pub fn load_char_rom_from_iie_rom(&mut self, rom_data: &[u8]) {
        if rom_data.len() == 32768 {
            // Apple IIe 32KB ROMの$0000-$07FFを確認
            // ただし、この領域にはDisk II Boot ROMなど他のデータが入っていることが多い
            // 文字ROMかどうかを判定するため、典型的なパターンをチェック
            
            // 文字ROMの典型的なパターン: 
            // - 各文字は8バイト
            // - 文字'@'(index 0)の典型的なパターンは特定のビットパターン
            // - 文字ROMの場合、最初の数バイトは特定のパターンになる
            
            // Disk II Boot ROMの典型的な先頭: $A2 $20 (LDX #$20)
            // これは文字ROMではない
            if rom_data[0] == 0xA2 && rom_data[1] == 0x20 {
                log::info!("$0000-$07FF contains Disk II Boot ROM, not character ROM");
                return;
            }
            
            // その他の非文字ROMパターンをスキップ
            // 文字ROMの場合、特定のパターンがあるはず
            // ここでは内蔵フォントを使用するため、何もしない
            log::info!("Using built-in character ROM for Apple IIe");
        }
    }

    /// デフォルトの文字ROMを初期化
    /// 既定の内蔵フォントを文字ROM形式に変換して初期化
    fn init_char_rom(&mut self) {
        const UPPER_CHARS: [char; 64] = [
            '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G',
            'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
            'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W',
            'X', 'Y', 'Z', '[', '\\', ']', '^', '_',
            ' ', '!', '"', '#', '$', '%', '&', '\'',
            '(', ')', '*', '+', ',', '-', '.', '/',
            '0', '1', '2', '3', '4', '5', '6', '7',
            '8', '9', ':', ';', '<', '=', '>', '?',
        ];
        const LOWER_CHARS: [char; 32] = [
            '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g',
            'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o',
            'p', 'q', 'r', 's', 't', 'u', 'v', 'w',
            'x', 'y', 'z', '{', '|', '}', '~', '\u{7f}',
        ];

        for (idx, ch) in UPPER_CHARS.iter().enumerate() {
            let glyph = compact_glyph_6x10_to_7x8(*ch);
            for (row, &byte) in glyph.iter().enumerate() {
                self.char_rom[idx * 8 + row] = byte;
            }
        }

        for (idx, ch) in LOWER_CHARS.iter().enumerate() {
            let glyph = compact_glyph_6x10_to_7x8(*ch);
            for (row, &byte) in glyph.iter().enumerate() {
                self.char_rom[(idx + 64) * 8 + row] = byte;
            }
        }
    }

    /// 画面を更新
    pub fn render(&mut self, memory: &Memory) {
        // 点滅カウンターを更新（約4Hzで点滅）
        self.flash_counter += 1;
        if self.flash_counter >= 15 {  // 60fps / 4 = 15フレーム
            self.flash_state = !self.flash_state;
            self.flash_counter = 0;
        }
        
        // 画面をクリア
        for pixel in self.framebuffer.iter_mut() {
            *pixel = 0x000000;
        }

        if memory.switches.text_mode {
            if memory.switches.col_80 && memory.is_iie() {
                self.render_text_80(memory);
            } else {
                self.render_text(memory);
            }
        } else if memory.switches.hires {
            if memory.switches.dhires && memory.switches.col_80 && memory.is_iie() {
                self.render_dhires(memory);
            } else {
                self.render_hires(memory);
            }
            if memory.switches.mixed_mode {
                if memory.switches.col_80 && memory.is_iie() {
                    self.render_text_80_bottom(memory);
                } else {
                    self.render_text_bottom(memory);
                }
            }
        } else {
            self.render_lores(memory);
            if memory.switches.mixed_mode {
                if memory.switches.col_80 && memory.is_iie() {
                    self.render_text_80_bottom(memory);
                } else {
                    self.render_text_bottom(memory);
                }
            }
        }
    }

    /// テキストモードのレンダリング（40桁）
    fn render_text(&mut self, memory: &Memory) {
        let base = if memory.switches.page2 { 0x0800 } else { 0x0400 };
        
        for row in 0..24 {
            let row_addr = base + Self::text_row_offset(row);
            for col in 0..40 {
                let ch = memory.main_ram[(row_addr + col) as usize];
                self.draw_char(col as usize, row as usize, ch);
            }
        }
    }

    /// テキストモード下部4行（mixedモード用）
    fn render_text_bottom(&mut self, memory: &Memory) {
        let base = if memory.switches.page2 { 0x0800 } else { 0x0400 };
        
        for row in 20..24 {
            let row_addr = base + Self::text_row_offset(row);
            for col in 0..40 {
                let ch = memory.main_ram[(row_addr + col) as usize];
                self.draw_char(col as usize, row as usize, ch);
            }
        }
    }

    /// テキスト行のメモリオフセットを計算
    /// Apple IIのテキスト画面は特殊なインターリーブ構造
    /// 行0-7:   $400, $480, $500, $580, $600, $680, $700, $780
    /// 行8-15:  $428, $4A8, $528, $5A8, $628, $6A8, $728, $7A8  
    /// 行16-23: $450, $4D0, $550, $5D0, $650, $6D0, $750, $7D0
    fn text_row_offset(row: usize) -> usize {
        let group = row / 8;      // 0, 1, or 2
        let line = row % 8;       // 0-7
        group * 0x28 + line * 0x80
    }

    /// 1文字を描画
    /// Apple IIの文字コード:
    ///   $00-$3F: Inverse (反転表示) - 文字ROM $00-$3F (大文字・記号)
    ///   $40-$7F: Flash (点滅表示) - 文字ROM $00-$3F (大文字・記号)
    ///   $80-$BF: Normal - 文字ROM $00-$3F (大文字・記号)
    ///   $C0-$DF: Normal - 文字ROM $00-$3F (大文字・記号、$C0-$DFは$80-$9Fと同じ)
    ///   $E0-$FF: Normal - 文字ROM $40-$5F (小文字、Apple IIe)
    fn draw_char(&mut self, col: usize, row: usize, ch: u8) {
        // 上位2ビットでモードを判定
        let mode = ch >> 6;
        let inverse = mode == 0;  // $00-$3F
        let flash = mode == 1;    // $40-$7F
        // mode == 2 or 3: Normal ($80-$FF)
        
        // 文字ROMアドレスの計算
        // Apple IIeの小文字対応:
        // $E0-$FF → 小文字フォント ($40-$5F)
        let char_index = if ch >= 0xE0 {
            // 小文字: $E0-$FF → フォントの $40-$5F 部分を参照
            // 'a' ($E1) → $41, 'p' ($F0) → $50, etc.
            0x40 + (ch & 0x1F) as usize
        } else {
            // $00-$DF: 下位6ビットがそのままインデックス
            (ch & 0x3F) as usize
        };
        
        let font_offset = char_index * 8;
        
        let fg = if self.monochrome { self.mono_color } else { 0xFFFFFF };
        let bg = 0x000000;
        
        // 点滅処理
        let do_inverse = inverse || (flash && self.flash_state);
        
        for y in 0..8 {
            let font_byte = if font_offset + y < self.char_rom.len() {
                self.char_rom[font_offset + y]
            } else {
                0
            };
            
            // Apple II文字ROMはビット0が左端、ビット6が右端
            // 内蔵フォントはMSBファーストで作成されているため、
            // ここでビット順序を反転して描画
            for x in 0..7 {
                // MSBファーストのフォントデータをそのまま描画
                // ビット6から順に描画（左から右へ）
                let pixel_on = (font_byte & (0x40 >> x)) != 0;
                let color = if do_inverse {
                    if pixel_on { bg } else { fg }
                } else {
                    if pixel_on { fg } else { bg }
                };
                
                let screen_x = col * 14 + x * 2;
                let screen_y = row * 16 + y * 2;
                
                if screen_x + 1 < SCREEN_WIDTH && screen_y + 1 < SCREEN_HEIGHT {
                    let idx = screen_y * SCREEN_WIDTH + screen_x;
                    self.framebuffer[idx] = color;
                    self.framebuffer[idx + 1] = color;
                    self.framebuffer[idx + SCREEN_WIDTH] = color;
                    self.framebuffer[idx + SCREEN_WIDTH + 1] = color;
                }
            }
        }
    }

    /// Lo-Resグラフィックスのレンダリング
    fn render_lores(&mut self, memory: &Memory) {
        let base = if memory.switches.page2 { 0x0800 } else { 0x0400 };
        let max_row = if memory.switches.mixed_mode { 20 } else { 24 };
        
        for row in 0..max_row {
            let row_addr = base + Self::text_row_offset(row);
            for col in 0..40 {
                let byte = memory.main_ram[(row_addr + col) as usize];
                let top_color = COLORS[(byte & 0x0F) as usize];
                let bottom_color = COLORS[(byte >> 4) as usize];
                
                self.draw_lores_block(col as usize, row as usize, top_color, bottom_color);
            }
        }
    }

    /// Lo-Resブロックを描画
    fn draw_lores_block(&mut self, col: usize, row: usize, top_color: u32, bottom_color: u32) {
        let x_start = col * 14;
        let y_start = row * 16;
        
        // 上半分（8ピクセル）
        for y in 0..8 {
            for x in 0..14 {
                if x_start + x < SCREEN_WIDTH && y_start + y < SCREEN_HEIGHT {
                    self.framebuffer[(y_start + y) * SCREEN_WIDTH + x_start + x] = top_color;
                }
            }
        }
        
        // 下半分（8ピクセル）
        for y in 8..16 {
            for x in 0..14 {
                if x_start + x < SCREEN_WIDTH && y_start + y < SCREEN_HEIGHT {
                    self.framebuffer[(y_start + y) * SCREEN_WIDTH + x_start + x] = bottom_color;
                }
            }
        }
    }

    /// Hi-Resグラフィックスのレンダリング
    fn render_hires(&mut self, memory: &Memory) {
        let base = if memory.switches.page2 { 0x4000 } else { 0x2000 };
        let max_row = if memory.switches.mixed_mode { 160 } else { 192 };
        
        let hires_colors: [u32; 10] = [
            COLORS[0],
            COLORS[3],
            COLORS[12],
            COLORS[12],
            COLORS[3],
            0x008AB5,
            0xFF7247,
            0xFF7247,
            0x008AB5,
            COLORS[15],
        ];

        for y in 0..max_row {
            let row_addr = base + Self::hires_row_offset(y);
            
            let mut b0: u8 = 0;
            let mut b1: u8 = memory.main_ram[row_addr as usize];
            
            for x in 0..40 {
                let b2: u8 = if x == 39 { 
                    0 
                } else { 
                    memory.main_ram[(row_addr + x + 1) as usize] 
                };
                
                let run: u16 = ((b0 as u16 & 0x60) >> 5)
                    | ((b1 as u16 & 0x7f) << 2)
                    | ((b2 as u16 & 0x03) << 9);
                
                let odd = ((x & 1) << 1) as usize;
                let offset = ((b1 & 0x80) >> 5) as usize;
                
                for i in 0..7 {
                    let left = (run >> (1 + i)) & 1;
                    let pixel = (run >> (2 + i)) & 1;
                    let right = (run >> (3 + i)) & 1;
                    
                    let idx = if self.monochrome {
                        if pixel != 0 { 9 } else { 0 }
                    } else if pixel != 0 {
                        if left != 0 || right != 0 {
                            9
                        } else {
                            offset + odd + (i & 1) + 1
                        }
                    } else if left != 0 && right != 0 {
                        offset + odd + 1 - (i & 1) + 1
                    } else {
                        0
                    };
                    
                    let color = if self.monochrome && idx == 9 {
                        self.mono_color
                    } else {
                        hires_colors[idx]
                    };
                    
                    let screen_x = x as usize * 14 + i * 2;
                    let screen_y = y * 2;
                    
                    if screen_x + 1 < SCREEN_WIDTH && screen_y + 1 < SCREEN_HEIGHT {
                        let fb_idx = screen_y * SCREEN_WIDTH + screen_x;
                        self.framebuffer[fb_idx] = color;
                        self.framebuffer[fb_idx + 1] = color;
                        self.framebuffer[fb_idx + SCREEN_WIDTH] = color;
                        self.framebuffer[fb_idx + SCREEN_WIDTH + 1] = color;
                    }
                }
                
                b0 = b1;
                b1 = b2;
            }
        }
    }

    /// Hi-Res行のメモリオフセットを計算
    fn hires_row_offset(row: usize) -> usize {
        let section = row / 64;
        let group = (row % 64) / 8;
        let line = row % 8;
        section * 0x28 + group * 0x80 + line * 0x400
    }
    
    /// 80桁テキストモードのレンダリング
    fn render_text_80(&mut self, memory: &Memory) {
        let base = if memory.switches.page2 && !memory.switches.store_80 { 
            0x0800 
        } else { 
            0x0400 
        };
        
        for row in 0..24 {
            let row_addr = base + Self::text_row_offset(row);
            for col in 0..80 {
                // 偶数列はAux RAM、奇数列はMain RAM
                let ch = if (col & 1) == 0 {
                    memory.aux_ram[(row_addr + col / 2) as usize]
                } else {
                    memory.main_ram[(row_addr + col / 2) as usize]
                };
                self.draw_char_80(col as usize, row as usize, ch);
            }
        }
    }
    
    /// 80桁テキストモード下部4行（mixedモード用）
    fn render_text_80_bottom(&mut self, memory: &Memory) {
        let base = if memory.switches.page2 && !memory.switches.store_80 { 
            0x0800 
        } else { 
            0x0400 
        };
        
        for row in 20..24 {
            let row_addr = base + Self::text_row_offset(row);
            for col in 0..80 {
                let ch = if (col & 1) == 0 {
                    memory.aux_ram[(row_addr + col / 2) as usize]
                } else {
                    memory.main_ram[(row_addr + col / 2) as usize]
                };
                self.draw_char_80(col as usize, row as usize, ch);
            }
        }
    }
    
    /// 80桁モード用文字描画（7x8ピクセル、半分の幅）
    fn draw_char_80(&mut self, col: usize, row: usize, ch: u8) {
        // 文字の属性を判定
        let (char_code, inverse, flash) = if ch < 0x40 {
            (ch + 0x40, true, false)
        } else if ch < 0x80 {
            (ch, false, true)
        } else if ch < 0xC0 {
            (ch - 0x40, true, false)
        } else {
            (ch - 0x40, false, false)
        };
        
        // 点滅中かつflash属性の場合は反転
        let should_invert = inverse || (flash && self.flash_state);
        
        // 文字ROMからフォントデータを取得
        let rom_idx = ((char_code as usize) & 0x3F) * 8;
        
        // 7x8ピクセルで描画（80桁モードは幅が半分）
        for char_row in 0..8 {
            let font_byte = if rom_idx + char_row < self.char_rom.len() {
                self.char_rom[rom_idx + char_row]
            } else {
                0
            };
            
            for char_col in 0..7 {
                let pixel_on = ((font_byte >> (6 - char_col)) & 1) != 0;
                let display_on = if should_invert { !pixel_on } else { pixel_on };
                
                // 80桁モードは1ピクセル幅（560ピクセル / 80桁 = 7ピクセル）
                let screen_x = col * 7 + char_col;
                // 縦は2倍
                let screen_y = row * 16 + char_row * 2;
                
                let color = if display_on { 0xFFFFFF } else { 0x000000 };
                
                if screen_x < SCREEN_WIDTH && screen_y + 1 < SCREEN_HEIGHT {
                    let fb_idx = screen_y * SCREEN_WIDTH + screen_x;
                    self.framebuffer[fb_idx] = color;
                    self.framebuffer[fb_idx + SCREEN_WIDTH] = color;
                }
            }
        }
    }
    
    /// ダブルHi-Resモードのレンダリング（560x192、16色）
    fn render_dhires(&mut self, memory: &Memory) {
        let base = if memory.switches.page2 && !memory.switches.store_80 {
            0x4000
        } else {
            0x2000
        };
        
        let max_row = if memory.switches.mixed_mode { 160 } else { 192 };

        for y in 0..max_row {
            let row_addr = base + Self::hires_row_offset(y);

            let mut bits = [0u8; SCREEN_WIDTH + 8];
            let mut color_mode = [false; SCREEN_WIDTH + 8];
            for byte_x in 0..40 {
                let aux_byte = memory.aux_ram[(row_addr + byte_x) as usize];
                let main_byte = memory.main_ram[(row_addr + byte_x) as usize];

                let bit_base = byte_x * 14;
                for bit in 0..7 {
                    bits[bit_base + bit] = (aux_byte >> bit) & 1;
                    color_mode[bit_base + bit] = (aux_byte & 0x80) == 0;
                    bits[bit_base + 7 + bit] = (main_byte >> bit) & 1;
                    color_mode[bit_base + 7 + bit] = (main_byte & 0x80) == 0;
                }
            }

            let screen_y = y * 2;
            if self.monochrome {
                for pixel_x in 0..SCREEN_WIDTH {
                    let color = if bits[pixel_x] != 0 {
                        self.mono_color
                    } else {
                        0x000000
                    };

                    if screen_y + 1 < SCREEN_HEIGHT {
                        let fb_idx = screen_y * SCREEN_WIDTH + pixel_x;
                        self.framebuffer[fb_idx] = color;
                        self.framebuffer[fb_idx + SCREEN_WIDTH] = color;
                    }
                }
            } else {
                for cell_x in 0..(SCREEN_WIDTH / 4) {
                    let pixel_x = cell_x * 4;
                    let cell_is_color = color_mode[pixel_x]
                        && color_mode[pixel_x + 1]
                        && color_mode[pixel_x + 2]
                        && color_mode[pixel_x + 3];
                    if !cell_is_color {
                        if screen_y + 1 < SCREEN_HEIGHT {
                            let fb_idx = screen_y * SCREEN_WIDTH + pixel_x;
                            for dx in 0..4 {
                                let color = if bits[pixel_x + dx] != 0 {
                                    0xFFFFFF
                                } else {
                                    0x000000
                                };
                                self.framebuffer[fb_idx + dx] = color;
                                self.framebuffer[fb_idx + SCREEN_WIDTH + dx] = color;
                            }
                        }
                        continue;
                    }

                    let raw = bits[pixel_x]
                        | (bits[pixel_x + 1] << 1)
                        | (bits[pixel_x + 2] << 2)
                        | (bits[pixel_x + 3] << 3);
                    let color_index = ((raw & 0x07) << 1) | ((raw & 0x08) >> 3);
                    let color = DHGR_COLORS[color_index as usize];

                    if screen_y + 1 < SCREEN_HEIGHT {
                        let fb_idx = screen_y * SCREEN_WIDTH + pixel_x;
                        for dx in 0..4 {
                            self.framebuffer[fb_idx + dx] = color;
                            self.framebuffer[fb_idx + SCREEN_WIDTH + dx] = color;
                        }
                    }
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::{VideoFetchPhase, VideoScanner};

    #[test]
    fn scanner_enters_hblank_after_visible_columns() {
        let mut scanner = VideoScanner::new();
        for _ in 0..VideoScanner::VISIBLE_COLUMNS {
            scanner.tick();
        }
        let pos = scanner.position();
        assert!(matches!(pos.fetch_phase, VideoFetchPhase::HBlank));
        assert!(pos.hblank);
        assert!(!pos.vblank);
    }

    #[test]
    fn scanner_enters_vblank_after_visible_scanlines() {
        let mut scanner = VideoScanner::new();
        for _ in 0..(VideoScanner::VISIBLE_SCANLINES as usize * VideoScanner::CYCLES_PER_SCANLINE as usize) {
            scanner.tick();
        }
        let pos = scanner.position();
        assert!(matches!(pos.fetch_phase, VideoFetchPhase::VBlank));
        assert!(pos.vblank);
        assert!(!pos.hblank);
    }

    #[test]
    fn scanner_wraps_to_next_frame_after_full_frame() {
        let mut scanner = VideoScanner::new();
        let total = VideoScanner::SCANLINES_PER_FRAME as usize * VideoScanner::CYCLES_PER_SCANLINE as usize;
        let mut new_frame = false;
        for _ in 0..total {
            new_frame = scanner.tick();
        }
        assert!(new_frame);
        let pos = scanner.position();
        assert_eq!(pos.frame, 1);
        assert_eq!(pos.scanline, 0);
        assert_eq!(pos.scanline_cycle, 0);
        assert!(matches!(pos.fetch_phase, VideoFetchPhase::VisibleByte { column: 0 }));
    }
}
