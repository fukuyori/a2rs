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
    
    // ブート完了後のPCトレース
    let mut last_pc_region = 0u16;
    let mut step = 0u64;
    
    for _ in 0..500_000 {
        let pc = emu.cpu.regs.pc;
        let region = pc & 0xFF00;
        
        // PC領域が変わったら報告
        if region != last_pc_region {
            println!("[{:7}] PC region changed: ${:04X} -> ${:04X}", step, last_pc_region, region);
            last_pc_region = region;
        }
        
        // $0800-$08FF（ブートセクタ）の実行を詳細にトレース
        if pc >= 0x0800 && pc <= 0x08FF && step < 100000 {
            let opcode = emu.read(pc);
            println!("[{:7}] ${:04X}: {:02X}  A={:02X} X={:02X} Y={:02X}", 
                     step, pc, opcode, emu.cpu.regs.a, emu.cpu.regs.x, emu.cpu.regs.y);
        }
        
        emu.step();
        step += 1;
    }
    
    println!("\nFinal PC: ${:04X}", emu.cpu.regs.pc);
}
