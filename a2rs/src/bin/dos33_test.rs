use a2rs::*;
use a2rs::cpu::MemoryBus;

fn main() {
    let mut emu = apple2::Apple2::new(memory::AppleModel::AppleIIPlus);
    
    let rom = std::fs::read("APPLE2.ROM").unwrap();
    emu.load_rom(&rom);
    
    let disk_rom = std::fs::read("roms/disk2.rom").unwrap();
    emu.load_disk_rom(&disk_rom).unwrap();
    
    let disk = std::fs::read("disks/DOS_3_3_System_Master_-_680-0210-A.dsk").unwrap();
    emu.load_disk(0, &disk).unwrap();
    
    emu.reset();
    
    // 60フレーム（約1秒）実行
    for frame in 0..120 {
        emu.run_frame();
        
        if frame % 20 == 0 {
            let pc = emu.cpu.regs.pc;
            let mut non_zero = 0;
            for i in 0..40 {
                if emu.read(0x0400 + i) != 0 { non_zero += 1; }
            }
            println!("Frame {:3}: PC=${:04X} text_nonzero={}", frame, pc, non_zero);
        }
    }
    
    println!();
    println!("After 120 frames (2 seconds):");
    println!("Total cycles: {}", emu.total_cycles);
    println!("PC: ${:04X}", emu.cpu.regs.pc);
    
    // テキストメモリ全体をダンプ
    println!("\nText Page 1:");
    for row in 0..24 {
        let base = 0x400u16 + (row / 8) * 0x80 + (row % 8) * 0x28;
        print!("Row {:2}: ", row);
        for col in 0..40 {
            let ch = emu.read(base + col);
            if ch >= 0xA0 {
                print!("{}", (ch - 0x80) as char);
            } else if ch == 0 {
                print!(".");
            } else if ch >= 0x80 && ch < 0xA0 {
                // inverse uppercase
                print!("{}", ((ch - 0x80) + 0x40) as char);
            } else {
                print!("?");
            }
        }
        println!();
    }
}
