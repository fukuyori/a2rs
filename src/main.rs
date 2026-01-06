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
use a2rs::apple2;
use a2rs::sound;
use a2rs::gamepad;
use a2rs::config;
use a2rs::gui;
use a2rs::profiler;
use a2rs::disk_log;

// テスト専用モジュール（main.rsのみ）
mod test_cpu;
mod debug_test;

use apple2::Apple2;
use memory::AppleModel;
#[allow(unused_imports)]
use cpu::MemoryBus;
use video::{SCREEN_WIDTH, SCREEN_HEIGHT};
use sound::{Speaker, AudioOutput};
use gamepad::GamepadManager;
use config::{Config, SaveSlots};
use gui::{Gui, EmulatorStatus, ToolbarButton, DiskMenuAction, TOOLBAR_HEIGHT, STATUSBAR_HEIGHT};
use gui::{DebuggerPanel, CpuRegisters, DiskDebugInfo, DEBUGGER_PANEL_WIDTH};
use profiler::{Profiler, Debugger};
use clap::Parser;
use minifb::{Key, Window, WindowOptions, KeyRepeat, MouseMode, MouseButton};
use std::fs;
use std::time::{Duration, Instant};

/// A2RS - Apple II Emulator in Rust
#[derive(Parser, Debug)]
#[command(name = "a2rs")]
#[command(author = "A2RS Project")]
#[command(version = "0.1.0")]
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
    #[arg(long, default_value = "1")]
    speed: u32,
    
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
    
    /// 起動ブーストのログを出力
    #[arg(long)]
    boost_log: bool,
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
fn get_available_disks() -> Vec<String> {
    let mut disks = Vec::new();
    
    // disksディレクトリを検索
    let disk_dirs = ["disks", ".", "roms"];
    
    for dir in &disk_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    let lower = file_name.to_lowercase();
                    if lower.ends_with(".dsk") || lower.ends_with(".do") || 
                       lower.ends_with(".po") || lower.ends_with(".nib") {
                        // フルパスで保存
                        let path = format!("{}/{}", dir, file_name);
                        if !disks.contains(&path) {
                            disks.push(path);
                        }
                    }
                }
            }
        }
    }
    
    disks.sort();
    disks
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
    env_logger::init();
    
    let args = Args::parse();
    
    // ディスクログレベルを設定
    let disk_log_level = parse_disk_log_level(&args.disk_log);
    disk_log::set_log_level(disk_log_level);
    
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
        match fs::read(rom_path) {
            Ok(data) => Some(data),
            Err(e) => {
                eprintln!("Failed to load ROM {}: {}", rom_path, e);
                None
            }
        }
    } else {
        None
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
    println!("A2RS - Apple II Emulator v0.1 ({:?})", model);

    // エミュレータを作成
    let mut emu = Apple2::new(model);

    // Disk II Boot ROMをロード
    let disk_rom_loaded = if let Some(disk_rom_path) = args.disk_rom {
        match fs::read(&disk_rom_path) {
            Ok(data) => {
                match emu.load_disk_rom(&data) {
                    Ok(()) => {
                        log::info!("Loaded Disk II Boot ROM: {}", disk_rom_path);
                        true
                    }
                    Err(e) => {
                        eprintln!("Failed to load Disk II Boot ROM: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read Disk II Boot ROM {}: {}", disk_rom_path, e);
                false
            }
        }
    } else {
        // 自動的にdisk2.romを探す
        let search_paths = [
            "roms/disk2.rom",
            "disk2.rom",
            "DISK2.rom",
            "roms/DISK2.rom",
        ];
        
        let mut loaded = false;
        for path in &search_paths {
            if let Ok(data) = fs::read(path) {
                if let Ok(()) = emu.load_disk_rom(&data) {
                    log::info!("Loaded Disk II Boot ROM: {}", path);
                    loaded = true;
                    break;
                }
            }
        }
        loaded
    };
    
    if !disk_rom_loaded {
        eprintln!("Note: Disk II Boot ROM not found (VBR mode will be used for DSK files)");
    }

    // ROMをロード
    if let Some(data) = rom_data {
        emu.load_rom(&data);
        // ROM loading message is already printed by memory.rs
    } else {
        // テスト用ROMを使用
        eprintln!("No ROM specified. Using built-in test ROM.");
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
        match fs::read(&disk1_path) {
            Ok(disk_data) => {
                match emu.load_disk(0, &disk_data) {
                    Ok(()) => log::info!("Loaded disk 1: {}", disk1_path),
                    Err(e) => eprintln!("Failed to load disk 1: {}", e),
                }
            }
            Err(e) => eprintln!("Failed to read disk 1 {}: {}", disk1_path, e),
        }
    }

    if let Some(disk2_path) = args.disk2 {
        match fs::read(&disk2_path) {
            Ok(disk_data) => {
                match emu.load_disk(1, &disk_data) {
                    Ok(()) => log::info!("Loaded disk 2: {}", disk2_path),
                    Err(e) => eprintln!("Failed to load disk 2: {}", e),
                }
            }
            Err(e) => eprintln!("Failed to read disk 2 {}: {}", disk2_path, e),
        }
    }

    // リセット
    emu.reset();
    
    // ディスク高速化を設定（デフォルトはオン）
    emu.set_fast_disk(true);
    
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
        run_with_window(&mut emu, args.speed, width, height, args.fullscreen, profile_opts);
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

fn run_with_window(emu: &mut Apple2, speed: u32, init_width: usize, init_height: usize, fullscreen: bool, profile_opts: ProfileOptions) {
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

    // 設定ファイルを読み込み
    let mut config = Config::load();
    
    // エミュレータ一時停止フラグ
    let mut paused = false;
    
    // カーソル関連
    let mut last_mouse_pos: (f32, f32) = (0.0, 0.0);
    let mut last_mouse_move = Instant::now();
    let mut cursor_visible = true;

    let base_frame_duration = Duration::from_micros(16667); // 60 FPS
    let mut prev_keys: Vec<Key> = Vec::new();
    let mut current_speed = speed;
    let mut fast_disk_enabled = true;
    
    // 起動ブースト: ディスクがロードされている場合、MAXスピードで起動
    let disk_loaded = emu.disk.drives[0].disk.disk_loaded;
    let mut boot_boost_active = disk_loaded;
    if boot_boost_active {
        current_speed = 0; // 0 = MAX
    }
    
    // オーディオ出力を初期化
    let mut audio_output = match AudioOutput::new() {
        Ok(audio) => Some(audio),
        Err(e) => {
            log::warn!("Audio initialization failed: {}", e);
            None
        }
    };
    let mut speaker = Speaker::new();
    let mut sound_enabled = true;
    
    // フレームレート計測用
    let mut frame_times: [f64; 60] = [16.667; 60]; // 過去60フレームの時間(ms)
    let mut frame_time_index = 0;
    let mut last_fps_update = Instant::now();
    let mut displayed_fps = 60.0;
    
    // 適応的品質調整（0-4の5段階）
    let mut quality_level: i32 = 4; // 最高品質から開始
    let mut auto_quality = true; // 自動品質調整ON/OFF
    let mut low_fps_seconds = 0u32;  // FPSが低い状態が続いた秒数
    let mut high_fps_seconds = 0u32; // FPSが高い状態が続いた秒数
    
    // セーブスロット（0-9）
    let mut current_slot: u8 = config.current_slot;
    
    // ゲームパッド初期化
    let mut gamepad_manager = match GamepadManager::new() {
        Ok(gp) => Some(gp),
        Err(e) => {
            log::debug!("Gamepad not available: {}", e);
            None
        }
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
        
        if click_event && !gui.fullscreen {
            if let Some(btn) = gui.mouse_click() {
                match btn {
                    ToolbarButton::PlayPause => {
                        paused = !paused;
                    }
                    ToolbarButton::Reset => {
                        emu.reset();
                    }
                    ToolbarButton::Disk1 => {
                        let disks = get_available_disks();
                        gui.open_disk_menu(0, disks);
                    }
                    ToolbarButton::Disk2 => {
                        let disks = get_available_disks();
                        gui.open_disk_menu(1, disks);
                    }
                    ToolbarButton::SwapDisks => {
                        emu.disk.swap_disks();
                    }
                    ToolbarButton::QuickSave => {
                        let state = emu.save_state();
                        let filename = SaveSlots::get_filename(current_slot);
                        if let Ok(json) = serde_json::to_string(&state) {
                            if let Ok(_) = std::fs::write(&filename, &json) {
                                println!("Saved to slot {}", current_slot);
                            }
                        }
                    }
                    ToolbarButton::QuickLoad => {
                        let filename = SaveSlots::get_filename(current_slot);
                        if let Ok(json) = std::fs::read_to_string(&filename) {
                            if let Ok(state) = serde_json::from_str(&json) {
                                if let Ok(_) = emu.load_state(&state) {
                                    println!("Loaded from slot {}", current_slot);
                                }
                            }
                        }
                    }
                    ToolbarButton::Screenshot => {
                        let filename = format!("screenshot_{}.png", 
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs());
                        let fb = emu.get_framebuffer();
                        if save_screenshot(&filename, fb, SCREEN_WIDTH, SCREEN_HEIGHT).is_ok() {
                            println!("Screenshot saved: {}", filename);
                        }
                    }
                    ToolbarButton::Fullscreen => {
                        gui.toggle_fullscreen();
                    }
                }
            }
        }
        
        // ESCでメニュー操作
        if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
            if gui.is_disk_menu_open() {
                gui.close_disk_menu();
            } else {
                gui.toggle_overlay();
            }
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
                    match action {
                        DiskMenuAction::Eject => {
                            emu.disk.eject_disk(drive);
                            println!("Ejected disk from drive {}", drive + 1);
                        }
                        DiskMenuAction::InsertDisk(index) => {
                            if let Some(disk_path) = gui.available_disks.get(index) {
                                let path = disk_path.clone();
                                if let Ok(data) = fs::read(&path) {
                                    let format = if path.to_lowercase().ends_with(".po") {
                                        disk::DiskFormat::Po
                                    } else if path.to_lowercase().ends_with(".nib") {
                                        disk::DiskFormat::Nib
                                    } else {
                                        disk::DiskFormat::Dsk
                                    };
                                    if emu.disk.insert_disk(drive, &data, format).is_ok() {
                                        println!("Inserted {} into drive {}", path, drive + 1);
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
                // バックスペース
                if window.is_key_pressed(Key::Backspace, KeyRepeat::Yes) {
                    gui.text_input_backspace();
                }
                // Enter で確定
                if window.is_key_pressed(Key::Enter, KeyRepeat::No) {
                    if let Some((item, value)) = gui.end_text_input() {
                        match item {
                            5 => config.rom_dir = value,
                            6 => config.disk_dir = value,
                            7 => config.screenshot_dir = value,
                            8 => config.save_dir = value,
                            _ => {}
                        }
                        config.ensure_directories();
                        let _ = config.save();
                    }
                }
                // Escape でキャンセル
                if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
                    gui.cancel_text_input();
                }
                // 文字入力（英数字とパス文字のみ）
                for key in window.get_keys() {
                    let ch = match key {
                        Key::A => Some('a'), Key::B => Some('b'), Key::C => Some('c'),
                        Key::D => Some('d'), Key::E => Some('e'), Key::F => Some('f'),
                        Key::G => Some('g'), Key::H => Some('h'), Key::I => Some('i'),
                        Key::J => Some('j'), Key::K => Some('k'), Key::L => Some('l'),
                        Key::M => Some('m'), Key::N => Some('n'), Key::O => Some('o'),
                        Key::P => Some('p'), Key::Q => Some('q'), Key::R => Some('r'),
                        Key::S => Some('s'), Key::T => Some('t'), Key::U => Some('u'),
                        Key::V => Some('v'), Key::W => Some('w'), Key::X => Some('x'),
                        Key::Y => Some('y'), Key::Z => Some('z'),
                        Key::Key0 => Some('0'), Key::Key1 => Some('1'), Key::Key2 => Some('2'),
                        Key::Key3 => Some('3'), Key::Key4 => Some('4'), Key::Key5 => Some('5'),
                        Key::Key6 => Some('6'), Key::Key7 => Some('7'), Key::Key8 => Some('8'),
                        Key::Key9 => Some('9'),
                        Key::Minus => Some('-'), Key::Period => Some('.'),
                        Key::Slash => Some('/'), Key::Backslash => Some('\\'),
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
                    }
                    1 => { // Fast Disk
                        fast_disk_enabled = !fast_disk_enabled;
                        emu.set_fast_disk(fast_disk_enabled);
                    }
                    2 => { // Quality
                        quality_level = (quality_level + 1) % 5;
                    }
                    3 => { // Auto Quality
                        auto_quality = !auto_quality;
                    }
                    5 => { // ROM Dir
                        gui.start_text_input(5, &config.rom_dir);
                    }
                    6 => { // Disk Dir
                        gui.start_text_input(6, &config.disk_dir);
                    }
                    7 => { // Screenshot Dir
                        gui.start_text_input(7, &config.screenshot_dir);
                    }
                    8 => { // Save Dir
                        gui.start_text_input(8, &config.save_dir);
                    }
                    _ => {}
                }
            }
        }
        
        // F1でスピード変更
        if window.is_key_pressed(Key::F1, KeyRepeat::No) {
            current_speed = match current_speed {
                1 => 2,
                2 => 5,
                5 => 10,
                10 => 0,  // MAX
                _ => 1,   // 0(MAX)や他の値から1に戻る
            };
            let speed_str = if current_speed == 0 { "MAX".to_string() } else { format!("x{}", current_speed) };
            println!("Speed: {}", speed_str);
        }
        
        // F11で全画面
        if window.is_key_pressed(Key::F11, KeyRepeat::No) {
            gui.toggle_fullscreen();
        }
        
        if window.is_key_pressed(Key::F12, KeyRepeat::No) {
            println!("Reset!");
            emu.reset();
            profiler.reset();
            debugger.reset();
            profiler.start_boot();
        }
        
        // F2でディスク高速化切り替え
        if window.is_key_pressed(Key::F2, KeyRepeat::No) {
            fast_disk_enabled = !fast_disk_enabled;
            emu.set_fast_disk(fast_disk_enabled);
            println!("Fast disk: {}", if fast_disk_enabled { "ON" } else { "OFF" });
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
        
        // Tab でデバッガパネル表示切り替え
        if window.is_key_pressed(Key::Tab, KeyRepeat::No) {
            debugger_panel.toggle();
            println!("Debugger panel: {}", if debugger_panel.visible { "ON" } else { "OFF" });
        }
        
        // デバッガパネルが表示中の場合のキー処理
        if debugger_panel.visible {
            // 左右でタブ切り替え
            if window.is_key_pressed(Key::Left, KeyRepeat::No) {
                debugger_panel.prev_tab();
            }
            if window.is_key_pressed(Key::Right, KeyRepeat::No) {
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
                debugger.step();
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
        
            // F8でセーブスロット選択（循環）
            if window.is_key_pressed(Key::F8, KeyRepeat::No) {
                current_slot = (current_slot + 1) % 10;
                let exists = SaveSlots::exists(current_slot);
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
                    let exists = SaveSlots::exists(current_slot);
                    println!("Save slot: {} {}", current_slot, if exists { "(has data)" } else { "(empty)" });
                }
            }
        }
        
        // F5でセーブ（現在のスロットに）
        if window.is_key_pressed(Key::F5, KeyRepeat::No) {
            let state = emu.save_state();
            let filename = SaveSlots::get_filename(current_slot);
            match serde_json::to_string(&state) {
                Ok(json) => {
                    match std::fs::write(&filename, &json) {
                        Ok(_) => {
                            println!("State saved to slot {} ({})", current_slot, filename);
                        }
                        Err(e) => println!("Failed to save state: {}", e),
                    }
                }
                Err(e) => println!("Failed to serialize state: {}", e),
            }
        }
        
        // F9でロード（現在のスロットから）
        if window.is_key_pressed(Key::F9, KeyRepeat::No) {
            let filename = SaveSlots::get_filename(current_slot);
            match std::fs::read_to_string(&filename) {
                Ok(json) => {
                    match serde_json::from_str(&json) {
                        Ok(state) => {
                            match emu.load_state(&state) {
                                Ok(_) => {
                                    println!("State loaded from slot {} ({})", current_slot, filename);
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
        if window.is_key_pressed(Key::F10, KeyRepeat::No) {
            let filename = format!("screenshot_{}.png", 
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs());
            
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
                    println!("Screenshot saved to {}", filename);
                }
                Err(e) => println!("Failed to save screenshot: {}", e),
            }
        }
        
        // F11の古い処理を削除（GUIで処理済み）

        let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
        let ctrl = window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl);
        
        // 現在押されているキーを取得
        let current_keys: Vec<Key> = window.get_keys()
            .iter()
            .filter(|k| key_to_apple2(**k, false, false).is_some())
            .copied()
            .collect();
        
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
        let (mut joy_left, mut joy_right, mut joy_up, mut joy_down) = (
            window.is_key_down(Key::Left),
            window.is_key_down(Key::Right),
            window.is_key_down(Key::Up),
            window.is_key_down(Key::Down),
        );
        let (mut button0, mut button1) = (
            window.is_key_down(Key::LeftAlt) || window.is_key_down(Key::Z),
            window.is_key_down(Key::RightAlt) || window.is_key_down(Key::X),
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
            let x_value = if joy_left { 0u8 } else if joy_right { 255u8 } else { 128u8 };
            emu.memory.set_paddle(0, x_value);
        }
        
        if let Some(gy) = gamepad_y {
            let y_value = ((gy + 1.0) * 127.5).clamp(0.0, 255.0) as u8;
            emu.memory.set_paddle(1, y_value);
        } else {
            let y_value = if joy_up { 0u8 } else if joy_down { 255u8 } else { 128u8 };
            emu.memory.set_paddle(1, y_value);
        }
        
        emu.memory.set_button(0, button0);
        emu.memory.set_button(1, button1);
        
        prev_keys = current_keys;

        // 一時停止中でなければエミュレーション実行
        if !paused {
            // 速度に応じてフレーム数を調整
            let frames_per_update = if current_speed == 0 { 10 } else { current_speed.max(1) };
            let frame_start_cycle = emu.total_cycles;
            for _ in 0..frames_per_update {
                emu.run_frame();
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
                        }
                        // $0800付近に来たらセクタ0ロード開始
                        else if pc >= 0x0800 && pc < 0x0900 {
                            profiler.set_boot_stage(BootStage::Sector0);
                        }
                    }
                    BootStage::Sector0 => {
                        // $B700-$BFFFはDOS領域
                        if pc >= 0xB700 && pc < 0xC000 {
                            profiler.set_boot_stage(BootStage::DosLoading);
                        }
                    }
                    BootStage::DosLoading => {
                        // DOS初期化完了を検出 (RWTSの初期化等)
                        // $9D00台（BASICプログラム領域）またはモニタに来たら
                        if pc >= 0x9D00 && pc < 0xB700 {
                            profiler.set_boot_stage(BootStage::DosInit);
                        }
                    }
                    BootStage::DosInit => {
                        // プロンプト表示を検出
                        // AppleSoft BASIC: $D553 (RESTART), $E003 (WARMSTART)
                        // Monitor: $FF59 (KEYIN)
                        if pc == 0xFD0C || pc == 0xFD1B || pc == 0xFF65 {
                            // キーボード入力待ち = ブート完了
                            profiler.set_boot_stage(BootStage::BasicPrompt);
                        }
                    }
                    BootStage::BasicPrompt => {
                        // しばらく待機後に完了とみなす（約1秒）
                        if let Some(boot_elapsed) = profiler.boot_elapsed() {
                            if boot_elapsed.as_secs() >= 3 {
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
            if sound_enabled && current_speed == 1 {
                // スピーカークリックを取得
                let clicks = emu.take_speaker_clicks();
                for cycle in clicks {
                    speaker.click(cycle);
                }
                
                // サンプルを生成して再生
                let cycles_per_frame = emu.total_cycles - frame_start_cycle;
                if cycles_per_frame > 0 {
                    if let Some(ref mut audio) = audio_output {
                        let samples = speaker.generate_samples(frame_start_cycle, cycles_per_frame);
                        audio.play_samples(samples);
                    }
                }
            } else {
                // 高速モード時はクリックを破棄
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
                disk1_active: emu.disk.motor_on && emu.disk.curr_drive == 0,
                disk2_active: emu.disk.motor_on && emu.disk.curr_drive == 1,
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
            let current_disk = if emu.disk.drives[drive].disk.disk_loaded {
                Some("(loaded)")
            } else {
                None
            };
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
                rom_dir: config.rom_dir.clone(),
                disk_dir: config.disk_dir.clone(),
                screenshot_dir: config.screenshot_dir.clone(),
                save_dir: config.save_dir.clone(),
            };
            gui.draw_overlay(&mut scaled_buffer, current_window_width, current_window_height, &status);
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
            // 下げる: 10秒間FPSが低い場合
            // 上げる: 60秒間FPSが高い場合
            if auto_quality {
                let fps_is_low = displayed_fps < 50.0;
                let fps_is_high = displayed_fps > 58.0;
                
                if fps_is_low {
                    low_fps_seconds += 1;
                    high_fps_seconds = 0;
                } else if fps_is_high {
                    high_fps_seconds += 1;
                    low_fps_seconds = 0;
                } else {
                    // 中間の場合はリセット
                    low_fps_seconds = 0;
                    high_fps_seconds = 0;
                }
                
                let old_quality = quality_level;
                
                // 10秒間FPSが低い場合、品質を下げる
                if low_fps_seconds >= 10 && quality_level > 0 {
                    quality_level -= 1;
                    low_fps_seconds = 0;
                }
                
                // 60秒間FPSが高い場合、品質を上げる
                if high_fps_seconds >= 60 && quality_level < 4 {
                    quality_level += 1;
                    high_fps_seconds = 0;
                }
                
                if old_quality != quality_level {
                    log::debug!("Auto quality adjusted to level {} (FPS: {:.1})", quality_level, displayed_fps);
                }
            }
            
            last_fps_update = Instant::now();
        }

        // 起動ブースト制御
        if boot_boost_active {
            // PC安定ループ検出で終了
            if emu.check_stable_loop() {
                boot_boost_active = false;
                current_speed = speed; // 元の速度に戻す
                log::debug!("Boot boost ended at {:.1}M cycles", emu.total_cycles as f64 / 1_000_000.0);
            }
            // ブースト中はcurrent_speed=0（MAX）を維持
            // ディスクタイミングは速度制限コードで自動的に維持される
            // （motor=ON時はスリープしない = 高速動作、タイミングはディスクエミュレーション側で維持）
        }

        // 速度制限（speed=0の場合は制限なし）
        // ディスク回転中はスロットル解除（AppleWin互換）
        let disk_busy = emu.disk.motor_on;
        if current_speed > 0 && !disk_busy {
            let frame_duration = base_frame_duration / current_speed;
            let elapsed = frame_start.elapsed();
            if elapsed < frame_duration {
                std::thread::sleep(frame_duration - elapsed);
            }
        }
    }

    // 設定を保存
    config.current_slot = current_slot;
    config.sound_enabled = sound_enabled;
    config.quality_level = quality_level;
    config.auto_quality = auto_quality;
    config.fast_disk = fast_disk_enabled;
    if let Err(e) = config.save() {
        eprintln!("Failed to save config: {}", e);
    }
}
