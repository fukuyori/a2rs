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
    
    // 初期化ループ完了まで実行
    for _ in 0..5000 {
        emu.step();
    }
    
    println!("PC=${:04X}", emu.cpu.regs.pc);
    
    // テーブル $0356-$03D5 を確認
    println!("\nDecode table at $0356-$03D5:");
    for row in 0..8 {
        print!("${:04X}: ", 0x0356 + row * 16);
        for col in 0..16 {
            let addr = 0x0356 + row * 16 + col;
            if addr <= 0x03D5 {
                print!("{:02X} ", emu.read(addr));
            }
        }
        println!();
    }
    
    // EOR参照時のアドレス計算
    println!("\nEOR $02D6,Y reference check:");
    println!("Y=$96 -> $02D6+$96 = ${:04X}", 0x02D6 + 0x96);
    println!("Y=$FF -> $02D6+$FF = ${:04X}", 0x02D6 + 0xFF);
}
