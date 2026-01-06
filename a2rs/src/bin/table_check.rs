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
    
    // 初期化ループ完了まで実行（約3000命令）
    for _ in 0..5000 {
        emu.step();
    }
    
    println!("After init loop, PC=${:04X}", emu.cpu.regs.pc);
    
    // XORデコードテーブル $02D6-$0355 を確認
    println!("\nXOR decode table at $02D6-$0335 (first 96 bytes):");
    for row in 0..6 {
        print!("${:04X}: ", 0x02D6 + row * 16);
        for col in 0..16 {
            print!("{:02X} ", emu.read(0x02D6 + row * 16 + col));
        }
        println!();
    }
    
    // READ_TABLEの期待値
    println!("\nExpected READ_TABLE values:");
    let read_table: [u8; 64] = [
        0x00, 0x01, 0x00, 0x00, 0x02, 0x03, 0x00, 0x04,
        0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x07, 0x08, 0x00, 0x00, 0x00, 0x09, 0x0A, 0x0B,
        0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13,
        0x00, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x1B, 0x00, 0x1C, 0x1D, 0x1E,
        0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x20, 0x21,
    ];
    for i in 0..64 {
        print!("{:02X} ", read_table[i]);
        if (i + 1) % 16 == 0 { println!(); }
    }
}
