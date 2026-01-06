//! アドレッシングモードの実装
//! 
//! 6502のアドレッシングモードを定義

use super::{Cpu, MemoryBus};

/// アドレッシングモードの種類
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum AddressingMode {
    /// 即値（Immediate） - #$nn
    Immediate,
    /// ゼロページ - $nn
    ZeroPage,
    /// ゼロページ,X - $nn,X
    ZeroPageX,
    /// ゼロページ,Y - $nn,Y
    ZeroPageY,
    /// 絶対 - $nnnn
    Absolute,
    /// 絶対,X - $nnnn,X
    AbsoluteX,
    /// 絶対,Y - $nnnn,Y
    AbsoluteY,
    /// 間接 - ($nnnn)
    Indirect,
    /// 間接,X（プリインデックス） - ($nn,X)
    IndirectX,
    /// 間接,Y（ポストインデックス） - ($nn),Y
    IndirectY,
    /// 間接（ゼロページ、65C02のみ） - ($nn)
    IndirectZeroPage,
    /// 相対（ブランチ命令用） - $nn
    Relative,
    /// 暗黙的/アキュムレータ
    Implied,
}

impl Cpu {
    //--------------------------------------------------
    // アドレッシングモードのヘルパー関数
    //--------------------------------------------------

    /// 即値を取得
    pub(super) fn get_immediate<M: MemoryBus>(&mut self, memory: &mut M) -> u8 {
        let value = memory.read(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.cycles += 1;
        value
    }

    /// ゼロページアドレスを取得
    pub(super) fn get_zeropage_addr<M: MemoryBus>(&mut self, memory: &mut M) -> u16 {
        let addr = memory.read(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.cycles += 1;
        addr
    }

    /// ゼロページ,Xアドレスを取得
    pub(super) fn get_zeropage_x_addr<M: MemoryBus>(&mut self, memory: &mut M) -> u16 {
        let base = memory.read(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.cycles += 2; // +1 for ZP read, +1 for X add
        base.wrapping_add(self.regs.x) as u16
    }

    /// ゼロページ,Yアドレスを取得
    pub(super) fn get_zeropage_y_addr<M: MemoryBus>(&mut self, memory: &mut M) -> u16 {
        let base = memory.read(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.cycles += 2;
        base.wrapping_add(self.regs.y) as u16
    }

    /// 絶対アドレスを取得
    pub(super) fn get_absolute_addr<M: MemoryBus>(&mut self, memory: &mut M) -> u16 {
        let low = memory.read(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        let high = memory.read(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.cycles += 2;
        (high << 8) | low
    }

    /// 絶対,Xアドレスを取得（読み込み用、ページ境界でペナルティ）
    pub(super) fn get_absolute_x_addr<M: MemoryBus>(&mut self, memory: &mut M, write: bool) -> u16 {
        let low = memory.read(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        let high = memory.read(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        let base = (high << 8) | low;
        let addr = base.wrapping_add(self.regs.x as u16);
        self.cycles += 2;
        // ページ境界を越えた場合、追加サイクル
        if write || (base & 0xFF00) != (addr & 0xFF00) {
            self.cycles += 1;
        }
        addr
    }

    /// 絶対,Yアドレスを取得（読み込み用、ページ境界でペナルティ）
    pub(super) fn get_absolute_y_addr<M: MemoryBus>(&mut self, memory: &mut M, write: bool) -> u16 {
        let low = memory.read(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        let high = memory.read(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        let base = (high << 8) | low;
        let addr = base.wrapping_add(self.regs.y as u16);
        self.cycles += 2;
        if write || (base & 0xFF00) != (addr & 0xFF00) {
            self.cycles += 1;
        }
        addr
    }

    /// 間接,Xアドレスを取得
    pub(super) fn get_indirect_x_addr<M: MemoryBus>(&mut self, memory: &mut M) -> u16 {
        let base = memory.read(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        let ptr = base.wrapping_add(self.regs.x);
        let low = memory.read(ptr as u16) as u16;
        let high = memory.read(ptr.wrapping_add(1) as u16) as u16;
        self.cycles += 4;
        (high << 8) | low
    }

    /// 間接,Yアドレスを取得
    pub(super) fn get_indirect_y_addr<M: MemoryBus>(&mut self, memory: &mut M, write: bool) -> u16 {
        let ptr = memory.read(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        let low = memory.read(ptr as u16) as u16;
        let high = memory.read(ptr.wrapping_add(1) as u16) as u16;
        let base = (high << 8) | low;
        let addr = base.wrapping_add(self.regs.y as u16);
        self.cycles += 3;
        if write || (base & 0xFF00) != (addr & 0xFF00) {
            self.cycles += 1;
        }
        addr
    }

    /// 間接アドレス（ゼロページ、65C02用）
    pub(super) fn get_indirect_zp_addr<M: MemoryBus>(&mut self, memory: &mut M) -> u16 {
        let ptr = memory.read(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        let low = memory.read(ptr as u16) as u16;
        let high = memory.read(ptr.wrapping_add(1) as u16) as u16;
        self.cycles += 3;
        (high << 8) | low
    }

    /// 相対アドレス（ブランチ用）
    #[allow(dead_code)]
    pub(super) fn get_relative_addr<M: MemoryBus>(&mut self, memory: &mut M) -> u16 {
        let offset = memory.read(self.regs.pc) as i8;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.cycles += 1;
        self.regs.pc.wrapping_add(offset as u16)
    }

    /// ブランチを実行（共通ロジック）
    pub(super) fn branch<M: MemoryBus>(&mut self, memory: &mut M, condition: bool) {
        let offset = memory.read(self.regs.pc) as i8;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.cycles += 1;
        
        if condition {
            let old_pc = self.regs.pc;
            self.regs.pc = self.regs.pc.wrapping_add(offset as u16);
            self.cycles += 1;
            // ページ境界を越えた場合、追加サイクル
            if (old_pc & 0xFF00) != (self.regs.pc & 0xFF00) {
                self.cycles += 1;
            }
        }
    }
}
