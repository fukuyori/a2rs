//! MOS 6502/65C02 CPU Emulator
//! 
//! Apple IIで使用される6502プロセッサのエミュレーション実装
//! Based on 6502 technical specifications and datasheet

mod opcodes;
mod opcodes2;
pub mod addressing;

/// CPUのステータスレジスタのフラグビット
pub mod flags {
    pub const CARRY: u8 = 0b0000_0001;      // C: キャリーフラグ
    pub const ZERO: u8 = 0b0000_0010;       // Z: ゼロフラグ
    pub const IRQ_DISABLE: u8 = 0b0000_0100; // I: 割り込み禁止フラグ
    pub const DECIMAL: u8 = 0b0000_1000;    // D: BCDモードフラグ
    pub const BREAK: u8 = 0b0001_0000;      // B: ブレークフラグ
    pub const UNUSED: u8 = 0b0010_0000;     // 未使用（常に1）
    pub const OVERFLOW: u8 = 0b0100_0000;   // V: オーバーフローフラグ
    pub const NEGATIVE: u8 = 0b1000_0000;   // N: 負数フラグ
}

/// CPUの種類
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CpuType {
    /// オリジナルのNMOS 6502 (Apple II, II+)
    Cpu6502,
    /// CMOS 65C02 (Apple IIe Enhanced, IIc)
    Cpu65C02,
}

/// CPUレジスタの状態
#[derive(Debug, Clone)]
pub struct Registers {
    /// アキュムレータ（A）
    pub a: u8,
    /// Xインデックスレジスタ
    pub x: u8,
    /// Yインデックスレジスタ  
    pub y: u8,
    /// スタックポインタ
    pub sp: u8,
    /// プログラムカウンタ
    pub pc: u16,
    /// ステータスレジスタ（プロセッサフラグ）
    pub status: u8,
}

impl Default for Registers {
    fn default() -> Self {
        Registers {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xFD,  // スタックは$01FDから開始
            pc: 0,
            status: flags::UNUSED | flags::IRQ_DISABLE,
        }
    }
}

impl Registers {
    /// フラグをセット
    pub fn set_flag(&mut self, flag: u8, value: bool) {
        if value {
            self.status |= flag;
        } else {
            self.status &= !flag;
        }
    }

    /// フラグを取得
    pub fn get_flag(&self, flag: u8) -> bool {
        (self.status & flag) != 0
    }

    /// ゼロフラグと負数フラグを値に基づいて更新
    pub fn update_zero_negative_flags(&mut self, value: u8) {
        self.set_flag(flags::ZERO, value == 0);
        self.set_flag(flags::NEGATIVE, (value & 0x80) != 0);
    }
}

/// メモリバスインターフェース
/// CPUがメモリにアクセスするために必要なトレイト
pub trait MemoryBus {
    /// メモリから1バイト読み取り
    fn read(&mut self, address: u16) -> u8;
    /// メモリに1バイト書き込み
    fn write(&mut self, address: u16, value: u8);
}

/// 6502 CPUエミュレータ
#[derive(Debug, Clone)]
pub struct Cpu {
    /// CPUレジスタ
    pub regs: Registers,
    /// CPUの種類（6502 or 65C02）
    pub cpu_type: CpuType,
    /// 累積サイクル数
    pub total_cycles: u64,
    /// 現在の命令で消費したサイクル
    pub cycles: u32,
    /// IRQ（割り込み要求）ライン
    pub irq_pending: bool,
    /// NMI（ノンマスカブル割り込み）ライン
    pub nmi_pending: bool,
    /// NMI検出のためのエッジ検出
    nmi_edge_detected: bool,
    /// 前回のNMIライン状態
    prev_nmi: bool,
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new(CpuType::Cpu6502)
    }
}

impl Cpu {
    /// 新しいCPUインスタンスを作成
    pub fn new(cpu_type: CpuType) -> Self {
        Cpu {
            regs: Registers::default(),
            cpu_type,
            total_cycles: 0,
            cycles: 0,
            irq_pending: false,
            nmi_pending: false,
            nmi_edge_detected: false,
            prev_nmi: false,
        }
    }

    /// CPUをリセット
    pub fn reset<M: MemoryBus>(&mut self, memory: &mut M) {
        self.regs = Registers::default();
        // リセットベクター（$FFFC-$FFFD）からPCを読み込み
        let low = memory.read(0xFFFC) as u16;
        let high = memory.read(0xFFFD) as u16;
        self.regs.pc = (high << 8) | low;
        self.cycles = 7; // リセットには7サイクル必要
        self.total_cycles += 7;
    }

    /// NMI（ノンマスカブル割り込み）を処理
    fn handle_nmi<M: MemoryBus>(&mut self, memory: &mut M) {
        // PCをスタックにプッシュ（上位バイト先）
        self.push_word(memory, self.regs.pc);
        // ステータスレジスタをプッシュ（Bフラグはクリア）
        let status = (self.regs.status | flags::UNUSED) & !flags::BREAK;
        self.push_byte(memory, status);
        // 割り込み禁止フラグをセット
        self.regs.set_flag(flags::IRQ_DISABLE, true);
        // NMIベクター（$FFFA-$FFFB）からPCを読み込み
        let low = memory.read(0xFFFA) as u16;
        let high = memory.read(0xFFFB) as u16;
        self.regs.pc = (high << 8) | low;
        self.cycles += 7;
        self.nmi_edge_detected = false;
    }

    /// IRQ（割り込み要求）を処理
    fn handle_irq<M: MemoryBus>(&mut self, memory: &mut M) {
        if self.regs.get_flag(flags::IRQ_DISABLE) {
            return;
        }
        // PCをスタックにプッシュ
        self.push_word(memory, self.regs.pc);
        // ステータスレジスタをプッシュ（Bフラグはクリア）
        let status = (self.regs.status | flags::UNUSED) & !flags::BREAK;
        self.push_byte(memory, status);
        // 割り込み禁止フラグをセット
        self.regs.set_flag(flags::IRQ_DISABLE, true);
        // IRQベクター（$FFFE-$FFFF）からPCを読み込み
        let low = memory.read(0xFFFE) as u16;
        let high = memory.read(0xFFFF) as u16;
        self.regs.pc = (high << 8) | low;
        self.cycles += 7;
    }

    /// 1命令を実行し、消費したサイクル数を返す
    pub fn step<M: MemoryBus>(&mut self, memory: &mut M) -> u32 {
        self.cycles = 0;

        // NMIのエッジ検出（立ち下がりで発生）
        if self.nmi_pending && !self.prev_nmi {
            self.nmi_edge_detected = true;
        }
        self.prev_nmi = self.nmi_pending;

        // NMI処理（最優先）
        if self.nmi_edge_detected {
            self.handle_nmi(memory);
            self.total_cycles += self.cycles as u64;
            return self.cycles;
        }

        // IRQ処理
        if self.irq_pending && !self.regs.get_flag(flags::IRQ_DISABLE) {
            self.handle_irq(memory);
            self.total_cycles += self.cycles as u64;
            return self.cycles;
        }

        // 命令をフェッチ
        let opcode = self.fetch_byte(memory);
        
        // 命令を実行
        self.execute_opcode(memory, opcode);

        self.total_cycles += self.cycles as u64;
        self.cycles
    }

    /// PCから1バイトフェッチしてPCをインクリメント
    fn fetch_byte<M: MemoryBus>(&mut self, memory: &mut M) -> u8 {
        let value = memory.read(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.cycles += 1;
        value
    }

    /// PCから2バイト（ワード）をフェッチ
    #[allow(dead_code)]
    fn fetch_word<M: MemoryBus>(&mut self, memory: &mut M) -> u16 {
        let low = self.fetch_byte(memory) as u16;
        let high = self.fetch_byte(memory) as u16;
        (high << 8) | low
    }

    /// スタックに1バイトプッシュ
    fn push_byte<M: MemoryBus>(&mut self, memory: &mut M, value: u8) {
        memory.write(0x0100 | self.regs.sp as u16, value);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
    }

    /// スタックから1バイトポップ
    fn pop_byte<M: MemoryBus>(&mut self, memory: &mut M) -> u8 {
        self.regs.sp = self.regs.sp.wrapping_add(1);
        memory.read(0x0100 | self.regs.sp as u16)
    }

    /// スタックに2バイトプッシュ（上位バイト先）
    fn push_word<M: MemoryBus>(&mut self, memory: &mut M, value: u16) {
        self.push_byte(memory, (value >> 8) as u8);
        self.push_byte(memory, value as u8);
    }

    /// スタックから2バイトポップ
    fn pop_word<M: MemoryBus>(&mut self, memory: &mut M) -> u16 {
        let low = self.pop_byte(memory) as u16;
        let high = self.pop_byte(memory) as u16;
        (high << 8) | low
    }

    /// オペコードを実行
    fn execute_opcode<M: MemoryBus>(&mut self, memory: &mut M, opcode: u8) {
        match opcode {
            // LDA - Load Accumulator
            0xA9 => self.lda_immediate(memory),
            0xA5 => self.lda_zeropage(memory),
            0xB5 => self.lda_zeropage_x(memory),
            0xAD => self.lda_absolute(memory),
            0xBD => self.lda_absolute_x(memory),
            0xB9 => self.lda_absolute_y(memory),
            0xA1 => self.lda_indirect_x(memory),
            0xB1 => self.lda_indirect_y(memory),

            // LDX - Load X Register
            0xA2 => self.ldx_immediate(memory),
            0xA6 => self.ldx_zeropage(memory),
            0xB6 => self.ldx_zeropage_y(memory),
            0xAE => self.ldx_absolute(memory),
            0xBE => self.ldx_absolute_y(memory),

            // LDY - Load Y Register
            0xA0 => self.ldy_immediate(memory),
            0xA4 => self.ldy_zeropage(memory),
            0xB4 => self.ldy_zeropage_x(memory),
            0xAC => self.ldy_absolute(memory),
            0xBC => self.ldy_absolute_x(memory),

            // STA - Store Accumulator
            0x85 => self.sta_zeropage(memory),
            0x95 => self.sta_zeropage_x(memory),
            0x8D => self.sta_absolute(memory),
            0x9D => self.sta_absolute_x(memory),
            0x99 => self.sta_absolute_y(memory),
            0x81 => self.sta_indirect_x(memory),
            0x91 => self.sta_indirect_y(memory),

            // STX - Store X Register
            0x86 => self.stx_zeropage(memory),
            0x96 => self.stx_zeropage_y(memory),
            0x8E => self.stx_absolute(memory),

            // STY - Store Y Register
            0x84 => self.sty_zeropage(memory),
            0x94 => self.sty_zeropage_x(memory),
            0x8C => self.sty_absolute(memory),

            // Transfer Instructions
            0xAA => self.tax(),    // TAX
            0x8A => self.txa(),    // TXA
            0xA8 => self.tay(),    // TAY
            0x98 => self.tya(),    // TYA
            0xBA => self.tsx(),    // TSX
            0x9A => self.txs(),    // TXS

            // Stack Instructions
            0x48 => self.pha(memory),  // PHA
            0x68 => self.pla(memory),  // PLA
            0x08 => self.php(memory),  // PHP
            0x28 => self.plp(memory),  // PLP

            // Arithmetic - ADC
            0x69 => self.adc_immediate(memory),
            0x65 => self.adc_zeropage(memory),
            0x75 => self.adc_zeropage_x(memory),
            0x6D => self.adc_absolute(memory),
            0x7D => self.adc_absolute_x(memory),
            0x79 => self.adc_absolute_y(memory),
            0x61 => self.adc_indirect_x(memory),
            0x71 => self.adc_indirect_y(memory),

            // Arithmetic - SBC
            0xE9 => self.sbc_immediate(memory),
            0xE5 => self.sbc_zeropage(memory),
            0xF5 => self.sbc_zeropage_x(memory),
            0xED => self.sbc_absolute(memory),
            0xFD => self.sbc_absolute_x(memory),
            0xF9 => self.sbc_absolute_y(memory),
            0xE1 => self.sbc_indirect_x(memory),
            0xF1 => self.sbc_indirect_y(memory),

            // Compare
            0xC9 => self.cmp_immediate(memory),
            0xC5 => self.cmp_zeropage(memory),
            0xD5 => self.cmp_zeropage_x(memory),
            0xCD => self.cmp_absolute(memory),
            0xDD => self.cmp_absolute_x(memory),
            0xD9 => self.cmp_absolute_y(memory),
            0xC1 => self.cmp_indirect_x(memory),
            0xD1 => self.cmp_indirect_y(memory),

            0xE0 => self.cpx_immediate(memory),
            0xE4 => self.cpx_zeropage(memory),
            0xEC => self.cpx_absolute(memory),

            0xC0 => self.cpy_immediate(memory),
            0xC4 => self.cpy_zeropage(memory),
            0xCC => self.cpy_absolute(memory),

            // Increment/Decrement
            0xE6 => self.inc_zeropage(memory),
            0xF6 => self.inc_zeropage_x(memory),
            0xEE => self.inc_absolute(memory),
            0xFE => self.inc_absolute_x(memory),

            0xC6 => self.dec_zeropage(memory),
            0xD6 => self.dec_zeropage_x(memory),
            0xCE => self.dec_absolute(memory),
            0xDE => self.dec_absolute_x(memory),

            0xE8 => self.inx(),
            0xC8 => self.iny(),
            0xCA => self.dex(),
            0x88 => self.dey(),

            // Logical - AND
            0x29 => self.and_immediate(memory),
            0x25 => self.and_zeropage(memory),
            0x35 => self.and_zeropage_x(memory),
            0x2D => self.and_absolute(memory),
            0x3D => self.and_absolute_x(memory),
            0x39 => self.and_absolute_y(memory),
            0x21 => self.and_indirect_x(memory),
            0x31 => self.and_indirect_y(memory),

            // Logical - ORA
            0x09 => self.ora_immediate(memory),
            0x05 => self.ora_zeropage(memory),
            0x15 => self.ora_zeropage_x(memory),
            0x0D => self.ora_absolute(memory),
            0x1D => self.ora_absolute_x(memory),
            0x19 => self.ora_absolute_y(memory),
            0x01 => self.ora_indirect_x(memory),
            0x11 => self.ora_indirect_y(memory),

            // Logical - EOR
            0x49 => self.eor_immediate(memory),
            0x45 => self.eor_zeropage(memory),
            0x55 => self.eor_zeropage_x(memory),
            0x4D => self.eor_absolute(memory),
            0x5D => self.eor_absolute_x(memory),
            0x59 => self.eor_absolute_y(memory),
            0x41 => self.eor_indirect_x(memory),
            0x51 => self.eor_indirect_y(memory),

            // Shifts
            0x0A => self.asl_accumulator(),
            0x06 => self.asl_zeropage(memory),
            0x16 => self.asl_zeropage_x(memory),
            0x0E => self.asl_absolute(memory),
            0x1E => self.asl_absolute_x(memory),

            0x4A => self.lsr_accumulator(),
            0x46 => self.lsr_zeropage(memory),
            0x56 => self.lsr_zeropage_x(memory),
            0x4E => self.lsr_absolute(memory),
            0x5E => self.lsr_absolute_x(memory),

            0x2A => self.rol_accumulator(),
            0x26 => self.rol_zeropage(memory),
            0x36 => self.rol_zeropage_x(memory),
            0x2E => self.rol_absolute(memory),
            0x3E => self.rol_absolute_x(memory),

            0x6A => self.ror_accumulator(),
            0x66 => self.ror_zeropage(memory),
            0x76 => self.ror_zeropage_x(memory),
            0x6E => self.ror_absolute(memory),
            0x7E => self.ror_absolute_x(memory),

            // BIT test
            0x24 => self.bit_zeropage(memory),
            0x2C => self.bit_absolute(memory),

            // Branch Instructions
            0x10 => self.bpl(memory),
            0x30 => self.bmi(memory),
            0x50 => self.bvc(memory),
            0x70 => self.bvs(memory),
            0x90 => self.bcc(memory),
            0xB0 => self.bcs(memory),
            0xD0 => self.bne(memory),
            0xF0 => self.beq(memory),

            // Jump/Call
            0x4C => self.jmp_absolute(memory),
            0x6C => self.jmp_indirect(memory),
            0x20 => self.jsr(memory),
            0x60 => self.rts(memory),

            // Interrupts
            0x00 => self.brk(memory),
            0x40 => self.rti(memory),

            // Flag Instructions
            0x18 => self.clc(),
            0x38 => self.sec(),
            0x58 => self.cli(),
            0x78 => self.sei(),
            0xB8 => self.clv(),
            0xD8 => self.cld(),
            0xF8 => self.sed(),

            // NOP
            0xEA => self.nop(),

            // 65C02 Extensions
            0x1A if self.cpu_type == CpuType::Cpu65C02 => self.ina(), // INC A
            0x3A if self.cpu_type == CpuType::Cpu65C02 => self.dea(), // DEC A
            0x80 if self.cpu_type == CpuType::Cpu65C02 => self.bra(memory), // BRA
            0x64 if self.cpu_type == CpuType::Cpu65C02 => self.stz_zeropage(memory),
            0x74 if self.cpu_type == CpuType::Cpu65C02 => self.stz_zeropage_x(memory),
            0x9C if self.cpu_type == CpuType::Cpu65C02 => self.stz_absolute(memory),
            0x9E if self.cpu_type == CpuType::Cpu65C02 => self.stz_absolute_x(memory),
            0x7C if self.cpu_type == CpuType::Cpu65C02 => self.jmp_absolute_x(memory),
            0x12 if self.cpu_type == CpuType::Cpu65C02 => self.ora_indirect(memory),
            0x32 if self.cpu_type == CpuType::Cpu65C02 => self.and_indirect(memory),
            0x52 if self.cpu_type == CpuType::Cpu65C02 => self.eor_indirect(memory),
            0x72 if self.cpu_type == CpuType::Cpu65C02 => self.adc_indirect(memory),
            0x92 if self.cpu_type == CpuType::Cpu65C02 => self.sta_indirect(memory),
            0xB2 if self.cpu_type == CpuType::Cpu65C02 => self.lda_indirect(memory),
            0xD2 if self.cpu_type == CpuType::Cpu65C02 => self.cmp_indirect(memory),
            0xF2 if self.cpu_type == CpuType::Cpu65C02 => self.sbc_indirect(memory),
            0xDA if self.cpu_type == CpuType::Cpu65C02 => self.phx(memory),
            0xFA if self.cpu_type == CpuType::Cpu65C02 => self.plx(memory),
            0x5A if self.cpu_type == CpuType::Cpu65C02 => self.phy(memory),
            0x7A if self.cpu_type == CpuType::Cpu65C02 => self.ply(memory),
            0x89 if self.cpu_type == CpuType::Cpu65C02 => self.bit_immediate(memory),
            0x34 if self.cpu_type == CpuType::Cpu65C02 => self.bit_zeropage_x(memory),
            0x3C if self.cpu_type == CpuType::Cpu65C02 => self.bit_absolute_x(memory),
            0x14 if self.cpu_type == CpuType::Cpu65C02 => self.trb_zeropage(memory),
            0x1C if self.cpu_type == CpuType::Cpu65C02 => self.trb_absolute(memory),
            0x04 if self.cpu_type == CpuType::Cpu65C02 => self.tsb_zeropage(memory),
            0x0C if self.cpu_type == CpuType::Cpu65C02 => self.tsb_absolute(memory),

            // 65C02 RMB (Reset Memory Bit)
            0x07 if self.cpu_type == CpuType::Cpu65C02 => self.rmb(memory, 0),
            0x17 if self.cpu_type == CpuType::Cpu65C02 => self.rmb(memory, 1),
            0x27 if self.cpu_type == CpuType::Cpu65C02 => self.rmb(memory, 2),
            0x37 if self.cpu_type == CpuType::Cpu65C02 => self.rmb(memory, 3),
            0x47 if self.cpu_type == CpuType::Cpu65C02 => self.rmb(memory, 4),
            0x57 if self.cpu_type == CpuType::Cpu65C02 => self.rmb(memory, 5),
            0x67 if self.cpu_type == CpuType::Cpu65C02 => self.rmb(memory, 6),
            0x77 if self.cpu_type == CpuType::Cpu65C02 => self.rmb(memory, 7),

            // 65C02 SMB (Set Memory Bit)
            0x87 if self.cpu_type == CpuType::Cpu65C02 => self.smb(memory, 0),
            0x97 if self.cpu_type == CpuType::Cpu65C02 => self.smb(memory, 1),
            0xA7 if self.cpu_type == CpuType::Cpu65C02 => self.smb(memory, 2),
            0xB7 if self.cpu_type == CpuType::Cpu65C02 => self.smb(memory, 3),
            0xC7 if self.cpu_type == CpuType::Cpu65C02 => self.smb(memory, 4),
            0xD7 if self.cpu_type == CpuType::Cpu65C02 => self.smb(memory, 5),
            0xE7 if self.cpu_type == CpuType::Cpu65C02 => self.smb(memory, 6),
            0xF7 if self.cpu_type == CpuType::Cpu65C02 => self.smb(memory, 7),

            // 65C02 BBR (Branch on Bit Reset)
            0x0F if self.cpu_type == CpuType::Cpu65C02 => self.bbr(memory, 0),
            0x1F if self.cpu_type == CpuType::Cpu65C02 => self.bbr(memory, 1),
            0x2F if self.cpu_type == CpuType::Cpu65C02 => self.bbr(memory, 2),
            0x3F if self.cpu_type == CpuType::Cpu65C02 => self.bbr(memory, 3),
            0x4F if self.cpu_type == CpuType::Cpu65C02 => self.bbr(memory, 4),
            0x5F if self.cpu_type == CpuType::Cpu65C02 => self.bbr(memory, 5),
            0x6F if self.cpu_type == CpuType::Cpu65C02 => self.bbr(memory, 6),
            0x7F if self.cpu_type == CpuType::Cpu65C02 => self.bbr(memory, 7),

            // 65C02 BBS (Branch on Bit Set)
            0x8F if self.cpu_type == CpuType::Cpu65C02 => self.bbs(memory, 0),
            0x9F if self.cpu_type == CpuType::Cpu65C02 => self.bbs(memory, 1),
            0xAF if self.cpu_type == CpuType::Cpu65C02 => self.bbs(memory, 2),
            0xBF if self.cpu_type == CpuType::Cpu65C02 => self.bbs(memory, 3),
            0xCF if self.cpu_type == CpuType::Cpu65C02 => self.bbs(memory, 4),
            0xDF if self.cpu_type == CpuType::Cpu65C02 => self.bbs(memory, 5),
            0xEF if self.cpu_type == CpuType::Cpu65C02 => self.bbs(memory, 6),
            0xFF if self.cpu_type == CpuType::Cpu65C02 => self.bbs(memory, 7),

            // 65C02 Multi-byte NOPs (2-byte: skip 1 operand)
            0x02 | 0x22 | 0x42 | 0x62 | 0x82 | 0xC2 | 0xE2 
                if self.cpu_type == CpuType::Cpu65C02 => {
                let _ = self.fetch_byte(memory); // 1バイトオペランドを読み飛ばす
                self.cycles += 1;
            }
            
            // 65C02 Multi-byte NOPs (2-byte: zero page style)
            0x44 | 0x54 | 0xD4 | 0xF4
                if self.cpu_type == CpuType::Cpu65C02 => {
                let _ = self.fetch_byte(memory); // ゼロページアドレスを読み飛ばす
                self.cycles += 2;
            }
            
            // 65C02 Multi-byte NOPs (3-byte: absolute style)
            0x5C | 0xDC | 0xFC
                if self.cpu_type == CpuType::Cpu65C02 => {
                let _ = self.fetch_byte(memory); // 絶対アドレス（2バイト）を読み飛ばす
                let _ = self.fetch_byte(memory);
                self.cycles += 4;
            }

            // 不明なオペコード（NOPとして扱うか、未定義動作）
            _ => {
                if self.cpu_type == CpuType::Cpu65C02 {
                    // 65C02では未定義オペコードはNOPになる
                    self.cycles += 1;
                } else {
                    // 6502では未定義オペコードは実行（ここでは簡略化してNOP扱い）
                    self.cycles += 1;
                }
            }
        }
    }
}
