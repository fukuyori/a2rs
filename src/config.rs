//! 設定ファイル管理モジュール
//!
//! エミュレータの設定をJSON形式で永続化

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 設定ファイルのデフォルトファイル名
const CONFIG_FILENAME: &str = "apple2_config.json";

/// 実行ファイルのディレクトリを取得
pub fn get_exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// 相対パスを実行ファイルディレクトリからの絶対パスに解決（グローバル関数、a2rs_home未使用）
pub fn resolve_path(relative: &str) -> PathBuf {
    let path = Path::new(relative);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        get_exe_dir().join(relative)
    }
}

/// 相対パスを指定されたベースディレクトリからの絶対パスに解決
pub fn resolve_path_with_base(base: &str, relative: &str) -> PathBuf {
    let path = Path::new(relative);
    if path.is_absolute() {
        path.to_path_buf()
    } else if base.is_empty() {
        get_exe_dir().join(relative)
    } else {
        let base_path = Path::new(base);
        if base_path.is_absolute() {
            base_path.join(relative)
        } else {
            get_exe_dir().join(base).join(relative)
        }
    }
}

/// 設定ファイルのパスを取得
pub fn get_config_path() -> PathBuf {
    get_exe_dir().join(CONFIG_FILENAME)
}

/// エミュレータ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// A2RSホームディレクトリ（相対パスの基準）
    /// 空または未設定の場合は実行ファイルのディレクトリを使用
    #[serde(default = "default_home_dir")]
    pub a2rs_home: String,
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
    /// 音量 (0.0 - 1.0)
    #[serde(default = "default_volume")]
    pub volume: f32,
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

fn default_home_dir() -> String { String::new() }
fn default_rom_dir() -> String { "roms".to_string() }
fn default_disk_dir() -> String { "disks".to_string() }
fn default_screenshot_dir() -> String { "screenshots".to_string() }
fn default_save_dir() -> String { "saves".to_string() }
fn default_volume() -> f32 { 0.5 }

impl Default for Config {
    fn default() -> Self {
        Config {
            a2rs_home: default_home_dir(),
            last_disk1: None,
            last_disk2: None,
            last_rom: None,
            speed: 1,
            fast_disk: true,
            sound_enabled: true,
            volume: default_volume(),
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
    /// 設定ファイルを読み込む（実行ファイルと同じディレクトリから）
    pub fn load() -> Self {
        Self::load_from(get_config_path())
    }
    
    /// オプション指定で設定ファイルを読み込む
    /// 優先順位:
    /// 1. config_path が指定されている場合はそれを使用
    /// 2. home_path が指定されている場合は home_path/apple2_config.json を探す
    /// 3. 実行ファイルディレクトリの apple2_config.json
    /// 
    /// home_path が指定されている場合、読み込んだ設定の a2rs_home を上書き
    pub fn load_with_options(config_path: Option<&str>, home_path: Option<&str>) -> (Self, PathBuf) {
        let config_file_path = if let Some(path) = config_path {
            // 明示的に設定ファイルが指定された
            PathBuf::from(path)
        } else if let Some(home) = home_path {
            // homeが指定された場合、そこの設定ファイルを探す
            let home_config = Path::new(home).join(CONFIG_FILENAME);
            if home_config.exists() {
                home_config
            } else {
                // homeに設定ファイルがなければ実行ファイルディレクトリを使用
                get_config_path()
            }
        } else {
            get_config_path()
        };
        
        let mut config = Self::load_from(&config_file_path);
        
        // コマンドラインのhome指定を優先
        if let Some(home) = home_path {
            config.a2rs_home = home.to_string();
        }
        
        (config, config_file_path)
    }

    /// 指定したパスから設定を読み込む
    pub fn load_from<P: AsRef<Path>>(path: P) -> Self {
        match fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str(&content) {
                    Ok(config) => config,
                    Err(e) => {
                        eprintln!("Failed to parse config {:?}: {}, using defaults", path.as_ref(), e);
                        Config::default()
                    }
                }
            }
            Err(_) => Config::default(),
        }
    }

    /// 設定ファイルを保存する（実行ファイルと同じディレクトリに）
    pub fn save(&self) -> Result<(), String> {
        self.save_to(get_config_path())
    }

    /// 指定したパスに設定を保存する
    pub fn save_to<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(path, json)
            .map_err(|e| format!("Failed to write config: {}", e))?;
        Ok(())
    }
    
    /// 相対パスをa2rs_homeからの絶対パスに解決
    pub fn resolve_path(&self, relative: &str) -> PathBuf {
        resolve_path_with_base(&self.a2rs_home, relative)
    }
    
    /// a2rs_homeの絶対パスを取得
    pub fn home_dir_path(&self) -> PathBuf {
        if self.a2rs_home.is_empty() {
            get_exe_dir()
        } else {
            let path = Path::new(&self.a2rs_home);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                get_exe_dir().join(&self.a2rs_home)
            }
        }
    }
    
    /// ディスクディレクトリの絶対パスを取得
    pub fn disk_dir_path(&self) -> PathBuf {
        self.resolve_path(&self.disk_dir)
    }
    
    /// スクリーンショットディレクトリの絶対パスを取得
    pub fn screenshot_dir_path(&self) -> PathBuf {
        self.resolve_path(&self.screenshot_dir)
    }
    
    /// セーブディレクトリの絶対パスを取得
    pub fn save_dir_path(&self) -> PathBuf {
        self.resolve_path(&self.save_dir)
    }
    
    /// ROMディレクトリの絶対パスを取得
    pub fn rom_dir_path(&self) -> PathBuf {
        self.resolve_path(&self.rom_dir)
    }
    
    /// ディレクトリが存在しなければ作成
    pub fn ensure_directories(&self) {
        let dirs = [
            self.disk_dir_path(),
            self.screenshot_dir_path(),
            self.save_dir_path(),
            self.rom_dir_path(),
        ];
        for dir in dirs {
            if !dir.exists() {
                let _ = fs::create_dir_all(&dir);
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
    /// セーブスロットのファイル名を取得
    pub fn get_filename(slot: u8) -> String {
        if slot == 0 {
            "quicksave.json".to_string()
        } else {
            format!("save_slot_{}.json", slot)
        }
    }
    
    /// 指定ディレクトリ内のセーブスロットパスを取得（絶対パスに解決）
    /// a2rs_home: 基準ディレクトリ（空の場合は実行ファイルディレクトリ）
    /// save_dir: セーブディレクトリ（相対または絶対）
    pub fn get_path(a2rs_home: &str, save_dir: &str, slot: u8) -> PathBuf {
        let filename = Self::get_filename(slot);
        resolve_path_with_base(a2rs_home, save_dir).join(filename)
    }

    /// スロットにセーブデータが存在するか確認（デフォルトディレクトリ）
    pub fn exists(slot: u8) -> bool {
        Self::get_path("", "saves", slot).exists()
    }
    
    /// 指定ディレクトリ内でスロットにセーブデータが存在するか確認
    pub fn exists_in(a2rs_home: &str, save_dir: &str, slot: u8) -> bool {
        Self::get_path(a2rs_home, save_dir, slot).exists()
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
