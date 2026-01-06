//! Apple II ビデオエミュレーション
//! 
//! テキスト、Lo-Res、Hi-Res各モードのレンダリング

use crate::memory::Memory;

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

/// Hi-Resカラー（モノクロ緑）
pub const HIRES_GREEN: u32 = 0x33FF33;
#[allow(dead_code)]
pub const HIRES_BLACK: u32 = 0x000000;

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
    /// Apple IIの文字ROMは128文字 x 8バイト = 1024バイト
    /// $00-$3F: 大文字・数字・記号
    /// $40-$5F: 小文字 (Apple IIe)
    fn init_char_rom(&mut self) {
        // Apple II標準文字セット（$00-$3F → 64文字：大文字・記号）
        let font_upper: [[u8; 8]; 64] = [
            // $00: @
            [0x1C, 0x22, 0x2A, 0x2E, 0x2C, 0x20, 0x1E, 0x00],
            // $01: A
            [0x08, 0x14, 0x22, 0x22, 0x3E, 0x22, 0x22, 0x00],
            // $02: B
            [0x3C, 0x22, 0x22, 0x3C, 0x22, 0x22, 0x3C, 0x00],
            // $03: C
            [0x1C, 0x22, 0x20, 0x20, 0x20, 0x22, 0x1C, 0x00],
            // $04: D
            [0x3C, 0x22, 0x22, 0x22, 0x22, 0x22, 0x3C, 0x00],
            // $05: E
            [0x3E, 0x20, 0x20, 0x3C, 0x20, 0x20, 0x3E, 0x00],
            // $06: F
            [0x3E, 0x20, 0x20, 0x3C, 0x20, 0x20, 0x20, 0x00],
            // $07: G
            [0x1E, 0x20, 0x20, 0x2E, 0x22, 0x22, 0x1E, 0x00],
            // $08: H
            [0x22, 0x22, 0x22, 0x3E, 0x22, 0x22, 0x22, 0x00],
            // $09: I
            [0x1C, 0x08, 0x08, 0x08, 0x08, 0x08, 0x1C, 0x00],
            // $0A: J
            [0x02, 0x02, 0x02, 0x02, 0x02, 0x22, 0x1C, 0x00],
            // $0B: K
            [0x22, 0x24, 0x28, 0x30, 0x28, 0x24, 0x22, 0x00],
            // $0C: L
            [0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x3E, 0x00],
            // $0D: M
            [0x22, 0x36, 0x2A, 0x2A, 0x22, 0x22, 0x22, 0x00],
            // $0E: N
            [0x22, 0x32, 0x2A, 0x26, 0x22, 0x22, 0x22, 0x00],
            // $0F: O
            [0x1C, 0x22, 0x22, 0x22, 0x22, 0x22, 0x1C, 0x00],
            // $10: P
            [0x3C, 0x22, 0x22, 0x3C, 0x20, 0x20, 0x20, 0x00],
            // $11: Q
            [0x1C, 0x22, 0x22, 0x22, 0x2A, 0x24, 0x1A, 0x00],
            // $12: R
            [0x3C, 0x22, 0x22, 0x3C, 0x28, 0x24, 0x22, 0x00],
            // $13: S
            [0x1C, 0x22, 0x20, 0x1C, 0x02, 0x22, 0x1C, 0x00],
            // $14: T
            [0x3E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x00],
            // $15: U
            [0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x1C, 0x00],
            // $16: V
            [0x22, 0x22, 0x22, 0x22, 0x14, 0x14, 0x08, 0x00],
            // $17: W
            [0x22, 0x22, 0x22, 0x2A, 0x2A, 0x36, 0x22, 0x00],
            // $18: X
            [0x22, 0x22, 0x14, 0x08, 0x14, 0x22, 0x22, 0x00],
            // $19: Y
            [0x22, 0x22, 0x14, 0x08, 0x08, 0x08, 0x08, 0x00],
            // $1A: Z
            [0x3E, 0x02, 0x04, 0x08, 0x10, 0x20, 0x3E, 0x00],
            // $1B: [
            [0x1E, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1E, 0x00],
            // $1C: \
            [0x00, 0x20, 0x10, 0x08, 0x04, 0x02, 0x00, 0x00],
            // $1D: ]
            [0x1E, 0x02, 0x02, 0x02, 0x02, 0x02, 0x1E, 0x00],
            // $1E: ^
            [0x08, 0x14, 0x22, 0x00, 0x00, 0x00, 0x00, 0x00],
            // $1F: _
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3F, 0x00],
            // $20: Space
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            // $21: !
            [0x08, 0x08, 0x08, 0x08, 0x08, 0x00, 0x08, 0x00],
            // $22: "
            [0x14, 0x14, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00],
            // $23: #
            [0x14, 0x14, 0x3E, 0x14, 0x3E, 0x14, 0x14, 0x00],
            // $24: $
            [0x08, 0x1E, 0x28, 0x1C, 0x0A, 0x3C, 0x08, 0x00],
            // $25: %
            [0x30, 0x32, 0x04, 0x08, 0x10, 0x26, 0x06, 0x00],
            // $26: &
            [0x10, 0x28, 0x28, 0x10, 0x2A, 0x24, 0x1A, 0x00],
            // $27: '
            [0x08, 0x08, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00],
            // $28: (
            [0x04, 0x08, 0x10, 0x10, 0x10, 0x08, 0x04, 0x00],
            // $29: )
            [0x10, 0x08, 0x04, 0x04, 0x04, 0x08, 0x10, 0x00],
            // $2A: *
            [0x00, 0x08, 0x2A, 0x1C, 0x2A, 0x08, 0x00, 0x00],
            // $2B: +
            [0x00, 0x08, 0x08, 0x3E, 0x08, 0x08, 0x00, 0x00],
            // $2C: ,
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x08, 0x10],
            // $2D: -
            [0x00, 0x00, 0x00, 0x3E, 0x00, 0x00, 0x00, 0x00],
            // $2E: .
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00],
            // $2F: /
            [0x00, 0x02, 0x04, 0x08, 0x10, 0x20, 0x00, 0x00],
            // $30: 0
            [0x1C, 0x22, 0x26, 0x2A, 0x32, 0x22, 0x1C, 0x00],
            // $31: 1
            [0x08, 0x18, 0x08, 0x08, 0x08, 0x08, 0x1C, 0x00],
            // $32: 2
            [0x1C, 0x22, 0x02, 0x0C, 0x10, 0x20, 0x3E, 0x00],
            // $33: 3
            [0x1C, 0x22, 0x02, 0x0C, 0x02, 0x22, 0x1C, 0x00],
            // $34: 4
            [0x04, 0x0C, 0x14, 0x24, 0x3E, 0x04, 0x04, 0x00],
            // $35: 5
            [0x3E, 0x20, 0x3C, 0x02, 0x02, 0x22, 0x1C, 0x00],
            // $36: 6
            [0x0E, 0x10, 0x20, 0x3C, 0x22, 0x22, 0x1C, 0x00],
            // $37: 7
            [0x3E, 0x02, 0x04, 0x08, 0x10, 0x10, 0x10, 0x00],
            // $38: 8
            [0x1C, 0x22, 0x22, 0x1C, 0x22, 0x22, 0x1C, 0x00],
            // $39: 9
            [0x1C, 0x22, 0x22, 0x1E, 0x02, 0x04, 0x38, 0x00],
            // $3A: :
            [0x00, 0x00, 0x08, 0x00, 0x00, 0x08, 0x00, 0x00],
            // $3B: ;
            [0x00, 0x00, 0x08, 0x00, 0x00, 0x08, 0x08, 0x10],
            // $3C: <
            [0x04, 0x08, 0x10, 0x20, 0x10, 0x08, 0x04, 0x00],
            // $3D: =
            [0x00, 0x00, 0x3E, 0x00, 0x3E, 0x00, 0x00, 0x00],
            // $3E: >
            [0x10, 0x08, 0x04, 0x02, 0x04, 0x08, 0x10, 0x00],
            // $3F: ?
            [0x1C, 0x22, 0x02, 0x04, 0x08, 0x00, 0x08, 0x00],
        ];

        // 小文字フォント（$40-$5F → 32文字）
        // Apple IIeの小文字は$60-$7Fにマップされるが、
        // 画面コード$E0-$FFの下位5ビットで参照される
        let font_lower: [[u8; 8]; 32] = [
            // $40: ` (grave accent) - 小文字セットの先頭
            [0x10, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            // $41: a
            [0x00, 0x00, 0x1C, 0x02, 0x1E, 0x22, 0x1E, 0x00],
            // $42: b
            [0x20, 0x20, 0x3C, 0x22, 0x22, 0x22, 0x3C, 0x00],
            // $43: c
            [0x00, 0x00, 0x1C, 0x20, 0x20, 0x20, 0x1C, 0x00],
            // $44: d
            [0x02, 0x02, 0x1E, 0x22, 0x22, 0x22, 0x1E, 0x00],
            // $45: e
            [0x00, 0x00, 0x1C, 0x22, 0x3E, 0x20, 0x1C, 0x00],
            // $46: f
            [0x0C, 0x10, 0x10, 0x3C, 0x10, 0x10, 0x10, 0x00],
            // $47: g
            [0x00, 0x00, 0x1E, 0x22, 0x22, 0x1E, 0x02, 0x1C],
            // $48: h
            [0x20, 0x20, 0x3C, 0x22, 0x22, 0x22, 0x22, 0x00],
            // $49: i
            [0x08, 0x00, 0x18, 0x08, 0x08, 0x08, 0x1C, 0x00],
            // $4A: j
            [0x04, 0x00, 0x04, 0x04, 0x04, 0x04, 0x24, 0x18],
            // $4B: k
            [0x20, 0x20, 0x24, 0x28, 0x30, 0x28, 0x24, 0x00],
            // $4C: l
            [0x18, 0x08, 0x08, 0x08, 0x08, 0x08, 0x1C, 0x00],
            // $4D: m
            [0x00, 0x00, 0x36, 0x2A, 0x2A, 0x2A, 0x22, 0x00],
            // $4E: n
            [0x00, 0x00, 0x3C, 0x22, 0x22, 0x22, 0x22, 0x00],
            // $4F: o
            [0x00, 0x00, 0x1C, 0x22, 0x22, 0x22, 0x1C, 0x00],
            // $50: p
            [0x00, 0x00, 0x3C, 0x22, 0x22, 0x3C, 0x20, 0x20],
            // $51: q
            [0x00, 0x00, 0x1E, 0x22, 0x22, 0x1E, 0x02, 0x02],
            // $52: r
            [0x00, 0x00, 0x2C, 0x32, 0x20, 0x20, 0x20, 0x00],
            // $53: s
            [0x00, 0x00, 0x1E, 0x20, 0x1C, 0x02, 0x3C, 0x00],
            // $54: t
            [0x10, 0x10, 0x3C, 0x10, 0x10, 0x10, 0x0C, 0x00],
            // $55: u
            [0x00, 0x00, 0x22, 0x22, 0x22, 0x22, 0x1E, 0x00],
            // $56: v
            [0x00, 0x00, 0x22, 0x22, 0x22, 0x14, 0x08, 0x00],
            // $57: w
            [0x00, 0x00, 0x22, 0x2A, 0x2A, 0x2A, 0x14, 0x00],
            // $58: x
            [0x00, 0x00, 0x22, 0x14, 0x08, 0x14, 0x22, 0x00],
            // $59: y
            [0x00, 0x00, 0x22, 0x22, 0x22, 0x1E, 0x02, 0x1C],
            // $5A: z
            [0x00, 0x00, 0x3E, 0x04, 0x08, 0x10, 0x3E, 0x00],
            // $5B: {
            [0x04, 0x08, 0x08, 0x10, 0x08, 0x08, 0x04, 0x00],
            // $5C: |
            [0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x00],
            // $5D: }
            [0x10, 0x08, 0x08, 0x04, 0x08, 0x08, 0x10, 0x00],
            // $5E: ~
            [0x00, 0x00, 0x10, 0x2A, 0x04, 0x00, 0x00, 0x00],
            // $5F: (DEL/block)
            [0x3E, 0x3E, 0x3E, 0x3E, 0x3E, 0x3E, 0x3E, 0x00],
        ];

        // 文字ROMに書き込み
        // $00-$3Fの64文字（大文字・記号）
        for (idx, char_data) in font_upper.iter().enumerate() {
            for (row, &byte) in char_data.iter().enumerate() {
                self.char_rom[idx * 8 + row] = byte;
            }
        }
        
        // $40-$5Fの32文字（小文字）
        for (idx, char_data) in font_lower.iter().enumerate() {
            for (row, &byte) in char_data.iter().enumerate() {
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
        
        // Hi-Res color lookup table
        // NTSC artifact colors based on horizontal pixel position and palette bit
        // Index: 0=black, 1=purple, 2=green, 3=green, 4=purple,
        //        5=blue, 6=orange, 7=orange, 8=blue, 9=white
        let hires_colors: [u32; 10] = [
            COLORS[0],  // 0: Black
            COLORS[3],  // 1: Purple
            COLORS[12], // 2: Green
            COLORS[12], // 3: Green
            COLORS[3],  // 4: Purple
            COLORS[6],  // 5: Blue
            COLORS[9],  // 6: Orange
            COLORS[9],  // 7: Orange
            COLORS[6],  // 8: Blue
            COLORS[15], // 9: White
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
                
                // last 2 pixels, current 7 pixels, next 2 pixels
                let run: u16 = ((b0 as u16 & 0x60) >> 5) |
                              ((b1 as u16 & 0x7f) << 2) |
                              ((b2 as u16 & 0x03) << 9);
                
                let odd = ((x & 1) << 1) as usize;
                let offset = ((b1 & 0x80) >> 5) as usize;
                
                for i in 0..7 {
                    let left = (run >> (1 + i)) & 1;
                    let pixel = (run >> (2 + i)) & 1;
                    let right = (run >> (3 + i)) & 1;
                    
                    let idx = if self.monochrome {
                        if pixel != 0 { 9 } else { 0 }
                    } else {
                        if pixel != 0 {
                            if left != 0 || right != 0 {
                                9 // white
                            } else {
                                offset + odd + (i & 1) + 1
                            }
                        } else {
                            if left != 0 && right != 0 {
                                offset + odd + 1 - (i & 1) + 1
                            } else {
                                0 // black
                            }
                        }
                    };
                    
                    let color = if self.monochrome && idx == 9 {
                        self.mono_color
                    } else {
                        hires_colors[idx]
                    };
                    
                    let screen_x = (x as usize * 14 + i * 2) as usize;
                    let screen_y = (y * 2) as usize;
                    
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
            
            // 各行は80バイト（Aux 40バイト + Main 40バイト が交互）
            for byte_x in 0..40 {
                // Aux RAM のバイト（偶数バイト位置）
                let aux_byte = memory.aux_ram[(row_addr + byte_x) as usize];
                // Main RAM のバイト（奇数バイト位置）
                let main_byte = memory.main_ram[(row_addr + byte_x) as usize];
                
                // 2バイト（14ピクセル分、各7ビット）から4ピクセルを抽出
                // ダブルHi-Resは4ビット/ピクセル
                // Aux[6:0] + Main[6:0] = 14ビット → 3.5ピクセル(4ビット*3 + 2ビット余り)
                // 実際は連続する28ビット（4バイト）から7ピクセルを生成
                
                // 簡略化: 各バイトの7ビットを14ピクセル分として描画
                let combined = ((main_byte as u16 & 0x7F) << 7) | (aux_byte as u16 & 0x7F);
                
                // 14ピクセル分を処理
                for bit in 0..14 {
                    let screen_x = byte_x as usize * 14 + bit;
                    let screen_y = y * 2;
                    
                    // 4ビットカラーを近似的に計算
                    // 実際のDHIRESは4ビット連続でカラーを決定
                    let nibble_pos = bit / 4;
                    let nibble = if nibble_pos == 0 {
                        aux_byte & 0x0F
                    } else if nibble_pos == 1 {
                        ((aux_byte >> 4) & 0x07) | ((main_byte & 0x01) << 3)
                    } else if nibble_pos == 2 {
                        (main_byte >> 1) & 0x0F
                    } else {
                        (main_byte >> 5) & 0x07
                    };
                    
                    // ピクセルがオンかどうか
                    let pixel_on = ((combined >> bit) & 1) != 0;
                    
                    let color = if self.monochrome {
                        if pixel_on { self.mono_color } else { 0x000000 }
                    } else {
                        // DHIRESの16色パレット
                        COLORS[nibble as usize & 0x0F]
                    };
                    
                    if screen_x < SCREEN_WIDTH && screen_y + 1 < SCREEN_HEIGHT {
                        let fb_idx = screen_y * SCREEN_WIDTH + screen_x;
                        self.framebuffer[fb_idx] = color;
                        self.framebuffer[fb_idx + SCREEN_WIDTH] = color;
                    }
                }
            }
        }
    }
}
