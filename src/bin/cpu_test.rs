//! Klaus2m5 6502機能テストランナー
//! 
//! 使用方法: cargo run --bin cpu_test

use std::fs;
use std::time::Instant;

// メインクレートからCPUモジュールを使用
use a2rs::cpu::{Cpu, CpuType, MemoryBus};

/// テスト用メモリ（64KB フラットメモリ）
struct TestMemory {
    ram: Vec<u8>,
}

impl TestMemory {
    fn new() -> Self {
        TestMemory {
            ram: vec![0; 65536],
        }
    }
    
    fn load(&mut self, address: u16, data: &[u8]) {
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

fn main() {
    println!("===========================================");
    println!("  Klaus2m5 6502 Functional Test Runner");
    println!("===========================================\n");
    
    // テストバイナリのパス
    let test_paths = [
        ("6502 Functional Test", 
         "tests/6502_65C02_functional_tests-master/bin_files/6502_functional_test.bin",
         CpuType::Cpu6502,
         0x0400u16),  // 開始アドレス
        ("65C02 Extended Opcodes Test",
         "tests/6502_65C02_functional_tests-master/bin_files/65C02_extended_opcodes_test.bin",
         CpuType::Cpu65C02,
         0x0400u16),
    ];
    
    for (name, path, cpu_type, start_addr) in test_paths.iter() {
        println!("----------------------------------------");
        println!("Test: {}", name);
        println!("File: {}", path);
        println!("CPU:  {:?}", cpu_type);
        println!("----------------------------------------");
        
        match fs::read(path) {
            Ok(data) => {
                run_test(&data, *cpu_type, *start_addr);
            }
            Err(e) => {
                println!("Error loading test file: {}", e);
                println!("Skipping...\n");
            }
        }
    }
}

fn run_test(data: &[u8], cpu_type: CpuType, start_addr: u16) {
    let mut memory = TestMemory::new();
    let mut cpu = Cpu::new(cpu_type);
    
    // テストバイナリをロード（$0000から）
    memory.load(0x0000, data);
    
    // リセットベクターを設定（$FFFCにstart_addrを書き込み）
    memory.ram[0xFFFC] = (start_addr & 0xFF) as u8;
    memory.ram[0xFFFD] = (start_addr >> 8) as u8;
    
    // CPUをリセット
    cpu.reset(&mut memory);
    
    println!("Starting at ${:04X}", cpu.regs.pc);
    println!("Running...\n");
    
    let start_time = Instant::now();
    let mut cycles: u64 = 0;
    let mut same_pc_count = 0;
    let max_cycles: u64 = 100_000_000; // 1億サイクルで停止
    
    // 実行
    loop {
        let current_pc = cpu.regs.pc;
        
        // デバッグ出力（最初の数命令）
        if cycles < 20 {
            let opcode = memory.read(current_pc);
            println!("  [{:8}] PC=${:04X} A=${:02X} X=${:02X} Y=${:02X} SP=${:02X} P=${:02X} | op=${:02X}",
                     cycles, current_pc, cpu.regs.a, cpu.regs.x, cpu.regs.y, 
                     cpu.regs.sp, cpu.regs.status, opcode);
        }
        
        // 1命令実行
        let step_cycles = cpu.step(&mut memory);
        cycles += step_cycles as u64;
        
        // 同じPCに留まっているかチェック（JMP *検出）
        if cpu.regs.pc == current_pc {
            same_pc_count += 1;
            if same_pc_count >= 2 {
                // 無限ループ検出
                let elapsed = start_time.elapsed();
                let mhz = cycles as f64 / elapsed.as_secs_f64() / 1_000_000.0;
                
                println!("\n----------------------------------------");
                println!("Loop detected at ${:04X}", current_pc);
                println!("Total cycles: {}", cycles);
                println!("Elapsed: {:?}", elapsed);
                println!("Speed: {:.2} MHz", mhz);
                
                // 成功判定
                // Klaus2m5のテストでは成功時に特定のアドレスで停止
                // 6502_functional_testの成功アドレスは$3469
                // 65C02_extended_opcodes_testの成功アドレスは$24F1
                if current_pc == 0x3469 || current_pc == 0x24F1 {
                    println!("\n*** TEST PASSED! ***");
                    println!("CPU emulation is working correctly.\n");
                } else {
                    println!("\n*** TEST FAILED ***");
                    println!("Trap at ${:04X}", current_pc);
                    println!("Check the listing file to identify the failed test.\n");
                    
                    // 周辺メモリをダンプ
                    dump_memory(&memory, current_pc);
                }
                
                return;
            }
        } else {
            same_pc_count = 0;
        }
        
        // サイクル上限チェック
        if cycles >= max_cycles {
            let elapsed = start_time.elapsed();
            println!("\n----------------------------------------");
            println!("Timeout after {} cycles", cycles);
            println!("Elapsed: {:?}", elapsed);
            println!("Last PC: ${:04X}", cpu.regs.pc);
            println!("\n*** TEST INCOMPLETE ***\n");
            return;
        }
        
        // 進捗表示（1000万サイクルごと）
        if cycles % 10_000_000 == 0 {
            println!("  Progress: {} million cycles, PC=${:04X}", 
                     cycles / 1_000_000, cpu.regs.pc);
        }
    }
}

fn dump_memory(memory: &TestMemory, addr: u16) {
    println!("\nMemory dump around ${:04X}:", addr);
    let start = (addr as usize).saturating_sub(16) & 0xFFF0;
    for row in 0..4 {
        let row_addr = start + row * 16;
        print!("  ${:04X}: ", row_addr);
        for col in 0..16 {
            let a = (row_addr + col) & 0xFFFF;
            print!("{:02X} ", memory.ram[a]);
        }
        println!();
    }
}
