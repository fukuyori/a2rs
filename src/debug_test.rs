//! デバッグ用テスト - テキスト画面に直接書き込んでテスト

use crate::cpu::{Cpu, CpuType, MemoryBus};
use crate::memory::{Memory, AppleModel};
use crate::video::{Video, SCREEN_WIDTH, SCREEN_HEIGHT};
use crate::apple2::{self, Apple2};
use std::fs::File;
use std::io::Write;

/// フレームバッファをPPM画像として保存
fn save_framebuffer_ppm(framebuffer: &[u32], path: &str) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(file, "P3")?;
    writeln!(file, "{} {}", SCREEN_WIDTH, SCREEN_HEIGHT)?;
    writeln!(file, "255")?;
    
    for y in 0..SCREEN_HEIGHT {
        for x in 0..SCREEN_WIDTH {
            let pixel = framebuffer[y * SCREEN_WIDTH + x];
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;
            write!(file, "{} {} {} ", r, g, b)?;
        }
        writeln!(file)?;
    }
    Ok(())
}

pub fn test_text_display() {
    println!("=== Text Display Debug Test ===\n");
    
    let mut memory = Memory::new(AppleModel::AppleIIPlus);
    let mut video = Video::new();
    
    // テキストモードを有効化
    memory.switches.text_mode = true;
    memory.switches.page2 = false;
    
    // 画面をスペースでクリア
    for addr in 0x0400..0x0800 {
        memory.main_ram[addr] = 0xA0; // space with high bit
    }
    
    // 行0 ($0400) に "HELLO" を書き込む
    let hello = [0xC8, 0xC5, 0xCC, 0xCC, 0xCF]; // "HELLO" with high bit
    for (i, &ch) in hello.iter().enumerate() {
        memory.main_ram[0x0400 + i] = ch;
    }
    
    // 行1 ($0480) に "WORLD" を書き込む
    let world = [0xD7, 0xCF, 0xD2, 0xCC, 0xC4]; // "WORLD" with high bit
    for (i, &ch) in world.iter().enumerate() {
        memory.main_ram[0x0480 + i] = ch;
    }
    
    // 行12 ($0628) に "APPLE II" を書き込む
    let apple = [0xC1, 0xD0, 0xD0, 0xCC, 0xC5, 0xA0, 0xC9, 0xC9];
    for (i, &ch) in apple.iter().enumerate() {
        memory.main_ram[0x0628 + i] = ch;
    }
    
    // メモリ内容を確認
    println!("Memory at $0400: {:02X} {:02X} {:02X} {:02X} {:02X}",
             memory.main_ram[0x0400], memory.main_ram[0x0401],
             memory.main_ram[0x0402], memory.main_ram[0x0403],
             memory.main_ram[0x0404]);
    
    // ビデオレンダリング
    video.render(&memory);
    
    // フレームバッファの一部を確認
    let non_black = video.framebuffer.iter().filter(|&&p| p != 0).count();
    println!("Non-black pixels: {}", non_black);
    
    // PPM画像として保存
    match save_framebuffer_ppm(&video.framebuffer, "debug_output.ppm") {
        Ok(()) => println!("Framebuffer saved to debug_output.ppm"),
        Err(e) => println!("Failed to save framebuffer: {}", e),
    }
    
    println!("\n=== Test Complete ===");
}

pub fn test_rom_execution() {
    println!("=== ROM Execution Debug Test ===\n");
    
    let mut memory = Memory::new(AppleModel::AppleIIPlus);
    let mut cpu = Cpu::new(CpuType::Cpu6502);
    
    // テストROMをロード
    let rom = apple2::create_test_rom();
    memory.load_rom(&rom);
    
    // リセットベクターを確認
    let reset_lo = memory.read(0xFFFC);
    let reset_hi = memory.read(0xFFFD);
    let reset_vec = (reset_hi as u16) << 8 | reset_lo as u16;
    println!("Reset vector: ${:04X}", reset_vec);
    
    // ROMの内容を表示
    println!("\nROM at $F000-$F00F:");
    for i in 0..16 {
        print!("{:02X} ", memory.read(0xF000 + i));
    }
    println!();
    
    // CPUを初期化
    cpu.regs.pc = reset_vec;
    cpu.regs.sp = 0xFF;
    cpu.regs.status = 0x24;
    
    // 5000命令実行（画面クリア+メッセージ表示に十分）
    println!("\nExecuting 5000 instructions...");
    for _ in 0..5000 {
        let pc = cpu.regs.pc;
        cpu.step(&mut memory);
        
        // 無限ループ検出
        if cpu.regs.pc == pc {
            println!("Reached main loop at ${:04X}", pc);
            break;
        }
    }
    
    println!("\nFinal CPU state:");
    println!("  PC=${:04X} A=${:02X} X=${:02X} Y=${:02X} SP=${:02X}",
             cpu.regs.pc, cpu.regs.a, cpu.regs.x, cpu.regs.y, cpu.regs.sp);
    
    // メモリの状態を確認
    println!("\nSoft switches state:");
    println!("  text_mode: {}", memory.switches.text_mode);
    println!("  page2: {}", memory.switches.page2);
    
    // テキスト画面の内容を表示
    println!("\nText screen memory:");
    println!("Row 0 ($0400): ");
    for i in 0..40 {
        let ch = memory.main_ram[0x0400 + i];
        print!("{:02X} ", ch);
    }
    println!();
    
    println!("Row 12 ($0628): ");
    for i in 0..40 {
        let ch = memory.main_ram[0x0628 + i];
        print!("{:02X} ", ch);
    }
    println!();
    
    // 文字として表示
    println!("\nAs characters:");
    print!("Row 0:  ");
    for i in 0..40 {
        let ch = memory.main_ram[0x0400 + i] & 0x7F;
        if ch >= 0x20 && ch < 0x7F {
            print!("{}", ch as char);
        } else {
            print!(".");
        }
    }
    println!();
    
    print!("Row 12: ");
    for i in 0..40 {
        let ch = memory.main_ram[0x0628 + i] & 0x7F;
        if ch >= 0x20 && ch < 0x7F {
            print!("{}", ch as char);
        } else {
            print!(".");
        }
    }
    println!();
    
    // ビデオをレンダリングしてPPM出力
    let mut video = Video::new();
    video.render(&memory);
    match save_framebuffer_ppm(&video.framebuffer, "rom_output.ppm") {
        Ok(()) => println!("\nFramebuffer saved to rom_output.ppm"),
        Err(e) => println!("\nFailed to save framebuffer: {}", e),
    }
    
    println!("\n=== Test Complete ===");
}

/// apple2dead.bin ROMのテスト
pub fn test_apple2dead_rom(rom_path: &str) {
    println!("=== Apple II ROM Test ===\n");
    
    // ROMサイズを確認してモデルを選択
    let rom_data = match std::fs::read(rom_path) {
        Ok(data) => data,
        Err(e) => {
            println!("Failed to load ROM: {}", e);
            return;
        }
    };
    
    let model = if rom_data.len() == 32768 {
        println!("Detected 32KB ROM - using Apple IIe model");
        AppleModel::AppleIIe
    } else {
        println!("Using Apple II+ model");
        AppleModel::AppleIIPlus
    };
    
    let mut emu = Apple2::new(model);
    
    // ROMをロード
    emu.load_rom(&rom_data);
    println!("Loaded ROM: {} ({} bytes)", rom_path, rom_data.len());
    
    // リセット
    emu.reset();
    println!("Reset vector: ${:04X}", emu.cpu.regs.pc);
    
    // 1億サイクル実行（RAMテストが完了するまで）
    println!("\nRunning 100M cycles...");
    emu.run_cycles(100_000_000);
    
    println!("\nFinal CPU state:");
    println!("  PC=${:04X} A=${:02X} X=${:02X} Y=${:02X} SP=${:02X}",
             emu.cpu.regs.pc, emu.cpu.regs.a, emu.cpu.regs.x, emu.cpu.regs.y, emu.cpu.regs.sp);
    
    // テキスト画面の内容を表示（全24行）
    println!("\n=== Text Screen Contents ===");
    let row_offsets: [(usize, &str); 24] = [
        (0x0400, "Row  0"), (0x0480, "Row  1"), (0x0500, "Row  2"), (0x0580, "Row  3"),
        (0x0600, "Row  4"), (0x0680, "Row  5"), (0x0700, "Row  6"), (0x0780, "Row  7"),
        (0x0428, "Row  8"), (0x04A8, "Row  9"), (0x0528, "Row 10"), (0x05A8, "Row 11"),
        (0x0628, "Row 12"), (0x06A8, "Row 13"), (0x0728, "Row 14"), (0x07A8, "Row 15"),
        (0x0450, "Row 16"), (0x04D0, "Row 17"), (0x0550, "Row 18"), (0x05D0, "Row 19"),
        (0x0650, "Row 20"), (0x06D0, "Row 21"), (0x0750, "Row 22"), (0x07D0, "Row 23"),
    ];
    
    for (addr, label) in row_offsets.iter() {
        print!("{}: ", label);
        for i in 0..40 {
            let ch = emu.memory.main_ram[addr + i] & 0x7F;
            if ch >= 0x20 && ch < 0x7F {
                print!("{}", ch as char);
            } else {
                print!(".");
            }
        }
        println!();
    }
    
    // ビデオをレンダリングしてPPM出力
    emu.video.render(&emu.memory);
    match save_framebuffer_ppm(&emu.video.framebuffer, "rom_test_output.ppm") {
        Ok(()) => println!("\nFramebuffer saved to rom_test_output.ppm"),
        Err(e) => println!("\nFailed to save framebuffer: {}", e),
    }
    
    println!("\n=== Test Complete ===");
}
