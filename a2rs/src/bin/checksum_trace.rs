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
    
    // $C6D3 (BNE after checksum) をトレース
    for step in 0..100_000 {
        let pc = emu.cpu.regs.pc;
        let a = emu.cpu.regs.a;
        let p = emu.cpu.regs.status;
        
        // $C6D0: EOR $02D6,Y (チェックサム検証)
        if pc == 0xC6D0 {
            let y = emu.cpu.regs.y;
            let table_val = emu.read(0x02D6 + y as u16);
            println!("[{:5}] Checksum EOR: A=${:02X} Y=${:02X} table=${:02X} result=${:02X}",
                     step, a, y, table_val, a ^ table_val);
        }
        
        // $C6D3: BNE $C65C (チェックサム検証失敗)
        if pc == 0xC6D3 {
            let z_flag = (p & 0x02) != 0;
            println!("[{:5}] BNE check: A=${:02X} Z={} ({})",
                     step, a, z_flag, if z_flag { "PASS" } else { "FAIL - retrying" });
            if z_flag {
                println!("Checksum passed! Continuing to decode...");
                break;
            }
        }
        
        emu.step();
    }
}
