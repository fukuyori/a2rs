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
    
    // パッチが適用されているか確認
    println!("Boot ROM at $C621-$C657:");
    for offset in [0x21, 0x22, 0x23, 0x4C, 0x4D, 0x4E, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57] {
        println!("  $C6{:02X}: ${:02X}", offset, emu.disk.boot_rom[offset]);
    }
    
    // 最初の500命令をトレース
    for i in 0..4000 {
        let pc = emu.cpu.regs.pc;
        let opcode = emu.read(pc);
        let x = emu.cpu.regs.x;
        let y = emu.cpu.regs.y;
        
        println!("[{:3}] ${:04X}: {:02X}  X=${:02X} Y=${:02X}", i, pc, opcode, x, y);
        
        emu.step();
        
        // $C661に到達したら停止
        if emu.cpu.regs.pc == 0xC661 {
            println!("Reached $C661 at step {}", i);
            break;
        }
    }
}
