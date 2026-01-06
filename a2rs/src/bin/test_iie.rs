//! Apple IIe ROM テスト

use a2rs::cpu::{Cpu, CpuType, MemoryBus};
use a2rs::memory::{Memory, AppleModel};

fn main() {
    println!("=== Apple IIe ROM Debug ===\n");
    
    let mut memory = Memory::new(AppleModel::AppleIIe);
    let mut cpu = Cpu::new(CpuType::Cpu6502);
    
    // ROMをロード
    let rom_data = std::fs::read("APPLE2E.ROM").expect("Failed to read ROM");
    memory.load_rom(&rom_data);
    
    // リセットベクターを確認
    let reset_lo = memory.read(0xFFFC);
    let reset_hi = memory.read(0xFFFD);
    let reset_vec = (reset_hi as u16) << 8 | reset_lo as u16;
    println!("Reset vector: ${:04X}", reset_vec);
    
    // ROMの一部を表示
    println!("\nROM at $FA62 (reset entry):");
    for i in 0..32 {
        print!("{:02X} ", memory.read(0xFA62 + i));
        if i % 16 == 15 { println!(); }
    }
    
    // CPUをリセット
    cpu.reset(&mut memory);
    println!("\nCPU after reset: PC=${:04X}", cpu.regs.pc);
    
    // 最初の100命令を実行しながらトレース
    println!("\nExecuting first 100 instructions:");
    for i in 0..100 {
        let pc = cpu.regs.pc;
        let opcode = memory.read(pc);
        let byte1 = memory.read(pc.wrapping_add(1));
        let byte2 = memory.read(pc.wrapping_add(2));
        
        if i < 20 || i >= 95 {
            println!("{:4}: ${:04X}: {:02X} {:02X} {:02X}  A={:02X} X={:02X} Y={:02X} SP={:02X}",
                     i, pc, opcode, byte1, byte2,
                     cpu.regs.a, cpu.regs.x, cpu.regs.y, cpu.regs.sp);
        } else if i == 20 {
            println!("  ...");
        }
        
        cpu.step(&mut memory);
    }
    
    // さらに実行
    println!("\nRunning 10000 more instructions...");
    for _ in 0..10000 {
        cpu.step(&mut memory);
    }
    
    println!("\nFinal state: PC=${:04X} A={:02X} X={:02X} Y={:02X} SP={:02X}",
             cpu.regs.pc, cpu.regs.a, cpu.regs.x, cpu.regs.y, cpu.regs.sp);
    
    // ソフトスイッチの状態
    println!("\nSoft switches:");
    println!("  text_mode: {}", memory.switches.text_mode);
    println!("  page2: {}", memory.switches.page2);
    println!("  hires: {}", memory.switches.hires);
    
    // テキスト画面の最初の数バイト
    println!("\nText page 1 ($0400):");
    for i in 0..40 {
        print!("{:02X} ", memory.main_ram[0x0400 + i]);
    }
    println!();
}
