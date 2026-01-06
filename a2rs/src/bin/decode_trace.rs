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
    
    let mut decode_count = 0;
    
    for step in 0..100_000 {
        let pc = emu.cpu.regs.pc;
        
        // $C6D5: LDY #$00 (デコードループ開始)
        if pc == 0xC6D5 {
            println!("[{:5}] Decode loop start", step);
        }
        
        // $C6DC: LDA ($26),Y (メインデータ読み取り)
        if pc == 0xC6DC && decode_count < 5 {
            let y = emu.cpu.regs.y;
            let ptr_lo = emu.read(0x26);
            let ptr_hi = emu.read(0x27);
            let ptr = (ptr_hi as u16) << 8 | ptr_lo as u16;
            let data = emu.read(ptr + y as u16);
            println!("[{:5}] LDA (${:04X}),Y=${:02X} = ${:02X}", step, ptr, y, data);
        }
        
        // $C6E6: STA ($26),Y (デコード後の書き込み)
        if pc == 0xC6E6 && decode_count < 5 {
            let a = emu.cpu.regs.a;
            let y = emu.cpu.regs.y;
            println!("[{:5}] STA decoded byte: A=${:02X} Y=${:02X}", step, a, y);
            decode_count += 1;
        }
        
        // $C6E5: JMP $0801
        if pc == 0x0801 {
            println!("[{:5}] Jumped to $0801 - boot code execution", step);
            break;
        }
        
        emu.step();
    }
    
    println!("\nDecoded data at $0800-$080F:");
    for i in 0..16 {
        print!("{:02X} ", emu.read(0x0800 + i));
    }
    println!();
}
