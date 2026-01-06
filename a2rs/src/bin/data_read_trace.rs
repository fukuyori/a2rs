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
    
    let mut data_count = 0;
    
    for step in 0..100_000 {
        let pc = emu.cpu.regs.pc;
        let a = emu.cpu.regs.a;
        let y = emu.cpu.regs.y;
        
        // $C6AF: EOR $02D6,Y (補助データのXORデコード)
        if pc == 0xC6AF {
            let table_val = emu.read(0x02D6 + y as u16);
            println!("[{:5}] EOR aux: Y=${:02X} A=${:02X} table=${:02X} result=${:02X}", 
                     step, y, a, table_val, a ^ table_val);
            data_count += 1;
            if data_count > 5 { break; }
        }
        
        // $C6C1: EOR $02D6,Y (メインデータのXORデコード) 
        if pc == 0xC6C1 {
            let table_val = emu.read(0x02D6 + y as u16);
            println!("[{:5}] EOR main: Y=${:02X} A=${:02X} table=${:02X} result=${:02X}",
                     step, y, a, table_val, a ^ table_val);
        }
        
        emu.step();
    }
}
