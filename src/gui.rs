//! A2RS GUI
//!
//! シンプルなGUIシステム - ツールバー、ステータスバー、オーバーレイメニュー

/// ツールバーの高さ
pub const TOOLBAR_HEIGHT: usize = 32;
/// ステータスバーの高さ
pub const STATUSBAR_HEIGHT: usize = 24;
/// アイコンサイズ
const ICON_SIZE: usize = 24;
/// アイコン間隔
const ICON_SPACING: usize = 8;

/// 色定義
const COLOR_TOOLBAR_BG: u32 = 0x2D2D2D;
const COLOR_STATUSBAR_BG: u32 = 0x1E1E1E;
const COLOR_ICON_NORMAL: u32 = 0xCCCCCC;
const COLOR_ICON_HOVER: u32 = 0xFFFFFF;
const COLOR_ICON_ACTIVE: u32 = 0x00FF88;
const COLOR_ICON_DISABLED: u32 = 0x666666;
const COLOR_TEXT: u32 = 0xCCCCCC;
const COLOR_TEXT_BRIGHT: u32 = 0xFFFFFF;
const COLOR_SEPARATOR: u32 = 0x444444;
#[allow(dead_code)]
const COLOR_OVERLAY_BG: u32 = 0xE0101020; // 半透明

/// ツールバーボタンID
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolbarButton {
    PlayPause,
    Reset,
    Disk1,
    Disk2,
    SwapDisks,
    QuickSave,
    QuickLoad,
    Screenshot,
    Fullscreen,
}

impl ToolbarButton {
    /// ボタンのアイコン文字を取得
    #[allow(dead_code)]
    fn icon(&self) -> &'static str {
        match self {
            ToolbarButton::PlayPause => ">",   // 再生
            ToolbarButton::Reset => "@",       // リセット（円形矢印風）
            ToolbarButton::Disk1 => "[1]",     // ディスク1
            ToolbarButton::Disk2 => "[2]",     // ディスク2
            ToolbarButton::SwapDisks => "<->", // 入れ替え
            ToolbarButton::QuickSave => "v",   // 下矢印（保存）
            ToolbarButton::QuickLoad => "^",   // 上矢印（読込）
            ToolbarButton::Screenshot => "*",  // カメラ風
            ToolbarButton::Fullscreen => "#",  // 全画面
        }
    }
    
    /// ツールチップ
    #[allow(dead_code)]
    pub fn tooltip(&self) -> &'static str {
        match self {
            ToolbarButton::PlayPause => "Pause/Resume",
            ToolbarButton::Reset => "Reset (F12)",
            ToolbarButton::Disk1 => "Disk 1",
            ToolbarButton::Disk2 => "Disk 2",
            ToolbarButton::SwapDisks => "Swap Disks",
            ToolbarButton::QuickSave => "Quick Save (F5)",
            ToolbarButton::QuickLoad => "Quick Load (F9)",
            ToolbarButton::Screenshot => "Screenshot (F10)",
            ToolbarButton::Fullscreen => "Fullscreen (F11)",
        }
    }
    
    /// 全ボタンを順番に取得
    pub fn all() -> &'static [ToolbarButton] {
        &[
            ToolbarButton::PlayPause,
            ToolbarButton::Reset,
            ToolbarButton::Disk1,
            ToolbarButton::Disk2,
            ToolbarButton::SwapDisks,
            ToolbarButton::QuickSave,
            ToolbarButton::QuickLoad,
            ToolbarButton::Screenshot,
            ToolbarButton::Fullscreen,
        ]
    }
}

/// エミュレータの状態（GUI表示用）
#[derive(Clone)]
pub struct EmulatorStatus {
    pub fps: f64,
    pub speed: u32,           // 0=MAX, 1=x1, 10=x10, etc.
    pub fast_disk: bool,
    pub save_slot: u8,
    pub sound_enabled: bool,
    pub gamepad_connected: bool,
    pub quality_level: i32,   // 0-4
    pub auto_quality: bool,
    pub paused: bool,
    #[allow(dead_code)]
    pub disk1_name: Option<String>,
    #[allow(dead_code)]
    pub disk2_name: Option<String>,
    pub disk1_active: bool,
    pub disk2_active: bool,
    // ディレクトリ設定
    pub rom_dir: String,
    pub disk_dir: String,
    pub screenshot_dir: String,
    pub save_dir: String,
}

impl Default for EmulatorStatus {
    fn default() -> Self {
        EmulatorStatus {
            fps: 60.0,
            speed: 1,
            fast_disk: true,
            save_slot: 0,
            sound_enabled: true,
            gamepad_connected: false,
            quality_level: 4,
            auto_quality: true,
            paused: false,
            disk1_name: None,
            disk2_name: None,
            disk1_active: false,
            disk2_active: false,
            rom_dir: "roms".to_string(),
            disk_dir: "disks".to_string(),
            screenshot_dir: "screenshots".to_string(),
            save_dir: "saves".to_string(),
        }
    }
}

/// ディスクメニューアクション
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiskMenuAction {
    Eject,
    InsertDisk(usize),  // ディスクリストのインデックス
}

/// GUI状態
pub struct Gui {
    /// マウス位置
    pub mouse_x: f32,
    pub mouse_y: f32,
    /// ホバー中のボタン
    pub hover_button: Option<ToolbarButton>,
    /// オーバーレイメニュー表示中
    pub overlay_visible: bool,
    /// オーバーレイメニューの選択インデックス
    pub overlay_selection: usize,
    /// 全画面モード
    pub fullscreen: bool,
    /// クリックされたボタン（フレームごとにクリア）
    clicked_button: Option<ToolbarButton>,
    /// ディスクメニュー表示中のドライブ (0 or 1)
    pub disk_menu_drive: Option<usize>,
    /// ディスクメニューの選択インデックス
    pub disk_menu_selection: usize,
    /// 利用可能なディスクリスト
    pub available_disks: Vec<String>,
    /// テキスト入力モード（編集中の項目番号）
    pub text_input_mode: Option<usize>,
    /// テキスト入力バッファ
    pub text_input_buffer: String,
}

impl Gui {
    pub fn new() -> Self {
        Gui {
            mouse_x: 0.0,
            mouse_y: 0.0,
            hover_button: None,
            overlay_visible: false,
            overlay_selection: 0,
            fullscreen: false,
            clicked_button: None,
            disk_menu_drive: None,
            disk_menu_selection: 0,
            available_disks: Vec::new(),
            text_input_mode: None,
            text_input_buffer: String::new(),
        }
    }
    
    /// テキスト入力モードを開始
    pub fn start_text_input(&mut self, item: usize, initial: &str) {
        self.text_input_mode = Some(item);
        self.text_input_buffer = initial.to_string();
    }
    
    /// テキスト入力モードを終了
    pub fn end_text_input(&mut self) -> Option<(usize, String)> {
        if let Some(item) = self.text_input_mode.take() {
            let result = self.text_input_buffer.clone();
            self.text_input_buffer.clear();
            return Some((item, result));
        }
        None
    }
    
    /// テキスト入力モードをキャンセル
    pub fn cancel_text_input(&mut self) {
        self.text_input_mode = None;
        self.text_input_buffer.clear();
    }
    
    /// テキスト入力モード中か
    pub fn is_text_input_mode(&self) -> bool {
        self.text_input_mode.is_some()
    }
    
    /// テキスト入力に文字を追加
    pub fn text_input_char(&mut self, ch: char) {
        if self.text_input_mode.is_some() && self.text_input_buffer.len() < 64 {
            self.text_input_buffer.push(ch);
        }
    }
    
    /// テキスト入力のバックスペース
    pub fn text_input_backspace(&mut self) {
        if self.text_input_mode.is_some() {
            self.text_input_buffer.pop();
        }
    }
    
    /// マウス位置を更新
    pub fn update_mouse(&mut self, x: f32, y: f32) {
        self.mouse_x = x;
        self.mouse_y = y;
        
        // ホバー判定
        if !self.fullscreen && y < TOOLBAR_HEIGHT as f32 {
            self.hover_button = self.get_button_at(x);
        } else {
            self.hover_button = None;
        }
    }
    
    /// マウスクリック処理
    pub fn mouse_click(&mut self) -> Option<ToolbarButton> {
        if let Some(btn) = self.hover_button {
            self.clicked_button = Some(btn);
            return Some(btn);
        }
        None
    }
    
    /// ディスクメニューを開く
    pub fn open_disk_menu(&mut self, drive: usize, disks: Vec<String>) {
        self.disk_menu_drive = Some(drive);
        self.disk_menu_selection = 0;
        self.available_disks = disks;
        self.overlay_visible = false;  // 設定メニューを閉じる
    }
    
    /// ディスクメニューを閉じる
    pub fn close_disk_menu(&mut self) {
        self.disk_menu_drive = None;
        self.disk_menu_selection = 0;
    }
    
    /// ディスクメニューが開いているか
    pub fn is_disk_menu_open(&self) -> bool {
        self.disk_menu_drive.is_some()
    }
    
    /// ディスクメニューの選択を上に移動
    pub fn disk_menu_up(&mut self) {
        if self.disk_menu_selection > 0 {
            self.disk_menu_selection -= 1;
        }
    }
    
    /// ディスクメニューの選択を下に移動
    pub fn disk_menu_down(&mut self) {
        let max_items = 1 + self.available_disks.len();  // Eject + ディスク数
        if self.disk_menu_selection < max_items - 1 {
            self.disk_menu_selection += 1;
        }
    }
    
    /// ディスクメニューの選択を確定
    pub fn disk_menu_select(&mut self) -> Option<(usize, DiskMenuAction)> {
        if let Some(drive) = self.disk_menu_drive {
            let action = if self.disk_menu_selection == 0 {
                DiskMenuAction::Eject
            } else {
                DiskMenuAction::InsertDisk(self.disk_menu_selection - 1)
            };
            self.close_disk_menu();
            return Some((drive, action));
        }
        None
    }
    
    /// 座標からボタンを取得
    fn get_button_at(&self, x: f32) -> Option<ToolbarButton> {
        let start_x = ICON_SPACING;
        let button_width = ICON_SIZE + ICON_SPACING + 8;  // draw_toolbarと同じ幅
        
        for (i, btn) in ToolbarButton::all().iter().enumerate() {
            let btn_x = start_x + i * button_width;
            if x >= btn_x as f32 && x < (btn_x + button_width - 4) as f32 {
                return Some(*btn);
            }
        }
        None
    }
    
    /// ツールバーを描画
    pub fn draw_toolbar(&self, buffer: &mut [u32], width: usize, status: &EmulatorStatus) {
        if self.fullscreen {
            return;
        }
        
        // 背景
        for y in 0..TOOLBAR_HEIGHT {
            for x in 0..width {
                buffer[y * width + x] = COLOR_TOOLBAR_BG;
            }
        }
        
        // ボタンを描画
        let start_x = ICON_SPACING;
        let button_width = ICON_SIZE + ICON_SPACING + 8;  // 少し広めに
        
        for (i, btn) in ToolbarButton::all().iter().enumerate() {
            let btn_x = start_x + i * button_width;
            let is_hover = self.hover_button == Some(*btn);
            
            // ボタンの状態に応じた色
            let color = if is_hover {
                COLOR_ICON_HOVER
            } else {
                match btn {
                    ToolbarButton::PlayPause if status.paused => COLOR_ICON_ACTIVE,
                    ToolbarButton::Disk1 if status.disk1_active => COLOR_ICON_ACTIVE,
                    ToolbarButton::Disk2 if status.disk2_active => COLOR_ICON_ACTIVE,
                    _ => COLOR_ICON_NORMAL,
                }
            };
            
            // グラフィカルアイコンを描画
            self.draw_icon(buffer, width, btn_x + 4, 4, *btn, status.paused, color);
        }
        
        // 下部の区切り線
        for x in 0..width {
            buffer[(TOOLBAR_HEIGHT - 1) * width + x] = COLOR_SEPARATOR;
        }
    }
    
    /// グラフィカルアイコンを描画
    fn draw_icon(&self, buffer: &mut [u32], buf_width: usize, x: usize, y: usize, 
                 btn: ToolbarButton, paused: bool, color: u32) {
        match btn {
            ToolbarButton::PlayPause => {
                if paused {
                    // 再生マーク（三角）
                    for row in 0..16 {
                        let w = row / 2 + 1;
                        for col in 0..w.min(8) {
                            self.set_pixel(buffer, buf_width, x + col + 4, y + row, color);
                        }
                    }
                } else {
                    // 一時停止マーク（||）
                    for row in 0..16 {
                        self.set_pixel(buffer, buf_width, x + 4, y + row, color);
                        self.set_pixel(buffer, buf_width, x + 5, y + row, color);
                        self.set_pixel(buffer, buf_width, x + 10, y + row, color);
                        self.set_pixel(buffer, buf_width, x + 11, y + row, color);
                    }
                }
            }
            ToolbarButton::Reset => {
                // 円形矢印（リセット）
                let cx = x + 10;
                let cy = y + 8;
                for angle in 0..28 {
                    let a = angle as f32 * 0.25;
                    let px = (cx as f32 + a.cos() * 6.0) as usize;
                    let py = (cy as f32 + a.sin() * 6.0) as usize;
                    self.set_pixel(buffer, buf_width, px, py, color);
                }
                // 矢印の先端
                self.set_pixel(buffer, buf_width, cx + 6, cy - 3, color);
                self.set_pixel(buffer, buf_width, cx + 7, cy - 2, color);
                self.set_pixel(buffer, buf_width, cx + 5, cy - 2, color);
            }
            ToolbarButton::Disk1 | ToolbarButton::Disk2 => {
                // フロッピーディスク
                let num = if btn == ToolbarButton::Disk1 { "1" } else { "2" };
                // ディスクの外枠
                for row in 0..14 {
                    self.set_pixel(buffer, buf_width, x + 2, y + row + 1, color);
                    self.set_pixel(buffer, buf_width, x + 17, y + row + 1, color);
                }
                for col in 2..18 {
                    self.set_pixel(buffer, buf_width, x + col, y + 1, color);
                    self.set_pixel(buffer, buf_width, x + col, y + 14, color);
                }
                // スライドシャッター
                for col in 5..15 {
                    self.set_pixel(buffer, buf_width, x + col, y + 3, color);
                    self.set_pixel(buffer, buf_width, x + col, y + 6, color);
                }
                // 番号
                self.draw_text(buffer, buf_width, x + 7, y + 8, num, color);
            }
            ToolbarButton::SwapDisks => {
                // 両方向矢印
                for col in 4..16 {
                    self.set_pixel(buffer, buf_width, x + col, y + 8, color);
                }
                // 左矢印
                self.set_pixel(buffer, buf_width, x + 4, y + 6, color);
                self.set_pixel(buffer, buf_width, x + 5, y + 7, color);
                self.set_pixel(buffer, buf_width, x + 4, y + 10, color);
                self.set_pixel(buffer, buf_width, x + 5, y + 9, color);
                // 右矢印
                self.set_pixel(buffer, buf_width, x + 15, y + 6, color);
                self.set_pixel(buffer, buf_width, x + 14, y + 7, color);
                self.set_pixel(buffer, buf_width, x + 15, y + 10, color);
                self.set_pixel(buffer, buf_width, x + 14, y + 9, color);
            }
            ToolbarButton::QuickSave => {
                // 下矢印（保存）
                for row in 2..10 {
                    self.set_pixel(buffer, buf_width, x + 9, y + row, color);
                    self.set_pixel(buffer, buf_width, x + 10, y + row, color);
                }
                for i in 0..4 {
                    self.set_pixel(buffer, buf_width, x + 6 + i, y + 10 + i, color);
                    self.set_pixel(buffer, buf_width, x + 13 - i, y + 10 + i, color);
                }
                // 下線
                for col in 4..16 {
                    self.set_pixel(buffer, buf_width, x + col, y + 15, color);
                }
            }
            ToolbarButton::QuickLoad => {
                // 上矢印（読み込み）
                for row in 6..14 {
                    self.set_pixel(buffer, buf_width, x + 9, y + row, color);
                    self.set_pixel(buffer, buf_width, x + 10, y + row, color);
                }
                for i in 0..4 {
                    self.set_pixel(buffer, buf_width, x + 6 + i, y + 5 - i, color);
                    self.set_pixel(buffer, buf_width, x + 13 - i, y + 5 - i, color);
                }
                // 下線
                for col in 4..16 {
                    self.set_pixel(buffer, buf_width, x + col, y + 15, color);
                }
            }
            ToolbarButton::Screenshot => {
                // カメラ
                for col in 3..17 {
                    self.set_pixel(buffer, buf_width, x + col, y + 4, color);
                    self.set_pixel(buffer, buf_width, x + col, y + 14, color);
                }
                for row in 4..15 {
                    self.set_pixel(buffer, buf_width, x + 3, y + row, color);
                    self.set_pixel(buffer, buf_width, x + 16, y + row, color);
                }
                // レンズ（円）
                let cx = x + 10;
                let cy = y + 9;
                for angle in 0..16 {
                    let a = angle as f32 * 0.4;
                    let px = (cx as f32 + a.cos() * 3.0) as usize;
                    let py = (cy as f32 + a.sin() * 3.0) as usize;
                    self.set_pixel(buffer, buf_width, px, py, color);
                }
            }
            ToolbarButton::Fullscreen => {
                // 四隅の矢印（全画面）
                // 左上
                for i in 0..5 { self.set_pixel(buffer, buf_width, x + 3, y + 3 + i, color); }
                for i in 0..5 { self.set_pixel(buffer, buf_width, x + 3 + i, y + 3, color); }
                // 右上
                for i in 0..5 { self.set_pixel(buffer, buf_width, x + 16, y + 3 + i, color); }
                for i in 0..5 { self.set_pixel(buffer, buf_width, x + 16 - i, y + 3, color); }
                // 左下
                for i in 0..5 { self.set_pixel(buffer, buf_width, x + 3, y + 13 - i, color); }
                for i in 0..5 { self.set_pixel(buffer, buf_width, x + 3 + i, y + 13, color); }
                // 右下
                for i in 0..5 { self.set_pixel(buffer, buf_width, x + 16, y + 13 - i, color); }
                for i in 0..5 { self.set_pixel(buffer, buf_width, x + 16 - i, y + 13, color); }
            }
        }
    }
    
    /// ピクセルを描画（境界チェック付き）
    fn set_pixel(&self, buffer: &mut [u32], width: usize, x: usize, y: usize, color: u32) {
        if x < width && y < buffer.len() / width {
            buffer[y * width + x] = color;
        }
    }
    
    /// ステータスバーを描画
    pub fn draw_statusbar(&self, buffer: &mut [u32], width: usize, height: usize, status: &EmulatorStatus) {
        if self.fullscreen {
            return;
        }
        
        let bar_y = height - STATUSBAR_HEIGHT;
        
        // 背景
        for y in bar_y..height {
            for x in 0..width {
                buffer[y * width + x] = COLOR_STATUSBAR_BG;
            }
        }
        
        // 上部の区切り線
        for x in 0..width {
            buffer[bar_y * width + x] = COLOR_SEPARATOR;
        }
        
        // ステータステキストを構築
        let fps_str = format!("{:.0} FPS", status.fps);
        let speed_str = if status.speed == 0 { "MAX".to_string() } else { format!("x{}", status.speed) };
        let disk_str = if status.fast_disk { "Disk: Fast" } else { "Disk: Normal" };
        let slot_str = format!("Slot: {}", status.save_slot);
        let sound_str = if status.sound_enabled { "[Sound]" } else { "[Mute]" };
        let gamepad_str = if status.gamepad_connected { "[Gamepad]" } else { "" };
        let quality_str = match status.quality_level {
            0 => "Lowest",
            1 => "Low",
            2 => "Medium",
            3 => "High",
            _ => "Ultra",
        };
        let auto_str = if status.auto_quality { " (Auto)" } else { "" };
        
        let full_status = format!(
            "{} | {} | {} | {} | {} {} | Quality: {}{}",
            fps_str, speed_str, disk_str, slot_str, sound_str, gamepad_str, quality_str, auto_str
        );
        
        self.draw_text(buffer, width, 8, bar_y + 6, &full_status, COLOR_TEXT);
    }
    
    /// オーバーレイメニューを描画
    pub fn draw_overlay(&self, buffer: &mut [u32], width: usize, height: usize, status: &EmulatorStatus) {
        if !self.overlay_visible {
            return;
        }
        
        // 半透明の背景
        for i in 0..buffer.len() {
            let pixel = buffer[i];
            let r = ((pixel >> 16) & 0xFF) / 2;
            let g = ((pixel >> 8) & 0xFF) / 2;
            let b = (pixel & 0xFF) / 2;
            buffer[i] = (r << 16) | (g << 8) | b;
        }
        
        // メニューパネル
        let menu_width = 280;
        let menu_height = 320;
        let menu_x = (width - menu_width) / 2;
        let menu_y = (height - menu_height) / 2;
        
        // パネル背景
        for y in menu_y..menu_y + menu_height {
            for x in menu_x..menu_x + menu_width {
                if y < height && x < width {
                    buffer[y * width + x] = 0x202030;
                }
            }
        }
        
        // 枠線
        for x in menu_x..menu_x + menu_width {
            if menu_y < height {
                buffer[menu_y * width + x] = COLOR_ICON_ACTIVE;
            }
            if menu_y + menu_height - 1 < height {
                buffer[(menu_y + menu_height - 1) * width + x] = COLOR_ICON_ACTIVE;
            }
        }
        for y in menu_y..menu_y + menu_height {
            if y < height {
                buffer[y * width + menu_x] = COLOR_ICON_ACTIVE;
                buffer[y * width + menu_x + menu_width - 1] = COLOR_ICON_ACTIVE;
            }
        }
        
        // タイトル
        self.draw_text(buffer, width, menu_x + 80, menu_y + 12, "SETTINGS (ESC)", COLOR_ICON_ACTIVE);
        
        // メニュー項目の値を事前に計算
        let speed_str = if status.speed == 0 { "MAX".to_string() } else { format!("x{}", status.speed) };
        let fast_disk_str = if status.fast_disk { "ON" } else { "OFF" };
        let quality_str = match status.quality_level {
            0 => "Lowest",
            1 => "Low", 
            2 => "Medium",
            3 => "High",
            _ => "Ultra",
        };
        let auto_quality_str = if status.auto_quality { "ON" } else { "OFF" };
        
        // ディレクトリ名を短縮表示
        let truncate = |s: &str, max: usize| -> String {
            if s.len() > max { format!("{}...", &s[..max-3]) } else { s.to_string() }
        };
        let rom_dir_str = truncate(&status.rom_dir, 12);
        let disk_dir_str = truncate(&status.disk_dir, 12);
        let screenshot_dir_str = truncate(&status.screenshot_dir, 12);
        let save_dir_str = truncate(&status.save_dir, 12);
        
        let items: Vec<(&str, String)> = vec![
            ("Speed", speed_str),
            ("Fast Disk", fast_disk_str.to_string()),
            ("Quality", quality_str.to_string()),
            ("Auto Quality", auto_quality_str.to_string()),
            ("---", "---".to_string()),
            ("ROM Dir", rom_dir_str),
            ("Disk Dir", disk_dir_str),
            ("Screenshot Dir", screenshot_dir_str),
            ("Save Dir", save_dir_str),
        ];
        
        for (i, (label, value)) in items.iter().enumerate() {
            let y = menu_y + 40 + i * 24;
            
            // 区切り線の場合
            if *label == "---" {
                for x in menu_x + 20..menu_x + menu_width - 20 {
                    if y < height && x < width {
                        buffer[y * width + x] = COLOR_SEPARATOR;
                    }
                }
                continue;
            }
            
            let color = if i == self.overlay_selection {
                COLOR_ICON_ACTIVE
            } else {
                COLOR_TEXT
            };
            
            // 選択インジケータ
            if i == self.overlay_selection {
                self.draw_text(buffer, width, menu_x + 12, y, ">", COLOR_ICON_ACTIVE);
            }
            
            self.draw_text(buffer, width, menu_x + 24, y, label, color);
            
            // テキスト入力モード中は入力バッファを表示
            if self.text_input_mode == Some(i) {
                let input_text = format!("{}_", &self.text_input_buffer);
                self.draw_text(buffer, width, menu_x + 150, y, &input_text, COLOR_ICON_HOVER);
            } else {
                self.draw_text(buffer, width, menu_x + 150, y, value, COLOR_TEXT_BRIGHT);
            }
        }
        
        // 操作説明
        self.draw_text(buffer, width, menu_x + 10, menu_y + menu_height - 30, 
            "Up/Down:Select Enter:Edit ESC:Close", COLOR_ICON_DISABLED);
    }
    
    /// ディスクメニューを描画
    pub fn draw_disk_menu(&self, buffer: &mut [u32], width: usize, height: usize, current_disk_name: Option<&str>) {
        let drive = match self.disk_menu_drive {
            Some(d) => d,
            None => return,
        };
        
        // 半透明の背景
        for i in 0..buffer.len() {
            let pixel = buffer[i];
            let r = ((pixel >> 16) & 0xFF) / 2;
            let g = ((pixel >> 8) & 0xFF) / 2;
            let b = (pixel & 0xFF) / 2;
            buffer[i] = (r << 16) | (g << 8) | b;
        }
        
        // メニューサイズ計算
        let item_count = 1 + self.available_disks.len();  // Eject + ディスク数
        let menu_width = 300;
        let menu_height = 60 + item_count * 20 + 20;
        let menu_x = (width - menu_width) / 2;
        let menu_y = (height - menu_height) / 2;
        
        // パネル背景
        for y in menu_y..menu_y + menu_height {
            for x in menu_x..menu_x + menu_width {
                if y < height && x < width {
                    buffer[y * width + x] = 0x202030;
                }
            }
        }
        
        // 枠線
        for x in menu_x..menu_x + menu_width {
            if menu_y < height {
                buffer[menu_y * width + x] = COLOR_ICON_ACTIVE;
            }
            if menu_y + menu_height - 1 < height {
                buffer[(menu_y + menu_height - 1) * width + x] = COLOR_ICON_ACTIVE;
            }
        }
        for y in menu_y..menu_y + menu_height {
            if y < height {
                buffer[y * width + menu_x] = COLOR_ICON_ACTIVE;
                buffer[y * width + menu_x + menu_width - 1] = COLOR_ICON_ACTIVE;
            }
        }
        
        // タイトル
        let title = format!("DISK {} MENU", drive + 1);
        self.draw_text(buffer, width, menu_x + 100, menu_y + 12, &title, COLOR_ICON_ACTIVE);
        
        // 現在のディスク名
        let current = current_disk_name.unwrap_or("(empty)");
        let current_label = format!("Current: {}", current);
        self.draw_text(buffer, width, menu_x + 20, menu_y + 32, &current_label, COLOR_TEXT);
        
        // メニュー項目
        let start_y = menu_y + 55;
        
        // Eject
        let is_selected = self.disk_menu_selection == 0;
        let color = if is_selected { COLOR_ICON_ACTIVE } else { COLOR_TEXT };
        let prefix = if is_selected { "> " } else { "  " };
        self.draw_text(buffer, width, menu_x + 20, start_y, &format!("{}[Eject]", prefix), color);
        
        // 利用可能なディスク
        for (i, disk_name) in self.available_disks.iter().enumerate() {
            let is_selected = self.disk_menu_selection == i + 1;
            let color = if is_selected { COLOR_ICON_ACTIVE } else { COLOR_TEXT };
            let prefix = if is_selected { "> " } else { "  " };
            // ディスク名を短縮
            let display_name = if disk_name.len() > 32 {
                format!("{}...", &disk_name[..29])
            } else {
                disk_name.clone()
            };
            self.draw_text(buffer, width, menu_x + 20, start_y + (i + 1) * 20, 
                &format!("{}{}", prefix, display_name), color);
        }
        
        // 操作説明
        self.draw_text(buffer, width, menu_x + 20, menu_y + menu_height - 20, 
            "Up/Down: Select  Enter: OK  ESC: Cancel", COLOR_ICON_DISABLED);
    }
    
    /// 簡易テキスト描画（固定幅フォント風）
    fn draw_text(&self, buffer: &mut [u32], buf_width: usize, x: usize, y: usize, text: &str, color: u32) {
        let char_width = 7;
        
        for (i, ch) in text.chars().enumerate() {
            let cx = x + i * char_width;
            if cx + char_width >= buf_width {
                break;
            }
            
            // 簡易的な文字描画（ドットパターン）
            let pattern = get_char_pattern(ch);
            for (row, &bits) in pattern.iter().enumerate() {
                for col in 0..6 {
                    if (bits >> (5 - col)) & 1 != 0 {
                        let px = cx + col;
                        let py = y + row;
                        if py < buffer.len() / buf_width {
                            buffer[py * buf_width + px] = color;
                        }
                    }
                }
            }
        }
    }
    
    /// オーバーレイメニューの選択を上に移動
    pub fn overlay_up(&mut self) {
        if self.overlay_selection > 0 {
            self.overlay_selection -= 1;
            // セパレータ(4)をスキップ
            if self.overlay_selection == 4 {
                self.overlay_selection -= 1;
            }
        }
    }
    
    /// オーバーレイメニューの選択を下に移動
    pub fn overlay_down(&mut self) {
        if self.overlay_selection < 8 {
            self.overlay_selection += 1;
            // セパレータ(4)をスキップ
            if self.overlay_selection == 4 {
                self.overlay_selection += 1;
            }
        }
    }
    
    /// オーバーレイの表示/非表示をトグル
    pub fn toggle_overlay(&mut self) {
        self.overlay_visible = !self.overlay_visible;
    }
    
    /// 全画面モードをトグル
    pub fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;
    }
}

impl Default for Gui {
    fn default() -> Self {
        Self::new()
    }
}

/// 簡易フォントパターン（6x10ピクセル）
fn get_char_pattern(ch: char) -> [u8; 10] {
    match ch {
        'A' => [0b001100, 0b010010, 0b100001, 0b100001, 0b111111, 0b100001, 0b100001, 0b100001, 0b000000, 0b000000],
        'B' => [0b111110, 0b100001, 0b100001, 0b111110, 0b100001, 0b100001, 0b100001, 0b111110, 0b000000, 0b000000],
        'C' => [0b011110, 0b100001, 0b100000, 0b100000, 0b100000, 0b100000, 0b100001, 0b011110, 0b000000, 0b000000],
        'D' => [0b111100, 0b100010, 0b100001, 0b100001, 0b100001, 0b100001, 0b100010, 0b111100, 0b000000, 0b000000],
        'E' => [0b111111, 0b100000, 0b100000, 0b111110, 0b100000, 0b100000, 0b100000, 0b111111, 0b000000, 0b000000],
        'F' => [0b111111, 0b100000, 0b100000, 0b111110, 0b100000, 0b100000, 0b100000, 0b100000, 0b000000, 0b000000],
        'G' => [0b011110, 0b100001, 0b100000, 0b100000, 0b100111, 0b100001, 0b100001, 0b011110, 0b000000, 0b000000],
        'H' => [0b100001, 0b100001, 0b100001, 0b111111, 0b100001, 0b100001, 0b100001, 0b100001, 0b000000, 0b000000],
        'I' => [0b011100, 0b001000, 0b001000, 0b001000, 0b001000, 0b001000, 0b001000, 0b011100, 0b000000, 0b000000],
        'J' => [0b000111, 0b000010, 0b000010, 0b000010, 0b000010, 0b100010, 0b100010, 0b011100, 0b000000, 0b000000],
        'K' => [0b100001, 0b100010, 0b100100, 0b111000, 0b100100, 0b100010, 0b100001, 0b100001, 0b000000, 0b000000],
        'L' => [0b100000, 0b100000, 0b100000, 0b100000, 0b100000, 0b100000, 0b100000, 0b111111, 0b000000, 0b000000],
        'M' => [0b100001, 0b110011, 0b101101, 0b100001, 0b100001, 0b100001, 0b100001, 0b100001, 0b000000, 0b000000],
        'N' => [0b100001, 0b110001, 0b101001, 0b100101, 0b100011, 0b100001, 0b100001, 0b100001, 0b000000, 0b000000],
        'O' => [0b011110, 0b100001, 0b100001, 0b100001, 0b100001, 0b100001, 0b100001, 0b011110, 0b000000, 0b000000],
        'P' => [0b111110, 0b100001, 0b100001, 0b111110, 0b100000, 0b100000, 0b100000, 0b100000, 0b000000, 0b000000],
        'Q' => [0b011110, 0b100001, 0b100001, 0b100001, 0b100101, 0b100010, 0b011110, 0b000001, 0b000000, 0b000000],
        'R' => [0b111110, 0b100001, 0b100001, 0b111110, 0b100100, 0b100010, 0b100001, 0b100001, 0b000000, 0b000000],
        'S' => [0b011110, 0b100001, 0b100000, 0b011110, 0b000001, 0b000001, 0b100001, 0b011110, 0b000000, 0b000000],
        'T' => [0b111111, 0b001000, 0b001000, 0b001000, 0b001000, 0b001000, 0b001000, 0b001000, 0b000000, 0b000000],
        'U' => [0b100001, 0b100001, 0b100001, 0b100001, 0b100001, 0b100001, 0b100001, 0b011110, 0b000000, 0b000000],
        'V' => [0b100001, 0b100001, 0b100001, 0b100001, 0b100001, 0b010010, 0b001100, 0b001000, 0b000000, 0b000000],
        'W' => [0b100001, 0b100001, 0b100001, 0b100001, 0b101101, 0b101101, 0b010010, 0b010010, 0b000000, 0b000000],
        'X' => [0b100001, 0b010010, 0b001100, 0b001100, 0b001100, 0b010010, 0b100001, 0b100001, 0b000000, 0b000000],
        'Y' => [0b100001, 0b010010, 0b001100, 0b001000, 0b001000, 0b001000, 0b001000, 0b001000, 0b000000, 0b000000],
        'Z' => [0b111111, 0b000001, 0b000010, 0b000100, 0b001000, 0b010000, 0b100000, 0b111111, 0b000000, 0b000000],
        'a' => [0b000000, 0b000000, 0b011110, 0b000001, 0b011111, 0b100001, 0b100001, 0b011111, 0b000000, 0b000000],
        'b' => [0b100000, 0b100000, 0b111110, 0b100001, 0b100001, 0b100001, 0b100001, 0b111110, 0b000000, 0b000000],
        'c' => [0b000000, 0b000000, 0b011110, 0b100000, 0b100000, 0b100000, 0b100000, 0b011110, 0b000000, 0b000000],
        'd' => [0b000001, 0b000001, 0b011111, 0b100001, 0b100001, 0b100001, 0b100001, 0b011111, 0b000000, 0b000000],
        'e' => [0b000000, 0b000000, 0b011110, 0b100001, 0b111111, 0b100000, 0b100001, 0b011110, 0b000000, 0b000000],
        'f' => [0b000110, 0b001000, 0b001000, 0b011110, 0b001000, 0b001000, 0b001000, 0b001000, 0b000000, 0b000000],
        'g' => [0b000000, 0b000000, 0b011111, 0b100001, 0b100001, 0b011111, 0b000001, 0b011110, 0b000000, 0b000000],
        'h' => [0b100000, 0b100000, 0b101110, 0b110001, 0b100001, 0b100001, 0b100001, 0b100001, 0b000000, 0b000000],
        'i' => [0b001000, 0b000000, 0b011000, 0b001000, 0b001000, 0b001000, 0b001000, 0b011100, 0b000000, 0b000000],
        'j' => [0b000010, 0b000000, 0b000110, 0b000010, 0b000010, 0b000010, 0b100010, 0b011100, 0b000000, 0b000000],
        'k' => [0b100000, 0b100000, 0b100010, 0b100100, 0b111000, 0b100100, 0b100010, 0b100001, 0b000000, 0b000000],
        'l' => [0b011000, 0b001000, 0b001000, 0b001000, 0b001000, 0b001000, 0b001000, 0b011100, 0b000000, 0b000000],
        'm' => [0b000000, 0b000000, 0b110110, 0b101001, 0b101001, 0b101001, 0b101001, 0b101001, 0b000000, 0b000000],
        'n' => [0b000000, 0b000000, 0b101110, 0b110001, 0b100001, 0b100001, 0b100001, 0b100001, 0b000000, 0b000000],
        'o' => [0b000000, 0b000000, 0b011110, 0b100001, 0b100001, 0b100001, 0b100001, 0b011110, 0b000000, 0b000000],
        'p' => [0b000000, 0b000000, 0b111110, 0b100001, 0b111110, 0b100000, 0b100000, 0b100000, 0b000000, 0b000000],
        'q' => [0b000000, 0b000000, 0b011111, 0b100001, 0b011111, 0b000001, 0b000001, 0b000001, 0b000000, 0b000000],
        'r' => [0b000000, 0b000000, 0b101110, 0b110000, 0b100000, 0b100000, 0b100000, 0b100000, 0b000000, 0b000000],
        's' => [0b000000, 0b000000, 0b011110, 0b100000, 0b011110, 0b000001, 0b000001, 0b111110, 0b000000, 0b000000],
        't' => [0b001000, 0b001000, 0b011110, 0b001000, 0b001000, 0b001000, 0b001000, 0b000110, 0b000000, 0b000000],
        'u' => [0b000000, 0b000000, 0b100001, 0b100001, 0b100001, 0b100001, 0b100011, 0b011101, 0b000000, 0b000000],
        'v' => [0b000000, 0b000000, 0b100001, 0b100001, 0b100001, 0b010010, 0b010010, 0b001100, 0b000000, 0b000000],
        'w' => [0b000000, 0b000000, 0b100001, 0b100001, 0b101101, 0b101101, 0b010010, 0b010010, 0b000000, 0b000000],
        'x' => [0b000000, 0b000000, 0b100001, 0b010010, 0b001100, 0b001100, 0b010010, 0b100001, 0b000000, 0b000000],
        'y' => [0b000000, 0b000000, 0b100001, 0b100001, 0b100001, 0b011111, 0b000001, 0b011110, 0b000000, 0b000000],
        'z' => [0b000000, 0b000000, 0b111111, 0b000010, 0b000100, 0b001000, 0b010000, 0b111111, 0b000000, 0b000000],
        '0' => [0b011110, 0b100001, 0b100011, 0b100101, 0b101001, 0b110001, 0b100001, 0b011110, 0b000000, 0b000000],
        '1' => [0b001000, 0b011000, 0b001000, 0b001000, 0b001000, 0b001000, 0b001000, 0b011100, 0b000000, 0b000000],
        '2' => [0b011110, 0b100001, 0b000001, 0b000010, 0b000100, 0b001000, 0b010000, 0b111111, 0b000000, 0b000000],
        '3' => [0b011110, 0b100001, 0b000001, 0b001110, 0b000001, 0b000001, 0b100001, 0b011110, 0b000000, 0b000000],
        '4' => [0b000010, 0b000110, 0b001010, 0b010010, 0b100010, 0b111111, 0b000010, 0b000010, 0b000000, 0b000000],
        '5' => [0b111111, 0b100000, 0b111110, 0b000001, 0b000001, 0b000001, 0b100001, 0b011110, 0b000000, 0b000000],
        '6' => [0b011110, 0b100000, 0b100000, 0b111110, 0b100001, 0b100001, 0b100001, 0b011110, 0b000000, 0b000000],
        '7' => [0b111111, 0b000001, 0b000010, 0b000100, 0b001000, 0b001000, 0b001000, 0b001000, 0b000000, 0b000000],
        '8' => [0b011110, 0b100001, 0b100001, 0b011110, 0b100001, 0b100001, 0b100001, 0b011110, 0b000000, 0b000000],
        '9' => [0b011110, 0b100001, 0b100001, 0b011111, 0b000001, 0b000001, 0b000001, 0b011110, 0b000000, 0b000000],
        ' ' => [0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000],
        ':' => [0b000000, 0b001100, 0b001100, 0b000000, 0b000000, 0b001100, 0b001100, 0b000000, 0b000000, 0b000000],
        '|' => [0b001000, 0b001000, 0b001000, 0b001000, 0b001000, 0b001000, 0b001000, 0b001000, 0b000000, 0b000000],
        '>' => [0b100000, 0b010000, 0b001000, 0b000100, 0b001000, 0b010000, 0b100000, 0b000000, 0b000000, 0b000000],
        '<' => [0b000100, 0b001000, 0b010000, 0b100000, 0b010000, 0b001000, 0b000100, 0b000000, 0b000000, 0b000000],
        '[' => [0b011110, 0b010000, 0b010000, 0b010000, 0b010000, 0b010000, 0b010000, 0b011110, 0b000000, 0b000000],
        ']' => [0b011110, 0b000010, 0b000010, 0b000010, 0b000010, 0b000010, 0b000010, 0b011110, 0b000000, 0b000000],
        '(' => [0b000100, 0b001000, 0b010000, 0b010000, 0b010000, 0b010000, 0b001000, 0b000100, 0b000000, 0b000000],
        ')' => [0b010000, 0b001000, 0b000100, 0b000100, 0b000100, 0b000100, 0b001000, 0b010000, 0b000000, 0b000000],
        '.' => [0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b001100, 0b001100, 0b000000, 0b000000],
        ',' => [0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b001100, 0b001000, 0b010000, 0b000000, 0b000000],
        '/' => [0b000001, 0b000010, 0b000100, 0b001000, 0b010000, 0b100000, 0b000000, 0b000000, 0b000000, 0b000000],
        '-' => [0b000000, 0b000000, 0b000000, 0b111111, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000],
        '_' => [0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b111111, 0b000000, 0b000000],
        '%' => [0b110001, 0b110010, 0b000100, 0b001000, 0b010000, 0b100110, 0b000110, 0b000000, 0b000000, 0b000000],
        '+' => [0b000000, 0b001000, 0b001000, 0b111110, 0b001000, 0b001000, 0b000000, 0b000000, 0b000000, 0b000000],
        '=' => [0b000000, 0b000000, 0b111111, 0b000000, 0b000000, 0b111111, 0b000000, 0b000000, 0b000000, 0b000000],
        '?' => [0b011110, 0b100001, 0b000001, 0b000110, 0b001000, 0b000000, 0b001000, 0b001000, 0b000000, 0b000000],
        '!' => [0b001000, 0b001000, 0b001000, 0b001000, 0b001000, 0b000000, 0b001000, 0b001000, 0b000000, 0b000000],
        '\'' => [0b001100, 0b001000, 0b010000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000],
        '"' => [0b010010, 0b010010, 0b100100, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000],
        '#' => [0b010010, 0b010010, 0b111111, 0b010010, 0b111111, 0b010010, 0b010010, 0b000000, 0b000000, 0b000000],
        '*' => [0b000000, 0b001000, 0b101010, 0b011100, 0b101010, 0b001000, 0b000000, 0b000000, 0b000000, 0b000000],
        '&' => [0b011000, 0b100100, 0b011000, 0b010000, 0b101001, 0b100110, 0b100110, 0b011001, 0b000000, 0b000000],
        '@' => [0b011110, 0b100001, 0b101101, 0b101011, 0b101110, 0b100000, 0b100001, 0b011110, 0b000000, 0b000000],
        '^' => [0b001000, 0b010100, 0b100010, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000],
        '~' => [0b000000, 0b000000, 0b010000, 0b101010, 0b000100, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000],
        '`' => [0b001000, 0b000100, 0b000010, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000, 0b000000],
        '$' => [0b001000, 0b011110, 0b101000, 0b011110, 0b001010, 0b011110, 0b001000, 0b000000, 0b000000, 0b000000],
        ';' => [0b000000, 0b001100, 0b001100, 0b000000, 0b000000, 0b001100, 0b001000, 0b010000, 0b000000, 0b000000],
        '\\' => [0b100000, 0b010000, 0b001000, 0b000100, 0b000010, 0b000001, 0b000000, 0b000000, 0b000000, 0b000000],
        '{' => [0b000110, 0b001000, 0b001000, 0b110000, 0b001000, 0b001000, 0b001000, 0b000110, 0b000000, 0b000000],
        '}' => [0b110000, 0b001000, 0b001000, 0b000110, 0b001000, 0b001000, 0b001000, 0b110000, 0b000000, 0b000000],
        _ => [0b111111, 0b100001, 0b100001, 0b100001, 0b100001, 0b100001, 0b100001, 0b111111, 0b000000, 0b000000], // □
    }
}

// ===================================
// デバッガパネル
// ===================================

use crate::profiler::{Profiler, ProfileCategory, BootStage, Debugger, DebuggerState, opcode_name};

/// デバッガパネルの幅
pub const DEBUGGER_PANEL_WIDTH: usize = 320;

/// デバッガパネルの色
const COLOR_DEBUG_BG: u32 = 0x1A1A2E;
const COLOR_DEBUG_HEADER: u32 = 0x16213E;
const COLOR_DEBUG_TEXT: u32 = 0xE0E0E0;
const COLOR_DEBUG_HIGHLIGHT: u32 = 0x00FF88;
const COLOR_DEBUG_WARNING: u32 = 0xFFAA00;
const COLOR_DEBUG_ERROR: u32 = 0xFF4444;
const COLOR_DEBUG_MUTED: u32 = 0x808080;
const COLOR_DEBUG_BAR_BG: u32 = 0x2A2A4E;
const COLOR_DEBUG_BAR_FG: u32 = 0x4488FF;

/// デバッガパネルのタブ
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DebuggerTab {
    /// プロファイラ
    Profiler,
    /// CPU状態
    Cpu,
    /// メモリ
    Memory,
    /// ディスク
    Disk,
    /// ブレークポイント
    Breakpoints,
}

impl DebuggerTab {
    pub fn name(&self) -> &'static str {
        match self {
            DebuggerTab::Profiler => "Profile",
            DebuggerTab::Cpu => "CPU",
            DebuggerTab::Memory => "Memory",
            DebuggerTab::Disk => "Disk",
            DebuggerTab::Breakpoints => "Break",
        }
    }
    
    pub fn all() -> &'static [DebuggerTab] {
        &[
            DebuggerTab::Profiler,
            DebuggerTab::Cpu,
            DebuggerTab::Memory,
            DebuggerTab::Disk,
            DebuggerTab::Breakpoints,
        ]
    }
}

/// デバッガパネル状態
pub struct DebuggerPanel {
    /// 表示中か
    pub visible: bool,
    /// 現在のタブ
    pub current_tab: DebuggerTab,
    /// メモリ表示開始アドレス
    pub memory_offset: u16,
    /// スクロールオフセット
    pub scroll_offset: usize,
}

impl Default for DebuggerPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl DebuggerPanel {
    pub fn new() -> Self {
        DebuggerPanel {
            visible: false,
            current_tab: DebuggerTab::Profiler,
            memory_offset: 0,
            scroll_offset: 0,
        }
    }
    
    /// パネルの表示/非表示を切り替え
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
    
    /// 次のタブに切り替え
    pub fn next_tab(&mut self) {
        let tabs = DebuggerTab::all();
        let current_idx = tabs.iter().position(|&t| t == self.current_tab).unwrap_or(0);
        self.current_tab = tabs[(current_idx + 1) % tabs.len()];
        self.scroll_offset = 0;
    }
    
    /// 前のタブに切り替え
    pub fn prev_tab(&mut self) {
        let tabs = DebuggerTab::all();
        let current_idx = tabs.iter().position(|&t| t == self.current_tab).unwrap_or(0);
        self.current_tab = tabs[(current_idx + tabs.len() - 1) % tabs.len()];
        self.scroll_offset = 0;
    }
    
    /// デバッガパネルを描画
    pub fn render(
        &self,
        buffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        x_offset: usize,
        profiler: &Profiler,
        debugger: &Debugger,
        cpu_regs: &CpuRegisters,
        memory: &[u8],
        disk_info: &DiskDebugInfo,
    ) {
        if !self.visible {
            return;
        }
        
        let panel_height = buffer_height;
        let panel_width = DEBUGGER_PANEL_WIDTH.min(buffer_width.saturating_sub(x_offset));
        
        // 背景
        for y in 0..panel_height {
            for x in 0..panel_width {
                let px = x_offset + x;
                if px < buffer_width && y < buffer_height {
                    buffer[y * buffer_width + px] = COLOR_DEBUG_BG;
                }
            }
        }
        
        // タブバー
        let tab_y = 0;
        let tab_height = 20;
        for y in tab_y..tab_y + tab_height {
            for x in 0..panel_width {
                let px = x_offset + x;
                if px < buffer_width && y < buffer_height {
                    buffer[y * buffer_width + px] = COLOR_DEBUG_HEADER;
                }
            }
        }
        
        // タブを描画
        let tabs = DebuggerTab::all();
        let tab_width = panel_width / tabs.len();
        for (i, tab) in tabs.iter().enumerate() {
            let tx = x_offset + i * tab_width;
            let color = if *tab == self.current_tab {
                COLOR_DEBUG_HIGHLIGHT
            } else {
                COLOR_DEBUG_MUTED
            };
            draw_text_small(buffer, buffer_width, tx + 4, tab_y + 6, tab.name(), color);
        }
        
        // コンテンツエリア
        let content_y = tab_height + 2;
        
        match self.current_tab {
            DebuggerTab::Profiler => {
                self.render_profiler(buffer, buffer_width, buffer_height, x_offset, content_y, panel_width, profiler);
            }
            DebuggerTab::Cpu => {
                self.render_cpu(buffer, buffer_width, buffer_height, x_offset, content_y, panel_width, cpu_regs, debugger);
            }
            DebuggerTab::Memory => {
                self.render_memory(buffer, buffer_width, buffer_height, x_offset, content_y, panel_width, memory);
            }
            DebuggerTab::Disk => {
                self.render_disk(buffer, buffer_width, buffer_height, x_offset, content_y, panel_width, disk_info, profiler);
            }
            DebuggerTab::Breakpoints => {
                self.render_breakpoints(buffer, buffer_width, buffer_height, x_offset, content_y, panel_width, debugger);
            }
        }
    }
    
    fn render_profiler(
        &self,
        buffer: &mut [u32],
        buffer_width: usize,
        _buffer_height: usize,
        x_offset: usize,
        y_start: usize,
        panel_width: usize,
        profiler: &Profiler,
    ) {
        let mut y = y_start;
        let line_height = 12;
        
        // FPS & CPU速度
        let fps_text = format!("FPS: {:.1}  CPU: {:.2} MHz", profiler.fps, profiler.cpu_mhz);
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &fps_text, COLOR_DEBUG_HIGHLIGHT);
        y += line_height + 4;
        
        // ブート段階
        let stage_color = match profiler.boot_stage {
            BootStage::Complete => COLOR_DEBUG_HIGHLIGHT,
            BootStage::Error(_) => COLOR_DEBUG_ERROR,
            _ => COLOR_DEBUG_WARNING,
        };
        let stage_text = format!("Boot: {}", profiler.boot_stage.name());
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &stage_text, stage_color);
        y += line_height;
        
        if let Some(elapsed) = profiler.boot_elapsed() {
            let time_text = format!("Time: {:.2}s", elapsed.as_secs_f64());
            draw_text_small(buffer, buffer_width, x_offset + 4, y, &time_text, COLOR_DEBUG_TEXT);
            y += line_height;
        }
        
        y += 4;
        
        // 区切り線
        for x in 0..panel_width - 8 {
            let px = x_offset + 4 + x;
            if px < buffer_width {
                buffer[y * buffer_width + px] = COLOR_DEBUG_MUTED;
            }
        }
        y += 6;
        
        // タイミング統計
        draw_text_small(buffer, buffer_width, x_offset + 4, y, "-- Timing --", COLOR_DEBUG_MUTED);
        y += line_height;
        
        let categories = [
            ProfileCategory::CpuExecution,
            ProfileCategory::DiskIO,
            ProfileCategory::VideoRender,
            ProfileCategory::GuiRender,
        ];
        
        for cat in categories {
            if let Some(stats) = profiler.get_stats(cat) {
                if stats.call_count > 0 {
                    // バー表示
                    let bar_width = panel_width - 100;
                    let max_time_ms = 16.0; // 60fps基準
                    let time_ms = stats.total_time.as_secs_f64() * 1000.0;
                    let bar_fill = ((time_ms / max_time_ms) * bar_width as f64).min(bar_width as f64) as usize;
                    
                    // バー背景
                    for x in 0..bar_width {
                        let px = x_offset + 80 + x;
                        if px < buffer_width {
                            let color = if x < bar_fill { COLOR_DEBUG_BAR_FG } else { COLOR_DEBUG_BAR_BG };
                            buffer[y * buffer_width + px] = color;
                            buffer[(y + 1) * buffer_width + px] = color;
                            buffer[(y + 2) * buffer_width + px] = color;
                        }
                    }
                    
                    draw_text_small(buffer, buffer_width, x_offset + 4, y, cat.name(), COLOR_DEBUG_TEXT);
                    
                    let time_text = format!("{:.1}ms", time_ms);
                    draw_text_small(buffer, buffer_width, x_offset + panel_width - 50, y, &time_text, COLOR_DEBUG_TEXT);
                    
                    y += line_height;
                }
            }
        }
        
        y += 4;
        
        // ディスク統計
        draw_text_small(buffer, buffer_width, x_offset + 4, y, "-- Disk --", COLOR_DEBUG_MUTED);
        y += line_height;
        
        let nibble_text = format!("Nibbles: {}", profiler.disk_info.nibbles_read);
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &nibble_text, COLOR_DEBUG_TEXT);
        y += line_height;
        
        let sector_text = format!("Sectors: {} (fail: {})", 
            profiler.disk_info.sectors_read, profiler.disk_info.sectors_failed);
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &sector_text, COLOR_DEBUG_TEXT);
        y += line_height;
        
        let track_text = format!("Track: {}", profiler.disk_info.current_track);
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &track_text, COLOR_DEBUG_TEXT);
    }
    
    fn render_cpu(
        &self,
        buffer: &mut [u32],
        buffer_width: usize,
        _buffer_height: usize,
        x_offset: usize,
        y_start: usize,
        _panel_width: usize,
        cpu: &CpuRegisters,
        debugger: &Debugger,
    ) {
        let mut y = y_start;
        let line_height = 12;
        
        // 状態
        let state_color = match debugger.state {
            DebuggerState::Running => COLOR_DEBUG_HIGHLIGHT,
            DebuggerState::Paused => COLOR_DEBUG_WARNING,
            DebuggerState::Stepping => COLOR_DEBUG_WARNING,
            DebuggerState::BreakpointHit => COLOR_DEBUG_ERROR,
        };
        let state_text = format!("State: {:?}", debugger.state);
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &state_text, state_color);
        y += line_height + 4;
        
        // レジスタ
        draw_text_small(buffer, buffer_width, x_offset + 4, y, "-- Registers --", COLOR_DEBUG_MUTED);
        y += line_height;
        
        let pc_text = format!("PC: ${:04X}", cpu.pc);
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &pc_text, COLOR_DEBUG_HIGHLIGHT);
        y += line_height;
        
        let a_text = format!("A:  ${:02X} ({})", cpu.a, cpu.a);
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &a_text, COLOR_DEBUG_TEXT);
        y += line_height;
        
        let x_text = format!("X:  ${:02X} ({})", cpu.x, cpu.x);
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &x_text, COLOR_DEBUG_TEXT);
        y += line_height;
        
        let y_text = format!("Y:  ${:02X} ({})", cpu.y, cpu.y);
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &y_text, COLOR_DEBUG_TEXT);
        y += line_height;
        
        let sp_text = format!("SP: ${:02X}", cpu.sp);
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &sp_text, COLOR_DEBUG_TEXT);
        y += line_height + 4;
        
        // フラグ
        draw_text_small(buffer, buffer_width, x_offset + 4, y, "-- Flags --", COLOR_DEBUG_MUTED);
        y += line_height;
        
        let flags = format!("N:{} V:{} B:{} D:{} I:{} Z:{} C:{}",
            if cpu.flags & 0x80 != 0 { "1" } else { "0" },
            if cpu.flags & 0x40 != 0 { "1" } else { "0" },
            if cpu.flags & 0x10 != 0 { "1" } else { "0" },
            if cpu.flags & 0x08 != 0 { "1" } else { "0" },
            if cpu.flags & 0x04 != 0 { "1" } else { "0" },
            if cpu.flags & 0x02 != 0 { "1" } else { "0" },
            if cpu.flags & 0x01 != 0 { "1" } else { "0" },
        );
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &flags, COLOR_DEBUG_TEXT);
        y += line_height + 4;
        
        // 現在の命令
        draw_text_small(buffer, buffer_width, x_offset + 4, y, "-- Current --", COLOR_DEBUG_MUTED);
        y += line_height;
        
        let opcode = cpu.current_opcode;
        let inst_text = format!("${:02X} {}", opcode, opcode_name(opcode));
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &inst_text, COLOR_DEBUG_HIGHLIGHT);
    }
    
    fn render_memory(
        &self,
        buffer: &mut [u32],
        buffer_width: usize,
        buffer_height: usize,
        x_offset: usize,
        y_start: usize,
        _panel_width: usize,
        memory: &[u8],
    ) {
        let mut y = y_start;
        let line_height = 10;
        
        draw_text_small(buffer, buffer_width, x_offset + 4, y, "-- Memory --", COLOR_DEBUG_MUTED);
        y += line_height;
        
        let addr_text = format!("Offset: ${:04X}", self.memory_offset);
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &addr_text, COLOR_DEBUG_TEXT);
        y += line_height + 2;
        
        // 16バイトずつ表示
        let mut addr = self.memory_offset as usize;
        let max_lines = (buffer_height - y) / line_height;
        
        for _ in 0..max_lines.min(16) {
            if addr >= memory.len() {
                break;
            }
            
            let mut line = format!("{:04X}:", addr);
            for i in 0..8 {
                if addr + i < memory.len() {
                    line.push_str(&format!(" {:02X}", memory[addr + i]));
                }
            }
            
            draw_text_small(buffer, buffer_width, x_offset + 4, y, &line, COLOR_DEBUG_TEXT);
            y += line_height;
            addr += 8;
        }
    }
    
    fn render_disk(
        &self,
        buffer: &mut [u32],
        buffer_width: usize,
        _buffer_height: usize,
        x_offset: usize,
        y_start: usize,
        panel_width: usize,
        disk: &DiskDebugInfo,
        profiler: &Profiler,
    ) {
        let mut y = y_start;
        let line_height = 12;
        
        // ドライブ状態
        draw_text_small(buffer, buffer_width, x_offset + 4, y, "-- Drive --", COLOR_DEBUG_MUTED);
        y += line_height;
        
        let motor_color = if disk.motor_on { COLOR_DEBUG_HIGHLIGHT } else { COLOR_DEBUG_MUTED };
        let motor_text = format!("Motor: {}", if disk.motor_on { "ON" } else { "OFF" });
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &motor_text, motor_color);
        y += line_height;
        
        let drive_text = format!("Drive: {}  Track: {}", disk.current_drive + 1, disk.current_track);
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &drive_text, COLOR_DEBUG_TEXT);
        y += line_height;
        
        let phase_text = format!("Phase: {}  Position: {}", disk.phase, disk.byte_position);
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &phase_text, COLOR_DEBUG_TEXT);
        y += line_height;
        
        let mode_text = format!("Mode: {}", if disk.write_mode { "WRITE" } else { "READ" });
        let mode_color = if disk.write_mode { COLOR_DEBUG_WARNING } else { COLOR_DEBUG_TEXT };
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &mode_text, mode_color);
        y += line_height;
        
        let latch_text = format!("Latch: ${:02X}", disk.latch);
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &latch_text, COLOR_DEBUG_TEXT);
        y += line_height + 4;
        
        // SafeFast状態
        draw_text_small(buffer, buffer_width, x_offset + 4, y, "-- SafeFast --", COLOR_DEBUG_MUTED);
        y += line_height;
        
        let fast_color = if disk.fastdisk_effective { COLOR_DEBUG_HIGHLIGHT } else { COLOR_DEBUG_MUTED };
        let fast_text = format!("Effective: {}", if disk.fastdisk_effective { "YES" } else { "NO" });
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &fast_text, fast_color);
        y += line_height;
        
        let mode_text = format!("Mode: {:?}", disk.speed_mode);
        draw_text_small(buffer, buffer_width, x_offset + 4, y, &mode_text, COLOR_DEBUG_TEXT);
        y += line_height;
        
        if disk.latched_off {
            draw_text_small(buffer, buffer_width, x_offset + 4, y, "LATCHED OFF!", COLOR_DEBUG_ERROR);
            y += line_height;
        }
        
        y += 4;
        
        // トラックヒートマップ
        draw_text_small(buffer, buffer_width, x_offset + 4, y, "-- Track Heatmap --", COLOR_DEBUG_MUTED);
        y += line_height;
        
        let heatmap_width = panel_width - 8;
        let track_width = heatmap_width / 35;
        
        // 最大アクセス数を見つける
        let max_access = profiler.disk_info.track_accesses.iter().max().copied().unwrap_or(1).max(1);
        
        for track in 0..35 {
            let access = profiler.disk_info.track_accesses[track];
            let intensity = ((access as f32 / max_access as f32) * 255.0) as u8;
            let color = 0xFF000000 | ((intensity as u32) << 8); // 緑のグラデーション
            
            let tx = x_offset + 4 + track * track_width;
            for dy in 0..8 {
                for dx in 0..track_width.saturating_sub(1) {
                    let px = tx + dx;
                    if px < buffer_width {
                        buffer[(y + dy) * buffer_width + px] = if access > 0 { color } else { COLOR_DEBUG_BAR_BG };
                    }
                }
            }
        }
        y += 10;
        
        // トラック番号
        for track in (0..35).step_by(5) {
            let tx = x_offset + 4 + track * track_width;
            let track_label = format!("{}", track);
            draw_text_small(buffer, buffer_width, tx, y, &track_label, COLOR_DEBUG_MUTED);
        }
    }
    
    fn render_breakpoints(
        &self,
        buffer: &mut [u32],
        buffer_width: usize,
        _buffer_height: usize,
        x_offset: usize,
        y_start: usize,
        _panel_width: usize,
        debugger: &Debugger,
    ) {
        let mut y = y_start;
        let line_height = 12;
        
        draw_text_small(buffer, buffer_width, x_offset + 4, y, "-- Breakpoints --", COLOR_DEBUG_MUTED);
        y += line_height;
        
        let bps = debugger.breakpoints();
        if bps.is_empty() {
            draw_text_small(buffer, buffer_width, x_offset + 4, y, "(none)", COLOR_DEBUG_MUTED);
            y += line_height;
        } else {
            for bp in bps {
                let status = if bp.enabled { "[*]" } else { "[ ]" };
                let bp_text = format!("{} #{}: ${:04X} (hits: {})", status, bp.id, bp.address, bp.hit_count);
                let color = if bp.enabled { COLOR_DEBUG_TEXT } else { COLOR_DEBUG_MUTED };
                draw_text_small(buffer, buffer_width, x_offset + 4, y, &bp_text, color);
                y += line_height;
            }
        }
        
        y += 8;
        draw_text_small(buffer, buffer_width, x_offset + 4, y, "-- Controls --", COLOR_DEBUG_MUTED);
        y += line_height;
        draw_text_small(buffer, buffer_width, x_offset + 4, y, "F6: Step", COLOR_DEBUG_TEXT);
        y += line_height;
        draw_text_small(buffer, buffer_width, x_offset + 4, y, "F7: Continue", COLOR_DEBUG_TEXT);
        y += line_height;
        draw_text_small(buffer, buffer_width, x_offset + 4, y, "F8: Break", COLOR_DEBUG_TEXT);
    }
}

/// CPU レジスタ情報（デバッガ用）
#[derive(Debug, Clone, Default)]
pub struct CpuRegisters {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub flags: u8,
    pub current_opcode: u8,
}

/// ディスクデバッグ情報
#[derive(Debug, Clone, Default)]
pub struct DiskDebugInfo {
    pub motor_on: bool,
    pub current_drive: usize,
    pub current_track: usize,
    pub phase: usize,
    pub byte_position: usize,
    pub write_mode: bool,
    pub latch: u8,
    pub fastdisk_effective: bool,
    pub speed_mode: String,
    pub latched_off: bool,
}

/// 小さいフォントでテキストを描画
fn draw_text_small(buffer: &mut [u32], buffer_width: usize, x: usize, y: usize, text: &str, color: u32) {
    let mut cx = x;
    for ch in text.chars() {
        let glyph = get_char_pattern(ch);
        for (row, &bits) in glyph.iter().enumerate().take(8) {
            for col in 0..6 {
                if bits & (1 << (5 - col)) != 0 {
                    let px = cx + col;
                    let py = y + row;
                    if px < buffer_width {
                        let idx = py * buffer_width + px;
                        if idx < buffer.len() {
                            buffer[idx] = color;
                        }
                    }
                }
            }
        }
        cx += 6;
    }
}
