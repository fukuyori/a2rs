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
    
    // 50Mサイクル実行
    emu.run_cycles(50_000_000);
    
    println!("After 50M cycles: PC=${:04X}", emu.cpu.regs.pc);
    
    // テキストメモリ全体をダンプ
    println!("\nText Page 1:");
    for row in 0..24 {
        let base = 0x400 + (row / 8) * 0x80 + (row % 8) * 0x28;
        print!("Row {:2}: ", row);
        for col in 0..40 {
            let ch = emu.read(base + col);
            if ch >= 0xA0 && ch <= 0xDF {
                print!("{}", (ch - 0x80) as char);
            } else if ch >= 0x80 && ch <= 0x9F {
                print!("{}", (ch - 0x40) as char);
            } else if ch == 0x00 {
                print!("@");
            } else {
                print!(".");
            }
        }
        println!();
    }
}
