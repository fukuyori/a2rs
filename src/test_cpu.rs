//! Klaus2m5 6502 Functional Test Runner
//!
//! テストの実行方法:
//! cargo run --release -- --test-cpu

use crate::cpu::{Cpu, CpuType, MemoryBus};
use std::fs;

/// テスト用のシンプルなメモリ（64KB RAM）
pub struct TestMemory {
    pub ram: Vec<u8>,
}

impl TestMemory {
    pub fn new() -> Self {
        TestMemory { ram: vec![0; 65536] }
    }
    
    pub fn load(&mut self, address: u16, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            let addr = (address as usize).wrapping_add(i) & 0xFFFF;
            self.ram[addr] = byte;
        }
    }
}

impl MemoryBus for TestMemory {
    fn read(&mut self, address: u16) -> u8 {
        self.ram[address as usize]
    }
    
    fn write(&mut self, address: u16, value: u8) {
        self.ram[address as usize] = value;
    }
}

/// Klaus2m5の6502機能テストを実行
/// 
/// テストバイナリは$0000からロードされ、$0400から実行開始
/// 成功すると$3469で無限ループ（JMP $3469）に入る
/// 失敗するとそれ以外のアドレスでトラップ（同じアドレスへのJMP）
pub fn run_functional_test(test_path: &str) -> Result<bool, String> {
    // テストバイナリをロード
    let data = fs::read(test_path)
        .map_err(|e| format!("Failed to load test file: {}", e))?;
    
    if data.len() != 65536 {
        return Err(format!("Expected 65536 bytes, got {}", data.len()));
    }
    
    println!("Running Klaus2m5 6502 Functional Test...");
    println!("Test file: {}", test_path);
    println!("Size: {} bytes", data.len());
    
    // CPUとメモリを初期化
    let mut cpu = Cpu::new(CpuType::Cpu6502);
    let mut memory = TestMemory::new();
    
    // テストバイナリをロード（$0000から）
    memory.load(0x0000, &data);
    
    // テスト開始アドレスを設定（$0400）
    cpu.regs.pc = 0x0400;
    cpu.regs.sp = 0xFF;
    cpu.regs.status = 0x00;
    
    // リセットベクターを設定（$FFFC-$FFFD）
    memory.ram[0xFFFC] = 0x00;
    memory.ram[0xFFFD] = 0x04;
    
    let mut cycles: u64 = 0;
    let mut trap_count = 0;
    let max_cycles: u64 = 100_000_000; // 1億サイクルでタイムアウト
    
    println!("\nStarting execution at ${:04X}", cpu.regs.pc);
    println!("Success address: $3469");
    println!("");
    
    loop {
        let pc_before = cpu.regs.pc;
        let step_cycles = cpu.step(&mut memory);
        cycles += step_cycles as u64;
        
        // 同じアドレスにいる（トラップ検出）
        if cpu.regs.pc == pc_before {
            trap_count += 1;
            if trap_count > 2 {
                // トラップ検出
                if cpu.regs.pc == 0x3469 {
                    println!("SUCCESS! Test passed at ${:04X}", cpu.regs.pc);
                    println!("Total cycles: {}", cycles);
                    return Ok(true);
                } else {
                    // テスト失敗 - どのテストで失敗したか調べる
                    let test_num = memory.ram[0x0200];
                    println!("FAILED! Trap at ${:04X}", cpu.regs.pc);
                    println!("Test number: ${:02X} ({})", test_num, test_num);
                    println!("Total cycles: {}", cycles);
                    println!("\nCPU State:");
                    println!("  A=${:02X} X=${:02X} Y=${:02X}", 
                             cpu.regs.a, cpu.regs.x, cpu.regs.y);
                    println!("  SP=${:02X} Status=${:02X}", 
                             cpu.regs.sp, cpu.regs.status);
                    print_status_flags(cpu.regs.status);
                    return Ok(false);
                }
            }
        } else {
            trap_count = 0;
        }
        
        // 進捗表示（100万サイクルごと）
        if cycles % 1_000_000 == 0 {
            print!("\rCycles: {}M, PC: ${:04X}    ", cycles / 1_000_000, cpu.regs.pc);
            use std::io::Write;
            std::io::stdout().flush().ok();
        }
        
        // タイムアウト
        if cycles >= max_cycles {
            println!("\n\nTIMEOUT after {} cycles", cycles);
            println!("Last PC: ${:04X}", cpu.regs.pc);
            return Ok(false);
        }
    }
}

/// 65C02拡張命令テストを実行
pub fn run_65c02_test(test_path: &str) -> Result<bool, String> {
    let data = fs::read(test_path)
        .map_err(|e| format!("Failed to load test file: {}", e))?;
    
    if data.len() != 65536 {
        return Err(format!("Expected 65536 bytes, got {}", data.len()));
    }
    
    println!("Running Klaus2m5 65C02 Extended Opcodes Test...");
    println!("Test file: {}", test_path);
    
    let mut cpu = Cpu::new(CpuType::Cpu65C02);
    let mut memory = TestMemory::new();
    
    memory.load(0x0000, &data);
    cpu.regs.pc = 0x0400;
    cpu.regs.sp = 0xFF;
    cpu.regs.status = 0x00;
    
    let mut cycles: u64 = 0;
    let mut trap_count = 0;
    let max_cycles: u64 = 100_000_000;
    
    println!("\nStarting execution at ${:04X}", cpu.regs.pc);
    
    loop {
        let pc_before = cpu.regs.pc;
        let step_cycles = cpu.step(&mut memory);
        cycles += step_cycles as u64;
        
        if cpu.regs.pc == pc_before {
            trap_count += 1;
            if trap_count > 2 {
                if cpu.regs.pc == 0x24f1 {
                    println!("\nSUCCESS! 65C02 Test passed at ${:04X}", cpu.regs.pc);
                    println!("Total cycles: {}", cycles);
                    return Ok(true);
                } else {
                    let test_num = memory.ram[0x0200];
                    println!("\nFAILED! Trap at ${:04X}", cpu.regs.pc);
                    println!("Test number: ${:02X}", test_num);
                    println!("Total cycles: {}", cycles);
                    return Ok(false);
                }
            }
        } else {
            trap_count = 0;
        }
        
        if cycles % 1_000_000 == 0 {
            print!("\rCycles: {}M, PC: ${:04X}    ", cycles / 1_000_000, cpu.regs.pc);
            use std::io::Write;
            std::io::stdout().flush().ok();
        }
        
        if cycles >= max_cycles {
            println!("\n\nTIMEOUT after {} cycles", cycles);
            return Ok(false);
        }
    }
}

fn print_status_flags(status: u8) {
    println!("  Flags: {}{}{}{}{}{}{}{}",
             if status & 0x80 != 0 { "N" } else { "n" },
             if status & 0x40 != 0 { "V" } else { "v" },
             if status & 0x20 != 0 { "-" } else { "-" },
             if status & 0x10 != 0 { "B" } else { "b" },
             if status & 0x08 != 0 { "D" } else { "d" },
             if status & 0x04 != 0 { "I" } else { "i" },
             if status & 0x02 != 0 { "Z" } else { "z" },
             if status & 0x01 != 0 { "C" } else { "c" });
}

/// 簡易CPUテスト（テストファイルなしで実行可能）
pub fn run_quick_tests() {
    println!("Running quick CPU tests...\n");
    
    let mut passed = 0;
    let mut failed = 0;
    
    // Test LDA immediate
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        mem.ram[0x0000] = 0xA9; // LDA #$42
        mem.ram[0x0001] = 0x42;
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.a == 0x42 && cpu.regs.pc == 0x0002 {
            println!("✓ LDA immediate");
            passed += 1;
        } else {
            println!("✗ LDA immediate: A=${:02X}, PC=${:04X}", cpu.regs.a, cpu.regs.pc);
            failed += 1;
        }
    }
    
    // Test LDX immediate
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        mem.ram[0x0000] = 0xA2; // LDX #$33
        mem.ram[0x0001] = 0x33;
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.x == 0x33 {
            println!("✓ LDX immediate");
            passed += 1;
        } else {
            println!("✗ LDX immediate: X=${:02X}", cpu.regs.x);
            failed += 1;
        }
    }
    
    // Test LDY immediate
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        mem.ram[0x0000] = 0xA0; // LDY #$55
        mem.ram[0x0001] = 0x55;
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.y == 0x55 {
            println!("✓ LDY immediate");
            passed += 1;
        } else {
            println!("✗ LDY immediate: Y=${:02X}", cpu.regs.y);
            failed += 1;
        }
    }
    
    // Test STA zero page
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        cpu.regs.a = 0x77;
        mem.ram[0x0000] = 0x85; // STA $10
        mem.ram[0x0001] = 0x10;
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if mem.ram[0x10] == 0x77 {
            println!("✓ STA zero page");
            passed += 1;
        } else {
            println!("✗ STA zero page: [$10]=${:02X}", mem.ram[0x10]);
            failed += 1;
        }
    }
    
    // Test ADC (no carry)
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        cpu.regs.a = 0x10;
        cpu.regs.status = 0x00; // Clear carry
        mem.ram[0x0000] = 0x69; // ADC #$20
        mem.ram[0x0001] = 0x20;
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.a == 0x30 {
            println!("✓ ADC immediate (no carry)");
            passed += 1;
        } else {
            println!("✗ ADC immediate: A=${:02X}", cpu.regs.a);
            failed += 1;
        }
    }
    
    // Test ADC (with carry in)
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        cpu.regs.a = 0x10;
        cpu.regs.status = 0x01; // Set carry
        mem.ram[0x0000] = 0x69; // ADC #$20
        mem.ram[0x0001] = 0x20;
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.a == 0x31 {
            println!("✓ ADC immediate (with carry)");
            passed += 1;
        } else {
            println!("✗ ADC immediate (with carry): A=${:02X}", cpu.regs.a);
            failed += 1;
        }
    }
    
    // Test SBC
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        cpu.regs.a = 0x50;
        cpu.regs.status = 0x01; // Set carry (no borrow)
        mem.ram[0x0000] = 0xE9; // SBC #$10
        mem.ram[0x0001] = 0x10;
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.a == 0x40 {
            println!("✓ SBC immediate");
            passed += 1;
        } else {
            println!("✗ SBC immediate: A=${:02X}", cpu.regs.a);
            failed += 1;
        }
    }
    
    // Test INX
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        cpu.regs.x = 0x05;
        mem.ram[0x0000] = 0xE8; // INX
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.x == 0x06 {
            println!("✓ INX");
            passed += 1;
        } else {
            println!("✗ INX: X=${:02X}", cpu.regs.x);
            failed += 1;
        }
    }
    
    // Test DEX
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        cpu.regs.x = 0x05;
        mem.ram[0x0000] = 0xCA; // DEX
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.x == 0x04 {
            println!("✓ DEX");
            passed += 1;
        } else {
            println!("✗ DEX: X=${:02X}", cpu.regs.x);
            failed += 1;
        }
    }
    
    // Test JMP absolute
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        mem.ram[0x0000] = 0x4C; // JMP $1234
        mem.ram[0x0001] = 0x34;
        mem.ram[0x0002] = 0x12;
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.pc == 0x1234 {
            println!("✓ JMP absolute");
            passed += 1;
        } else {
            println!("✗ JMP absolute: PC=${:04X}", cpu.regs.pc);
            failed += 1;
        }
    }
    
    // Test BNE (branch taken)
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        cpu.regs.status = 0x00; // Z=0
        mem.ram[0x0000] = 0xD0; // BNE +5
        mem.ram[0x0001] = 0x05;
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.pc == 0x0007 {
            println!("✓ BNE (taken)");
            passed += 1;
        } else {
            println!("✗ BNE (taken): PC=${:04X}", cpu.regs.pc);
            failed += 1;
        }
    }
    
    // Test BNE (branch not taken)
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        cpu.regs.status = 0x02; // Z=1
        mem.ram[0x0000] = 0xD0; // BNE +5
        mem.ram[0x0001] = 0x05;
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.pc == 0x0002 {
            println!("✓ BNE (not taken)");
            passed += 1;
        } else {
            println!("✗ BNE (not taken): PC=${:04X}", cpu.regs.pc);
            failed += 1;
        }
    }
    
    // Test JSR/RTS
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        mem.ram[0x0000] = 0x20; // JSR $0010
        mem.ram[0x0001] = 0x10;
        mem.ram[0x0002] = 0x00;
        mem.ram[0x0010] = 0x60; // RTS
        cpu.regs.pc = 0x0000;
        cpu.regs.sp = 0xFF;
        cpu.step(&mut mem); // JSR
        if cpu.regs.pc != 0x0010 {
            println!("✗ JSR: PC=${:04X}", cpu.regs.pc);
            failed += 1;
        } else {
            cpu.step(&mut mem); // RTS
            if cpu.regs.pc == 0x0003 {
                println!("✓ JSR/RTS");
                passed += 1;
            } else {
                println!("✗ RTS: PC=${:04X}", cpu.regs.pc);
                failed += 1;
            }
        }
    }
    
    // Test PHA/PLA
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        cpu.regs.a = 0x42;
        cpu.regs.sp = 0xFF;
        mem.ram[0x0000] = 0x48; // PHA
        mem.ram[0x0001] = 0xA9; // LDA #$00
        mem.ram[0x0002] = 0x00;
        mem.ram[0x0003] = 0x68; // PLA
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem); // PHA
        cpu.step(&mut mem); // LDA #$00
        cpu.step(&mut mem); // PLA
        if cpu.regs.a == 0x42 {
            println!("✓ PHA/PLA");
            passed += 1;
        } else {
            println!("✗ PHA/PLA: A=${:02X}", cpu.regs.a);
            failed += 1;
        }
    }
    
    // Test AND
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        cpu.regs.a = 0xFF;
        mem.ram[0x0000] = 0x29; // AND #$0F
        mem.ram[0x0001] = 0x0F;
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.a == 0x0F {
            println!("✓ AND immediate");
            passed += 1;
        } else {
            println!("✗ AND immediate: A=${:02X}", cpu.regs.a);
            failed += 1;
        }
    }
    
    // Test ORA
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        cpu.regs.a = 0xF0;
        mem.ram[0x0000] = 0x09; // ORA #$0F
        mem.ram[0x0001] = 0x0F;
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.a == 0xFF {
            println!("✓ ORA immediate");
            passed += 1;
        } else {
            println!("✗ ORA immediate: A=${:02X}", cpu.regs.a);
            failed += 1;
        }
    }
    
    // Test EOR
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        cpu.regs.a = 0xFF;
        mem.ram[0x0000] = 0x49; // EOR #$0F
        mem.ram[0x0001] = 0x0F;
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.a == 0xF0 {
            println!("✓ EOR immediate");
            passed += 1;
        } else {
            println!("✗ EOR immediate: A=${:02X}", cpu.regs.a);
            failed += 1;
        }
    }
    
    // Test ASL accumulator
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        cpu.regs.a = 0x40;
        mem.ram[0x0000] = 0x0A; // ASL A
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.a == 0x80 {
            println!("✓ ASL accumulator");
            passed += 1;
        } else {
            println!("✗ ASL accumulator: A=${:02X}", cpu.regs.a);
            failed += 1;
        }
    }
    
    // Test LSR accumulator
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        cpu.regs.a = 0x80;
        mem.ram[0x0000] = 0x4A; // LSR A
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        if cpu.regs.a == 0x40 {
            println!("✓ LSR accumulator");
            passed += 1;
        } else {
            println!("✗ LSR accumulator: A=${:02X}", cpu.regs.a);
            failed += 1;
        }
    }
    
    // Test CMP (equal)
    {
        let mut cpu = Cpu::new(CpuType::Cpu6502);
        let mut mem = TestMemory::new();
        cpu.regs.a = 0x42;
        mem.ram[0x0000] = 0xC9; // CMP #$42
        mem.ram[0x0001] = 0x42;
        cpu.regs.pc = 0x0000;
        cpu.step(&mut mem);
        let z = (cpu.regs.status & 0x02) != 0;
        let c = (cpu.regs.status & 0x01) != 0;
        if z && c {
            println!("✓ CMP (equal)");
            passed += 1;
        } else {
            println!("✗ CMP (equal): Z={}, C={}", z, c);
            failed += 1;
        }
    }
    
    println!("\n========================================");
    println!("Results: {} passed, {} failed", passed, failed);
    println!("========================================");
}
