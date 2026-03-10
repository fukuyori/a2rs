//! 設定ファイル管理モジュール
//!
//! エミュレータの設定をJSON形式で永続化

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// 実行ファイルのディレクトリを取得
pub fn get_exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        env::var_os("HOME").map(PathBuf::from)
    }
}



fn default_config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(get_exe_dir).join("a2rs")
    }
    #[cfg(target_os = "macos")]
    {
        home_dir()
            .unwrap_or_else(get_exe_dir)
            .join("Library")
            .join("Application Support")
            .join("a2rs")
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        home_dir()
            .unwrap_or_else(get_exe_dir)
            .join(".config")
            .join("a2rs")
    }
}

fn default_a2rs_home_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(get_exe_dir).join("a2rs")
    }
    #[cfg(target_os = "macos")]
    {
        home_dir()
            .unwrap_or_else(get_exe_dir)
            .join("Library")
            .join("Application Support")
            .join("a2rs")
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        home_dir()
            .unwrap_or_else(get_exe_dir)
            .join(".local")
            .join("share")
            .join("a2rs")
    }
}

fn default_config_candidates() -> Vec<PathBuf> {
    let config_dir = default_config_dir();
    vec![
        config_dir.join("config.json"),
        config_dir.join("apple2_config.json"),
    ]
}

/// 相対パスを指定されたベースディレクトリからの絶対パスに解決
pub fn resolve_path_with_base(base: &str, relative: &str) -> PathBuf {
    let path = Path::new(relative);
    if path.is_absolute() {
        return path.to_path_buf();
    }

    let base_path = if base.is_empty() {
        default_a2rs_home_path()
    } else {
        let base_candidate = Path::new(base);
        if base_candidate.is_absolute() {
            base_candidate.to_path_buf()
        } else {
            get_exe_dir().join(base_candidate)
        }
    };

    base_path.join(relative)
}

fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        return home_dir().unwrap_or_else(get_exe_dir);
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir().unwrap_or_else(get_exe_dir).join(rest);
    }
    #[cfg(windows)]
    {
        if let Some(rest) = path.strip_prefix("~\\") {
            return home_dir().unwrap_or_else(get_exe_dir).join(rest);
        }
    }
    PathBuf::from(path)
}

fn resolve_against(base_dir: &Path, raw: &str) -> PathBuf {
    let path = expand_tilde(raw);
    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

/// 設定ファイルのパスを取得
pub fn get_config_path() -> PathBuf {
    default_config_candidates()
        .into_iter()
        .next()
        .unwrap_or_else(|| get_exe_dir().join("config.json"))
}

/// エミュレータ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// A2RSホームディレクトリ（相対パスの基準）
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
    /// ROMディレクトリ（通常は roms）
    #[serde(default = "default_rom_dir")]
    pub rom_dir: String,
    /// ディスクイメージディレクトリ（通常は disks）
    #[serde(default = "default_disk_dir")]
    pub disk_dir: String,
    /// スクリーンショットディレクトリ（独立パス。未設定時は a2rs_home/screenshots）
    #[serde(default = "default_screenshot_dir")]
    pub screenshot_dir: String,
    /// セーブデータディレクトリ（通常は saves）
    #[serde(default = "default_save_dir")]
    pub save_dir: String,
    /// ゲームパッド設定（未指定時は現行のデフォルトを使用）
    #[serde(default)]
    pub gamepad: GamepadConfig,
    /// Phase 8 実験的オプション（未指定時は安全設定）
    #[serde(default)]
    pub experimental: ExperimentalConfig,
}

fn default_home_dir() -> String { String::new() }
fn default_rom_dir() -> String { "roms".to_string() }
fn default_disk_dir() -> String { "disks".to_string() }
fn default_screenshot_dir() -> String { String::new() }
fn default_save_dir() -> String { "saves".to_string() }

fn default_volume() -> f32 { 0.5 }
fn default_disk_sequencer_mode() -> String { "safe".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentalConfig {
    #[serde(default = "default_disk_sequencer_mode")]
    pub disk_sequencer_mode: String,
    #[serde(default)]
    pub weak_bits: bool,
    #[serde(default)]
    pub write_splice: bool,
    #[serde(default)]
    pub disk_debug_logging: bool,
}

impl Default for ExperimentalConfig {
    fn default() -> Self {
        Self {
            disk_sequencer_mode: default_disk_sequencer_mode(),
            weak_bits: false,
            write_splice: false,
            disk_debug_logging: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamepadConfig {
    #[serde(default = "default_gamepad_enabled")]
    pub enabled: bool,
    #[serde(default = "default_gamepad_deadzone")]
    pub deadzone: f32,
    #[serde(default)]
    pub show_debug_overlay: bool,
    #[serde(default = "default_button_a_names")]
    pub button_a_names: Vec<String>,
    #[serde(default = "default_button_b_names")]
    pub button_b_names: Vec<String>,
    #[serde(default = "default_button_x_names")]
    pub button_x_names: Vec<String>,
    #[serde(default = "default_button_y_names")]
    pub button_y_names: Vec<String>,
    #[serde(default = "default_button_lb_names")]
    pub button_lb_names: Vec<String>,
    #[serde(default = "default_button_rb_names")]
    pub button_rb_names: Vec<String>,
    #[serde(default = "default_button_start_names")]
    pub button_start_names: Vec<String>,
    #[serde(default = "default_button_select_names")]
    pub button_select_names: Vec<String>,
    #[serde(default = "default_raw_button_a_codes")]
    pub raw_button_a_codes: Vec<u32>,
    #[serde(default = "default_raw_button_b_codes")]
    pub raw_button_b_codes: Vec<u32>,
    #[serde(default = "default_raw_axis_x_code")]
    pub raw_axis_x_code: u32,
    #[serde(default = "default_raw_axis_y_code")]
    pub raw_axis_y_code: u32,
    #[serde(default = "default_raw_hat_x_code")]
    pub raw_hat_x_code: u32,
    #[serde(default = "default_raw_hat_y_code")]
    pub raw_hat_y_code: u32,
}

fn default_gamepad_enabled() -> bool { true }
fn default_gamepad_deadzone() -> f32 { 0.15 }
fn default_button_a_names() -> Vec<String> { vec!["South".into(), "C".into()] }
fn default_button_b_names() -> Vec<String> { vec!["East".into()] }
fn default_button_x_names() -> Vec<String> { vec!["West".into()] }
fn default_button_y_names() -> Vec<String> { vec!["North".into()] }
fn default_button_lb_names() -> Vec<String> { vec!["LeftTrigger".into(), "LeftTrigger2".into()] }
fn default_button_rb_names() -> Vec<String> { vec!["RightTrigger".into(), "RightTrigger2".into()] }
fn default_button_start_names() -> Vec<String> { vec!["Start".into(), "Mode".into()] }
fn default_button_select_names() -> Vec<String> { vec!["Select".into()] }
fn default_raw_button_a_codes() -> Vec<u32> { vec![289, 297] }
fn default_raw_button_b_codes() -> Vec<u32> { vec![288, 296] }
fn default_raw_axis_x_code() -> u32 { 0 }
fn default_raw_axis_y_code() -> u32 { 1 }
fn default_raw_hat_x_code() -> u32 { 16 }
fn default_raw_hat_y_code() -> u32 { 17 }

impl Default for GamepadConfig {
    fn default() -> Self {
        Self {
            enabled: default_gamepad_enabled(),
            deadzone: default_gamepad_deadzone(),
            show_debug_overlay: false,
            button_a_names: default_button_a_names(),
            button_b_names: default_button_b_names(),
            button_x_names: default_button_x_names(),
            button_y_names: default_button_y_names(),
            button_lb_names: default_button_lb_names(),
            button_rb_names: default_button_rb_names(),
            button_start_names: default_button_start_names(),
            button_select_names: default_button_select_names(),
            raw_button_a_codes: default_raw_button_a_codes(),
            raw_button_b_codes: default_raw_button_b_codes(),
            raw_axis_x_code: default_raw_axis_x_code(),
            raw_axis_y_code: default_raw_axis_y_code(),
            raw_hat_x_code: default_raw_hat_x_code(),
            raw_hat_y_code: default_raw_hat_y_code(),
        }
    }
}

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
            gamepad: GamepadConfig::default(),
            experimental: ExperimentalConfig::default(),
        }
    }
}

impl Config {
    /// 設定ファイルを読み込む（OS既定パス）
    pub fn load() -> Self {
        Self::load_from(get_config_path())
    }

    /// オプション指定で設定ファイルを読み込む
    pub fn load_with_options(config_path: Option<&str>, home_path: Option<&str>) -> (Self, PathBuf) {
        let candidate_paths = if let Some(path) = config_path {
            vec![PathBuf::from(path)]
        } else {
            default_config_candidates()
        };

        let config_file_path = candidate_paths
            .iter()
            .find(|p| p.exists())
            .cloned()
            .unwrap_or_else(|| candidate_paths.into_iter().next().unwrap_or_else(get_config_path));

        let mut config = Self::load_from(&config_file_path);

        if let Some(home) = home_path {
            config.a2rs_home = home.to_string();
        }

        (config, config_file_path)
    }

    /// 指定したパスから設定を読み込む
    pub fn load_from<P: AsRef<Path>>(path: P) -> Self {
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Failed to parse config {:?}: {}, using defaults", path.as_ref(), e);
                    Config::default()
                }
            },
            Err(_) => Config::default(),
        }
    }

    /// 設定ファイルを保存する（OS既定パス）
    pub fn save(&self) -> Result<(), String> {
        self.save_to(get_config_path())
    }

    /// 指定したパスに設定を保存する
    pub fn save_to<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(path, json)
            .map_err(|e| format!("Failed to write config: {}", e))?;
        Ok(())
    }

    fn config_dir_path(&self) -> PathBuf {
        get_config_path()
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(get_exe_dir)
    }

    /// a2rs_homeの絶対パスを取得
    pub fn home_dir_path(&self) -> PathBuf {
        let raw = self.a2rs_home.trim();
        if raw.is_empty() {
            return default_a2rs_home_path();
        }
        if raw == "." {
            eprintln!(
                "Warning: invalid a2rs_home \".\" in config; falling back to {:?}",
                default_a2rs_home_path()
            );
            return default_a2rs_home_path();
        }
        let path = expand_tilde(raw);
        if path.is_absolute() {
            path
        } else {
            self.config_dir_path().join(path)
        }
    }

    /// 相対パスをa2rs_homeからの絶対パスに解決
    pub fn resolve_path(&self, relative: &str) -> PathBuf {
        let path = Path::new(relative);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.home_dir_path().join(path)
        }
    }

    /// ディスクディレクトリの絶対パスを取得
    pub fn disk_dir_path(&self) -> PathBuf {
        let raw = self.disk_dir.trim();
        if raw.is_empty() {
            self.home_dir_path().join("disks")
        } else {
            let path = expand_tilde(raw);
            if path.is_absolute() { path } else { self.home_dir_path().join(path) }
        }
    }

    /// スクリーンショットディレクトリの絶対パスを取得
    pub fn screenshot_dir_path(&self) -> PathBuf {
        let raw = self.screenshot_dir.trim();
        if raw.is_empty() {
            self.home_dir_path().join("screenshots")
        } else {
            resolve_against(&self.config_dir_path(), raw)
        }
    }

    /// セーブディレクトリの絶対パスを取得
    pub fn save_dir_path(&self) -> PathBuf {
        self.home_dir_path().join("saves")
    }

    /// ROMディレクトリの絶対パスを取得
    pub fn rom_dir_path(&self) -> PathBuf {
        let raw = self.rom_dir.trim();
        if raw.is_empty() {
            self.home_dir_path().join("roms")
        } else {
            let path = expand_tilde(raw);
            if path.is_absolute() { path } else { self.home_dir_path().join(path) }
        }
    }

    /// ディレクトリが存在しなければ作成
    pub fn ensure_directories(&self) {
        for dir in [self.save_dir_path(), self.screenshot_dir_path()] {
            if !dir.exists() {
                let _ = fs::create_dir_all(&dir);
            }
        }
    }

    /// セーブファイルのパスを取得
    #[allow(dead_code)]
    pub fn get_save_path(&self, slot: u8) -> String {
        SaveSlots::get_path_in(&self.save_dir_path(), slot)
            .to_string_lossy()
            .into_owned()
    }

    /// スクリーンショットのパスを取得
    #[allow(dead_code)]
    pub fn get_screenshot_path(&self, timestamp: u64) -> String {
        self.screenshot_dir_path()
            .join(format!("a2rs_{}.png", timestamp))
            .to_string_lossy()
            .into_owned()
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

    pub fn get_path_in(save_dir: &Path, slot: u8) -> PathBuf {
        save_dir.join(Self::get_filename(slot))
    }

    /// 指定ディレクトリ内のセーブスロットパスを取得（後方互換）
    pub fn get_path(a2rs_home: &str, save_dir: &str, slot: u8) -> PathBuf {
        let base = if save_dir.trim().is_empty() {
            if a2rs_home.trim().is_empty() {
                default_a2rs_home_path().join("saves")
            } else {
                resolve_path_with_base(a2rs_home, "saves")
            }
        } else {
            resolve_path_with_base(a2rs_home, save_dir)
        };
        Self::get_path_in(&base, slot)
    }

    pub fn exists(slot: u8) -> bool {
        Self::get_path_in(&default_a2rs_home_path().join("saves"), slot).exists()
    }

    pub fn exists_in(save_dir: &Path, slot: u8) -> bool {
        Self::get_path_in(save_dir, slot).exists()
    }

    #[allow(dead_code)]
    pub fn get_all_status() -> [bool; 10] {
        let mut status = [false; 10];
        for i in 0..10 {
            status[i] = Self::exists(i as u8);
        }
        status
    }
}
