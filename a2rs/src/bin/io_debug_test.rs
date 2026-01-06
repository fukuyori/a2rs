//! I/Oデバッグテスト

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
    
    println!("Starting from PC=${:04X}", emu.cpu.regs.pc);
    
    // ステップ実行して$C0E0-$C0EFへのアクセスを監視
    for i in 0..50000 {
        let pc = emu.cpu.regs.pc;
        let opcode = emu.read(pc);
        
        // LDA abs,X ($BD) を検出
        if opcode == 0xBD {
            let lo = emu.read(pc + 1);
            let hi = emu.read(pc + 2);
            let base_addr = (hi as u16) << 8 | lo as u16;
            let x = emu.cpu.regs.x;
            let eff_addr = base_addr.wrapping_add(x as u16);
            
            if eff_addr >= 0xC0E0 && eff_addr <= 0xC0EF {
                println!("[{:6}] ${:04X}: LDA ${:04X},X (X=${:02X}) -> ${:04X}", 
                         i, pc, base_addr, x, eff_addr);
                println!("         Before: motor={}, spinning={}", 
                         emu.disk.motor_on, emu.disk.drives[0].spinning);
            }
        }
        
        emu.step();
        
        // モーターがオンになったら報告
        if emu.disk.motor_on && i < 200 {
            println!("[{:6}] Motor turned ON! PC=${:04X}", i, emu.cpu.regs.pc);
        }
    }
    
    println!("\nFinal state:");
    println!("  PC: ${:04X}", emu.cpu.regs.pc);
    println!("  Motor: {}, Spinning: {}", emu.disk.motor_on, emu.disk.drives[0].spinning);
}
