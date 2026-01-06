use a2rs::*;
use a2rs::cpu::MemoryBus;

fn main() {
    let mut emu = apple2::Apple2::new(memory::AppleModel::AppleIIPlus);
    
    let rom = std::fs::read("APPLE2.ROM").unwrap();
    emu.load_rom(&rom);
    
    let disk_rom = std::fs::read("roms/disk2.rom").unwrap();
    emu.load_disk_rom(&disk_rom).unwrap();
    
    let disk = std::fs::read("disks/Apple_II_Graphics_Demo.DSK").unwrap();
    emu.load_disk(0, &disk).unwrap();
    
    emu.reset();
    
    // 各マイルストーンでのPCを記録
    let milestones = [100_000u64, 500_000, 1_000_000, 5_000_000, 10_000_000];
    let mut current_cycles = 0u64;
    
    for milestone in milestones {
        let to_run = milestone - current_cycles;
        emu.run_cycles(to_run);
        current_cycles = milestone;
        
        // テキストメモリの最初の行をチェック
        let mut non_zero = 0;
        for i in 0..40 {
            if emu.read(0x0400 + i) != 0 { non_zero += 1; }
        }
        
        println!("After {:>10} cycles: PC=${:04X}, non-zero text bytes row0: {}", 
                 milestone, emu.cpu.regs.pc, non_zero);
    }
    
    println!();
    println!("Final text memory row 0:");
    for i in 0..40 {
        let ch = emu.read(0x0400 + i);
        if ch >= 0xA0 {
            print!("{}", (ch - 0x80) as char);
        } else if ch == 0 {
            print!(".");
        } else {
            print!("?");
        }
    }
    println!();
}
