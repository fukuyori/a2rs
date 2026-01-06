//! A2RS Debug Runner - ログオプション付き実行ファイル
//!
//! 使い方:
//!   a2rs_debug [OPTIONS] [disk.dsk]
//!
//! オプション:
//!   -l, --log <LEVEL>    ログレベル: none, flow, state, decide, all (default: none)
//!   -c, --cycles <N>     実行サイクル数 (default: 1000000)
//!   -r, --rom <FILE>     Apple II ROM ファイル
//!   -d, --disk-rom <FILE> Disk II ROM ファイル
//!   -t, --trace          CPU命令トレース (最初の100命令)
//!   -s, --screen         終了時に画面表示
//!   -h, --help           ヘルプ表示

use a2rs::apple2::Apple2;
use a2rs::memory::AppleModel;
use a2rs::disk_log::{set_log_level, DiskLogLevel};
use std::env;
use std::fs;

fn print_help() {
    println!("A2RS Debug Runner - Apple II Emulator with Logging");
    println!();
    println!("Usage: a2rs_debug [OPTIONS] [disk.dsk]");
    println!();
    println!("Options:");
    println!("  -l, --log <LEVEL>     Log level: none, flow, state, decide, all");
    println!("                        Can combine: flow+state, flow+decide, etc.");
    println!("  -c, --cycles <N>      Cycles to run (default: 1000000)");
    println!("  -r, --rom <FILE>      Apple II ROM file");
    println!("  -d, --disk-rom <FILE> Disk II ROM file (default: roms/disk2.rom)");
    println!("  -t, --trace           Enable CPU trace (first 100 instructions)");
    println!("  -s, --screen          Show screen at end");
    println!("  -h, --help            Show this help");
    println!();
    println!("Log Levels:");
    println!("  none    - No disk logging");
    println!("  flow    - High-level events (Motor ON/OFF, Sync, Boot)");
    println!("  state   - State transitions (Track changes)");
    println!("  decide  - FastDisk decisions (Enable/Disable reasons)");
    println!("  all     - All of the above + nibble dumps");
    println!();
    println!("Examples:");
    println!("  a2rs_debug dos33.dsk");
    println!("  a2rs_debug -l flow dos33.dsk");
    println!("  a2rs_debug -l flow+state+decide -c 5000000 dos33.dsk");
    println!("  a2rs_debug -l all -t -s dos33.dsk");
}

fn parse_log_level(s: &str) -> DiskLogLevel {
    let mut level = DiskLogLevel::empty();
    
    for part in s.to_lowercase().split('+') {
        match part.trim() {
            "none" => {}
            "flow" => level |= DiskLogLevel::FLOW,
            "state" => level |= DiskLogLevel::STATE,
            "decide" => level |= DiskLogLevel::DECIDE,
            "nibble" => level |= DiskLogLevel::NIBBLE,
            "all" => level = DiskLogLevel::FLOW | DiskLogLevel::STATE 
                           | DiskLogLevel::DECIDE | DiskLogLevel::NIBBLE,
            _ => eprintln!("Warning: Unknown log level '{}'", part),
        }
    }
    
    level
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // デフォルト設定
    let mut disk_file: Option<String> = None;
    let mut rom_file: Option<String> = None;
    let mut disk_rom_file = "roms/disk2.rom".to_string();
    let mut log_level = DiskLogLevel::empty();
    let mut cycles: u64 = 1_000_000;
    let mut trace = false;
    let mut show_screen = false;
    
    // 引数パース
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-l" | "--log" => {
                i += 1;
                if i < args.len() {
                    log_level = parse_log_level(&args[i]);
                }
            }
            "-c" | "--cycles" => {
                i += 1;
                if i < args.len() {
                    cycles = args[i].parse().unwrap_or(1_000_000);
                }
            }
            "-r" | "--rom" => {
                i += 1;
                if i < args.len() {
                    rom_file = Some(args[i].clone());
                }
            }
            "-d" | "--disk-rom" => {
                i += 1;
                if i < args.len() {
                    disk_rom_file = args[i].clone();
                }
            }
            "-t" | "--trace" => {
                trace = true;
            }
            "-s" | "--screen" => {
                show_screen = true;
            }
            arg if !arg.starts_with('-') => {
                disk_file = Some(arg.to_string());
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
            }
        }
        i += 1;
    }
    
    // ログレベル設定
    set_log_level(log_level);
    
    println!("=== A2RS Debug Runner ===");
    println!("Log level: {:?}", log_level);
    println!("Cycles: {}", cycles);
    println!();
    
    // エミュレータ初期化
    let mut emu = Apple2::new(AppleModel::AppleIIPlus);
    
    // Disk II ROM ロード
    if let Ok(data) = fs::read(&disk_rom_file) {
        if emu.load_disk_rom(&data).is_ok() {
            println!("Loaded Disk II ROM: {}", disk_rom_file);
        }
    } else {
        println!("Disk II ROM not found: {} (using VBR mode)", disk_rom_file);
    }
    
    // Apple II ROM ロード
    if let Some(ref path) = rom_file {
        if let Ok(data) = fs::read(path) {
            emu.load_rom(&data);
            println!("Loaded Apple II ROM: {}", path);
        } else {
            eprintln!("Failed to load ROM: {}", path);
        }
    } else {
        // テストROMを使用
        let test_rom = a2rs::apple2::create_test_rom();
        emu.load_rom(&test_rom);
        println!("Using test ROM (Monitor stubs)");
    }
    
    // ディスクイメージロード
    if let Some(ref path) = disk_file {
        if let Ok(data) = fs::read(path) {
            if emu.load_disk(0, &data).is_ok() {
                println!("Loaded disk: {}", path);
            } else {
                eprintln!("Failed to load disk: {}", path);
            }
        } else {
            eprintln!("Disk file not found: {}", path);
        }
    }
    
    println!();
    
    // リセット
    emu.reset();
    
    // CPUトレース（オプション）
    if trace {
        println!("--- CPU Trace (first 100 instructions) ---");
        for i in 0..100 {
            let pc = emu.cpu.regs.pc;
            // メモリから直接読む（スロットROM対応）
            let read_byte = |addr: u16| -> u8 {
                let a = addr as usize;
                if a < 0xC000 {
                    emu.memory.main_ram.get(a).copied().unwrap_or(0)
                } else if addr >= 0xC600 && addr < 0xC700 {
                    // Disk II Boot ROM ($C600-$C6FF)
                    emu.disk.boot_rom.get((addr - 0xC600) as usize).copied().unwrap_or(0)
                } else if addr >= 0xD000 {
                    emu.memory.rom.get((addr - 0xD000) as usize).copied().unwrap_or(0)
                } else {
                    0
                }
            };
            
            let op = read_byte(pc);
            let op1 = read_byte(pc.wrapping_add(1));
            let op2 = read_byte(pc.wrapping_add(2));
            
            println!("{:3}: ${:04X}: {:02X} {:02X} {:02X}  A=${:02X} X=${:02X} Y=${:02X} S=${:02X}",
                i, pc, op, op1, op2,
                emu.cpu.regs.a, emu.cpu.regs.x, emu.cpu.regs.y, emu.cpu.regs.sp);
            
            emu.step();
        }
        println!("--- End of trace ---\n");
        
        // 残りのサイクルを実行
        let remaining = cycles.saturating_sub(100);
        if remaining > 0 {
            println!("Running {} more cycles...\n", remaining);
            for _ in 0..remaining {
                emu.step();
            }
        }
    } else {
        // 通常実行
        println!("Running {} cycles...\n", cycles);
        for _ in 0..cycles {
            emu.step();
        }
    }
    
    // 終了時の状態
    println!("\n=== Final State ===");
    println!("PC: ${:04X}", emu.cpu.regs.pc);
    println!("A=${:02X} X=${:02X} Y=${:02X} SP=${:02X}",
        emu.cpu.regs.a, emu.cpu.regs.x, emu.cpu.regs.y, emu.cpu.regs.sp);
    println!("FastDisk mode: {:?}", emu.disk.speed_mode);
    
    // 画面表示（オプション）
    if show_screen {
        println!("\n=== Screen ===");
        // Apple II テキスト行アドレス
        let row_addrs = [
            0x400, 0x480, 0x500, 0x580, 0x600, 0x680, 0x700, 0x780,
            0x428, 0x4A8, 0x528, 0x5A8, 0x628, 0x6A8, 0x728, 0x7A8,
            0x450, 0x4D0, 0x550, 0x5D0, 0x650, 0x6D0, 0x750, 0x7D0,
        ];
        
        for (i, &base) in row_addrs.iter().enumerate() {
            let line: String = (0..40)
                .map(|j| {
                    let ch = emu.memory.main_ram[base + j] & 0x7F;
                    if ch >= 0x20 && ch < 0x7F { ch as char } else { '.' }
                })
                .collect();
            println!("Row {:2}: [{}]", i, line);
        }
    }
}
