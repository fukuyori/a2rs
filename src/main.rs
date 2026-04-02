//! A2RS - Apple II Emulator in Rust
//! 
//! Version 0.1
//! 
//! A2RS は Rust で書かれた高精度な Apple II エミュレータです。
//! 
//! # 機能
//! - 6502/65C02 CPUエミュレーション
//! - Apple II / II+ / IIe メモリマップ
//! - テキスト、Lo-Res、Hi-Resビデオモード
//! - Disk IIエミュレーション（DSK/NIB形式）
//! - SafeFast高速化
//! - プロファイラ/デバッガUI
//! 
//! # 使用方法
//! ```
//! a2rs -1 dos33.dsk
//! ```

// ライブラリからすべてのモジュールをインポート
use a2rs::cpu;
use a2rs::memory;
use a2rs::video;
use a2rs::disk;
use a2rs::disk::DiskSequencerMode;
use a2rs::apple2;
use a2rs::sound;
use a2rs::gamepad;
use a2rs::config;
use a2rs::gui;
use a2rs::profiler;
use a2rs::disk_log;
use a2rs::rom_resolver::{self, RomKind};
use a2rs::disk_resolver;
use a2rs::woz;

// テスト専用モジュール（main.rsのみ）
mod test_cpu;
mod debug_test;

use apple2::Apple2;
use memory::AppleModel;
#[allow(unused_imports)]
use cpu::MemoryBus;
use video::{SCREEN_WIDTH, SCREEN_HEIGHT, VideoTimingDiagnostics};
use sound::{Speaker, AudioOutput};
use gamepad::GamepadManager;
use config::{Config, SaveSlots, get_exe_dir};
use gui::{Gui, EmulatorStatus, ToolbarButton, DiskMenuAction, TOOLBAR_HEIGHT, STATUSBAR_HEIGHT};
use gui::{DebuggerPanel, CpuRegisters, DiskDebugInfo, DEBUGGER_PANEL_WIDTH};
use gui::draw_text_debug;
use profiler::{Profiler, Debugger};
use clap::Parser;
use minifb::{Key, Window, WindowOptions, KeyRepeat, MouseMode, MouseButton};
use std::fs;

use std::time::{Duration, Instant};

#[cfg(all(target_os = "linux", feature = "x11-drag"))]
mod linux_window_drag {
    use std::process;
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, Window};

    pub struct DragHelper {
        conn: x11rb::rust_connection::RustConnection,
        root: Window,
        pid_atom: u32,
        client_list_atom: u32,
        target_window: Option<Window>,
        pid: u32,
    }

    impl DragHelper {
        pub fn new() -> Option<Self> {
            let (conn, screen_num) = x11rb::connect(None).ok()?;
            let root = conn.setup().roots.get(screen_num)?.root;
            let pid_atom = conn.intern_atom(false, b"_NET_WM_PID").ok()?.reply().ok()?.atom;
            let client_list_atom = conn.intern_atom(false, b"_NET_CLIENT_LIST").ok()?.reply().ok()?.atom;
            let mut helper = Self {
                conn,
                root,
                pid_atom,
                client_list_atom,
                target_window: None,
                pid: process::id(),
            };
            helper.target_window = helper.find_own_window();
            Some(helper)
        }

        pub fn global_pointer_position(&mut self) -> Option<(i32, i32)> {
            if self.target_window.is_none() {
                self.target_window = self.find_own_window();
            }
            let reply = self.conn.query_pointer(self.root).ok()?.reply().ok()?;
            Some((i32::from(reply.root_x), i32::from(reply.root_y)))
        }

        fn find_own_window(&self) -> Option<Window> {
            let reply = self.conn.get_property(false, self.root, self.client_list_atom, AtomEnum::WINDOW, 0, u32::MAX).ok()?.reply().ok()?;
            let windows = reply.value32()?;
            for win in windows {
                let prop = self.conn.get_property(false, win, self.pid_atom, AtomEnum::CARDINAL, 0, 1).ok()?.reply().ok()?;
                let pid = prop.value32()?.next()?;
                if pid == self.pid {
                    return Some(win);
                }
            }
            None
        }
    }
}

#[cfg(not(all(target_os = "linux", feature = "x11-drag")))]
mod linux_window_drag {
    pub struct DragHelper;
    impl DragHelper {
        pub fn new() -> Option<Self> { None }
        pub fn global_pointer_position(&mut self) -> Option<(i32, i32)> { None }
    }
}



const DEFAULT_DISK_EXTS: &[&str] = &["dsk", "do", "po", "nib", "woz"];

fn draw_linux_gamepad_debug_overlay(buffer: &mut [u32], width: usize, height: usize, lines: &[String]) {
    if lines.is_empty() || width == 0 || height == 0 {
        return;
    }

    let x = 8usize;
    let y = TOOLBAR_HEIGHT + 8;
    let line_h = 10usize;
    let padding = 6usize;
    let max_chars = lines.iter().map(|s| s.chars().count()).max().unwrap_or(0);
    let box_w = ((max_chars * 6) + padding * 2).min(width.saturating_sub(x + 4));
    let box_h = ((lines.len() * line_h) + padding * 2).min(height.saturating_sub(y + 4));

    if box_w == 0 || box_h == 0 {
        return;
    }

    for row in 0..box_h {
        let py = y + row;
        if py >= height { break; }
        let row_base = py * width;
        for col in 0..box_w {
            let px = x + col;
            if px >= width { break; }
            let idx = row_base + px;
            let border = row == 0 || row + 1 == box_h || col == 0 || col + 1 == box_w;
            buffer[idx] = if border { 0x00A0FF88 } else { 0x00101010 };
        }
    }

    for (i, line) in lines.iter().enumerate() {
        let ty = y + padding + i * line_h;
        if ty + 8 >= height { break; }
        draw_text_debug(buffer, width, x + padding, ty, line, 0x00F0F0F0);
    }
}

fn draw_video_timing_debug_overlay(buffer: &mut [u32], width: usize, height: usize, lines: &[String]) {
    if lines.is_empty() || width == 0 || height == 0 {
        return;
    }

    let max_chars = lines.iter().map(|s| s.chars().count()).max().unwrap_or(0);
    let line_h = 10usize;
    let padding = 6usize;
    let box_w = ((max_chars * 6) + padding * 2).min(width.saturating_sub(12));
    let box_h = ((lines.len() * line_h) + padding * 2).min(height.saturating_sub(TOOLBAR_HEIGHT + STATUSBAR_HEIGHT + 12));
    if box_w == 0 || box_h == 0 {
        return;
    }

    let x = width.saturating_sub(box_w + 8);
    let y = TOOLBAR_HEIGHT + 8;

    for row in 0..box_h {
        let py = y + row;
        if py >= height { break; }
        let row_base = py * width;
        for col in 0..box_w {
            let px = x + col;
            if px >= width { break; }
            let idx = row_base + px;
            let border = row == 0 || row + 1 == box_h || col == 0 || col + 1 == box_w;
            buffer[idx] = if border { 0x00FFD060 } else { 0x00101018 };
        }
    }

    for (i, line) in lines.iter().enumerate() {
        let ty = y + padding + i * line_h;
        if ty + 8 >= height { break; }
        draw_text_debug(buffer, width, x + padding, ty, line, 0x00F0F0F0);
    }
}


fn format_cpu_flags(flags: u8) -> String {
    let bits = [
        ('N', 0x80),
        ('V', 0x40),
        ('U', 0x20),
        ('B', 0x10),
        ('D', 0x08),
        ('I', 0x04),
        ('Z', 0x02),
        ('C', 0x01),
    ];
    bits.iter()
        .map(|(ch, bit)| if flags & bit != 0 { *ch } else { '-' })
        .collect()
}

fn build_runtime_debug_lines(emu: &Apple2, paused: bool) -> Vec<String> {
    let pos = emu.video_position();
    let drive = &emu.disk.drives[emu.disk.curr_drive];
    let drive_label = if drive.disk.disk_loaded {
        format!(
            "D{} T{} HT{}.{} PH{} BYTE{}",
            emu.disk.curr_drive + 1,
            drive.current_track(),
            drive.phase / 2,
            if drive.phase % 2 == 0 { 0 } else { 5 },
            drive.phase,
            drive.disk.byte_position,
        )
    } else {
        format!("D{} EMPTY", emu.disk.curr_drive + 1)
    };

    vec![
        format!("DEBUG {}", if paused { "PAUSED" } else { "RUN" }),
        format!(
            "PC:{:04X} OP:{:02X}",
            emu.cpu.regs.pc,
            emu.memory.main_ram[emu.cpu.regs.pc as usize]
        ),
        format!(
            "A:{:02X} X:{:02X} Y:{:02X}",
            emu.cpu.regs.a,
            emu.cpu.regs.x,
            emu.cpu.regs.y
        ),
        format!("SP:{:02X} P:{}", emu.cpu.regs.sp, format_cpu_flags(emu.cpu.regs.status)),
        format!("CYC:{}", emu.total_cycles),
        format!(
            "LINE:{} COL:{} H:{} V:{}",
            pos.scanline,
            pos.scanline_cycle,
            if pos.hblank { 1 } else { 0 },
            if pos.vblank { 1 } else { 0 }
        ),
        format!("FBUS:{:04X}={:02X}", emu.floating_bus_address(), emu.floating_bus()),
        format!("{}", drive_label),
        format!("MOTOR:{} WRITE:{}", if emu.disk.motor_on { "ON" } else { "OFF" }, if emu.disk.write_mode { "ON" } else { "OFF" }),
        "Ctrl+F10 Pause/Run".to_string(),
        "Ctrl+F11 Step".to_string(),
        "Ctrl+F8 Overlay".to_string(),
    ]
}

/// A2RS - Apple II Emulator in Rust
#[derive(Parser, Debug)]
#[command(name = "a2rs")]
#[command(author = "A2RS Project")]
#[command(version)]
#[command(about = "A2RS - Apple II Emulator in Rust", long_about = None)]
struct Args {
    /// ディスクイメージファイル（ドライブ1）
    #[arg(short = '1', long)]
    disk1: Option<String>,

    /// ディスクイメージファイル（ドライブ2）  
    #[arg(short = '2', long)]
    disk2: Option<String>,

    /// Apple IIモデル (auto, ii, ii+, iie, iie-enhanced)
    /// autoの場合はROMサイズから自動検出
    #[arg(short, long, default_value = "auto")]
    model: String,

    /// ROMファイル
    #[arg(short, long)]
    rom: Option<String>,

    /// Disk II Boot ROM (256 bytes)
    #[arg(long)]
    disk_rom: Option<String>,

    /// ヘッドレスモード（GUIなし）
    #[arg(long)]
    headless: bool,

    /// 実行するサイクル数（ヘッドレスモード用）
    #[arg(long, default_value = "1000000")]
    cycles: u64,
    
    /// CPUテストを実行（Klaus2m5 6502 functional test）
    #[arg(long)]
    test_cpu: bool,
    
    /// クイックCPUテストを実行
    #[arg(long)]
    quick_test: bool,
    
    /// 65C02テストを実行
    #[arg(long)]
    test_65c02: bool,
    
    /// ビデオデバッグテスト
    #[arg(long)]
    debug_video: bool,
    
    /// ROM実行デバッグテスト
    #[arg(long)]
    debug_rom: bool,
    
    /// apple2dead.bin ROMテスト
    #[arg(long)]
    test_dead: Option<String>,
    
    /// 速度倍率（1=通常、2=2倍速、0=最高速）
    #[arg(long)]
    speed: Option<u32>, // 0 = MAX; omitted => config file
    
    /// 高速ディスク（ディスクアクセスを高速化）
    #[arg(long)]
    fast_disk: bool,
    
    /// フルスクリーン風表示（ボーダーレスウィンドウ）
    #[arg(long)]
    fullscreen: bool,
    
    /// ウィンドウサイズ（幅x高さ、例: 1280x960）
    #[arg(long, default_value = "640x480")]
    size: String,
    
    /// プロファイラを有効化
    #[arg(long)]
    profile: bool,
    
    /// プロファイルデータの出力先ファイル
    #[arg(long)]
    profile_output: Option<String>,
    
    /// プロファイル出力間隔（秒）
    #[arg(long, default_value = "5")]
    profile_interval: u64,
    
    /// ブート完了後にプロファイルを出力して終了
    #[arg(long)]
    profile_boot: bool,
    
    /// ディスクログレベル: none, flow, state, decide, all
    /// 複数指定可: flow+state+decide
    #[arg(long, default_value = "none")]
    disk_log: String,
    
    /// ブート/AccurateBoost の詳細ログを出力
    #[arg(long)]
    boost_log: bool,

    /// タイミング統計ログを1秒ごとに出力
    #[arg(long)]
    timing_log: bool,

    /// 通常ログレベル (off, error, warn, info, debug, trace)
    #[arg(long, default_value = "off")]
    log_level: String,
    
    /// 設定ファイルのパス（指定しない場合はa2rs_home/apple2_config.jsonまたは実行ファイルディレクトリ）
    #[arg(short, long)]
    config: Option<String>,
    
    /// A2RSホームディレクトリ（相対パスの基準、設定ファイルより優先）
    #[arg(long)]
    home: Option<String>,
}

/// スクリーンショットのファイル名を生成
fn screenshot_filename() -> String {
    chrono::Local::now()
        .format("a2rs_%Y%m%d_%H%M%S.png")
        .to_string()
}

/// スクリーンショットをPNGで保存
fn save_screenshot(filename: &str, fb: &[u32], width: usize, height: usize) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(filename)?;
    let w = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    
    let mut writer = encoder.write_header()?;
    
    let mut rgb_data = Vec::with_capacity(width * height * 3);
    for pixel in fb.iter() {
        rgb_data.push(((pixel >> 16) & 0xFF) as u8);
        rgb_data.push(((pixel >> 8) & 0xFF) as u8);
        rgb_data.push((pixel & 0xFF) as u8);
    }
    
    writer.write_image_data(&rgb_data)?;
    Ok(())
}

/// ディスクディレクトリからディスクファイル一覧を取得
fn get_available_disks(config: &Config) -> Vec<String> {
    disk_resolver::list_available_disks(&config.disk_dir_path(), DEFAULT_DISK_EXTS)
}

/// 最速のニアレストネイバースケーリング（アスペクト比維持）
fn scale_nearest_aspect_fast(src: &[u32], src_w: usize, src_h: usize, dst: &mut [u32], dst_w: usize, dst_h: usize) {
    // アスペクト比を計算
    let src_aspect = (src_w << 16) / src_h;
    let dst_aspect = (dst_w << 16) / dst_h;
    
    let (scale_w, scale_h, offset_x, offset_y) = if src_aspect > dst_aspect {
        let scale_w = dst_w;
        let scale_h = (dst_w * src_h) / src_w;
        let offset_y = (dst_h.saturating_sub(scale_h)) / 2;
        (scale_w, scale_h, 0, offset_y)
    } else {
        let scale_h = dst_h;
        let scale_w = (dst_h * src_w) / src_h;
        let offset_x = (dst_w.saturating_sub(scale_w)) / 2;
        (scale_w, scale_h, offset_x, 0)
    };
    
    // 固定小数点
    let x_step = (src_w << 16) / scale_w.max(1);
    let y_step = (src_h << 16) / scale_h.max(1);
    
    // 背景を黒でクリア
    dst.fill(0);
    
    let mut src_y_fixed = 0usize;
    
    for dst_y in 0..scale_h {
        let src_y = (src_y_fixed >> 16).min(src_h - 1);
        let row = src_y * src_w;
        let out_row = (dst_y + offset_y) * dst_w + offset_x;
        
        let mut src_x_fixed = 0usize;
        
        for dst_x in 0..scale_w {
            let src_x = (src_x_fixed >> 16).min(src_w - 1);
            dst[out_row + dst_x] = src[row + src_x];
            src_x_fixed += x_step;
        }
        
        src_y_fixed += y_step;
    }
}

/// 高速バイリニア補間でスケーリング（アスペクト比維持、整数演算）
fn scale_bilinear_aspect_fast(src: &[u32], src_w: usize, src_h: usize, dst: &mut [u32], dst_w: usize, dst_h: usize) {
    // アスペクト比を計算
    let src_aspect = (src_w << 16) / src_h;
    let dst_aspect = (dst_w << 16) / dst_h;
    
    let (scale_w, scale_h, offset_x, offset_y) = if src_aspect > dst_aspect {
        let scale_w = dst_w;
        let scale_h = (dst_w * src_h) / src_w;
        let offset_y = (dst_h.saturating_sub(scale_h)) / 2;
        (scale_w, scale_h, 0, offset_y)
    } else {
        let scale_h = dst_h;
        let scale_w = (dst_h * src_w) / src_h;
        let offset_x = (dst_w.saturating_sub(scale_w)) / 2;
        (scale_w, scale_h, offset_x, 0)
    };
    
    // 固定小数点（16ビット小数部）
    let x_step = ((src_w - 1) << 16) / scale_w.max(1);
    let y_step = ((src_h - 1) << 16) / scale_h.max(1);
    
    // 背景を黒でクリア
    dst.fill(0);
    
    let mut src_y_fixed = 0usize;
    
    for dst_y in 0..scale_h {
        let src_y = src_y_fixed >> 16;
        let y_frac = ((src_y_fixed & 0xFFFF) >> 8) as u32; // 0-255
        let y_frac_inv = 256 - y_frac;
        
        let src_y2 = (src_y + 1).min(src_h - 1);
        let row0 = src_y * src_w;
        let row1 = src_y2 * src_w;
        
        let out_y = dst_y + offset_y;
        let out_row = out_y * dst_w;
        
        let mut src_x_fixed = 0usize;
        
        for dst_x in 0..scale_w {
            let src_x = src_x_fixed >> 16;
            let x_frac = ((src_x_fixed & 0xFFFF) >> 8) as u32;
            let x_frac_inv = 256 - x_frac;
            
            let src_x2 = (src_x + 1).min(src_w - 1);
            
            let p00 = src[row0 + src_x];
            let p10 = src[row0 + src_x2];
            let p01 = src[row1 + src_x];
            let p11 = src[row1 + src_x2];
            
            // バイリニア補間（整数演算）
            let w00 = x_frac_inv * y_frac_inv;
            let w10 = x_frac * y_frac_inv;
            let w01 = x_frac_inv * y_frac;
            let w11 = x_frac * y_frac;
            
            let r = (((p00 >> 16) & 0xFF) * w00 + ((p10 >> 16) & 0xFF) * w10 
                   + ((p01 >> 16) & 0xFF) * w01 + ((p11 >> 16) & 0xFF) * w11) >> 16;
            let g = (((p00 >> 8) & 0xFF) * w00 + ((p10 >> 8) & 0xFF) * w10 
                   + ((p01 >> 8) & 0xFF) * w01 + ((p11 >> 8) & 0xFF) * w11) >> 16;
            let b = ((p00 & 0xFF) * w00 + (p10 & 0xFF) * w10 
                   + (p01 & 0xFF) * w01 + (p11 & 0xFF) * w11) >> 16;
            
            dst[out_row + dst_x + offset_x] = (r << 16) | (g << 8) | b;
            
            src_x_fixed += x_step;
        }
        
        src_y_fixed += y_step;
    }
}

/// CRTスキャンラインエフェクトを適用
fn apply_scanlines(buffer: &mut [u32], width: usize, height: usize, intensity: u32) {
    // 2行ごとに暗くする
    for y in 0..height {
        if y % 2 == 1 {
            let row_start = y * width;
            for x in 0..width {
                let pixel = buffer[row_start + x];
                let r = ((pixel >> 16) & 0xFF) * intensity / 256;
                let g = ((pixel >> 8) & 0xFF) * intensity / 256;
                let b = (pixel & 0xFF) * intensity / 256;
                buffer[row_start + x] = (r << 16) | (g << 8) | b;
            }
        }
    }
}

/// CRTブルーム（明るい部分の滲み）エフェクト
fn apply_bloom(buffer: &mut [u32], width: usize, height: usize, threshold: u32, strength: u32) {
    // 簡易的なブルーム: 明るいピクセルの周囲に光を追加
    // 効率のため、4ピクセルごとにサンプリング
    let step = 2;
    
    for y in (step..height - step).step_by(step) {
        for x in (step..width - step).step_by(step) {
            let idx = y * width + x;
            let pixel = buffer[idx];
            
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;
            
            // 明るさ判定
            let brightness = (r + g + b) / 3;
            if brightness > threshold {
                let glow = ((brightness - threshold) * strength / 256).min(64);
                
                // 周囲のピクセルに光を追加
                for dy in 0..step {
                    for dx in 0..step {
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx < width && ny < height {
                            let nidx = ny * width + nx;
                            let np = buffer[nidx];
                            let nr = (((np >> 16) & 0xFF) + glow).min(255);
                            let ng = (((np >> 8) & 0xFF) + glow).min(255);
                            let nb = ((np & 0xFF) + glow).min(255);
                            buffer[nidx] = (nr << 16) | (ng << 8) | nb;
                        }
                    }
                }
            }
        }
    }
}

/// CRT曲面効果（バレル歪み）
#[allow(dead_code)]
fn apply_crt_curvature(src: &[u32], dst: &mut [u32], width: usize, height: usize, curvature: f32) {
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    
    for y in 0..height {
        let dy = (y as f32 - cy) / cy;
        for x in 0..width {
            let dx = (x as f32 - cx) / cx;
            
            // バレル歪み計算
            let dist_sq = dx * dx + dy * dy;
            let factor = 1.0 + curvature * dist_sq;
            
            let src_x = ((dx * factor) * cx + cx) as i32;
            let src_y = ((dy * factor) * cy + cy) as i32;
            
            let dst_idx = y * width + x;
            
            if src_x >= 0 && src_x < width as i32 && src_y >= 0 && src_y < height as i32 {
                let src_idx = src_y as usize * width + src_x as usize;
                dst[dst_idx] = src[src_idx];
            } else {
                // 画面外は黒
                dst[dst_idx] = 0;
            }
        }
    }
}

/// RGBシャドウマスク効果（CRTのRGBサブピクセル模倣）
#[allow(dead_code)]
fn apply_shadow_mask(buffer: &mut [u32], width: usize, height: usize, intensity: u32) {
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let pixel = buffer[idx];
            
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;
            
            // 3ピクセル周期でRGBを強調
            let (r_mult, g_mult, b_mult) = match x % 3 {
                0 => (256, intensity, intensity),    // R強調
                1 => (intensity, 256, intensity),    // G強調
                _ => (intensity, intensity, 256),    // B強調
            };
            
            let r = (r * r_mult / 256).min(255);
            let g = (g * g_mult / 256).min(255);
            let b = (b * b_mult / 256).min(255);
            
            buffer[idx] = (r << 16) | (g << 8) | b;
        }
    }
}

/// 高速フレーム補間（整数演算、blend=25%固定）
fn blend_frames_fast(current: &[u32], previous: &mut [u32]) {
    // 25% previous + 75% current（シフト演算で高速化）
    for i in 0..current.len().min(previous.len()) {
        let curr = current[i];
        let prev = previous[i];
        
        // 各成分を計算: (prev + curr*3) / 4
        let r = ((((prev >> 16) & 0xFF) + ((curr >> 16) & 0xFF) * 3) >> 2) & 0xFF;
        let g = ((((prev >> 8) & 0xFF) + ((curr >> 8) & 0xFF) * 3) >> 2) & 0xFF;
        let b = (((prev & 0xFF) + (curr & 0xFF) * 3) >> 2) & 0xFF;
        
        previous[i] = (r << 16) | (g << 8) | b;
    }
}

/// 高速ガウシアンブラー（3x3カーネル、整数演算）
#[allow(dead_code)]
fn apply_gaussian_blur_fast(src: &[u32], dst: &mut [u32], width: usize, height: usize) {
    // 3x3ガウシアンカーネル（整数版: 1,2,1 / 2,4,2 / 1,2,1、合計16で割る）
    for y in 0..height {
        let y0 = if y == 0 { 0 } else { y - 1 };
        let y2 = if y >= height - 1 { height - 1 } else { y + 1 };
        
        for x in 0..width {
            let x0 = if x == 0 { 0 } else { x - 1 };
            let x2 = if x >= width - 1 { width - 1 } else { x + 1 };
            
            // 9ピクセルを取得
            let p00 = src[y0 * width + x0];
            let p10 = src[y0 * width + x];
            let p20 = src[y0 * width + x2];
            let p01 = src[y * width + x0];
            let p11 = src[y * width + x];
            let p21 = src[y * width + x2];
            let p02 = src[y2 * width + x0];
            let p12 = src[y2 * width + x];
            let p22 = src[y2 * width + x2];
            
            // R成分
            let r = (((p00 >> 16) & 0xFF) + ((p10 >> 16) & 0xFF) * 2 + ((p20 >> 16) & 0xFF)
                   + ((p01 >> 16) & 0xFF) * 2 + ((p11 >> 16) & 0xFF) * 4 + ((p21 >> 16) & 0xFF) * 2
                   + ((p02 >> 16) & 0xFF) + ((p12 >> 16) & 0xFF) * 2 + ((p22 >> 16) & 0xFF)) >> 4;
            
            // G成分
            let g = (((p00 >> 8) & 0xFF) + ((p10 >> 8) & 0xFF) * 2 + ((p20 >> 8) & 0xFF)
                   + ((p01 >> 8) & 0xFF) * 2 + ((p11 >> 8) & 0xFF) * 4 + ((p21 >> 8) & 0xFF) * 2
                   + ((p02 >> 8) & 0xFF) + ((p12 >> 8) & 0xFF) * 2 + ((p22 >> 8) & 0xFF)) >> 4;
            
            // B成分
            let b = ((p00 & 0xFF) + (p10 & 0xFF) * 2 + (p20 & 0xFF)
                   + (p01 & 0xFF) * 2 + (p11 & 0xFF) * 4 + (p21 & 0xFF) * 2
                   + (p02 & 0xFF) + (p12 & 0xFF) * 2 + (p22 & 0xFF)) >> 4;
            
            dst[y * width + x] = (r << 16) | (g << 8) | b;
        }
    }
}

/// 軽いシャープネス強調（アンシャープマスク風）
fn apply_light_sharpen(buffer: &mut [u32], width: usize, height: usize, strength: i32) {
    // シンプルな3x3シャープネス: 中央を強調、周囲を減算
    // strength: 強度 (10-50程度が適切)
    let mut temp = vec![0u32; buffer.len()];
    
    for y in 1..height-1 {
        for x in 1..width-1 {
            let idx = y * width + x;
            let center = buffer[idx];
            
            // 上下左右の平均
            let top = buffer[(y - 1) * width + x];
            let bottom = buffer[(y + 1) * width + x];
            let left = buffer[y * width + x - 1];
            let right = buffer[y * width + x + 1];
            
            let avg_r = (((top >> 16) & 0xFF) + ((bottom >> 16) & 0xFF) 
                       + ((left >> 16) & 0xFF) + ((right >> 16) & 0xFF)) / 4;
            let avg_g = (((top >> 8) & 0xFF) + ((bottom >> 8) & 0xFF) 
                       + ((left >> 8) & 0xFF) + ((right >> 8) & 0xFF)) / 4;
            let avg_b = ((top & 0xFF) + (bottom & 0xFF) 
                       + (left & 0xFF) + (right & 0xFF)) / 4;
            
            let c_r = (center >> 16) & 0xFF;
            let c_g = (center >> 8) & 0xFF;
            let c_b = center & 0xFF;
            
            // シャープネス: center + (center - avg) * strength / 100
            let new_r = (c_r as i32 + (c_r as i32 - avg_r as i32) * strength / 100).clamp(0, 255) as u32;
            let new_g = (c_g as i32 + (c_g as i32 - avg_g as i32) * strength / 100).clamp(0, 255) as u32;
            let new_b = (c_b as i32 + (c_b as i32 - avg_b as i32) * strength / 100).clamp(0, 255) as u32;
            
            temp[idx] = (new_r << 16) | (new_g << 8) | new_b;
        }
    }
    
    // 結果をコピー（境界部分は元のまま）
    for y in 1..height-1 {
        let row_start = y * width + 1;
        let row_end = y * width + width - 1;
        buffer[row_start..row_end].copy_from_slice(&temp[row_start..row_end]);
    }
}

/// キーコードをApple IIの文字コードに変換
fn key_to_apple2(key: Key, shift: bool, ctrl: bool) -> Option<u8> {
    // Ctrl+キーの場合、制御文字を返す
    if ctrl {
        return match key {
            Key::A => Some(0x01),
            Key::B => Some(0x02),
            Key::C => Some(0x03),
            Key::D => Some(0x04),
            Key::E => Some(0x05),
            Key::F => Some(0x06),
            Key::G => Some(0x07),
            Key::H => Some(0x08),
            Key::I => Some(0x09),
            Key::J => Some(0x0A),
            Key::K => Some(0x0B),
            Key::L => Some(0x0C),
            Key::M => Some(0x0D),
            Key::N => Some(0x0E),
            Key::O => Some(0x0F),
            Key::P => Some(0x10),
            Key::Q => Some(0x11),
            Key::R => Some(0x12),
            Key::S => Some(0x13),
            Key::T => Some(0x14),
            Key::U => Some(0x15),
            Key::V => Some(0x16),
            Key::W => Some(0x17),
            Key::X => Some(0x18),
            Key::Y => Some(0x19),
            Key::Z => Some(0x1A),
            _ => None,
        };
    }
    
    match key {
        Key::A => Some(if shift { b'A' } else { b'A' }),
        Key::B => Some(if shift { b'B' } else { b'B' }),
        Key::C => Some(if shift { b'C' } else { b'C' }),
        Key::D => Some(if shift { b'D' } else { b'D' }),
        Key::E => Some(if shift { b'E' } else { b'E' }),
        Key::F => Some(if shift { b'F' } else { b'F' }),
        Key::G => Some(if shift { b'G' } else { b'G' }),
        Key::H => Some(if shift { b'H' } else { b'H' }),
        Key::I => Some(if shift { b'I' } else { b'I' }),
        Key::J => Some(if shift { b'J' } else { b'J' }),
        Key::K => Some(if shift { b'K' } else { b'K' }),
        Key::L => Some(if shift { b'L' } else { b'L' }),
        Key::M => Some(if shift { b'M' } else { b'M' }),
        Key::N => Some(if shift { b'N' } else { b'N' }),
        Key::O => Some(if shift { b'O' } else { b'O' }),
        Key::P => Some(if shift { b'P' } else { b'P' }),
        Key::Q => Some(if shift { b'Q' } else { b'Q' }),
        Key::R => Some(if shift { b'R' } else { b'R' }),
        Key::S => Some(if shift { b'S' } else { b'S' }),
        Key::T => Some(if shift { b'T' } else { b'T' }),
        Key::U => Some(if shift { b'U' } else { b'U' }),
        Key::V => Some(if shift { b'V' } else { b'V' }),
        Key::W => Some(if shift { b'W' } else { b'W' }),
        Key::X => Some(if shift { b'X' } else { b'X' }),
        Key::Y => Some(if shift { b'Y' } else { b'Y' }),
        Key::Z => Some(if shift { b'Z' } else { b'Z' }),
        Key::Key0 => Some(if shift { b')' } else { b'0' }),
        Key::Key1 => Some(if shift { b'!' } else { b'1' }),
        Key::Key2 => Some(if shift { b'@' } else { b'2' }),
        Key::Key3 => Some(if shift { b'#' } else { b'3' }),
        Key::Key4 => Some(if shift { b'$' } else { b'4' }),
        Key::Key5 => Some(if shift { b'%' } else { b'5' }),
        Key::Key6 => Some(if shift { b'^' } else { b'6' }),
        Key::Key7 => Some(if shift { b'&' } else { b'7' }),
        Key::Key8 => Some(if shift { b'*' } else { b'8' }),
        Key::Key9 => Some(if shift { b'(' } else { b'9' }),
        Key::Space => Some(b' '),
        Key::Enter => Some(0x0D),
        Key::Backspace => Some(0x08),
        Key::Left => Some(0x08),   // Apple II: Left = Backspace
        Key::Right => Some(0x15),  // Apple II: Right = Ctrl+U
        Key::Up => Some(0x0B),     // Apple II: Up = Ctrl+K
        Key::Down => Some(0x0A),   // Apple II: Down = Ctrl+J
        Key::Escape => Some(0x1B),
        Key::Tab => Some(0x09),
        Key::Comma => Some(if shift { b'<' } else { b',' }),
        Key::Period => Some(if shift { b'>' } else { b'.' }),
        Key::Slash => Some(if shift { b'?' } else { b'/' }),
        Key::Semicolon => Some(if shift { b':' } else { b';' }),
        Key::Apostrophe => Some(if shift { b'"' } else { b'\'' }),
        Key::LeftBracket => Some(if shift { b'{' } else { b'[' }),
        Key::RightBracket => Some(if shift { b'}' } else { b']' }),
        Key::Minus => Some(if shift { b'_' } else { b'-' }),
        Key::Equal => Some(if shift { b'+' } else { b'=' }),
        Key::Backslash => Some(if shift { b'|' } else { b'\\' }),
        Key::Backquote => Some(if shift { b'~' } else { b'`' }),
        _ => None,
    }
}

/// ディスクログレベルをパース
fn parse_disk_log_level(s: &str) -> disk_log::DiskLogLevel {
    let mut level = disk_log::DiskLogLevel::empty();
    
    for part in s.to_lowercase().split('+') {
        match part.trim() {
            "none" => {}
            "flow" => level |= disk_log::DiskLogLevel::FLOW,
            "state" => level |= disk_log::DiskLogLevel::STATE,
            "decide" => level |= disk_log::DiskLogLevel::DECIDE,
            "nibble" => level |= disk_log::DiskLogLevel::NIBBLE,
            "all" => level = disk_log::DiskLogLevel::FLOW 
                           | disk_log::DiskLogLevel::STATE 
                           | disk_log::DiskLogLevel::DECIDE 
                           | disk_log::DiskLogLevel::NIBBLE,
            _ => {}
        }
    }
    
    level
}

fn main() {
    let args = Args::parse();

    let log_filter = match args.log_level.to_lowercase().as_str() {
        "off" | "none" => "off",
        "error" => "error",
        "warn" | "warning" => "warn",
        "info" => "info",
        "debug" => "debug",
        "trace" => "trace",
        _ => "info",
    };

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(log_filter)
    ).init();
    
    // ディスクログレベルを設定
    let disk_log_level = parse_disk_log_level(&args.disk_log);
    disk_log::set_log_level(disk_log_level);
    // 起動初期段階でも設定ファイルを参照できるように読み込む
    let startup_config_path = args.config.clone();
    let startup_home_path = args.home.clone();
    let (startup_config, startup_config_file_path) = Config::load_with_options(
        startup_config_path.as_deref(),
        startup_home_path.as_deref(),
    );
    log::info!("Startup config file for ROM search: {:?}", startup_config_file_path);
    
    // クイックテストモード
    if args.quick_test {
        test_cpu::run_quick_tests();
        return;
    }
    
    // ビデオデバッグテスト
    if args.debug_video {
        debug_test::test_text_display();
        return;
    }
    
    // ROM実行デバッグテスト
    if args.debug_rom {
        debug_test::test_rom_execution();
        return;
    }
    
    // apple2dead.bin ROMテスト
    if let Some(rom_path) = args.test_dead {
        debug_test::test_apple2dead_rom(&rom_path);
        return;
    }
    
    // Klaus2m5 CPUテストモード
    if args.test_cpu {
        let test_path = "tests/6502_65C02_functional_tests-master/bin_files/6502_functional_test.bin";
        match test_cpu::run_functional_test(test_path) {
            Ok(passed) => {
                std::process::exit(if passed { 0 } else { 1 });
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
    
    // 65C02テストモード
    if args.test_65c02 {
        let test_path = "tests/6502_65C02_functional_tests-master/bin_files/65C02_extended_opcodes_test.bin";
        match test_cpu::run_65c02_test(test_path) {
            Ok(passed) => {
                std::process::exit(if passed { 0 } else { 1 });
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
    
    // ROMを先に読み込んでモデルを自動検出
    let rom_data = if let Some(ref rom_path) = args.rom {
        match rom_resolver::resolve_rom_arg(rom_path, &startup_config.rom_dir_path()) {
            Ok(resolved) => match fs::read(&resolved) {
                Ok(data) => Some(data),
                Err(e) => {
                    eprintln!("Failed to load ROM {:?}: {}", resolved, e);
                    None
                }
            },
            Err(tried) => {
                eprintln!("Failed to resolve ROM {}", rom_path);
                for p in tried { eprintln!("  searched: {:?}", p); }
                None
            }
        }
    } else {
        let rom_dir = startup_config.rom_dir_path();
        let rom_search = rom_resolver::find_rom_candidates(&rom_dir, RomKind::Main);
        rom_resolver::log_rom_search_result(RomKind::Main, &rom_dir, &rom_search);
        if let Some(found_path) = rom_search.selected {
            match fs::read(&found_path) {
                Ok(data) => {
                    log::info!("Loaded main ROM from rom_dir {:?}: {:?}", rom_dir, found_path);
                    Some(data)
                }
                Err(e) => {
                    eprintln!("Failed to read auto-detected ROM {:?}: {}", found_path, e);
                    None
                }
            }
        } else {
            None
        }
    };

    // モデルを解析（"auto"の場合はROMサイズから自動検出）
    let model = match args.model.to_lowercase().as_str() {
        "auto" => {
            if let Some(ref data) = rom_data {
                Apple2::detect_model_from_rom(data)
            } else {
                AppleModel::AppleIIPlus
            }
        }
        "ii" | "apple2" => AppleModel::AppleII,
        "ii+" | "iip" | "apple2+" | "apple2plus" => AppleModel::AppleIIPlus,
        "iie" | "apple2e" => AppleModel::AppleIIe,
        "iie-enhanced" | "iie+" | "apple2ee" => AppleModel::AppleIIeEnhanced,
        _ => {
            eprintln!("Unknown model: {}. Using Apple II+", args.model);
            AppleModel::AppleIIPlus
        }
    };

    // バナー表示
    const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
    println!("A2RS - Apple II Emulator v{} ({:?})", APP_VERSION, model);

    // エミュレータを作成
    let mut emu = Apple2::new(model);

    // Disk II Boot ROMをロード
    let disk_rom_loaded = if let Some(disk_rom_path) = args.disk_rom {
        match rom_resolver::resolve_rom_arg(&disk_rom_path, &startup_config.rom_dir_path()) {
            Ok(resolved) => match fs::read(&resolved) {
                Ok(data) => {
                    match emu.load_disk_rom(&data) {
                        Ok(()) => {
                            log::info!("Loaded Disk II Boot ROM: {:?}", resolved);
                            true
                        }
                        Err(e) => {
                            eprintln!("Failed to load Disk II Boot ROM: {}", e);
                            false
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read Disk II Boot ROM {:?}: {}", resolved, e);
                    false
                }
            },
            Err(tried) => {
                eprintln!("Failed to resolve Disk II Boot ROM {}", disk_rom_path);
                for p in tried { eprintln!("  searched: {:?}", p); }
                false
            }
        }
    } else {
        // 設定ファイルの rom_dir から、ファイル名に disk を含む *.rom / *.bin を自動探索
        let rom_dir = startup_config.rom_dir_path();
        let rom_search = rom_resolver::find_rom_candidates(&rom_dir, RomKind::Disk);
        rom_resolver::log_rom_search_result(RomKind::Disk, &rom_dir, &rom_search);
        if let Some(found_path) = rom_search.selected {
            match fs::read(&found_path) {
                Ok(data) => {
                    if let Ok(()) = emu.load_disk_rom(&data) {
                        log::info!("Loaded Disk II Boot ROM from rom_dir {:?}: {:?}", rom_dir, found_path);
                        true
                    } else {
                        eprintln!("Failed to load auto-detected Disk II Boot ROM {:?}", found_path);
                        false
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read auto-detected Disk II Boot ROM {:?}: {}", found_path, e);
                    false
                }
            }
        } else {
            false
        }
    };
    
    if !disk_rom_loaded {
        eprintln!("Note: Disk II Boot ROM not found in {:?} (VBR mode will be used for DSK files)", startup_config.rom_dir_path());
    }

    // ROMをロード
    if let Some(data) = rom_data {
        emu.load_rom(&data);
        // ROM loading message is already printed by memory.rs
    } else {
        // テスト用ROMを使用
        eprintln!(
            "No main ROM found via --rom or rom_dir {:?}. Using built-in test ROM.",
            startup_config.rom_dir_path()
        );
        let test_rom = apple2::create_test_rom();
        emu.load_rom(&test_rom);
        // Monitorスタブモードを有効化
        emu.monitor_stub_mode = true;
    }
    
    // Apple IIc + 外部Disk II ROM: メモリに再コピー（load_romで上書きされるため）
    if disk_rom_loaded {
        // disk.boot_romの内容をメモリにコピー
        let boot_rom = emu.disk.boot_rom;
        emu.memory.copy_disk_boot_rom(&boot_rom);
    }

    // ディスクをロード
    if let Some(disk1_path) = args.disk1 {
        match disk_resolver::resolve_disk_image(&disk1_path, &startup_config.disk_dir_path(), DEFAULT_DISK_EXTS) {
        Ok(resolved) => {
            match fs::read(&resolved) {
                Ok(disk_data) => {
                    match emu.load_disk(0, &disk_data) {
                        Ok(()) => log::info!("Loaded disk 1: {:?}", resolved),
                        Err(e) => eprintln!("Failed to load disk 1: {}", e),
                    }
                }
                Err(e) => eprintln!("Failed to read disk 1 {:?}: {}", resolved, e),
            }
        }
        Err(tried) => {
            eprintln!("Failed to resolve disk 1 {}", disk1_path);
            for p in tried { eprintln!("  searched: {:?}", p); }
        }
        }
    }

    if let Some(disk2_path) = args.disk2 {
        match disk_resolver::resolve_disk_image(&disk2_path, &startup_config.disk_dir_path(), DEFAULT_DISK_EXTS) {
        Ok(resolved) => {
            match fs::read(&resolved) {
                Ok(disk_data) => {
                    match emu.load_disk(1, &disk_data) {
                        Ok(()) => log::info!("Loaded disk 2: {:?}", resolved),
                        Err(e) => eprintln!("Failed to load disk 2: {}", e),
                    }
                }
                Err(e) => eprintln!("Failed to read disk 2 {:?}: {}", resolved, e),
            }
        }
        Err(tried) => {
            eprintln!("Failed to resolve disk 2 {}", disk2_path);
            for p in tried { eprintln!("  searched: {:?}", p); }
        }
        }
    }

    // リセット
    emu.reset();

    // ディスク高速化を設定（デフォルトはオン）
    emu.set_fast_disk(true);

    // WOZ/NIBディスクはコピープロテクトのために正確な回転タイミングが必要
    // Strictシーケンサモードへ自動切り替え
    emu.disk.ensure_woz_sequencer_mode();

    // 起動ブーストログを設定
    if args.boost_log {
        emu.boost_log = true;
        log::info!("Boot boost logging enabled");
    }

    if args.headless {
        run_headless(&mut emu, args.cycles);
    } else {
        // ウィンドウサイズをパース
        let (width, height) = parse_size(&args.size).unwrap_or((640, 480));
        let profile_opts = ProfileOptions {
            enabled: args.profile,
            output: args.profile_output.clone(),
            interval: args.profile_interval,
            boot_only: args.profile_boot,
        };
        run_with_window(
            &mut emu,
            args.speed,
            width,
            height,
            args.fullscreen,
            profile_opts,
            args.config.clone(),
            args.home.clone(),
            args.boost_log,
            args.timing_log,
        );
    }
}

fn parse_size(s: &str) -> Option<(usize, usize)> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() == 2 {
        let w = parts[0].parse().ok()?;
        let h = parts[1].parse().ok()?;
        Some((w, h))
    } else {
        None
    }
}

fn run_headless(emu: &mut Apple2, cycles: u64) {
    let start = Instant::now();
    emu.run_cycles(cycles);
    let elapsed = start.elapsed();

    let mhz = (cycles as f64) / elapsed.as_secs_f64() / 1_000_000.0;
    println!("Executed {} cycles in {:?} ({:.2} MHz effective)", cycles, elapsed, mhz);
    println!("Final PC: ${:04X}", emu.cpu.regs.pc);
}

/// プロファイラオプション
struct ProfileOptions {
    enabled: bool,
    output: Option<String>,
    interval: u64,
    boot_only: bool,
}

fn run_with_window(emu: &mut Apple2, speed_override: Option<u32>, init_width: usize, init_height: usize, fullscreen: bool, profile_opts: ProfileOptions, config_path: Option<String>, home_path: Option<String>, boost_log_enabled: bool, timing_log_enabled: bool) {
    // 初期ウィンドウサイズ
    // GUI用にツールバーとステータスバーの高さを考慮したウィンドウサイズ
    let gui_height = TOOLBAR_HEIGHT + STATUSBAR_HEIGHT;
    let init_window_width: usize = init_width;
    let init_window_height: usize = init_height + gui_height;
    
    let mut window = match Window::new(
        "A2RS - Apple II Emulator",
        init_window_width,
        init_window_height,
        WindowOptions {
            resize: true,
            scale: minifb::Scale::X1,
            borderless: fullscreen,
            ..WindowOptions::default()
        },
    ) {
        Ok(win) => win,
        Err(e) => {
            eprintln!("Failed to create window: {}", e);
            return;
        }
    };

    window.set_target_fps(60);
    
    // GUI初期化
    let mut gui = Gui::new();
    gui.fullscreen = fullscreen;
    
    // デバッガパネル初期化
    let mut debugger_panel = DebuggerPanel::new();
    let mut video_timing_overlay = false;
    let mut disk_realism_overlay = false;
    let mut runtime_debug_overlay = false;
    let mut runtime_single_step_requested = false;
    
    // プロファイラとデバッガ初期化
    let mut profiler = Profiler::new();
    let mut debugger = Debugger::new();
    profiler.enabled = profile_opts.enabled;
    profiler.start_boot();
    
    // プロファイラファイル出力設定
    let profile_output = profile_opts.output.clone();
    let profile_interval = Duration::from_secs(profile_opts.interval);
    let profile_boot_only = profile_opts.boot_only;
    let mut last_profile_output = Instant::now();
    
    if profile_opts.enabled {
        log::info!("Profiler enabled (output: {:?})", profile_output);
        // デバッガパネルも自動で表示
        debugger_panel.visible = true;
    }
    
    // スケーリング用バッファ（動的にリサイズ）
    let mut scaled_buffer = vec![0u32; init_window_width * init_window_height];
    let mut current_window_width = init_window_width;
    let mut current_window_height = init_window_height;
    
    // エフェクト用バッファ
    let mut prev_frame = vec![0u32; SCREEN_WIDTH * SCREEN_HEIGHT];
    let mut effect_buffer = vec![0u32; init_window_width * init_window_height];
    
    // エフェクト設定
    let frame_blend_enabled = true;

    // 設定ファイルを読み込み（コマンドラインオプションを考慮）
    let (mut config, config_file_path) = Config::load_with_options(
        config_path.as_deref(),
        home_path.as_deref()
    );

    // 起動時の速度は、CLIで明示指定があればそれを優先し、未指定なら設定ファイルを使う。
    let configured_speed = speed_override.unwrap_or(config.speed);
    // 実行中は current_speed が変化しうるが、起動時・リセット時に戻る基準は configured_speed。
    config.speed = configured_speed;
    
    // 起動情報を表示
    println!("=== A2RS Apple II Emulator ===");
    println!("Executable dir: {:?}", get_exe_dir());
    println!("Config file: {:?}", config_file_path);
    let home_display = if config.a2rs_home.trim().is_empty() { "(default)".to_string() } else { config.a2rs_home.clone() };
    println!("A2RS Home: {} -> {:?}", home_display, config.home_dir_path());
    println!("Directories:");
    println!("  ROM:         {} -> {:?}", config.rom_dir, config.rom_dir_path());
    println!("  Disks:       {} -> {:?}", config.disk_dir, config.disk_dir_path());
    println!("  Screenshots: {} -> {:?}", config.screenshot_dir, config.screenshot_dir_path());
    println!("  Saves:       {} -> {:?}", config.save_dir, config.save_dir_path());
    println!("Edit config file to change directories.");
    println!("Experimental disk mode: {} (weak_bits={}, write_splice={}, disk_debug_logging={})",
        config.experimental.disk_sequencer_mode,
        config.experimental.weak_bits,
        config.experimental.write_splice,
        config.experimental.disk_debug_logging);
    println!();
    
    // エミュレータ一時停止フラグ
    let mut paused = false;
    
    // カーソル関連
    let mut last_mouse_pos: (f32, f32) = (0.0, 0.0);
    let mut last_mouse_move = Instant::now();
    let mut cursor_visible = true;

    // ウィンドウドラッグ関連
    let mut window_drag_pending = false;
    let mut window_dragging = false;
    let mut drag_start_pointer: (i32, i32) = (0, 0);
    let mut drag_start_window: (isize, isize) = window.get_position();
    let mut drag_helper = linux_window_drag::DragHelper::new();

    let base_frame_duration = Duration::from_micros(16667); // 60 FPS
    const APPLE2_CPU_HZ: f64 = 1_023_000.0;
    const STEP_CHUNK_CYCLES: u64 = 256;
    const MAX_CATCHUP_SECONDS: f64 = 0.25;
    let mut prev_keys: Vec<Key> = Vec::new();
    let mut current_speed = configured_speed;
    let fast_disk_enabled = true;  // 高速ディスクは常にON
    emu.set_fast_disk(true);
    let seq_mode = match config.experimental.disk_sequencer_mode.as_str() {
        "strict" => DiskSequencerMode::Strict,
        "transitional" => DiskSequencerMode::Transitional,
        _ => DiskSequencerMode::Safe,
    };
    emu.configure_experimental_disk(
        seq_mode,
        config.experimental.weak_bits,
        config.experimental.write_splice,
        config.experimental.disk_debug_logging,
    );
    // WOZ/NIBディスクはコピープロテクトのために正確な回転タイミングが必要
    // config適用後にStrictモードへ自動切り替え
    emu.disk.ensure_woz_sequencer_mode();

    // Phase 2: AccurateBoost
    // ディスク回転中は emu.run_frame() の回数を増やし、CPUとディスクを同じ仮想時間軸で前進させる。
    // これにより「ただの早送り」ではなく、低レベル挙動を保ったまま体感速度を改善する。
    let accurate_boost_enabled = true;
    let mut accurate_boost_active = false;
    let mut accurate_boost_multiplier: u32 = 1;
    let mut last_boost_debug_log = Instant::now();

    // 起動ブースト: MAX指定時のみ有効
    let disk_loaded = emu.disk.drives[0].disk.disk_loaded;
    // 起動時の表示速度は常にユーザー設定値を使用する。
    // boot_boost_active は内部の加速判定にのみ使い、current_speed は上書きしない。
    let mut boot_boost_active = (configured_speed == 0) && disk_loaded;
    
    // オーディオ出力を初期化
    let mut audio_output = match AudioOutput::new() {
        Ok(audio) => Some(audio),
        Err(e) => {
            log::warn!("Audio initialization failed: {}", e);
            None
        }
    };
    let mut speaker = Speaker::new();
    speaker.set_volume(config.volume);
    let mut sound_enabled = config.sound_enabled;
    
    // GUIの音量も設定から初期化
    gui.set_volume(config.volume);
    
    // フレームレート計測用
    let mut frame_times: [f64; 60] = [16.667; 60]; // 過去60フレームの時間(ms)
    let mut frame_time_index = 0;
    let mut last_fps_update = Instant::now();
    let mut displayed_fps = 60.0;
    let mut timing_last_host = Instant::now();
    let mut cycle_accumulator = 0.0f64;
    let mut timing_stats_last_log = Instant::now();
    let mut timing_stats_window_start = Instant::now();
    let mut timing_stats_cycles: u64 = 0;
    let mut timing_stats_frames: u64 = 0;
    let mut suppress_timing_log_until: Option<Instant> = None;
    let mut next_video_cycle = emu.total_cycles + Apple2::CYCLES_PER_FRAME;
    
    // 適応的品質調整（0-4の5段階）
    let mut quality_level: i32 = config.quality_level.clamp(0, 4);
    let mut auto_quality = config.auto_quality;
    let mut high_fps_seconds = 0u32; // FPSが高い状態が続いた秒数
    
    // セーブスロット（0-9）
    let mut current_slot: u8 = config.current_slot;
    
    // ゲームパッド初期化
    let mut gamepad_manager = if config.gamepad.enabled {
        match GamepadManager::new(config.gamepad.clone()) {
            Ok(gp) => Some(gp),
            Err(e) => {
                log::debug!("Gamepad not available: {}", e);
                None
            }
        }
    } else {
        log::debug!("Gamepad disabled by config");
        None
    };

    while window.is_open() && emu.running {
        let frame_start = Instant::now();
        
        // ウィンドウサイズの変更を検出
        let (win_w, win_h) = window.get_size();
        if win_w != current_window_width || win_h != current_window_height {
            current_window_width = win_w;
            current_window_height = win_h;
            scaled_buffer.resize(win_w * win_h, 0);
            effect_buffer.resize(win_w * win_h, 0);
        }
        
        // マウス処理
        let mouse_pos = window.get_mouse_pos(MouseMode::Clamp);
        if let Some((mx, my)) = mouse_pos {
            // マウス移動検出
            if (mx - last_mouse_pos.0).abs() > 1.0 || (my - last_mouse_pos.1).abs() > 1.0 {
                last_mouse_pos = (mx, my);
                last_mouse_move = Instant::now();
                if !cursor_visible {
                    window.set_cursor_visibility(true);
                    cursor_visible = true;
                }
            }
            gui.update_mouse(mx, my);
        }
        
        // 5秒経過でカーソル非表示
        if cursor_visible && last_mouse_move.elapsed() > Duration::from_secs(5) {
            window.set_cursor_visibility(false);
            cursor_visible = false;
        }
        
        // マウスクリック検出
        let mouse_clicked = window.get_mouse_down(MouseButton::Left);
        static mut MOUSE_WAS_DOWN: bool = false;
        let click_event = unsafe {
            let was_down = MOUSE_WAS_DOWN;
            MOUSE_WAS_DOWN = mouse_clicked;
            mouse_clicked && !was_down
        };
        
        // 音量スライダーのドラッグ処理
        if gui.volume_dragging {
            if mouse_clicked {
                if gui.update_volume_from_mouse(current_window_width) {
                    speaker.set_volume(gui.get_volume());
                }
            } else {
                gui.end_volume_drag();
            }
        }

        // ウィンドウドラッグ処理
        if gui.fullscreen || gui.overlay_visible || gui.is_disk_menu_open() || gui.volume_dragging {
            window_drag_pending = false;
            window_dragging = false;
        } else if mouse_clicked {
            if !window_drag_pending && !window_dragging && gui.is_in_toolbar_drag_zone(current_window_width) {
                window_drag_pending = true;
                drag_start_window = window.get_position();
                drag_start_pointer = drag_helper
                    .as_mut()
                    .and_then(|h| h.global_pointer_position())
                    .unwrap_or((gui.mouse_x.round() as i32, gui.mouse_y.round() as i32));
            }

            if window_drag_pending || window_dragging {
                let current_pointer = drag_helper
                    .as_mut()
                    .and_then(|h| h.global_pointer_position())
                    .unwrap_or((gui.mouse_x.round() as i32, gui.mouse_y.round() as i32));
                let dx = current_pointer.0 - drag_start_pointer.0;
                let dy = current_pointer.1 - drag_start_pointer.1;

                if !window_dragging && (dx.abs() >= 3 || dy.abs() >= 3) {
                    window_dragging = true;
                }

                if window_dragging {
                    window.set_position(
                        drag_start_window.0 + dx as isize,
                        drag_start_window.1 + dy as isize,
                    );
                }
            }
        } else {
            window_drag_pending = false;
            window_dragging = false;
        }
        
        if click_event && !gui.fullscreen && !window_dragging {
            // 音量スライダーのクリック
            if gui.is_over_volume_slider(current_window_width) {
                gui.start_volume_drag(current_window_width);
                speaker.set_volume(gui.get_volume());
            }
            // ディスクメニューが開いている場合は、メニュー内クリックを優先
            else if gui.is_disk_menu_open() {
                if let Some((drive, action)) = gui.disk_menu_click(current_window_width, current_window_height) {
                    speaker.trigger_ui_click();  // 選択決定音
                    match action {
                        DiskMenuAction::Eject => {
                            match emu.disk.eject_disk_with_flush(drive) {
                                Ok(Some(path)) => println!("Saved and ejected disk from drive {} -> {}", drive + 1, path),
                                Ok(None) => println!("Ejected disk from drive {}", drive + 1),
                                Err(err) => eprintln!("Failed to eject disk from drive {}: {}", drive + 1, err),
                            }
                        }
                        DiskMenuAction::InsertDisk(index) => {
                            if let Some(disk_path) = gui.available_disks.get(index) {
                                let path = disk_path.clone();
                                if let Ok(data) = fs::read(&path) {
                                    let path_lower = path.to_lowercase();
                                    if path_lower.ends_with(".woz") {
                                        match woz::parse_woz(&data) {
                                            Ok(result) => {
                                                if emu.disk.insert_disk_with_name(drive, &result.nib_data, disk::DiskFormat::Woz, Some(path.clone())).is_ok() {
                                                    emu.disk.drives[drive].disk.track_nibble_counts = Some(result.track_nibble_counts);
                                                    emu.disk.drives[drive].disk.woz_bitstreams = Some(result.bitstreams);
                                                    emu.disk.drives[drive].disk.woz_bit_counts = Some(result.bit_counts);
                                                    emu.disk.ensure_woz_sequencer_mode();
                                                    println!("Inserted WOZ {} into drive {}", path, drive + 1);
                                                }
                                            }
                                            Err(e) => eprintln!("Failed to parse WOZ {}: {}", path, e),
                                        }
                                    } else {
                                        let format = if path_lower.ends_with(".po") {
                                            disk::DiskFormat::Po
                                        } else if path_lower.ends_with(".nib") {
                                            disk::DiskFormat::Nib
                                        } else {
                                            disk::DiskFormat::Dsk
                                        };
                                        if emu.disk.insert_disk_with_name(drive, &data, format, Some(path.clone())).is_ok() {
                                            println!("Inserted {} into drive {}", path, drive + 1);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // メニュー外クリックでキャンセル（クリック音なし、静かにキャンセル）
                }
            } else if let Some(btn) = gui.mouse_click() {
                match btn {
                    ToolbarButton::PlayPause => {
                        paused = !paused;
                        gui.trigger_button_highlight(btn);
                        speaker.trigger_ui_click();
                    }
                    ToolbarButton::Reset => {
                        emu.reset();
                        current_speed = configured_speed;
                        timing_last_host = Instant::now();
                        cycle_accumulator = 0.0;
                        next_video_cycle = emu.total_cycles + Apple2::CYCLES_PER_FRAME;
                        timing_stats_last_log = Instant::now();
                        timing_stats_window_start = timing_stats_last_log;
                        timing_stats_cycles = 0;
                        timing_stats_frames = 0;
                        suppress_timing_log_until = Some(timing_stats_last_log + Duration::from_secs(1));
                        gui.trigger_reset_highlight();
                        speaker.trigger_reset_sound();
                        // リセット時のブースト再開はMAX指定時のみ
                        if current_speed == 0 && emu.disk.drives[0].disk.disk_loaded {
                            boot_boost_active = true;
                        } else {
                            boot_boost_active = false;
                        }
                    }
                    ToolbarButton::Disk1 => {
                        let disks = get_available_disks(&config);
                        let current_filename = emu.disk.drives[0].disk.filename.clone();
                        gui.open_disk_menu_at_current(0, disks, current_filename);
                    }
                    ToolbarButton::Disk2 => {
                        let disks = get_available_disks(&config);
                        let current_filename = emu.disk.drives[1].disk.filename.clone();
                        gui.open_disk_menu_at_current(1, disks, current_filename);
                    }
                    ToolbarButton::SwapDisks => {
                        emu.disk.swap_disks();
                        gui.trigger_button_highlight(btn);
                        speaker.trigger_ui_click();
                    }
                    ToolbarButton::QuickSave => {
                        // セーブディレクトリを作成（実行ファイルからの相対パス）
                        let save_dir = config.save_dir_path();
                        let _ = fs::create_dir_all(&save_dir);
                        let state = emu.save_state();
                        let filepath = SaveSlots::get_path_in(&config.save_dir_path(), current_slot);
                        if let Ok(json) = serde_json::to_string(&state) {
                            if let Ok(_) = std::fs::write(&filepath, &json) {
                                println!("Saved to slot {} ({:?})", current_slot, filepath);
                                gui.trigger_button_highlight(btn);
                                speaker.trigger_ui_click();
                            }
                        }
                    }
                    ToolbarButton::QuickLoad => {
                        let filepath = SaveSlots::get_path_in(&config.save_dir_path(), current_slot);
                        if let Ok(json) = std::fs::read_to_string(&filepath) {
                            if let Ok(state) = serde_json::from_str(&json) {
                                if let Ok(_) = emu.load_state(&state) {
                                    emu.render_video();
                                    timing_last_host = Instant::now();
                                    cycle_accumulator = 0.0;
                                    next_video_cycle = emu.total_cycles + Apple2::CYCLES_PER_FRAME;
                                    println!("Loaded from slot {} ({:?})", current_slot, filepath);
                                    gui.trigger_button_highlight(btn);
                                    speaker.trigger_ui_click();
                                }
                            }
                        }
                    }
                    ToolbarButton::Screenshot => {
                        // スクリーンショットディレクトリを作成（実行ファイルからの相対パス）
                        let screenshot_dir = config.screenshot_dir_path();
                        let _ = fs::create_dir_all(&screenshot_dir);
                        let filename = screenshot_dir.join(screenshot_filename());
                        let fb = emu.get_framebuffer();
                        if let Some(filename_str) = filename.to_str() {
                            if save_screenshot(filename_str, fb, SCREEN_WIDTH, SCREEN_HEIGHT).is_ok() {
                                println!("Screenshot saved: {}", filename_str);
                                gui.trigger_button_highlight(btn);
                                speaker.trigger_ui_click();
                            }
                        }
                    }
                    ToolbarButton::Fullscreen => {
                        // 全画面モードは削除（機能無効化）
                    }
                }
            }
        }
        
        // ESCでメニューを閉じる（オーバーレイを開くのはF1）
        if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
            if gui.is_disk_menu_open() {
                gui.close_disk_menu();
                // キャンセル時はクリック音なし
            } else if gui.overlay_visible {
                gui.toggle_overlay();
                // オーバーレイを閉じる
            }
            // それ以外の場合はApple IIにESCキーを送信（key_to_apple2で処理される）
        }
        
        // ディスクメニュー操作
        if gui.is_disk_menu_open() {
            if window.is_key_pressed(Key::Up, KeyRepeat::Yes) {
                gui.disk_menu_up();
            }
            if window.is_key_pressed(Key::Down, KeyRepeat::Yes) {
                gui.disk_menu_down();
            }
            if window.is_key_pressed(Key::Enter, KeyRepeat::No) {
                if let Some((drive, action)) = gui.disk_menu_select() {
                    speaker.trigger_ui_click();  // 選択決定音
                    match action {
                        DiskMenuAction::Eject => {
                            match emu.disk.eject_disk_with_flush(drive) {
                                Ok(Some(path)) => println!("Saved and ejected disk from drive {} -> {}", drive + 1, path),
                                Ok(None) => println!("Ejected disk from drive {}", drive + 1),
                                Err(err) => eprintln!("Failed to eject disk from drive {}: {}", drive + 1, err),
                            }
                        }
                        DiskMenuAction::InsertDisk(index) => {
                            if let Some(disk_path) = gui.available_disks.get(index) {
                                let path = disk_path.clone();
                                if let Ok(data) = fs::read(&path) {
                                    let path_lower = path.to_lowercase();
                                    if path_lower.ends_with(".woz") {
                                        match woz::parse_woz(&data) {
                                            Ok(result) => {
                                                if emu.disk.insert_disk_with_name(drive, &result.nib_data, disk::DiskFormat::Woz, Some(path.clone())).is_ok() {
                                                    emu.disk.drives[drive].disk.track_nibble_counts = Some(result.track_nibble_counts);
                                                    emu.disk.drives[drive].disk.woz_bitstreams = Some(result.bitstreams);
                                                    emu.disk.drives[drive].disk.woz_bit_counts = Some(result.bit_counts);
                                                    emu.disk.ensure_woz_sequencer_mode();
                                                    println!("Inserted WOZ {} into drive {}", path, drive + 1);
                                                }
                                            }
                                            Err(e) => eprintln!("Failed to parse WOZ {}: {}", path, e),
                                        }
                                    } else {
                                        let format = if path_lower.ends_with(".po") {
                                            disk::DiskFormat::Po
                                        } else if path_lower.ends_with(".nib") {
                                            disk::DiskFormat::Nib
                                        } else {
                                            disk::DiskFormat::Dsk
                                        };
                                        if emu.disk.insert_disk_with_name(drive, &data, format, Some(path.clone())).is_ok() {
                                            println!("Inserted {} into drive {}", path, drive + 1);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // オーバーレイ操作
        else if gui.overlay_visible {
            if window.is_key_pressed(Key::Up, KeyRepeat::Yes) && !gui.is_text_input_mode() {
                gui.overlay_up();
            }
            if window.is_key_pressed(Key::Down, KeyRepeat::Yes) && !gui.is_text_input_mode() {
                gui.overlay_down();
            }
            
            // テキスト入力モード中
            if gui.is_text_input_mode() {
                let ctrl = window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl);
                
                // Ctrl+V でクリップボードからペースト
                if ctrl && window.is_key_pressed(Key::V, KeyRepeat::No) {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if let Ok(text) = clipboard.get_text() {
                            // パス文字のみをフィルタリングして追加
                            for c in text.chars() {
                                if c.is_alphanumeric() || c == '/' || c == '\\' || c == ':' 
                                    || c == '.' || c == '-' || c == '_' || c == ' ' {
                                    gui.text_input_char(c);
                                }
                            }
                        }
                    }
                }
                // バックスペース
                else if window.is_key_pressed(Key::Backspace, KeyRepeat::Yes) {
                    gui.text_input_backspace();
                }
                // Enter で確定
                else if window.is_key_pressed(Key::Enter, KeyRepeat::No) {
                    if let Some((item, value)) = gui.end_text_input() {
                        match item {
                            3 => config.a2rs_home = value,
                            4 => config.rom_dir = value,
                            5 => config.disk_dir = value,
                            6 => config.screenshot_dir = value,
                            7 => config.save_dir = value,
                            _ => {}
                        }
                        config.ensure_directories();
                    }
                }
                // Escape でキャンセル
                else if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
                    gui.cancel_text_input();
                }
                // 文字入力（Ctrl押下中は無視）
                else if !ctrl {
                    for key in window.get_keys() {
                        let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
                        let ch = match key {
                            Key::A => Some(if shift { 'A' } else { 'a' }),
                            Key::B => Some(if shift { 'B' } else { 'b' }),
                            Key::C => Some(if shift { 'C' } else { 'c' }),
                            Key::D => Some(if shift { 'D' } else { 'd' }),
                            Key::E => Some(if shift { 'E' } else { 'e' }),
                            Key::F => Some(if shift { 'F' } else { 'f' }),
                            Key::G => Some(if shift { 'G' } else { 'g' }),
                            Key::H => Some(if shift { 'H' } else { 'h' }),
                            Key::I => Some(if shift { 'I' } else { 'i' }),
                            Key::J => Some(if shift { 'J' } else { 'j' }),
                            Key::K => Some(if shift { 'K' } else { 'k' }),
                            Key::L => Some(if shift { 'L' } else { 'l' }),
                            Key::M => Some(if shift { 'M' } else { 'm' }),
                            Key::N => Some(if shift { 'N' } else { 'n' }),
                            Key::O => Some(if shift { 'O' } else { 'o' }),
                            Key::P => Some(if shift { 'P' } else { 'p' }),
                            Key::Q => Some(if shift { 'Q' } else { 'q' }),
                            Key::R => Some(if shift { 'R' } else { 'r' }),
                            Key::S => Some(if shift { 'S' } else { 's' }),
                            Key::T => Some(if shift { 'T' } else { 't' }),
                            Key::U => Some(if shift { 'U' } else { 'u' }),
                            Key::V => Some(if shift { 'V' } else { 'v' }),
                            Key::W => Some(if shift { 'W' } else { 'w' }),
                            Key::X => Some(if shift { 'X' } else { 'x' }),
                            Key::Y => Some(if shift { 'Y' } else { 'y' }),
                            Key::Z => Some(if shift { 'Z' } else { 'z' }),
                            Key::Key0 => Some('0'), Key::Key1 => Some('1'), Key::Key2 => Some('2'),
                            Key::Key3 => Some('3'), Key::Key4 => Some('4'), Key::Key5 => Some('5'),
                            Key::Key6 => Some('6'), Key::Key7 => Some('7'), Key::Key8 => Some('8'),
                            Key::Key9 => Some('9'),
                            Key::Minus => Some(if shift { '_' } else { '-' }),
                            Key::Period => Some('.'),
                            Key::Slash => Some('/'), Key::Backslash => Some('\\'),
                            Key::Semicolon => Some(if shift { ':' } else { ';' }),
                            Key::Space => Some(' '),
                            _ => None,
                        };
                        if let Some(c) = ch {
                            // 前のフレームで押されていなければ入力
                            static mut LAST_CHAR: Option<char> = None;
                            unsafe {
                                if LAST_CHAR != Some(c) {
                                    gui.text_input_char(c);
                                    LAST_CHAR = Some(c);
                                }
                            }
                        }
                    }
                }
                // キーが離されたらリセット
                if window.get_keys().is_empty() {
                    unsafe {
                        static mut LAST_CHAR: Option<char> = None;
                        LAST_CHAR = None;
                    }
                }
            } else if window.is_key_pressed(Key::Enter, KeyRepeat::No) {
                // メニュー項目の操作
                match gui.overlay_selection {
                    0 => { // Speed
                        current_speed = match current_speed {
                            0 => 1, 1 => 2, 2 => 5, 5 => 10, 10 => 0, _ => 1
                        };
                        timing_last_host = Instant::now();
                        cycle_accumulator = 0.0;
                        next_video_cycle = emu.total_cycles + Apple2::CYCLES_PER_FRAME;
                        timing_stats_last_log = Instant::now();
                        timing_stats_window_start = timing_stats_last_log;
                        timing_stats_cycles = 0;
                        timing_stats_frames = 0;
                        suppress_timing_log_until = Some(timing_stats_last_log + Duration::from_secs(1));
                    }
                    1 => { // Quality (was Fast Disk, now Quality)
                        quality_level = (quality_level + 1) % 5;
                    }
                    2 => { // Auto Quality
                        auto_quality = !auto_quality;
                    }
                    3 => { // A2RS Home
                        gui.start_text_input(3, &config.a2rs_home);
                    }
                    4 => { // ROM Dir
                        gui.start_text_input(4, &config.rom_dir);
                    }
                    5 => { // Disk Dir
                        gui.start_text_input(5, &config.disk_dir);
                    }
                    6 => { // Screenshot Dir
                        gui.start_text_input(6, &config.screenshot_dir);
                    }
                    7 => { // Save Dir
                        gui.start_text_input(7, &config.save_dir);
                    }
                    _ => {}
                }
            }
        }
        
        let ctrl = window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl);

        // F1で設定メニュー表示/非表示
        if window.is_key_pressed(Key::F1, KeyRepeat::No) {
            gui.toggle_overlay();
            speaker.trigger_ui_click();
        }
        
        // F2でスピード変更
        if window.is_key_pressed(Key::F2, KeyRepeat::No) {
            current_speed = match current_speed {
                1 => 2,
                2 => 5,
                5 => 10,
                10 => 0,  // MAX
                _ => 1,   // 0(MAX)や他の値から1に戻る
            };
            let speed_str = if current_speed == 0 { "MAX".to_string() } else { format!("x{}", current_speed) };
            println!("Speed: {}", speed_str);
            timing_last_host = Instant::now();
            cycle_accumulator = 0.0;
            next_video_cycle = emu.total_cycles + Apple2::CYCLES_PER_FRAME;
            timing_stats_last_log = Instant::now();
            timing_stats_window_start = timing_stats_last_log;
            timing_stats_cycles = 0;
            timing_stats_frames = 0;
            suppress_timing_log_until = Some(timing_stats_last_log + Duration::from_secs(1));
        }
        
        // F11でデバッガパネル表示切り替え
        if !ctrl && window.is_key_pressed(Key::F11, KeyRepeat::No) {
            debugger_panel.toggle();
            println!("Debugger panel: {}", if debugger_panel.visible { "ON" } else { "OFF" });
        }
        
        // Ctrl+F8: ランタイムデバッグオーバーレイ
        if ctrl && window.is_key_pressed(Key::F8, KeyRepeat::No) {
            runtime_debug_overlay = !runtime_debug_overlay;
            println!("Runtime debug overlay: {}", if runtime_debug_overlay { "ON" } else { "OFF" });
        }

        // Ctrl+F10: 一時停止/再開
        if ctrl && window.is_key_pressed(Key::F10, KeyRepeat::No) {
            paused = !paused;
            if paused {
                debugger.pause();
            } else {
                debugger.resume();
                timing_last_host = Instant::now();
                cycle_accumulator = 0.0;
                next_video_cycle = emu.total_cycles + Apple2::CYCLES_PER_FRAME;
            }
            println!("Debugger runtime: {}", if paused { "PAUSED" } else { "RUNNING" });
        }

        // Ctrl+F11: 単命令ステップ
        if ctrl && window.is_key_pressed(Key::F11, KeyRepeat::No) {
            paused = true;
            debugger.pause();
            runtime_single_step_requested = true;
        }

        if window.is_key_pressed(Key::F12, KeyRepeat::No) {
            emu.reset();
            current_speed = configured_speed;
            gui.trigger_reset_highlight();
            speaker.trigger_reset_sound();
            profiler.reset();
            debugger.reset();
            profiler.start_boot();
            timing_last_host = Instant::now();
            cycle_accumulator = 0.0;
            next_video_cycle = emu.total_cycles + Apple2::CYCLES_PER_FRAME;
            timing_stats_last_log = Instant::now();
            timing_stats_window_start = timing_stats_last_log;
            timing_stats_cycles = 0;
            timing_stats_frames = 0;
            suppress_timing_log_until = Some(timing_stats_last_log + Duration::from_secs(1));
            // リセット時のブースト再開はMAX指定時のみ
            if current_speed == 0 && emu.disk.drives[0].disk.disk_loaded {
                boot_boost_active = true;
            } else {
                boot_boost_active = false;
            }
        }
        
        // F3で品質切り替え（自動/手動）
        if window.is_key_pressed(Key::F3, KeyRepeat::No) {
            if auto_quality {
                // 自動→手動に切り替え
                auto_quality = false;
                let quality_name = match quality_level {
                    0 => "Lowest",
                    1 => "Low",
                    2 => "Medium",
                    3 => "High",
                    _ => "Ultra",
                };
                println!("Quality: Manual mode (current: {})", quality_name);
            } else {
                // 手動で品質を切り替え（0-4の5段階）
                quality_level = (quality_level + 1) % 5;
                let quality_name = match quality_level {
                    0 => "Lowest (fastest)",
                    1 => "Low",
                    2 => "Medium",
                    3 => "High",
                    _ => "Ultra",
                };
                println!("Quality: {}", quality_name);
            }
        }
        
        // F4で自動品質調整ON/OFF
        if window.is_key_pressed(Key::F4, KeyRepeat::No) {
            auto_quality = !auto_quality;
            println!("Auto quality: {}", if auto_quality { "ON" } else { "OFF" });
        }
        
        // デバッガパネルが表示中の場合のキー処理
        if debugger_panel.visible {
            // Tabでタブ切り替え（次のタブへ）
            if window.is_key_pressed(Key::Tab, KeyRepeat::No) {
                debugger_panel.next_tab();
            }
            
            // メモリタブでのスクロール
            if debugger_panel.current_tab == gui::DebuggerTab::Memory {
                if window.is_key_pressed(Key::Up, KeyRepeat::Yes) {
                    debugger_panel.memory_offset = debugger_panel.memory_offset.saturating_sub(0x80);
                }
                if window.is_key_pressed(Key::Down, KeyRepeat::Yes) {
                    debugger_panel.memory_offset = debugger_panel.memory_offset.saturating_add(0x80);
                }
                if window.is_key_pressed(Key::PageUp, KeyRepeat::No) {
                    debugger_panel.memory_offset = debugger_panel.memory_offset.saturating_sub(0x400);
                }
                if window.is_key_pressed(Key::PageDown, KeyRepeat::No) {
                    debugger_panel.memory_offset = debugger_panel.memory_offset.saturating_add(0x400);
                }
            }
            
            // F6: ステップ実行
            if window.is_key_pressed(Key::F6, KeyRepeat::No) {
                paused = true;
                debugger.pause();
                runtime_single_step_requested = true;
            }
            
            // F7: 継続
            if window.is_key_pressed(Key::F7, KeyRepeat::No) {
                debugger.resume();
                paused = false;
            }
            
            // F8: ブレーク
            if window.is_key_pressed(Key::F8, KeyRepeat::No) {
                debugger.pause();
                paused = true;
            }
        } else {
            // デバッガパネル非表示時のF6/F8
            // F6でサウンドON/OFF
            if window.is_key_pressed(Key::F6, KeyRepeat::No) {
                sound_enabled = !sound_enabled;
                speaker.set_enabled(sound_enabled);
                println!("Sound: {}", if sound_enabled { "ON" } else { "OFF" });
            }

            // F7 / Ctrl+F7 overlay toggle
            if window.is_key_pressed(Key::F7, KeyRepeat::No) {
                let ctrl_overlay = window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl);
                if ctrl_overlay {
                    disk_realism_overlay = !disk_realism_overlay;
                    println!("Disk realism overlay: {}", if disk_realism_overlay { "ON" } else { "OFF" });
                } else {
                    video_timing_overlay = !video_timing_overlay;
                    println!("Video timing overlay: {}", if video_timing_overlay { "ON" } else { "OFF" });
                }
            }
        
            // F8でセーブスロット選択（循環）
            if !ctrl && window.is_key_pressed(Key::F8, KeyRepeat::No) {
                current_slot = (current_slot + 1) % 10;
                let exists = SaveSlots::exists_in(&config.save_dir_path(), current_slot);
                println!("Save slot: {} {}", current_slot, if exists { "(has data)" } else { "(empty)" });
            }
        } // デバッガパネル非表示時のif文終了
        
        // Ctrl+0-9でスロット直接選択
        let ctrl = window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl);
        if ctrl {
            for (i, key) in [Key::Key0, Key::Key1, Key::Key2, Key::Key3, Key::Key4,
                             Key::Key5, Key::Key6, Key::Key7, Key::Key8, Key::Key9].iter().enumerate() {
                if window.is_key_pressed(*key, KeyRepeat::No) {
                    current_slot = i as u8;
                    let exists = SaveSlots::exists_in(&config.save_dir_path(), current_slot);
                    println!("Save slot: {} {}", current_slot, if exists { "(has data)" } else { "(empty)" });
                }
            }
        }
        
        // F5でセーブ（現在のスロットに）
        if window.is_key_pressed(Key::F5, KeyRepeat::No) {
            let save_dir = config.save_dir_path();
            let _ = fs::create_dir_all(&save_dir);
            let state = emu.save_state();
            let filepath = SaveSlots::get_path_in(&config.save_dir_path(), current_slot);
            match serde_json::to_string(&state) {
                Ok(json) => {
                    match std::fs::write(&filepath, &json) {
                        Ok(_) => {
                            println!("State saved to slot {} ({:?})", current_slot, filepath);
                        }
                        Err(e) => println!("Failed to save state: {}", e),
                    }
                }
                Err(e) => println!("Failed to serialize state: {}", e),
            }
        }
        
        // F9でロード（現在のスロットから）
        if window.is_key_pressed(Key::F9, KeyRepeat::No) {
            let filepath = SaveSlots::get_path_in(&config.save_dir_path(), current_slot);
            match std::fs::read_to_string(&filepath) {
                Ok(json) => {
                    match serde_json::from_str(&json) {
                        Ok(state) => {
                            match emu.load_state(&state) {
                                Ok(_) => {
                                    emu.render_video();
                                    timing_last_host = Instant::now();
                                    cycle_accumulator = 0.0;
                                    next_video_cycle = emu.total_cycles + Apple2::CYCLES_PER_FRAME;
                                    println!("State loaded from slot {} ({:?})", current_slot, filepath);
                                }
                                Err(e) => println!("Failed to load state: {}", e),
                            }
                        }
                        Err(e) => println!("Failed to parse state: {}", e),
                    }
                }
                Err(_) => println!("Slot {} is empty", current_slot),
            }
        }
        
        // F10でスクリーンショット
        if !ctrl && window.is_key_pressed(Key::F10, KeyRepeat::No) {
            let screenshot_dir = config.screenshot_dir_path();
            let _ = fs::create_dir_all(&screenshot_dir);
            let filename = screenshot_dir.join(screenshot_filename());
            
            let fb = emu.get_framebuffer();
            
            // PNGファイルを作成
            let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                let file = std::fs::File::create(&filename)?;
                let w = std::io::BufWriter::new(file);
                let mut encoder = png::Encoder::new(w, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32);
                encoder.set_color(png::ColorType::Rgb);
                encoder.set_depth(png::BitDepth::Eight);
                
                let mut writer = encoder.write_header()?;
                
                // RGB データを作成
                let mut rgb_data = Vec::with_capacity(SCREEN_WIDTH * SCREEN_HEIGHT * 3);
                for pixel in fb.iter() {
                    rgb_data.push(((pixel >> 16) & 0xFF) as u8); // R
                    rgb_data.push(((pixel >> 8) & 0xFF) as u8);  // G
                    rgb_data.push((pixel & 0xFF) as u8);          // B
                }
                
                writer.write_image_data(&rgb_data)?;
                Ok(())
            })();
            
            match result {
                Ok(_) => {
                    println!("Screenshot saved to {:?}", filename);
                }
                Err(e) => println!("Failed to save screenshot: {}", e),
            }
        }
        
        // F11の古い処理を削除（GUIで処理済み）

        let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
        let ctrl = window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl);
        
        // メニューが開いている間はエミュレータへのキー入力をブロック
        let menu_open = gui.is_disk_menu_open() || gui.overlay_visible;
        
        // 現在押されているキーを取得
        // 矢印キーはジョイスティック（パドル）専用とし、キーボードストローブには送らない。
        // ストローブに残った矢印キーコードがゲームのキーボード判定を妨害するため。
        let current_keys: Vec<Key> = if menu_open {
            Vec::new()
        } else {
            window.get_keys()
                .iter()
                .filter(|k| {
                    !matches!(k, Key::Up | Key::Down | Key::Left | Key::Right)
                        && key_to_apple2(**k, false, false).is_some()
                })
                .copied()
                .collect()
        };

        // 新しく押されたキーを検出（前フレームには押されていなかったキー）
        for key in &current_keys {
            if !prev_keys.contains(key) {
                if let Some(ch) = key_to_apple2(*key, shift, ctrl) {
                    emu.key_down(ch);
                }
            }
        }
        
        // ゲームパッド更新
        if let Some(ref mut gp) = gamepad_manager {
            gp.update();
        }
        
        // ジョイスティック入力（キーボード + ゲームパッド）
        // メニューが開いている間は矢印キーをブロック
        let (mut joy_left, mut joy_right, mut joy_up, mut joy_down) = if menu_open {
            (false, false, false, false)
        } else {
            (
                window.is_key_down(Key::Left),
                window.is_key_down(Key::Right),
                window.is_key_down(Key::Up),
                window.is_key_down(Key::Down),
            )
        };
        let (mut button0, mut button1) = (
            window.is_key_down(Key::LeftAlt) || window.is_key_down(Key::X),
            window.is_key_down(Key::RightAlt) || window.is_key_down(Key::Z),
        );
        
        // ゲームパッドからの入力をマージ
        let mut gamepad_x: Option<f32> = None;
        let mut gamepad_y: Option<f32> = None;
        
        if let Some(ref gp) = gamepad_manager {
            let state = gp.state();
            if gp.is_connected() {
                // Dパッド
                joy_left |= state.dpad_left;
                joy_right |= state.dpad_right;
                joy_up |= state.dpad_up;
                joy_down |= state.dpad_down;
                
                // 左スティック（アナログ）
                if state.left_x.abs() > 0.1 || state.left_y.abs() > 0.1 {
                    gamepad_x = Some(state.left_x);
                    gamepad_y = Some(state.left_y);
                }
                
                // ボタン（A/B または X/Y）
                button0 |= state.button_a || state.button_x;
                button1 |= state.button_b || state.button_y;
            }
        }
        
        // パドル値を設定
        if let Some(gx) = gamepad_x {
            // アナログスティックの値を0-255に変換
            let x_value = ((gx + 1.0) * 127.5).clamp(0.0, 255.0) as u8;
            emu.memory.set_paddle(0, x_value);
        } else {
            // デジタル入力
            // Apple IIジョイスティックの実範囲は約10-245
            // 左右のみ押している時はY軸を中央に保つ
            let x_value = if joy_left { 10u8 } else if joy_right { 245u8 } else { 128u8 };
            emu.memory.set_paddle(0, x_value);
        }

        if let Some(gy) = gamepad_y {
            let y_value = ((gy + 1.0) * 127.5).clamp(0.0, 255.0) as u8;
            emu.memory.set_paddle(1, y_value);
        } else {
            let y_value = if joy_up { 10u8 } else if joy_down { 245u8 } else { 128u8 };
            emu.memory.set_paddle(1, y_value);
        }
        
        emu.memory.set_button(0, button0);
        emu.memory.set_button(1, button1);

        prev_keys = current_keys;

        if paused && runtime_single_step_requested {
            let _step_cycles = emu.step_instruction();
            emu.render_video();
            timing_last_host = Instant::now();
            cycle_accumulator = 0.0;
            next_video_cycle = emu.total_cycles + Apple2::CYCLES_PER_FRAME;
            runtime_single_step_requested = false;
            debugger.step_complete();
        }

        // 一時停止中でなければエミュレーション実行
        if !paused {
            let frame_start_cycle = emu.total_cycles;
            let allow_aggressive_boost = current_speed == 0;

            if current_speed == 0 {
                // MAX: 従来どおりフレーム駆動 + boost を許可
                let base_frames_per_update: u32 = 10;
                let rwts_boost_override = allow_aggressive_boost && emu.disk.motor_on && emu.disk.rwts_session_active();
                let disk_boost_multiplier: u32 = if accurate_boost_enabled
                    && (!boot_boost_active || rwts_boost_override)
                {
                    emu.disk.suggested_accurate_boost_multiplier() as u32
                } else {
                    1
                };
                let frames_per_update: u32 = if accurate_boost_enabled
                    && emu.disk.motor_on
                    && disk_boost_multiplier > 1
                    && (!boot_boost_active || rwts_boost_override)
                {
                    base_frames_per_update.saturating_mul(disk_boost_multiplier.max(1))
                } else {
                    base_frames_per_update
                };

                let should_log_boost = accurate_boost_enabled
                    && emu.disk.motor_on
                    && frames_per_update > base_frames_per_update;

                let boost_decision = if !accurate_boost_enabled {
                    "disabled"
                } else if boot_boost_active && !rwts_boost_override {
                    "suppressed_by_boot_boost"
                } else if !emu.disk.motor_on {
                    "motor_off"
                } else if frames_per_update <= base_frames_per_update {
                    "base_speed_already_high_enough"
                } else {
                    "active"
                };

                if should_log_boost {
                    if !accurate_boost_active || accurate_boost_multiplier != disk_boost_multiplier {
                        accurate_boost_active = true;
                        accurate_boost_multiplier = disk_boost_multiplier;
                        last_boost_debug_log = Instant::now();
                        log::info!(
                            "[BOOST] AccurateBoost active: x{} (mode={} rwts_session={} drive={} track={})",
                            accurate_boost_multiplier,
                            emu.disk.speed_mode_name(),
                            emu.disk.rwts_session_active(),
                            emu.disk.curr_drive + 1,
                            emu.disk.drives[emu.disk.curr_drive].current_track(),
                        );
                    }
                } else if accurate_boost_active {
                    log::info!(
                        "[BOOST] AccurateBoost inactive: mode={} motor_on={} drive={} track={}",
                        emu.disk.speed_mode_name(),
                        emu.disk.motor_on,
                        emu.disk.curr_drive + 1,
                        emu.disk.drives[emu.disk.curr_drive].current_track(),
                    );
                    accurate_boost_active = false;
                    accurate_boost_multiplier = 1;
                    last_boost_debug_log = Instant::now();
                }

                if boost_log_enabled && last_boost_debug_log.elapsed() >= Duration::from_secs(1) {
                    log::info!(
                        "[BOOST-DEBUG] decision={} multiplier={} base_frames={} frames_per_update={} mode={} rwts_session={} motor_on={} boot_boost={} drive={} track={}",
                        boost_decision,
                        disk_boost_multiplier,
                        base_frames_per_update,
                        frames_per_update,
                        emu.disk.speed_mode_name(),
                        emu.disk.rwts_session_active(),
                        emu.disk.motor_on,
                        boot_boost_active,
                        emu.disk.curr_drive + 1,
                        emu.disk.drives[emu.disk.curr_drive].current_track(),
                    );
                    last_boost_debug_log = Instant::now();
                }

                for _ in 0..frames_per_update {
                    emu.run_frame();
                }
            } else {
                // 通常速度: 実時間からサイクル予算を計算し、x1/x2/x5/x10 を正確に近づける
                let now = Instant::now();
                let elapsed = now.saturating_duration_since(timing_last_host);
                timing_last_host = now;

                let speed_multiplier = current_speed.max(1) as f64;
                let target_hz = APPLE2_CPU_HZ * speed_multiplier;
                cycle_accumulator += elapsed.as_secs_f64() * target_hz;
                let max_catchup_cycles = target_hz * MAX_CATCHUP_SECONDS;
                if cycle_accumulator > max_catchup_cycles {
                    cycle_accumulator = max_catchup_cycles;
                }

                while cycle_accumulator >= STEP_CHUNK_CYCLES as f64 {
                    emu.run_cycles(STEP_CHUNK_CYCLES);
                    cycle_accumulator -= STEP_CHUNK_CYCLES as f64;
                }

                while emu.total_cycles >= next_video_cycle {
                    emu.render_video();
                    next_video_cycle = next_video_cycle.saturating_add(Apple2::CYCLES_PER_FRAME);
                }

                if accurate_boost_active {
                    log::info!(
                        "[BOOST] AccurateBoost inactive: mode={} motor_on={} drive={} track={}",
                        emu.disk.speed_mode_name(),
                        emu.disk.motor_on,
                        emu.disk.curr_drive + 1,
                        emu.disk.drives[emu.disk.curr_drive].current_track(),
                    );
                    accurate_boost_active = false;
                    accurate_boost_multiplier = 1;
                    last_boost_debug_log = Instant::now();
                }
            }

            let executed_cycles = emu.total_cycles.saturating_sub(frame_start_cycle);
            timing_stats_cycles = timing_stats_cycles.saturating_add(executed_cycles);
            timing_stats_frames = timing_stats_frames.saturating_add(1);
            let timing_log_suppressed = suppress_timing_log_until
                .map(|until| Instant::now() < until)
                .unwrap_or(false);
            if timing_log_enabled && !timing_log_suppressed && timing_stats_last_log.elapsed() >= Duration::from_secs(1) {
                let elapsed_secs = timing_stats_window_start.elapsed().as_secs_f64().max(1e-6);
                let actual_hz = timing_stats_cycles as f64 / elapsed_secs;
                let target_hz = if current_speed == 0 {
                    None
                } else {
                    Some(APPLE2_CPU_HZ * current_speed.max(1) as f64)
                };
                let speed_label = if current_speed == 0 {
                    "MAX".to_string()
                } else {
                    format!("x{}", current_speed)
                };
                log::info!(
                    "[TIMING] speed={} actual_hz={:.0} target_hz={} frame_count={} total_cycles={} accumulator={:.1} fastdisk={} rwts_session={} motor_on={} boost_active={} boost_multiplier={}",
                    speed_label,
                    actual_hz,
                    target_hz.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "MAX".to_string()),
                    emu.frame_count,
                    emu.total_cycles,
                    cycle_accumulator,
                    emu.is_fast_disk_enabled(),
                    emu.disk.rwts_session_active(),
                    emu.disk.motor_on,
                    accurate_boost_active,
                    accurate_boost_multiplier,
                );
                timing_stats_last_log = Instant::now();
                timing_stats_window_start = timing_stats_last_log;
                timing_stats_cycles = 0;
                timing_stats_frames = 0;
            }
            
            // プロファイラ: ブート段階の自動検出
            if profiler.enabled {
                let pc = emu.cpu.regs.pc;
                
                // ブート段階を判定
                use profiler::BootStage;
                match profiler.boot_stage {
                    BootStage::Init | BootStage::BootRom => {
                        // $C600-$C6FFはDisk II Boot ROM
                        if pc >= 0xC600 && pc <= 0xC6FF {
                            if profiler.boot_stage == BootStage::Init {
                                profiler.set_boot_stage(BootStage::BootRom);
                            }
                        } else {
                            // Boot ROMを抜けたらCompleteへ
                            if profiler.boot_stage == BootStage::BootRom {
                                profiler.set_boot_stage(BootStage::Complete);
                            }
                        }
                    }
                    _ => {}
                }
                
                // ディスク情報を更新
                profiler.disk_info.current_track = emu.disk.drives[emu.disk.curr_drive].current_track();
                profiler.cpu_info.last_pc = pc;
            }
            
            // オーディオ処理
            if sound_enabled {
                if current_speed == 1 {
                    // 通常速度時：スピーカークリックを処理
                    let clicks = emu.take_speaker_clicks();
                    for cycle in clicks {
                        speaker.click(cycle);
                    }
                } else {
                    // 高速モード時はスピーカークリックを破棄
                    emu.take_speaker_clicks();
                }
                
                // サンプルを生成して再生（リセット音やUIクリック音は常に処理）
                let cycles_per_frame = emu.total_cycles - frame_start_cycle;
                if let Some(ref mut audio) = audio_output {
                    let samples = speaker.generate_samples(frame_start_cycle, cycles_per_frame.max(17030));
                    audio.play_samples(samples);
                }
            } else {
                // サウンド無効時はクリックを破棄
                emu.take_speaker_clicks();
            }
        }

        // フレームバッファを取得
        let fb = emu.get_framebuffer();
        
        // GUIの高さを考慮した描画領域を計算
        let gui_height = if gui.fullscreen { 0 } else { TOOLBAR_HEIGHT + STATUSBAR_HEIGHT };
        let draw_height = current_window_height.saturating_sub(gui_height);
        let draw_y_offset = if gui.fullscreen { 0 } else { TOOLBAR_HEIGHT };
        
        // まずバッファをクリア
        for pixel in scaled_buffer.iter_mut() {
            *pixel = 0x000000;
        }
        
        // 品質レベルに応じた処理（5段階）
        // 0=Lowest, 1=Low, 2=Medium, 3=High, 4=Ultra
        // 一時バッファに描画してからオフセットを適用
        let mut temp_buffer = vec![0u32; current_window_width * draw_height.max(1)];
        
        match quality_level {
            0 => {
                // Lowest: ニアレストネイバーのみ（最速）
                scale_nearest_aspect_fast(fb, SCREEN_WIDTH, SCREEN_HEIGHT, &mut temp_buffer, current_window_width, draw_height);
            }
            1 => {
                // Low: バイリニアのみ
                scale_bilinear_aspect_fast(fb, SCREEN_WIDTH, SCREEN_HEIGHT, &mut temp_buffer, current_window_width, draw_height);
            }
            2 => {
                // Medium: フレーム補間 + バイリニア
                let processed_frame = if frame_blend_enabled {
                    blend_frames_fast(fb, &mut prev_frame);
                    &prev_frame
                } else {
                    fb
                };
                scale_bilinear_aspect_fast(processed_frame, SCREEN_WIDTH, SCREEN_HEIGHT, &mut temp_buffer, current_window_width, draw_height);
            }
            3 => {
                // High: フレーム補間 + バイリニア + シャープネス + スキャンライン
                let processed_frame = if frame_blend_enabled {
                    blend_frames_fast(fb, &mut prev_frame);
                    &prev_frame
                } else {
                    fb
                };
                scale_bilinear_aspect_fast(processed_frame, SCREEN_WIDTH, SCREEN_HEIGHT, &mut temp_buffer, current_window_width, draw_height);
                // シャープネス強調
                apply_light_sharpen(&mut temp_buffer, current_window_width, draw_height, 30);
                // スキャンラインを適用
                apply_scanlines(&mut temp_buffer, current_window_width, draw_height, 200);
            }
            _ => {
                // Ultra: フレーム補間 + バイリニア + シャープネス + スキャンライン + ブルーム
                let processed_frame = if frame_blend_enabled {
                    blend_frames_fast(fb, &mut prev_frame);
                    &prev_frame
                } else {
                    fb
                };
                scale_bilinear_aspect_fast(processed_frame, SCREEN_WIDTH, SCREEN_HEIGHT, &mut temp_buffer, current_window_width, draw_height);
                // シャープネス強調
                apply_light_sharpen(&mut temp_buffer, current_window_width, draw_height, 40);
                // スキャンライン + ブルーム
                apply_scanlines(&mut temp_buffer, current_window_width, draw_height, 210);
                apply_bloom(&mut temp_buffer, current_window_width, draw_height, 200, 80);
            }
        }
        
        // 一時バッファをオフセットを適用してメインバッファにコピー
        for y in 0..draw_height {
            let src_row = y * current_window_width;
            let dst_row = (y + draw_y_offset) * current_window_width;
            for x in 0..current_window_width {
                if dst_row + x < scaled_buffer.len() && src_row + x < temp_buffer.len() {
                    scaled_buffer[dst_row + x] = temp_buffer[src_row + x];
                }
            }
        }
        
        // GUI描画（全画面でない場合）
        if !gui.fullscreen {
            // ディスクドライブの状態を取得
            let (_, disk1_reading, disk1_writing) = emu.disk.get_drive_status(0);
            let (_, disk2_reading, disk2_writing) = emu.disk.get_drive_status(1);
            
            // エミュレータ状態を構築
            let status = EmulatorStatus {
                fps: displayed_fps,
                speed: current_speed,
                fast_disk: fast_disk_enabled,
                save_slot: current_slot,
                sound_enabled,
                gamepad_connected: gamepad_manager.as_ref().map_or(false, |g| g.is_connected()),
                quality_level,
                auto_quality,
                paused,
                disk1_name: None, // TODO: ディスク名を取得
                disk2_name: None,
                disk1_active: disk1_reading && !disk1_writing,  // 読み込み中（書き込みでない）
                disk2_active: disk2_reading && !disk2_writing,  // 読み込み中（書き込みでない）
                disk1_writing,
                disk2_writing,
                a2rs_home: config.a2rs_home.clone(),
                rom_dir: config.rom_dir.clone(),
                disk_dir: config.disk_dir.clone(),
                screenshot_dir: config.screenshot_dir.clone(),
                save_dir: config.save_dir.clone(),
            };
            
            gui.draw_toolbar(&mut scaled_buffer, current_window_width, &status);
            gui.draw_statusbar(&mut scaled_buffer, current_window_width, current_window_height, &status);
        }
        
        // ディスクメニュー描画（オーバーレイとは別）
        if gui.is_disk_menu_open() {
            let drive = gui.disk_menu_drive.unwrap_or(0);
            let current_disk = emu.disk.drives[drive].disk.filename.as_deref();
            gui.draw_disk_menu(&mut scaled_buffer, current_window_width, current_window_height, current_disk);
        }
        
        // オーバーレイメニュー描画
        if gui.overlay_visible {
            let status = EmulatorStatus {
                fps: displayed_fps,
                speed: current_speed,
                fast_disk: fast_disk_enabled,
                save_slot: current_slot,
                sound_enabled,
                gamepad_connected: gamepad_manager.as_ref().map_or(false, |g| g.is_connected()),
                quality_level,
                auto_quality,
                paused,
                disk1_name: None,
                disk2_name: None,
                disk1_active: false,
                disk2_active: false,
                disk1_writing: false,
                disk2_writing: false,
                a2rs_home: config.a2rs_home.clone(),
                rom_dir: config.rom_dir.clone(),
                disk_dir: config.disk_dir.clone(),
                screenshot_dir: config.screenshot_dir.clone(),
                save_dir: config.save_dir.clone(),
            };
            gui.draw_overlay(&mut scaled_buffer, current_window_width, current_window_height, &status);
        }
        
        #[cfg(feature = "gamepad")]
        if cfg!(target_os = "linux") && config.gamepad.show_debug_overlay {
            if let Some(ref gp) = gamepad_manager {
                let lines = gp.debug_lines();
                draw_linux_gamepad_debug_overlay(&mut scaled_buffer, current_window_width, current_window_height, &lines);
            }
        }

        // デバッガパネルを描画
        if debugger_panel.visible {
            let cpu_regs = CpuRegisters {
                pc: emu.cpu.regs.pc,
                a: emu.cpu.regs.a,
                x: emu.cpu.regs.x,
                y: emu.cpu.regs.y,
                sp: emu.cpu.regs.sp,
                flags: emu.cpu.regs.status,
                current_opcode: emu.memory.main_ram[emu.cpu.regs.pc as usize],
            };
            
            let disk_debug = DiskDebugInfo {
                motor_on: emu.disk.motor_on,
                current_drive: emu.disk.curr_drive,
                current_track: emu.disk.drives[emu.disk.curr_drive].current_track(),
                phase: emu.disk.drives[emu.disk.curr_drive].phase as usize,
                byte_position: emu.disk.drives[emu.disk.curr_drive].disk.byte_position,
                write_mode: emu.disk.write_mode,
                latch: emu.disk.latch,
                fastdisk_effective: emu.disk.is_fastdisk_effective(),
                speed_mode: format!("{:?}", emu.disk.speed_mode),
                latched_off: !emu.disk.is_fastdisk_effective() && emu.disk.enhance_disk,
            };
            
            let panel_x = current_window_width.saturating_sub(DEBUGGER_PANEL_WIDTH);
            debugger_panel.render(
                &mut scaled_buffer,
                current_window_width,
                current_window_height,
                panel_x,
                &profiler,
                &debugger,
                &cpu_regs,
                &emu.memory.main_ram[..],
                &disk_debug,
            );
        }
        
        if video_timing_overlay {
            let diag = VideoTimingDiagnostics::from_position(
                emu.video_position(),
                emu.floating_bus_address(),
                emu.floating_bus(),
            );
            let lines = diag.lines();
            draw_video_timing_debug_overlay(&mut scaled_buffer, current_window_width, current_window_height, &lines);
        }

        if disk_realism_overlay {
            let lines = emu.disk.realism_lines();
            draw_video_timing_debug_overlay(&mut scaled_buffer, current_window_width, current_window_height, &lines);
        }

        if runtime_debug_overlay {
            let lines = build_runtime_debug_lines(&emu, paused);
            draw_video_timing_debug_overlay(&mut scaled_buffer, current_window_width, current_window_height, &lines);
        }

        // プロファイラのフレーム終了処理
        profiler.end_frame();
        
        // プロファイルデータの定期出力
        if profiler.enabled {
            // 定期出力
            if let Some(ref path) = profile_output {
                if last_profile_output.elapsed() >= profile_interval {
                    // ファイル拡張子に応じて出力形式を選択
                    let result = if path.ends_with(".json") {
                        profiler.write_json(path)
                    } else if path.ends_with(".csv") {
                        profiler.write_csv(path)
                    } else {
                        profiler.write_to_file(path)
                    };
                    
                    if let Err(e) = result {
                        eprintln!("Failed to write profile: {}", e);
                    }
                    last_profile_output = Instant::now();
                }
            }
            
            // ブート完了時の処理
            if profile_boot_only && profiler.boot_stage == profiler::BootStage::Complete {
                println!("\n{}", profiler.detailed_report());
                
                if let Some(ref path) = profile_output {
                    let result = if path.ends_with(".json") {
                        profiler.write_json(path)
                    } else if path.ends_with(".csv") {
                        profiler.write_csv(path)
                    } else {
                        profiler.write_to_file(path)
                    };
                    
                    match result {
                        Ok(_) => println!("Profile written to: {}", path),
                        Err(e) => eprintln!("Failed to write profile: {}", e),
                    }
                }
                
                println!("Boot profiling complete. Exiting.");
                break;
            }
        }
        
        let _ = window.update_with_buffer(&scaled_buffer, current_window_width, current_window_height);
        
        // フレーム時間を計測
        let frame_time = frame_start.elapsed().as_secs_f64() * 1000.0; // ms
        frame_times[frame_time_index] = frame_time;
        frame_time_index = (frame_time_index + 1) % 60;
        
        // 1秒ごとにFPS表示を更新し、品質を自動調整
        if last_fps_update.elapsed() >= Duration::from_secs(1) {
            let avg_frame_time: f64 = frame_times.iter().sum::<f64>() / 60.0;
            displayed_fps = 1000.0 / avg_frame_time;
            
            // 自動品質調整（5段階: 0-4）
            // 下げる: FPS低下時は即応（次のFPS更新タイミングで1段階下げる）
            // 上げる: 高FPSがしばらく続いたときだけ1段階上げる
            if auto_quality {
                let fps_is_low = displayed_fps < 50.0;
                let fps_is_high = displayed_fps > 59.0;

                let old_quality = quality_level;

                if fps_is_low {
                    high_fps_seconds = 0;
                    if quality_level > 0 {
                        quality_level -= 1;
                    }
                } else if fps_is_high {
                    high_fps_seconds += 1;
                    if high_fps_seconds >= 300 && quality_level < 4 {
                        quality_level += 1;
                        high_fps_seconds = 0;
                    }
                } else {
                    high_fps_seconds = 0;
                }

                if old_quality != quality_level {
                    log::debug!("Auto quality adjusted to level {} (FPS: {:.1})", quality_level, displayed_fps);
                }
            }
            
            last_fps_update = Instant::now();
        }

        // 起動ブースト制御（MAX指定時のみ）
        if current_speed == 0 && boot_boost_active {
            // PC安定ループ検出で終了
            if emu.check_stable_loop() {
                boot_boost_active = false;
                current_speed = configured_speed; // 元のユーザー設定速度を維持
                timing_last_host = Instant::now();
                cycle_accumulator = 0.0;
                next_video_cycle = emu.total_cycles + Apple2::CYCLES_PER_FRAME;
                timing_stats_last_log = Instant::now();
                timing_stats_window_start = timing_stats_last_log;
                timing_stats_cycles = 0;
                timing_stats_frames = 0;
                suppress_timing_log_until = Some(timing_stats_last_log + Duration::from_secs(1));
                log::debug!("Boot boost ended at {:.1}M cycles", emu.total_cycles as f64 / 1_000_000.0);
            }
            // 表示速度と通常の速度制御は current_speed (= ユーザー設定値) を維持
            // ディスクタイミングは速度制限コードで自動的に維持される
            // （motor=ON時はスリープしない = 高速動作、タイミングはディスクエミュレーション側で維持）
        }

        // 速度制限
        // 通常速度は cycle_accumulator 方式でペース制御するため、ここでは軽い待機だけ行う。
        // MAX 時のみ従来どおりディスク中のスロットル解除を許可する。
        if current_speed == 0 {
            let allow_aggressive_boost = true;
            let disk_busy = emu.disk.motor_on;
            let bypass_throttle_for_disk = allow_aggressive_boost && disk_busy;
            if !bypass_throttle_for_disk {
                let elapsed = frame_start.elapsed();
                if elapsed < base_frame_duration {
                    std::thread::sleep(base_frame_duration - elapsed);
                }
            }
        } else if cycle_accumulator < STEP_CHUNK_CYCLES as f64 {
            std::thread::sleep(Duration::from_micros(500));
        }
    }

    // 設定ファイルは読み込み専用とし、終了時に書き戻さない。
    // 実行中の値はこのセッション内だけで有効。
    let _ = (current_slot, sound_enabled, quality_level, auto_quality, video_timing_overlay, disk_realism_overlay, runtime_debug_overlay, runtime_single_step_requested);
    let _ = gui.get_volume();
}
