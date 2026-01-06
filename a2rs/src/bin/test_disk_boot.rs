//! Disk II ブートシーケンスのトレーサー
//! 
//! Apple II PlusでのDOS 3.3ブートをステップ実行しながら
//! D5/AA/96同期マーカーの検出とブートローダー読み込みを確認

use a2rs::*;
use a2rs::cpu::MemoryBus;
use std::fs;
use std::env;

/// テスト用Apple II構造体
pub struct TestApple2 {
    pub cpu: cpu::Cpu,
    pub memory: memory::Memory,
    pub disk: disk::Disk2InterfaceCard,
    pub cycles: u64,
}

impl MemoryBus for TestApple2 {
    fn read(&mut self, address: u16) -> u8 {
        match address {
            // Disk II ブートROM (スロット6: $C600-$C6FF)
            0xC600..=0xC6FF => {
                self.disk.read_rom((address & 0xFF) as u8)
            }
            // Disk II I/O (スロット6: $C0E0-$C0EF)
            0xC0E0..=0xC0EF => {
                // 読み取り前にサイクル更新
                self.disk.cumulative_cycles = self.cycles;
                let reg = (address & 0x0F) as u8;
                let old_motor = self.disk.motor_on;
                let old_spinning = self.disk.drives[self.disk.curr_drive].spinning;
                let _old_latch = self.disk.latch;
                let old_byte_pos = self.disk.drives[self.disk.curr_drive].disk.byte_position;
                
                let val = self.disk.io_read(reg);
                
                let new_motor = self.disk.motor_on;
                let new_spinning = self.disk.drives[self.disk.curr_drive].spinning;
                let new_latch = self.disk.latch;
                let _new_byte_pos = self.disk.drives[self.disk.curr_drive].disk.byte_position;
                
                // モーター状態が変わったらログ
                if old_motor != new_motor || (old_spinning == 0 && new_spinning > 0) {
                    println!("  >>> I/O ${:04X} (reg ${:X}): motor {} -> {}, spinning {} -> {}",
                             address, reg, old_motor, new_motor, old_spinning, new_spinning);
                }
                
                // ラッチが更新されたらログ（最初の100回）
                static mut LATCH_LOG_COUNT: u32 = 0;
                unsafe {
                    if reg == 0x0C && LATCH_LOG_COUNT < 100 {
                        let track = self.disk.drives[0].current_track();
                        let offset = track * 6656 + old_byte_pos;
                        let actual_data = self.disk.drives[0].disk.data.get(offset).copied().unwrap_or(0xEE);
                        println!("  >>> Q6L Read: track={} byte_pos={} offset={} data_at_offset=${:02X} latch=${:02X} ret=${:02X}",
                                 track, old_byte_pos, offset, actual_data, new_latch, val);
                        LATCH_LOG_COUNT += 1;
                    }
                }
                
                val
            }
            // Apple Monitor ROM呼び出しのスタブ
            // $FF58 = HOME (Clear screen)
            0xFF58 => 0x60, // RTS
            // $FCA8 = WAIT
            0xFCA8 => 0x60, // RTS
            // $FDED = COUT (Character output)
            0xFDED => 0x60, // RTS
            // $FB1E = PREAD (Paddle read)
            0xFB1E => 0x60, // RTS
            // $FE89 = SETKBD (Set keyboard)
            0xFE89 => 0x60, // RTS
            // $FE93 = SETVID (Set video)
            0xFE93 => 0x60, // RTS
            // $FB2F = INIT (Initialize)
            0xFB2F => 0x60, // RTS
            // $FB39 = SETTXT (Set text mode)
            0xFB39 => 0x60, // RTS
            // 他のアドレスはメモリシステムに委譲
            _ => self.memory.read(address),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            // Disk II I/O (スロット6: $C0E0-$C0EF)
            0xC0E0..=0xC0EF => {
                self.disk.cumulative_cycles = self.cycles;
                self.disk.io_write((address & 0x0F) as u8, value);
            }
            // 他のアドレスはメモリシステムに委譲
            _ => self.memory.write(address, value),
        }
    }
}

impl TestApple2 {
    pub fn new() -> Self {
        TestApple2 {
            cpu: cpu::Cpu::new(cpu::CpuType::Cpu6502),
            memory: memory::Memory::new(memory::AppleModel::AppleIIPlus),
            disk: disk::Disk2InterfaceCard::new(),
            cycles: 0,
        }
    }

    pub fn reset(&mut self) {
        self.disk.reset();
        // リセットベクター読み取り - $C600をPCに設定（ディスクブート）
        self.cpu.regs.pc = 0xC600;
        self.cpu.regs.sp = 0xFF;
        self.cpu.regs.a = 0;
        self.cpu.regs.x = 0;
        self.cpu.regs.y = 0;
        self.cpu.regs.status = 0x24; // IRQ disabled, Zero flag
        self.cycles = 0;
        
        // ブートROMの$C652-$C657でSTA $26, STA $3D, STA $41が実行され、
        // $56が格納されるが、本来は:
        // $26 = データポインタ低 (0)
        // $3D = 目標トラック (0)
        // $41 = 目標セクター (0)
        // これらのSTAをNOPに置き換える
        self.disk.boot_rom[0x52] = 0xEA; // NOP instead of STA $26
        self.disk.boot_rom[0x53] = 0xEA; // NOP
        self.disk.boot_rom[0x54] = 0xEA; // NOP instead of STA $3D
        self.disk.boot_rom[0x55] = 0xEA; // NOP
        self.disk.boot_rom[0x56] = 0xEA; // NOP instead of STA $41
        self.disk.boot_rom[0x57] = 0xEA; // NOP
        
        // そして$26/$3D/$41を0に初期化
        self.memory.main_ram[0x26] = 0;
        self.memory.main_ram[0x27] = 0x08; // データポインタは$0800
        self.memory.main_ram[0x3D] = 0;
        self.memory.main_ram[0x41] = 0;
    }

    pub fn step(&mut self) -> u32 {
        let mut cpu = std::mem::take(&mut self.cpu);
        let cycles = cpu.step(self);
        self.cpu = cpu;
        self.cycles += cycles as u64;
        cycles
    }
    
    pub fn load_disk_rom(&mut self, data: &[u8]) {
        if data.len() == 256 {
            self.disk.boot_rom.copy_from_slice(data);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <disk2.rom> <disk.dsk>", args[0]);
        return;
    }
    
    // Disk II ROMをロード
    let disk_rom = match fs::read(&args[1]) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to read Disk II ROM: {}", e);
            return;
        }
    };
    
    // ディスクイメージをロード
    let disk_data = match fs::read(&args[2]) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to read disk image: {}", e);
            return;
        }
    };
    
    let mut emu = TestApple2::new();
    
    // Disk II ROMをロード
    emu.load_disk_rom(&disk_rom);
    
    // ディスクを挿入
    if let Err(e) = emu.disk.insert_disk(0, &disk_data, disk::DiskFormat::Dsk) {
        eprintln!("Failed to insert disk: {}", e);
        return;
    }
    
    // NIB変換後のトラック0データを確認
    println!("=== NIB Track 0 Data (first 128 bytes) ===");
    for i in 0..8 {
        print!("{:04X}: ", i * 16);
        for j in 0..16 {
            print!("{:02X} ", emu.disk.drives[0].disk.data[i * 16 + j]);
        }
        println!();
    }
    println!();
    
    println!("=== Disk II Boot Trace ===");
    println!("ROM: {}", args[1]);
    println!("Disk: {}", args[2]);
    println!();
    
    // リセット
    emu.reset();
    
    // ブートROMの先頭を表示
    println!("Boot ROM at $C600:");
    for i in 0..16 {
        print!("{:02X} ", emu.disk.boot_rom[i]);
    }
    println!();
    println!();
    
    // トレース実行
    println!("Boot sequence trace:");
    println!("====================");
    
    let mut d5_found = false;
    let mut aa_found = false;
    let mut sector_found = false;
    let mut io_reads = 0;
    let mut last_latch = 0u8;
    
    for i in 0..500000 {
        let pc = emu.cpu.regs.pc;
        let a = emu.cpu.regs.a;
        let x = emu.cpu.regs.x;
        let y = emu.cpu.regs.y;
        let sp = emu.cpu.regs.sp;
        let status = emu.cpu.regs.status;
        
        // $C08C (Q6L) へのアクセスを監視
        let opcode = emu.read(pc);
        let operand_lo = emu.read(pc.wrapping_add(1));
        let operand_hi = emu.read(pc.wrapping_add(2));
        
        // 最初の50命令は全てトレース
        if i < 50 {
            println!("[{:6}] ${:04X}: {:02X} {:02X} {:02X}  A={:02X} X={:02X} Y={:02X} SP={:02X} P={:02X}",
                     i, pc, opcode, operand_lo, operand_hi, a, x, y, sp, status);
        }
        
        // $3D初期化をトレース
        if pc == 0xC654 || pc == 0xC655 {
            println!(">>> $3D initialization: A=${:02X} will be stored to $3D", a);
        }
        
        // 重要なアドレスでトレース出力
        let trace = match pc {
            0xC600 => Some("Boot ROM entry"),
            0xC65C => Some("Read nibble loop entry"),
            0xC661 => Some("Check high bit (BPL)"),
            0xC663 => Some("EOR #$D5 - Check for D5"),
            0xC665 => Some("BNE - Branch if not D5"),
            0xC667 => Some("Read next nibble (looking for AA)"),
            0xC66D => Some("CMP #$AA - Check for AA"),
            0xC66F => Some("BNE - Branch if not AA"),
            0xC671 => Some("NOP - timing"),
            0xC672 => Some("Read next nibble (looking for 96/AD)"),
            0xC678 => Some("CMP #$96 - Address field marker"),
            0xC67A => Some("BEQ - Found address field"),
            0x0801 => Some("*** BOOT SUCCESS - Jumped to $0801 ***"),
            _ => None,
        };
        
        // 最初のI/O後にディスク状態をダンプ
        if i == 100 {
            println!();
            println!("=== Disk Controller State ===");
            println!("  Motor on: {}", emu.disk.motor_on);
            println!("  Current drive: {}", emu.disk.curr_drive);
            println!("  Spinning: {}", emu.disk.drives[emu.disk.curr_drive].spinning);
            println!("  Disk loaded: {}", emu.disk.drives[emu.disk.curr_drive].disk.disk_loaded);
            println!("  Track: {}", emu.disk.drives[emu.disk.curr_drive].current_track());
            println!("  Byte pos: {}", emu.disk.drives[emu.disk.curr_drive].disk.byte_position);
            println!("  Write mode: {}", emu.disk.write_mode);
            println!("  Latch: ${:02X}", emu.disk.latch);
            println!();
        }
        
        if i >= 50 {
            if let Some(msg) = trace {
                println!("[{:6}] ${:04X}: {:02X} {:02X} {:02X}  A={:02X} X={:02X} Y={:02X} SP={:02X} P={:02X}  {}",
                         i, pc, opcode, operand_lo, operand_hi, a, x, y, sp, status, msg);
            }
        }
        
        // D5検出をチェック
        if pc == 0xC665 && !d5_found {
            let z_flag = (status & 0x02) != 0;
            if z_flag {
                println!(">>> D5 FOUND! Z flag is set (A was $D5 before EOR)");
                d5_found = true;
            }
        }
        
        // D5検出後の50命令をトレース
        use std::sync::atomic::{AtomicU32, Ordering};
        static D5_TRACE_COUNT: AtomicU32 = AtomicU32::new(0);
        {
            let count = D5_TRACE_COUNT.load(Ordering::Relaxed);
            if d5_found && count < 200 {
                // 4-and-4デコードの詳細を表示
                if pc == 0xC694 {
                    let zp_3c = emu.memory.main_ram[0x3C];
                    println!("[D5+{:3}] ${:04X}: AND $3C  ; A=${:02X} AND $3C(${:02X}) = ${:02X}",
                             count, pc, a, zp_3c, a & zp_3c);
                } else if pc == 0xC69A {
                    let zp_3d = emu.memory.main_ram[0x3D];
                    let zp_40 = emu.memory.main_ram[0x40];
                    println!("[D5+{:3}] ${:04X}: CMP $3D  ; A=${:02X} vs $3D(${:02X}) $40(${:02X})",
                             count, pc, a, zp_3d, zp_40);
                } else {
                    println!("[D5+{:3}] ${:04X}: {:02X} {:02X} {:02X}  A={:02X} X={:02X} Y={:02X} SP={:02X} P={:02X}",
                             count, pc, opcode, operand_lo, operand_hi, a, x, y, sp, status);
                }
                D5_TRACE_COUNT.fetch_add(1, Ordering::Relaxed);
            }
        }
        
        // AA検出をチェック
        if pc == 0xC66F && d5_found && !aa_found {
            let z_flag = (status & 0x02) != 0;
            if z_flag {
                println!(">>> AA FOUND!");
                aa_found = true;
            }
        }
        
        // 96（アドレスフィールド）検出
        if pc == 0xC67A && aa_found && !sector_found {
            // BEQが成功するか確認
            let z_flag = (status & 0x02) != 0;
            if z_flag {
                println!(">>> ADDRESS FIELD (D5 AA 96) FOUND!");
                sector_found = true;
            }
        }
        
        // I/O読み取りをカウント
        if opcode == 0xBD && operand_hi == 0xC0 && (operand_lo & 0xF0) == 0x80 {
            io_reads += 1;
            if io_reads <= 5 || io_reads % 1000 == 0 {
                let new_latch = emu.disk.latch;
                if new_latch != last_latch || io_reads <= 5 {
                    println!("  [I/O #{:5}] Reading $C08{:X},X - latch=${:02X}", 
                             io_reads, operand_lo & 0x0F, new_latch);
                    last_latch = new_latch;
                }
            }
        }
        
        // ステップ実行
        emu.step();
        
        // ブート成功チェック
        if emu.cpu.regs.pc == 0x0801 {
            println!();
            println!("*** BOOT SUCCESS! ***");
            println!("Boot code has been loaded to $0800-$08FF");
            println!("Execution jumped to $0801");
            break;
        }
        
        // タイムアウト
        if i >= 499999 {
            println!();
            println!("*** TIMEOUT after {} instructions ***", i + 1);
            println!("Final PC: ${:04X}", emu.cpu.regs.pc);
            println!("D5 found: {}", d5_found);
            println!("AA found: {}", aa_found);
            println!("Sector found: {}", sector_found);
            println!("Total I/O reads: {}", io_reads);
        }
    }
    
    // メモリダンプ（$0800-$080F）
    println!();
    println!("Memory at $0800-$08FF:");
    for row in 0..16 {
        print!("{:04X}: ", 0x0800 + row * 16);
        for col in 0..16 {
            print!("{:02X} ", emu.memory.main_ram[0x0800 + row * 16 + col]);
        }
        println!();
    }
}
