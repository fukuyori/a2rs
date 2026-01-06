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
    
    // 50Mサイクル実行
    emu.run_cycles(50_000_000);
    
    println!("After 50M cycles:");
    println!("PC: ${:04X}", emu.cpu.regs.pc);
    println!("Text mode: {}", emu.memory.switches.text_mode);
    
    // テキストメモリのraw dump
    println!("\nText memory $0400-$07FF raw:");
    for row in 0..24 {
        let base = 0x400u16 + (row / 8) * 0x80 + (row % 8) * 0x28;
        print!("${:04X}: ", base);
        for col in 0..40 {
            print!("{:02X} ", emu.read(base + col));
        }
        println!();
    }
}
