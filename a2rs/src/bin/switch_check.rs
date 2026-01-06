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
    println!();
    println!("Soft switches:");
    println!("  Text mode: {}", emu.memory.switches.text_mode);
    println!("  Hires: {}", emu.memory.switches.hires);
    println!("  Mixed mode: {}", emu.memory.switches.mixed_mode);
    println!("  Page2: {}", emu.memory.switches.page2);
    
    // Hi-resグラフィックメモリの確認
    println!();
    println!("Hi-res page 1 ($2000-$200F):");
    for i in 0..16 {
        print!("{:02X} ", emu.read(0x2000 + i));
    }
    println!();
}
