use a2rs::*;
use a2rs::cpu::MemoryBus;

fn main() {
    let mut emu = apple2::Apple2::new(memory::AppleModel::AppleIIPlus);
    
    let rom = std::fs::read("APPLE2.ROM").expect("ROM read failed");
    emu.load_rom(&rom);
    
    let disk_rom = std::fs::read("roms/disk2.rom").expect("Disk ROM read failed");
    emu.load_disk_rom(&disk_rom).expect("Disk ROM load failed");
    
    let disk = std::fs::read("disks/Apple_II_Graphics_Demo.DSK").expect("Disk read failed");
    emu.load_disk(0, &disk).expect("Disk load failed");
    
    emu.reset();
    
    // $C620-$C660の範囲をトレース
    for i in 0..5000 {
        let pc = emu.cpu.regs.pc;
        let x = emu.cpu.regs.x;
        let y = emu.cpu.regs.y;
        
        if pc >= 0xC620 && pc <= 0xC665 {
            let opcode = emu.read(pc);
            println!("[{:4}] ${:04X}: {:02X}  X=${:02X} Y=${:02X}  motor={}", 
                     i, pc, opcode, x, y, emu.disk.motor_on);
        }
        
        emu.step();
        
        if pc == 0xC661 {
            println!("Reached $C661 - motor={}, spinning={}", 
                     emu.disk.motor_on, emu.disk.drives[0].spinning);
            break;
        }
    }
}
