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
    
    // 60フレーム実行（各フレーム約17030サイクル）
    for frame in 0..60 {  // 60フレーム = 約1秒
        let start_cycles = emu.total_cycles;
        emu.run_frame();
        let end_cycles = emu.total_cycles;
        
        if frame % 10 == 0 {
            let pc = emu.cpu.regs.pc;
            let motor = emu.disk.motor_on;
            let latch = emu.disk.latch;
            
            // テキストメモリの最初の行の非ゼロバイト数
            let mut non_zero = 0;
            for i in 0..40 {
                if emu.read(0x0400 + i) != 0 { non_zero += 1; }
            }
            
            println!("Frame {:3}: PC=${:04X} motor={} latch=${:02X} text_nonzero={} cycles_ran={}",
                     frame, pc, motor, latch, non_zero, end_cycles - start_cycles);
        }
    }
    
    println!();
    println!("After 60 frames:");
    println!("Total cycles: {}", emu.total_cycles);
    println!("PC: ${:04X}", emu.cpu.regs.pc);
    
    // テキストメモリの最初の行
    print!("Text row 0: ");
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
