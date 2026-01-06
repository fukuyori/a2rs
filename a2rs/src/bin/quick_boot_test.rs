//! 簡易ブートテスト

use a2rs::*;
use a2rs::cpu::MemoryBus;

fn main() {
    let mut emu = apple2::Apple2::new(memory::AppleModel::AppleIIPlus);
    
    // ROMをロード
    let rom = std::fs::read("APPLE2.ROM").expect("ROM read failed");
    emu.load_rom(&rom);
    
    // Disk ROMをロード  
    let disk_rom = std::fs::read("roms/disk2.rom").expect("Disk ROM read failed");
    emu.load_disk_rom(&disk_rom).expect("Disk ROM load failed");
    
    // ディスクをロード
    let disk = std::fs::read("disks/Apple_II_Graphics_Demo.DSK").expect("Disk read failed");
    emu.load_disk(0, &disk).expect("Disk load failed");
    
    // リセット
    emu.reset();
    
    println!("Initial state:");
    println!("  PC: ${:04X}", emu.cpu.regs.pc);
    println!("  Motor on: {}", emu.disk.motor_on);
    println!("  Spinning: {}", emu.disk.drives[0].spinning);
    println!("  Disk loaded: {}", emu.disk.drives[0].disk.disk_loaded);
    
    // 1000サイクル実行して状態を確認
    for i in 0..100 {
        emu.run_cycles(1000);
        
        if i < 5 || i % 10 == 0 {
            println!("\nAfter {} cycles:", (i+1) * 1000);
            println!("  PC: ${:04X}", emu.cpu.regs.pc);
            println!("  Motor: {}, Spinning: {}", emu.disk.motor_on, emu.disk.drives[0].spinning);
            println!("  Latch: ${:02X}", emu.disk.latch);
            println!("  Track: {}, Byte pos: {}", 
                     emu.disk.drives[0].current_track(),
                     emu.disk.drives[0].disk.byte_position);
        }
    }
    
    // メモリダンプ
    println!("\n\nMemory at $0800-$080F:");
    for i in 0..16 {
        print!("{:02X} ", emu.read(0x0800 + i));
    }
    println!();
}
