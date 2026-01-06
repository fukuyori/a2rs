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
    
    // 10Mサイクル実行
    emu.run_cycles(10_000_000);
    
    println!("After 10M cycles: PC=${:04X}", emu.cpu.regs.pc);
    
    // テキストメモリ全体をダンプ ($0400-$07FF)
    println!("\nText Page 1 ($0400-$07FF):");
    let mut non_zero = 0;
    for row in 0..24 {
        let base = 0x400 + (row / 8) * 0x80 + (row % 8) * 0x28;
        print!("Row {:2} (${:04X}): ", row, base);
        for col in 0..40 {
            let ch = emu.read(base + col);
            if ch != 0 { non_zero += 1; }
            if ch >= 0xA0 && ch <= 0xDF {
                print!("{}", (ch - 0x80) as char);
            } else if ch >= 0x80 && ch <= 0x9F {
                print!("{}", (ch - 0x40) as char);
            } else {
                print!(".");
            }
        }
        println!();
    }
    println!("\nNon-zero bytes in text memory: {}", non_zero);
    
    // $0800-$08FFも確認
    println!("\nBoot sector ($0800-$080F):");
    for i in 0..16 {
        print!("{:02X} ", emu.read(0x0800 + i));
    }
    println!();
}
