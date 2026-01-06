//! 設定ファイル管理モジュール
//!
//! エミュレータの設定をJSON形式で永続化

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 設定ファイルのデフォルトパス
const CONFIG_FILE: &str = "apple2_config.json";

/// エミュレータ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 最後に使用したディスク1のパス
    pub last_disk1: Option<String>,
    /// 最後に使用したディスク2のパス
    pub last_disk2: Option<String>,
    /// 最後に使用したROMのパス
    pub last_rom: Option<String>,
    /// 速度設定（1=通常、0=最速）
    pub speed: u32,
    /// 高速ディスク有効
    pub fast_disk: bool,
    /// サウンド有効
    pub sound_enabled: bool,
    /// 品質レベル（0-4）
    pub quality_level: i32,
    /// 自動品質調整
    pub auto_quality: bool,
    /// ウィンドウサイズ（幅）
    pub window_width: usize,
    /// ウィンドウサイズ（高さ）
    pub window_height: usize,
    /// 現在のセーブスロット
    pub current_slot: u8,
    /// ROMディレクトリ
    #[serde(default = "default_rom_dir")]
    pub rom_dir: String,
    /// ディスクイメージディレクトリ
    #[serde(default = "default_disk_dir")]
    pub disk_dir: String,
    /// スクリーンショットディレクトリ
    #[serde(default = "default_screenshot_dir")]
    pub screenshot_dir: String,
    /// セーブデータディレクトリ
    #[serde(default = "default_save_dir")]
    pub save_dir: String,
}

fn default_rom_dir() -> String { "roms".to_string() }
fn default_disk_dir() -> String { "disks".to_string() }
fn default_screenshot_dir() -> String { "screenshots".to_string() }
fn default_save_dir() -> String { "saves".to_string() }

impl Default for Config {
    fn default() -> Self {
        Config {
            last_disk1: None,
            last_disk2: None,
            last_rom: None,
            speed: 1,
            fast_disk: true,
            sound_enabled: true,
            quality_level: 4,
            auto_quality: true,
            window_width: 560,
            window_height: 384,
            current_slot: 0,
            rom_dir: default_rom_dir(),
            disk_dir: default_disk_dir(),
            screenshot_dir: default_screenshot_dir(),
            save_dir: default_save_dir(),
        }
    }
}

impl Config {
    /// 設定ファイルを読み込む
    pub fn load() -> Self {
        Self::load_from(CONFIG_FILE)
    }

    /// 指定したパスから設定を読み込む
    pub fn load_from<P: AsRef<Path>>(path: P) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => {
                match serde_json::from_str(&content) {
                    Ok(config) => config,
                    Err(e) => {
                        eprintln!("Failed to parse config: {}, using defaults", e);
                        Config::default()
                    }
                }
            }
            Err(_) => Config::default(),
        }
    }

    /// 設定ファイルを保存する
    pub fn save(&self) -> Result<(), String> {
        self.save_to(CONFIG_FILE)
    }

    /// 指定したパスに設定を保存する
    pub fn save_to<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(path, json)
            .map_err(|e| format!("Failed to write config: {}", e))?;
        Ok(())
    }
    
    /// ディレクトリが存在しなければ作成
    pub fn ensure_directories(&self) {
        for dir in [&self.rom_dir, &self.disk_dir, &self.screenshot_dir, &self.save_dir] {
            if !dir.is_empty() && !Path::new(dir).exists() {
                let _ = fs::create_dir_all(dir);
            }
        }
    }
    
    /// セーブファイルのパスを取得
    #[allow(dead_code)]
    pub fn get_save_path(&self, slot: u8) -> String {
        let filename = if slot == 0 {
            "quicksave.json".to_string()
        } else {
            format!("save_slot_{}.json", slot)
        };
        if self.save_dir.is_empty() {
            filename
        } else {
            format!("{}/{}", self.save_dir, filename)
        }
    }
    
    /// スクリーンショットのパスを取得
    #[allow(dead_code)]
    pub fn get_screenshot_path(&self, timestamp: u64) -> String {
        let filename = format!("screenshot_{}.png", timestamp);
        if self.screenshot_dir.is_empty() {
            filename
        } else {
            format!("{}/{}", self.screenshot_dir, filename)
        }
    }
}

/// セーブスロット管理
pub struct SaveSlots;

impl SaveSlots {
    /// セーブスロットのファイル名を取得（レガシー互換）
    pub fn get_filename(slot: u8) -> String {
        if slot == 0 {
            "quicksave.json".to_string()
        } else {
            format!("save_slot_{}.json", slot)
        }
    }
    
    /// 指定ディレクトリ内のセーブスロットパスを取得
    #[allow(dead_code)]
    pub fn get_path(save_dir: &str, slot: u8) -> String {
        let filename = Self::get_filename(slot);
        if save_dir.is_empty() {
            filename
        } else {
            format!("{}/{}", save_dir, filename)
        }
    }

    /// スロットにセーブデータが存在するか確認
    pub fn exists(slot: u8) -> bool {
        Path::new(&Self::get_filename(slot)).exists()
    }
    
    /// 指定ディレクトリ内でスロットにセーブデータが存在するか確認
    #[allow(dead_code)]
    pub fn exists_in(save_dir: &str, slot: u8) -> bool {
        Path::new(&Self::get_path(save_dir, slot)).exists()
    }

    /// 全スロットの状態を取得（存在するかどうか）
    #[allow(dead_code)]
    pub fn get_all_status() -> [bool; 10] {
        let mut status = [false; 10];
        for i in 0..10 {
            status[i] = Self::exists(i as u8);
        }
        status
    }
}
