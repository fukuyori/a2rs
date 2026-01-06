//! Xレジスタ追跡テスト

use a2rs::*;

fn main() {
    let mut emu = apple2::Apple2::new(memory::AppleModel::AppleIIPlus);
    
    let rom = std::fs::read("APPLE2.ROM").expect("ROM read failed");
    emu.load_rom(&rom);
    
    let disk_rom = std::fs::read("roms/disk2.rom").expect("Disk ROM read failed");
    emu.load_disk_rom(&disk_rom).expect("Disk ROM load failed");
    
    let disk = std::fs::read("disks/Apple_II_Graphics_Demo.DSK").expect("Disk read failed");
    emu.load_disk(0, &disk).expect("Disk load failed");
    
    emu.reset();
    
    println!("Starting from PC=${:04X}, X=${:02X}", emu.cpu.regs.pc, emu.cpu.regs.x);
    
    // ステップ実行して$C638でXレジスタの値を確認
    for i in 0..5000 {
        let pc = emu.cpu.regs.pc;
        let x = emu.cpu.regs.x;
        let a = emu.cpu.regs.a;
        let y = emu.cpu.regs.y;
        
        // 重要なアドレスでトレース
        if pc >= 0xC620 && pc <= 0xC640 && i < 500 {
            println!("[{:4}] PC=${:04X} A=${:02X} X=${:02X} Y=${:02X}", i, pc, a, x, y);
        }
        
        emu.step();
    }
    
    println!("\nFinal: PC=${:04X}, X=${:02X}", emu.cpu.regs.pc, emu.cpu.regs.x);
}
