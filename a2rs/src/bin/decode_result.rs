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
    
    // $C6E5 (JMP $0801) に到達するまで実行
    for _ in 0..50_000 {
        if emu.cpu.regs.pc == 0xC6E5 {
            println!("Reached JMP $0801 at PC=$C6E5");
            break;
        }
        emu.step();
    }
    
    // デコード後のデータを確認
    println!("\nDecoded data at $0800-$080F:");
    for i in 0..16 {
        print!("{:02X} ", emu.read(0x0800 + i));
    }
    println!();
    
    println!("\nExpected (raw sector 0):");
    println!("01 A5 27 C9 09 D0 18 A5 2B 4A 4A 4A 4A 09 C0 85");
    
    // 補助バッファの内容
    println!("\nAux buffer at $0300-$030F:");
    for i in 0..16 {
        print!("{:02X} ", emu.read(0x0300 + i));
    }
    println!();
}
