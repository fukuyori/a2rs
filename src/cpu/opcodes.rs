//! オペコードの実装
//! 
//! 6502/65C02の全オペコードを実装

use super::{Cpu, MemoryBus, flags, CpuType};

impl Cpu {
    //--------------------------------------------------
    // LDA - Load Accumulator
    //--------------------------------------------------
    pub(super) fn lda_immediate<M: MemoryBus>(&mut self, memory: &mut M) {
        self.regs.a = self.get_immediate(memory);
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn lda_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        self.regs.a = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn lda_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        self.regs.a = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn lda_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        self.regs.a = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn lda_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, false);
        self.regs.a = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn lda_absolute_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_y_addr(memory, false);
        self.regs.a = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn lda_indirect_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_x_addr(memory);
        self.regs.a = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn lda_indirect_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_y_addr(memory, false);
        self.regs.a = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn lda_indirect<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_zp_addr(memory);
        self.regs.a = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    //--------------------------------------------------
    // LDX - Load X Register
    //--------------------------------------------------
    pub(super) fn ldx_immediate<M: MemoryBus>(&mut self, memory: &mut M) {
        self.regs.x = self.get_immediate(memory);
        self.regs.update_zero_negative_flags(self.regs.x);
    }

    pub(super) fn ldx_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        self.regs.x = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.x);
    }

    pub(super) fn ldx_zeropage_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_y_addr(memory);
        self.regs.x = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.x);
    }

    pub(super) fn ldx_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        self.regs.x = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.x);
    }

    pub(super) fn ldx_absolute_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_y_addr(memory, false);
        self.regs.x = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.x);
    }

    //--------------------------------------------------
    // LDY - Load Y Register
    //--------------------------------------------------
    pub(super) fn ldy_immediate<M: MemoryBus>(&mut self, memory: &mut M) {
        self.regs.y = self.get_immediate(memory);
        self.regs.update_zero_negative_flags(self.regs.y);
    }

    pub(super) fn ldy_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        self.regs.y = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.y);
    }

    pub(super) fn ldy_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        self.regs.y = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.y);
    }

    pub(super) fn ldy_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        self.regs.y = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.y);
    }

    pub(super) fn ldy_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, false);
        self.regs.y = memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.y);
    }

    //--------------------------------------------------
    // STA - Store Accumulator
    //--------------------------------------------------
    pub(super) fn sta_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        memory.write(addr, self.regs.a);
        self.cycles += 1;
    }

    pub(super) fn sta_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        memory.write(addr, self.regs.a);
        self.cycles += 1;
    }

    pub(super) fn sta_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        memory.write(addr, self.regs.a);
        self.cycles += 1;
    }

    pub(super) fn sta_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, true);
        memory.write(addr, self.regs.a);
        self.cycles += 1;
    }

    pub(super) fn sta_absolute_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_y_addr(memory, true);
        memory.write(addr, self.regs.a);
        self.cycles += 1;
    }

    pub(super) fn sta_indirect_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_x_addr(memory);
        memory.write(addr, self.regs.a);
        self.cycles += 1;
    }

    pub(super) fn sta_indirect_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_y_addr(memory, true);
        memory.write(addr, self.regs.a);
        self.cycles += 1;
    }

    pub(super) fn sta_indirect<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_zp_addr(memory);
        memory.write(addr, self.regs.a);
        self.cycles += 1;
    }

    //--------------------------------------------------
    // STX - Store X Register
    //--------------------------------------------------
    pub(super) fn stx_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        memory.write(addr, self.regs.x);
        self.cycles += 1;
    }

    pub(super) fn stx_zeropage_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_y_addr(memory);
        memory.write(addr, self.regs.x);
        self.cycles += 1;
    }

    pub(super) fn stx_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        memory.write(addr, self.regs.x);
        self.cycles += 1;
    }

    //--------------------------------------------------
    // STY - Store Y Register
    //--------------------------------------------------
    pub(super) fn sty_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        memory.write(addr, self.regs.y);
        self.cycles += 1;
    }

    pub(super) fn sty_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        memory.write(addr, self.regs.y);
        self.cycles += 1;
    }

    pub(super) fn sty_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        memory.write(addr, self.regs.y);
        self.cycles += 1;
    }

    //--------------------------------------------------
    // STZ - Store Zero (65C02)
    //--------------------------------------------------
    pub(super) fn stz_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        memory.write(addr, 0);
        self.cycles += 1;
    }

    pub(super) fn stz_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        memory.write(addr, 0);
        self.cycles += 1;
    }

    pub(super) fn stz_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        memory.write(addr, 0);
        self.cycles += 1;
    }

    pub(super) fn stz_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, true);
        memory.write(addr, 0);
        self.cycles += 1;
    }

    //--------------------------------------------------
    // Transfer Instructions
    //--------------------------------------------------
    pub(super) fn tax(&mut self) {
        self.regs.x = self.regs.a;
        self.regs.update_zero_negative_flags(self.regs.x);
        self.cycles += 1;
    }

    pub(super) fn txa(&mut self) {
        self.regs.a = self.regs.x;
        self.regs.update_zero_negative_flags(self.regs.a);
        self.cycles += 1;
    }

    pub(super) fn tay(&mut self) {
        self.regs.y = self.regs.a;
        self.regs.update_zero_negative_flags(self.regs.y);
        self.cycles += 1;
    }

    pub(super) fn tya(&mut self) {
        self.regs.a = self.regs.y;
        self.regs.update_zero_negative_flags(self.regs.a);
        self.cycles += 1;
    }

    pub(super) fn tsx(&mut self) {
        self.regs.x = self.regs.sp;
        self.regs.update_zero_negative_flags(self.regs.x);
        self.cycles += 1;
    }

    pub(super) fn txs(&mut self) {
        self.regs.sp = self.regs.x;
        self.cycles += 1;
    }

    //--------------------------------------------------
    // Stack Instructions
    //--------------------------------------------------
    pub(super) fn pha<M: MemoryBus>(&mut self, memory: &mut M) {
        self.push_byte(memory, self.regs.a);
        self.cycles += 2;
    }

    pub(super) fn pla<M: MemoryBus>(&mut self, memory: &mut M) {
        self.regs.a = self.pop_byte(memory);
        self.regs.update_zero_negative_flags(self.regs.a);
        self.cycles += 3;
    }

    pub(super) fn php<M: MemoryBus>(&mut self, memory: &mut M) {
        let status = self.regs.status | flags::BREAK | flags::UNUSED;
        self.push_byte(memory, status);
        self.cycles += 2;
    }

    pub(super) fn plp<M: MemoryBus>(&mut self, memory: &mut M) {
        self.regs.status = self.pop_byte(memory);
        self.regs.set_flag(flags::UNUSED, true);
        self.regs.set_flag(flags::BREAK, false);
        self.cycles += 3;
    }

    pub(super) fn phx<M: MemoryBus>(&mut self, memory: &mut M) {
        self.push_byte(memory, self.regs.x);
        self.cycles += 2;
    }

    pub(super) fn plx<M: MemoryBus>(&mut self, memory: &mut M) {
        self.regs.x = self.pop_byte(memory);
        self.regs.update_zero_negative_flags(self.regs.x);
        self.cycles += 3;
    }

    pub(super) fn phy<M: MemoryBus>(&mut self, memory: &mut M) {
        self.push_byte(memory, self.regs.y);
        self.cycles += 2;
    }

    pub(super) fn ply<M: MemoryBus>(&mut self, memory: &mut M) {
        self.regs.y = self.pop_byte(memory);
        self.regs.update_zero_negative_flags(self.regs.y);
        self.cycles += 3;
    }

    //--------------------------------------------------
    // ADC - Add with Carry
    //--------------------------------------------------
    fn do_adc(&mut self, value: u8) {
        let carry = if self.regs.get_flag(flags::CARRY) { 1u16 } else { 0u16 };
        
        if self.regs.get_flag(flags::DECIMAL) {
            // BCDモード
            let mut low = (self.regs.a & 0x0F) as u16 + (value & 0x0F) as u16 + carry;
            let mut high = (self.regs.a >> 4) as u16 + (value >> 4) as u16;
            
            if low > 9 {
                low -= 10;
                high += 1;
            }
            
            let result = if high > 9 {
                self.regs.set_flag(flags::CARRY, true);
                (((high - 10) << 4) | (low & 0x0F)) as u8
            } else {
                self.regs.set_flag(flags::CARRY, false);
                ((high << 4) | (low & 0x0F)) as u8
            };
            
            if self.cpu_type == CpuType::Cpu65C02 {
                self.regs.update_zero_negative_flags(result);
            }
            self.regs.a = result;
        } else {
            let result = self.regs.a as u16 + value as u16 + carry;
            let result8 = result as u8;
            
            self.regs.set_flag(flags::CARRY, result > 0xFF);
            self.regs.set_flag(
                flags::OVERFLOW,
                ((self.regs.a ^ result8) & (value ^ result8) & 0x80) != 0
            );
            self.regs.update_zero_negative_flags(result8);
            self.regs.a = result8;
        }
    }

    pub(super) fn adc_immediate<M: MemoryBus>(&mut self, memory: &mut M) {
        let value = self.get_immediate(memory);
        self.do_adc(value);
    }

    pub(super) fn adc_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_adc(value);
    }

    pub(super) fn adc_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_adc(value);
    }

    pub(super) fn adc_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_adc(value);
    }

    pub(super) fn adc_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, false);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_adc(value);
    }

    pub(super) fn adc_absolute_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_y_addr(memory, false);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_adc(value);
    }

    pub(super) fn adc_indirect_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_x_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_adc(value);
    }

    pub(super) fn adc_indirect_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_y_addr(memory, false);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_adc(value);
    }

    pub(super) fn adc_indirect<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_zp_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_adc(value);
    }

    //--------------------------------------------------
    // SBC - Subtract with Carry
    //--------------------------------------------------
    fn do_sbc(&mut self, value: u8) {
        // SBCはADCの補数として実装
        if self.regs.get_flag(flags::DECIMAL) {
            let carry = if self.regs.get_flag(flags::CARRY) { 0i16 } else { 1i16 };
            let mut low = (self.regs.a & 0x0F) as i16 - (value & 0x0F) as i16 - carry;
            let mut high = (self.regs.a >> 4) as i16 - (value >> 4) as i16;
            
            if low < 0 {
                low += 10;
                high -= 1;
            }
            
            let result = if high < 0 {
                self.regs.set_flag(flags::CARRY, false);
                (((high + 10) << 4) | (low & 0x0F)) as u8
            } else {
                self.regs.set_flag(flags::CARRY, true);
                ((high << 4) | (low & 0x0F)) as u8
            };
            
            if self.cpu_type == CpuType::Cpu65C02 {
                self.regs.update_zero_negative_flags(result);
            }
            self.regs.a = result;
        } else {
            self.do_adc(!value);
        }
    }

    pub(super) fn sbc_immediate<M: MemoryBus>(&mut self, memory: &mut M) {
        let value = self.get_immediate(memory);
        self.do_sbc(value);
    }

    pub(super) fn sbc_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_sbc(value);
    }

    pub(super) fn sbc_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_sbc(value);
    }

    pub(super) fn sbc_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_sbc(value);
    }

    pub(super) fn sbc_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, false);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_sbc(value);
    }

    pub(super) fn sbc_absolute_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_y_addr(memory, false);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_sbc(value);
    }

    pub(super) fn sbc_indirect_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_x_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_sbc(value);
    }

    pub(super) fn sbc_indirect_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_y_addr(memory, false);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_sbc(value);
    }

    pub(super) fn sbc_indirect<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_zp_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_sbc(value);
    }
}
