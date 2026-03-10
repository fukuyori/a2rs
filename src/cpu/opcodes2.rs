//! オペコードの実装（パート2）

use super::{Cpu, MemoryBus, flags};

impl Cpu {
    //--------------------------------------------------
    // Compare Instructions
    //--------------------------------------------------
    fn do_compare(&mut self, register: u8, value: u8) {
        let result = register.wrapping_sub(value);
        self.regs.set_flag(flags::CARRY, register >= value);
        self.regs.update_zero_negative_flags(result);
    }

    pub(super) fn cmp_immediate<M: MemoryBus>(&mut self, memory: &mut M) {
        let value = self.get_immediate(memory);
        self.do_compare(self.regs.a, value);
    }

    pub(super) fn cmp_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_compare(self.regs.a, value);
    }

    pub(super) fn cmp_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_compare(self.regs.a, value);
    }

    pub(super) fn cmp_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_compare(self.regs.a, value);
    }

    pub(super) fn cmp_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, false);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_compare(self.regs.a, value);
    }

    pub(super) fn cmp_absolute_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_y_addr(memory, false);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_compare(self.regs.a, value);
    }

    pub(super) fn cmp_indirect_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_x_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_compare(self.regs.a, value);
    }

    pub(super) fn cmp_indirect_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_y_addr(memory, false);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_compare(self.regs.a, value);
    }

    pub(super) fn cmp_indirect<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_zp_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_compare(self.regs.a, value);
    }

    pub(super) fn cpx_immediate<M: MemoryBus>(&mut self, memory: &mut M) {
        let value = self.get_immediate(memory);
        self.do_compare(self.regs.x, value);
    }

    pub(super) fn cpx_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_compare(self.regs.x, value);
    }

    pub(super) fn cpx_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_compare(self.regs.x, value);
    }

    pub(super) fn cpy_immediate<M: MemoryBus>(&mut self, memory: &mut M) {
        let value = self.get_immediate(memory);
        self.do_compare(self.regs.y, value);
    }

    pub(super) fn cpy_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_compare(self.regs.y, value);
    }

    pub(super) fn cpy_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.do_compare(self.regs.y, value);
    }

    //--------------------------------------------------
    // Increment/Decrement Memory
    //--------------------------------------------------
    pub(super) fn inc_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        let value = memory.read(addr).wrapping_add(1);
        memory.write(addr, value);
        self.cycles += 3;
        self.regs.update_zero_negative_flags(value);
    }

    pub(super) fn inc_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        let value = memory.read(addr).wrapping_add(1);
        memory.write(addr, value);
        self.cycles += 3;
        self.regs.update_zero_negative_flags(value);
    }

    pub(super) fn inc_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        let value = memory.read(addr).wrapping_add(1);
        memory.write(addr, value);
        self.cycles += 3;
        self.regs.update_zero_negative_flags(value);
    }

    pub(super) fn inc_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, true);
        let value = memory.read(addr).wrapping_add(1);
        memory.write(addr, value);
        self.cycles += 3;
        self.regs.update_zero_negative_flags(value);
    }

    pub(super) fn dec_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        let value = memory.read(addr).wrapping_sub(1);
        memory.write(addr, value);
        self.cycles += 3;
        self.regs.update_zero_negative_flags(value);
    }

    pub(super) fn dec_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        let value = memory.read(addr).wrapping_sub(1);
        memory.write(addr, value);
        self.cycles += 3;
        self.regs.update_zero_negative_flags(value);
    }

    pub(super) fn dec_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        let value = memory.read(addr).wrapping_sub(1);
        memory.write(addr, value);
        self.cycles += 3;
        self.regs.update_zero_negative_flags(value);
    }

    pub(super) fn dec_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, true);
        let value = memory.read(addr).wrapping_sub(1);
        memory.write(addr, value);
        self.cycles += 3;
        self.regs.update_zero_negative_flags(value);
    }

    pub(super) fn inx(&mut self) {
        self.regs.x = self.regs.x.wrapping_add(1);
        self.regs.update_zero_negative_flags(self.regs.x);
        self.cycles += 1;
    }

    pub(super) fn iny(&mut self) {
        self.regs.y = self.regs.y.wrapping_add(1);
        self.regs.update_zero_negative_flags(self.regs.y);
        self.cycles += 1;
    }

    pub(super) fn dex(&mut self) {
        self.regs.x = self.regs.x.wrapping_sub(1);
        self.regs.update_zero_negative_flags(self.regs.x);
        self.cycles += 1;
    }

    pub(super) fn dey(&mut self) {
        self.regs.y = self.regs.y.wrapping_sub(1);
        self.regs.update_zero_negative_flags(self.regs.y);
        self.cycles += 1;
    }

    pub(super) fn ina(&mut self) {
        self.regs.a = self.regs.a.wrapping_add(1);
        self.regs.update_zero_negative_flags(self.regs.a);
        self.cycles += 1;
    }

    pub(super) fn dea(&mut self) {
        self.regs.a = self.regs.a.wrapping_sub(1);
        self.regs.update_zero_negative_flags(self.regs.a);
        self.cycles += 1;
    }

    //--------------------------------------------------
    // Logical Operations
    //--------------------------------------------------
    pub(super) fn and_immediate<M: MemoryBus>(&mut self, memory: &mut M) {
        self.regs.a &= self.get_immediate(memory);
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn and_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        self.regs.a &= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn and_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        self.regs.a &= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn and_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        self.regs.a &= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn and_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, false);
        self.regs.a &= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn and_absolute_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_y_addr(memory, false);
        self.regs.a &= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn and_indirect_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_x_addr(memory);
        self.regs.a &= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn and_indirect_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_y_addr(memory, false);
        self.regs.a &= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn and_indirect<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_zp_addr(memory);
        self.regs.a &= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn ora_immediate<M: MemoryBus>(&mut self, memory: &mut M) {
        self.regs.a |= self.get_immediate(memory);
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn ora_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        self.regs.a |= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn ora_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        self.regs.a |= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn ora_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        self.regs.a |= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn ora_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, false);
        self.regs.a |= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn ora_absolute_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_y_addr(memory, false);
        self.regs.a |= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn ora_indirect_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_x_addr(memory);
        self.regs.a |= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn ora_indirect_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_y_addr(memory, false);
        self.regs.a |= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn ora_indirect<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_zp_addr(memory);
        self.regs.a |= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn eor_immediate<M: MemoryBus>(&mut self, memory: &mut M) {
        self.regs.a ^= self.get_immediate(memory);
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn eor_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        self.regs.a ^= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn eor_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        self.regs.a ^= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn eor_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        self.regs.a ^= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn eor_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, false);
        self.regs.a ^= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn eor_absolute_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_y_addr(memory, false);
        self.regs.a ^= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn eor_indirect_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_x_addr(memory);
        self.regs.a ^= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn eor_indirect_y<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_y_addr(memory, false);
        self.regs.a ^= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    pub(super) fn eor_indirect<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_indirect_zp_addr(memory);
        self.regs.a ^= memory.read(addr);
        self.cycles += 1;
        self.regs.update_zero_negative_flags(self.regs.a);
    }

    //--------------------------------------------------
    // Shift Operations
    //--------------------------------------------------
    pub(super) fn asl_accumulator(&mut self) {
        self.regs.set_flag(flags::CARRY, (self.regs.a & 0x80) != 0);
        self.regs.a <<= 1;
        self.regs.update_zero_negative_flags(self.regs.a);
        self.cycles += 1;
    }

    pub(super) fn asl_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        let mut value = memory.read(addr);
        self.regs.set_flag(flags::CARRY, (value & 0x80) != 0);
        value <<= 1;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    pub(super) fn asl_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        let mut value = memory.read(addr);
        self.regs.set_flag(flags::CARRY, (value & 0x80) != 0);
        value <<= 1;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    pub(super) fn asl_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        let mut value = memory.read(addr);
        self.regs.set_flag(flags::CARRY, (value & 0x80) != 0);
        value <<= 1;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    pub(super) fn asl_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, true);
        let mut value = memory.read(addr);
        self.regs.set_flag(flags::CARRY, (value & 0x80) != 0);
        value <<= 1;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    pub(super) fn lsr_accumulator(&mut self) {
        self.regs.set_flag(flags::CARRY, (self.regs.a & 0x01) != 0);
        self.regs.a >>= 1;
        self.regs.update_zero_negative_flags(self.regs.a);
        self.cycles += 1;
    }

    pub(super) fn lsr_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        let mut value = memory.read(addr);
        self.regs.set_flag(flags::CARRY, (value & 0x01) != 0);
        value >>= 1;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    pub(super) fn lsr_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        let mut value = memory.read(addr);
        self.regs.set_flag(flags::CARRY, (value & 0x01) != 0);
        value >>= 1;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    pub(super) fn lsr_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        let mut value = memory.read(addr);
        self.regs.set_flag(flags::CARRY, (value & 0x01) != 0);
        value >>= 1;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    pub(super) fn lsr_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, true);
        let mut value = memory.read(addr);
        self.regs.set_flag(flags::CARRY, (value & 0x01) != 0);
        value >>= 1;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    pub(super) fn rol_accumulator(&mut self) {
        let carry = if self.regs.get_flag(flags::CARRY) { 1 } else { 0 };
        self.regs.set_flag(flags::CARRY, (self.regs.a & 0x80) != 0);
        self.regs.a = (self.regs.a << 1) | carry;
        self.regs.update_zero_negative_flags(self.regs.a);
        self.cycles += 1;
    }

    pub(super) fn rol_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        let mut value = memory.read(addr);
        let carry = if self.regs.get_flag(flags::CARRY) { 1 } else { 0 };
        self.regs.set_flag(flags::CARRY, (value & 0x80) != 0);
        value = (value << 1) | carry;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    pub(super) fn rol_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        let mut value = memory.read(addr);
        let carry = if self.regs.get_flag(flags::CARRY) { 1 } else { 0 };
        self.regs.set_flag(flags::CARRY, (value & 0x80) != 0);
        value = (value << 1) | carry;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    pub(super) fn rol_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        let mut value = memory.read(addr);
        let carry = if self.regs.get_flag(flags::CARRY) { 1 } else { 0 };
        self.regs.set_flag(flags::CARRY, (value & 0x80) != 0);
        value = (value << 1) | carry;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    pub(super) fn rol_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, true);
        let mut value = memory.read(addr);
        let carry = if self.regs.get_flag(flags::CARRY) { 1 } else { 0 };
        self.regs.set_flag(flags::CARRY, (value & 0x80) != 0);
        value = (value << 1) | carry;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    pub(super) fn ror_accumulator(&mut self) {
        let carry = if self.regs.get_flag(flags::CARRY) { 0x80 } else { 0 };
        self.regs.set_flag(flags::CARRY, (self.regs.a & 0x01) != 0);
        self.regs.a = (self.regs.a >> 1) | carry;
        self.regs.update_zero_negative_flags(self.regs.a);
        self.cycles += 1;
    }

    pub(super) fn ror_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        let mut value = memory.read(addr);
        let carry = if self.regs.get_flag(flags::CARRY) { 0x80 } else { 0 };
        self.regs.set_flag(flags::CARRY, (value & 0x01) != 0);
        value = (value >> 1) | carry;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    pub(super) fn ror_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        let mut value = memory.read(addr);
        let carry = if self.regs.get_flag(flags::CARRY) { 0x80 } else { 0 };
        self.regs.set_flag(flags::CARRY, (value & 0x01) != 0);
        value = (value >> 1) | carry;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    pub(super) fn ror_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        let mut value = memory.read(addr);
        let carry = if self.regs.get_flag(flags::CARRY) { 0x80 } else { 0 };
        self.regs.set_flag(flags::CARRY, (value & 0x01) != 0);
        value = (value >> 1) | carry;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    pub(super) fn ror_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, true);
        let mut value = memory.read(addr);
        let carry = if self.regs.get_flag(flags::CARRY) { 0x80 } else { 0 };
        self.regs.set_flag(flags::CARRY, (value & 0x01) != 0);
        value = (value >> 1) | carry;
        memory.write(addr, value);
        self.regs.update_zero_negative_flags(value);
        self.cycles += 3;
    }

    //--------------------------------------------------
    // BIT Test
    //--------------------------------------------------
    pub(super) fn bit_immediate<M: MemoryBus>(&mut self, memory: &mut M) {
        let value = self.get_immediate(memory);
        self.regs.set_flag(flags::ZERO, (self.regs.a & value) == 0);
        // Note: 65C02 BIT immediate doesn't affect N and V flags
    }

    pub(super) fn bit_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.regs.set_flag(flags::ZERO, (self.regs.a & value) == 0);
        self.regs.set_flag(flags::OVERFLOW, (value & 0x40) != 0);
        self.regs.set_flag(flags::NEGATIVE, (value & 0x80) != 0);
    }

    pub(super) fn bit_zeropage_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_x_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.regs.set_flag(flags::ZERO, (self.regs.a & value) == 0);
        self.regs.set_flag(flags::OVERFLOW, (value & 0x40) != 0);
        self.regs.set_flag(flags::NEGATIVE, (value & 0x80) != 0);
    }

    pub(super) fn bit_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        let value = memory.read(addr);
        self.cycles += 1;
        self.regs.set_flag(flags::ZERO, (self.regs.a & value) == 0);
        self.regs.set_flag(flags::OVERFLOW, (value & 0x40) != 0);
        self.regs.set_flag(flags::NEGATIVE, (value & 0x80) != 0);
    }

    pub(super) fn bit_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_x_addr(memory, false);
        let value = memory.read(addr);
        self.cycles += 1;
        self.regs.set_flag(flags::ZERO, (self.regs.a & value) == 0);
        self.regs.set_flag(flags::OVERFLOW, (value & 0x40) != 0);
        self.regs.set_flag(flags::NEGATIVE, (value & 0x80) != 0);
    }

    //--------------------------------------------------
    // TRB/TSB (65C02)
    //--------------------------------------------------
    pub(super) fn trb_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        let value = memory.read(addr);
        self.regs.set_flag(flags::ZERO, (self.regs.a & value) == 0);
        memory.write(addr, value & !self.regs.a);
        self.cycles += 3;
    }

    pub(super) fn trb_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        let value = memory.read(addr);
        self.regs.set_flag(flags::ZERO, (self.regs.a & value) == 0);
        memory.write(addr, value & !self.regs.a);
        self.cycles += 3;
    }

    pub(super) fn tsb_zeropage<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_zeropage_addr(memory);
        let value = memory.read(addr);
        self.regs.set_flag(flags::ZERO, (self.regs.a & value) == 0);
        memory.write(addr, value | self.regs.a);
        self.cycles += 3;
    }

    pub(super) fn tsb_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        let value = memory.read(addr);
        self.regs.set_flag(flags::ZERO, (self.regs.a & value) == 0);
        memory.write(addr, value | self.regs.a);
        self.cycles += 3;
    }

    //--------------------------------------------------
    // Branch Instructions
    //--------------------------------------------------
    pub(super) fn bpl<M: MemoryBus>(&mut self, memory: &mut M) {
        let cond = !self.regs.get_flag(flags::NEGATIVE);
        self.branch(memory, cond);
    }

    pub(super) fn bmi<M: MemoryBus>(&mut self, memory: &mut M) {
        let cond = self.regs.get_flag(flags::NEGATIVE);
        self.branch(memory, cond);
    }

    pub(super) fn bvc<M: MemoryBus>(&mut self, memory: &mut M) {
        let cond = !self.regs.get_flag(flags::OVERFLOW);
        self.branch(memory, cond);
    }

    pub(super) fn bvs<M: MemoryBus>(&mut self, memory: &mut M) {
        let cond = self.regs.get_flag(flags::OVERFLOW);
        self.branch(memory, cond);
    }

    pub(super) fn bcc<M: MemoryBus>(&mut self, memory: &mut M) {
        let cond = !self.regs.get_flag(flags::CARRY);
        self.branch(memory, cond);
    }

    pub(super) fn bcs<M: MemoryBus>(&mut self, memory: &mut M) {
        let cond = self.regs.get_flag(flags::CARRY);
        self.branch(memory, cond);
    }

    pub(super) fn bne<M: MemoryBus>(&mut self, memory: &mut M) {
        let cond = !self.regs.get_flag(flags::ZERO);
        self.branch(memory, cond);
    }

    pub(super) fn beq<M: MemoryBus>(&mut self, memory: &mut M) {
        let cond = self.regs.get_flag(flags::ZERO);
        self.branch(memory, cond);
    }

    pub(super) fn bra<M: MemoryBus>(&mut self, memory: &mut M) {
        self.branch(memory, true);
    }

    //--------------------------------------------------
    // Jump and Call
    //--------------------------------------------------
    pub(super) fn jmp_absolute<M: MemoryBus>(&mut self, memory: &mut M) {
        self.regs.pc = self.get_absolute_addr(memory);
    }

    pub(super) fn jmp_indirect<M: MemoryBus>(&mut self, memory: &mut M) {
        let low_addr = memory.read(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        let high_addr = memory.read(self.regs.pc) as u16;
        self.regs.pc = self.regs.pc.wrapping_add(1);
        let ptr = (high_addr << 8) | low_addr;
        
        let low = memory.read(ptr) as u16;
        let high = if self.cpu_type == super::CpuType::Cpu65C02 {
            // 65C02: ページ境界バグが修正されている
            memory.read(ptr.wrapping_add(1)) as u16
        } else {
            // 6502バグ：ページ境界でのラップアラウンド
            memory.read((ptr & 0xFF00) | ((ptr.wrapping_add(1)) & 0x00FF)) as u16
        };
        self.regs.pc = (high << 8) | low;
        self.cycles += 3;
    }

    pub(super) fn jmp_absolute_x<M: MemoryBus>(&mut self, memory: &mut M) {
        let base = self.get_absolute_addr(memory);
        let addr = base.wrapping_add(self.regs.x as u16);
        let low = memory.read(addr) as u16;
        let high = memory.read(addr.wrapping_add(1)) as u16;
        self.regs.pc = (high << 8) | low;
        self.cycles += 2;
    }

    pub(super) fn jsr<M: MemoryBus>(&mut self, memory: &mut M) {
        let addr = self.get_absolute_addr(memory);
        let return_addr = self.regs.pc.wrapping_sub(1);
        self.push_word(memory, return_addr);
        self.regs.pc = addr;
        self.cycles += 2;
    }

    pub(super) fn rts<M: MemoryBus>(&mut self, memory: &mut M) {
        self.regs.pc = self.pop_word(memory).wrapping_add(1);
        self.cycles += 4;
    }

    //--------------------------------------------------
    // Interrupts
    //--------------------------------------------------
    pub(super) fn brk<M: MemoryBus>(&mut self, memory: &mut M) {
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.push_word(memory, self.regs.pc);
        let status = self.regs.status | flags::BREAK | flags::UNUSED;
        self.push_byte(memory, status);
        self.regs.set_flag(flags::IRQ_DISABLE, true);
        
        // 65C02: BRK後にDフラグをクリア
        if self.cpu_type == super::CpuType::Cpu65C02 {
            self.regs.set_flag(flags::DECIMAL, false);
        }
        
        let low = memory.read(0xFFFE) as u16;
        let high = memory.read(0xFFFF) as u16;
        self.regs.pc = (high << 8) | low;
        self.cycles += 5;
    }

    pub(super) fn rti<M: MemoryBus>(&mut self, memory: &mut M) {
        self.regs.status = self.pop_byte(memory);
        self.regs.set_flag(flags::UNUSED, true);
        self.regs.set_flag(flags::BREAK, false);
        self.regs.pc = self.pop_word(memory);
        self.cycles += 4;
    }

    //--------------------------------------------------
    // Flag Instructions
    //--------------------------------------------------
    pub(super) fn clc(&mut self) {
        self.regs.set_flag(flags::CARRY, false);
        self.cycles += 1;
    }

    pub(super) fn sec(&mut self) {
        self.regs.set_flag(flags::CARRY, true);
        self.cycles += 1;
    }

    pub(super) fn cli(&mut self) {
        self.regs.set_flag(flags::IRQ_DISABLE, false);
        self.cycles += 1;
    }

    pub(super) fn sei(&mut self) {
        self.regs.set_flag(flags::IRQ_DISABLE, true);
        self.cycles += 1;
    }

    pub(super) fn clv(&mut self) {
        self.regs.set_flag(flags::OVERFLOW, false);
        self.cycles += 1;
    }

    pub(super) fn cld(&mut self) {
        self.regs.set_flag(flags::DECIMAL, false);
        self.cycles += 1;
    }

    pub(super) fn sed(&mut self) {
        self.regs.set_flag(flags::DECIMAL, true);
        self.cycles += 1;
    }

    //--------------------------------------------------
    // NOP
    //--------------------------------------------------
    pub(super) fn nop(&mut self) {
        self.cycles += 1;
    }

    //--------------------------------------------------
    // 65C02 Bit Manipulation Instructions
    //--------------------------------------------------
    
    /// RMB - Reset Memory Bit (65C02)
    /// RMB0-RMB7: $07, $17, $27, $37, $47, $57, $67, $77
    pub(super) fn rmb<M: MemoryBus>(&mut self, memory: &mut M, bit: u8) {
        let addr = self.get_zeropage_addr(memory);
        let mut value = memory.read(addr);
        self.cycles += 1;
        value &= !(1 << bit);
        memory.write(addr, value);
        self.cycles += 1;
    }

    /// SMB - Set Memory Bit (65C02)
    /// SMB0-SMB7: $87, $97, $A7, $B7, $C7, $D7, $E7, $F7
    pub(super) fn smb<M: MemoryBus>(&mut self, memory: &mut M, bit: u8) {
        let addr = self.get_zeropage_addr(memory);
        let mut value = memory.read(addr);
        self.cycles += 1;
        value |= 1 << bit;
        memory.write(addr, value);
        self.cycles += 1;
    }

    /// BBR - Branch on Bit Reset (65C02)
    /// BBR0-BBR7: $0F, $1F, $2F, $3F, $4F, $5F, $6F, $7F
    pub(super) fn bbr<M: MemoryBus>(&mut self, memory: &mut M, bit: u8) {
        let zp_addr = self.fetch_byte(memory) as u16;
        let value = memory.read(zp_addr);
        self.cycles += 1;
        let offset = self.fetch_byte(memory) as i8;
        
        if (value & (1 << bit)) == 0 {
            // ビットが0ならブランチ
            let new_pc = (self.regs.pc as i32).wrapping_add(offset as i32) as u16;
            self.regs.pc = new_pc;
            self.cycles += 1;
        }
    }

    /// BBS - Branch on Bit Set (65C02)
    /// BBS0-BBS7: $8F, $9F, $AF, $BF, $CF, $DF, $EF, $FF
    pub(super) fn bbs<M: MemoryBus>(&mut self, memory: &mut M, bit: u8) {
        let zp_addr = self.fetch_byte(memory) as u16;
        let value = memory.read(zp_addr);
        self.cycles += 1;
        let offset = self.fetch_byte(memory) as i8;
        
        if (value & (1 << bit)) != 0 {
            // ビットが1ならブランチ
            let new_pc = (self.regs.pc as i32).wrapping_add(offset as i32) as u16;
            self.regs.pc = new_pc;
            self.cycles += 1;
        }
    }
}
