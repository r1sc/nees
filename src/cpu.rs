use core::panic;

use bitfield_struct::bitfield;

use crate::bus::Bus;

#[bitfield(u8)]
pub struct StatusRegister {
    carry: bool,
    zero: bool,
    interrupt_inhibit: bool,
    decimal: bool,
    break_: bool,
    _ignored: bool,
    overflow: bool,
    negative: bool,
}

pub enum OperandType {
    Implied,
    Immediate(u8),
    Accumulator,
    Memory(u16),
    Relative(i8),
}

pub struct MOS6502<T: Bus> {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub status: StatusRegister,
    pub sp: u8,
    pub penaltyaddr: bool,
    pub penaltyop: bool,
    pub clockticks: u32,
    pub addrtable: [fn(&mut Self, &mut T) -> OperandType; 256],
    pub optable: [fn(&mut Self, &OperandType, &mut T); 256],
}

const TICKTABLE: [u32; 256] = [
    /*        |  0  |  1  |  2  |  3  |  4  |  5  |  6  |  7  |  8  |  9  |  A  |  B  |  C  |  D  |  E  |  F  |     */
    /* 0 */
    7, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 4, 4, 6, 6, /* 0 */
    /* 1 */ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, /* 1 */
    /* 2 */ 6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 4, 4, 6, 6, /* 2 */
    /* 3 */ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, /* 3 */
    /* 4 */ 6, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 3, 4, 6, 6, /* 4 */
    /* 5 */ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, /* 5 */
    /* 6 */ 6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 5, 4, 6, 6, /* 6 */
    /* 7 */ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, /* 7 */
    /* 8 */ 2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4, /* 8 */
    /* 9 */ 2, 6, 2, 6, 4, 4, 4, 4, 2, 5, 2, 5, 5, 5, 5, 5, /* 9 */
    /* A */ 2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4, /* A */
    /* B */ 2, 5, 2, 5, 4, 4, 4, 4, 2, 4, 2, 4, 4, 4, 4, 4, /* B */
    /* C */ 2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6, /* C */
    /* D */ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, /* D */
    /* E */ 2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6, /* E */
    /* F */ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, /* F */
];

impl<T: Bus> MOS6502<T> {
    fn read8(&mut self, bus: &mut T) -> u8 {
        let value = bus.cpu_read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        value
    }

    fn read16(&mut self, bus: &mut T) -> u16 {
        let lo = self.read8(bus) as u16;
        let hi = self.read8(bus) as u16;
        (hi << 8) | lo
    }

    fn imm(&mut self, bus: &mut T) -> OperandType {
        OperandType::Immediate(self.read8(bus))
    }

    fn abso(&mut self, bus: &mut T) -> OperandType {
        OperandType::Memory(self.read16(bus))
    }

    fn absx(&mut self, bus: &mut T) -> OperandType {
        let mut ea = self.read16(bus);
        let page = ea & 0xFF00;
        ea = ea.wrapping_add(self.x as u16);

        if page != (ea & 0xFF00) {
            self.penaltyaddr = true;
        }
        OperandType::Memory(ea)
    }

    fn absy(&mut self, bus: &mut T) -> OperandType {
        let mut ea = self.read16(bus);
        let page = ea & 0xFF00;
        ea = ea.wrapping_add(self.y as u16);

        if page != (ea & 0xFF00) {
            self.penaltyaddr = true;
        }
        OperandType::Memory(ea)
    }

    // Implied, no operand
    fn imp(&mut self, _bus: &mut T) -> OperandType {
        OperandType::Implied
    }

    fn ind(&mut self, bus: &mut T) -> OperandType {
        let eahelp = self.read16(bus);
        let eahelp2 = (eahelp & 0xFF00) | ((eahelp + 1) & 0x00FF);
        
        let lo = bus.cpu_read(eahelp) as u16;
        let hi = bus.cpu_read(eahelp2) as u16;

        OperandType::Memory(lo | (hi << 8))
    }

    fn indx(&mut self, bus: &mut T) -> OperandType {
        let zp_address = self.read8(bus).wrapping_add(self.x);
        let lo = bus.cpu_read(zp_address as u16) as u16;
        let hi = bus.cpu_read(zp_address.wrapping_add(1) as u16) as u16;
        OperandType::Memory((hi << 8) | lo)
    }

    fn indy(&mut self, bus: &mut T) -> OperandType {
        let eahelp = self.read8(bus) as u16;
        let eahelp2 = (eahelp & 0xFF00) | ((eahelp + 1) & 0x00FF);

        let lo = bus.cpu_read(eahelp) as u16;
        let hi = (bus.cpu_read(eahelp2) as u16) << 8;
        let mut ea = lo | hi;
        let startpage = ea & 0xFF00;

        ea += self.y as u16;

        if startpage != (ea & 0xFF00) {
            self.penaltyaddr = true;
        }

        OperandType::Memory(ea)
    }

    fn rel(&mut self, bus: &mut T) -> OperandType {
        let reladdr = self.read8(bus) as i8;
        OperandType::Relative(reladdr)
    }

    fn zp(&mut self, bus: &mut T) -> OperandType {
        OperandType::Memory(self.read8(bus) as u16)
    }

    fn zpx(&mut self, bus: &mut T) -> OperandType {
        let zp_addr = self.read8(bus);
        OperandType::Memory(zp_addr.wrapping_add(self.x) as u16)
    }

    fn zpy(&mut self, bus: &mut T) -> OperandType {
        let zp_addr = self.read8(bus);
        OperandType::Memory(zp_addr.wrapping_add(self.y) as u16)
    }

    fn acc(&mut self, _bus: &mut T) -> OperandType {
        OperandType::Accumulator
    }

    pub fn new() -> Self {
        #[rustfmt::skip]
        let addrtable = [
/*        |  0  |  1  |  2  |  3  |  4  |  5  |  6  |  7  |  8  |  9  |  A  |  B  |  C  |  D  |  E  |  F  |     */
/* 0 */     Self::imp, Self::indx, Self:: imp, Self::indx, Self::  zp, Self::  zp, Self::  zp, Self::  zp, Self:: imp, Self:: imm, Self:: acc, Self:: imm, Self::abso, Self::abso, Self::abso, Self::abso, /* 0 */
/* 1 */     Self::rel, Self::indy, Self:: imp, Self::indy, Self:: zpx, Self:: zpx, Self:: zpx, Self:: zpx, Self:: imp, Self::absy, Self:: imp, Self::absy, Self::absx, Self::absx, Self::absx, Self::absx, /* 1 */
/* 2 */    Self::abso, Self::indx, Self:: imp, Self::indx, Self::  zp, Self::  zp, Self::  zp, Self::  zp, Self:: imp, Self:: imm, Self:: acc, Self:: imm, Self::abso, Self::abso, Self::abso, Self::abso, /* 2 */
/* 3 */     Self::rel, Self::indy, Self:: imp, Self::indy, Self:: zpx, Self:: zpx, Self:: zpx, Self:: zpx, Self:: imp, Self::absy, Self:: imp, Self::absy, Self::absx, Self::absx, Self::absx, Self::absx, /* 3 */
/* 4 */     Self::imp, Self::indx, Self:: imp, Self::indx, Self::  zp, Self::  zp, Self::  zp, Self::  zp, Self:: imp, Self:: imm, Self:: acc, Self:: imm, Self::abso, Self::abso, Self::abso, Self::abso, /* 4 */
/* 5 */     Self::rel, Self::indy, Self:: imp, Self::indy, Self:: zpx, Self:: zpx, Self:: zpx, Self:: zpx, Self:: imp, Self::absy, Self:: imp, Self::absy, Self::absx, Self::absx, Self::absx, Self::absx, /* 5 */
/* 6 */     Self::imp, Self::indx, Self:: imp, Self::indx, Self::  zp, Self::  zp, Self::  zp, Self::  zp, Self:: imp, Self:: imm, Self:: acc, Self:: imm, Self::ind, Self::abso, Self::abso, Self::abso, /* 6 */
/* 7 */     Self::rel, Self::indy, Self:: imp, Self::indy, Self:: zpx, Self:: zpx, Self:: zpx, Self:: zpx, Self:: imp, Self::absy, Self:: imp, Self::absy, Self::absx, Self::absx, Self::absx, Self::absx, /* 7 */
/* 8 */     Self::imm, Self::indx, Self:: imm, Self::indx, Self::  zp, Self::  zp, Self::  zp, Self::  zp, Self:: imp, Self:: imm, Self:: imp, Self:: imm, Self::abso, Self::abso, Self::abso, Self::abso, /* 8 */
/* 9 */     Self::rel, Self::indy, Self:: imp, Self::indy, Self:: zpx, Self:: zpx, Self:: zpy, Self:: zpy, Self:: imp, Self::absy, Self:: imp, Self::absy, Self::absx, Self::absx, Self::absy, Self::absy, /* 9 */
/* A */     Self::imm, Self::indx, Self:: imm, Self::indx, Self::  zp, Self::  zp, Self::  zp, Self::  zp, Self:: imp, Self:: imm, Self:: imp, Self:: imm, Self::abso, Self::abso, Self::abso, Self::abso, /* A */
/* B */     Self::rel, Self::indy, Self:: imp, Self::indy, Self:: zpx, Self:: zpx, Self:: zpy, Self:: zpy, Self:: imp, Self::absy, Self:: imp, Self::absy, Self::absx, Self::absx, Self::absy, Self::absy, /* B */
/* C */     Self::imm, Self::indx, Self:: imm, Self::indx, Self::  zp, Self::  zp, Self::  zp, Self::  zp, Self:: imp, Self:: imm, Self:: imp, Self:: imm, Self::abso, Self::abso, Self::abso, Self::abso, /* C */
/* D */     Self::rel, Self::indy, Self:: imp, Self::indy, Self:: zpx, Self:: zpx, Self:: zpx, Self:: zpx, Self:: imp, Self::absy, Self:: imp, Self::absy, Self::absx, Self::absx, Self::absx, Self::absx, /* D */
/* E */     Self::imm, Self::indx, Self:: imm, Self::indx, Self::  zp, Self::  zp, Self::  zp, Self::  zp, Self:: imp, Self:: imm, Self:: imp, Self:: imm, Self::abso, Self::abso, Self::abso, Self::abso, /* E */
/* F */     Self::rel, Self::indy, Self:: imp, Self::indy, Self:: zpx, Self:: zpx, Self:: zpx, Self:: zpx, Self:: imp, Self::absy, Self:: imp, Self::absy, Self::absx, Self::absx, Self::absx, Self::absx  /* F */
        ];

        #[rustfmt::skip]
        let optable = [
    /*        |  0  |  1  |  2  |  3  |  4  |  5  |  6  |  7  |  8  |  9  |  A  |  B  |  C  |  D  |  E  |  F  |      */
	/* 0 */      Self::brk, Self::ora,  Self::nop,  Self::slo,  Self::nop, Self::ora, Self::asl, Self::slo, Self::php, Self::ora, Self::asl, Self::nop, Self::nop, Self::ora, Self::asl, Self::slo, /* 0 */
	/* 1 */      Self::bpl, Self::ora,  Self::nop,  Self::slo,  Self::nop, Self::ora, Self::asl, Self::slo, Self::clc, Self::ora, Self::nop, Self::slo, Self::nop, Self::ora, Self::asl, Self::slo, /* 1 */
	/* 2 */      Self::jsr, Self::and,  Self::nop,  Self::rla,  Self::bit, Self::and, Self::rol, Self::rla, Self::plp, Self::and, Self::rol, Self::nop, Self::bit, Self::and, Self::rol, Self::rla, /* 2 */
	/* 3 */      Self::bmi, Self::and,  Self::nop,  Self::rla,  Self::nop, Self::and, Self::rol, Self::rla, Self::sec, Self::and, Self::nop, Self::rla, Self::nop, Self::and, Self::rol, Self::rla, /* 3 */
	/* 4 */      Self::rti, Self::eor,  Self::nop,  Self::sre,  Self::nop, Self::eor, Self::lsr, Self::sre, Self::pha, Self::eor, Self::lsr, Self::nop, Self::jmp, Self::eor, Self::lsr, Self::sre, /* 4 */
	/* 5 */      Self::bvc, Self::eor,  Self::nop,  Self::sre,  Self::nop, Self::eor, Self::lsr, Self::sre, Self::cli, Self::eor, Self::nop, Self::sre, Self::nop, Self::eor, Self::lsr, Self::sre, /* 5 */
	/* 6 */      Self::rts, Self::adc,  Self::nop,  Self::rra,  Self::nop, Self::adc, Self::ror, Self::rra, Self::pla, Self::adc, Self::ror, Self::nop, Self::jmp, Self::adc, Self::ror, Self::rra, /* 6 */
	/* 7 */      Self::bvs, Self::adc,  Self::nop,  Self::rra,  Self::nop, Self::adc, Self::ror, Self::rra, Self::sei, Self::adc, Self::nop, Self::rra, Self::nop, Self::adc, Self::ror, Self::rra, /* 7 */
	/* 8 */      Self::nop, Self::sta,  Self::nop,  Self::sax,  Self::sty, Self::sta, Self::stx, Self::sax, Self::dey, Self::nop, Self::txa, Self::nop, Self::sty, Self::sta, Self::stx, Self::sax, /* 8 */
	/* 9 */      Self::bcc, Self::sta,  Self::nop,  Self::nop,  Self::sty, Self::sta, Self::stx, Self::sax, Self::tya, Self::sta, Self::txs, Self::nop, Self::nop, Self::sta, Self::nop, Self::nop, /* 9 */
	/* A */      Self::ldy, Self::lda,  Self::ldx,  Self::lax,  Self::ldy, Self::lda, Self::ldx, Self::lax, Self::tay, Self::lda, Self::tax, Self::nop, Self::ldy, Self::lda, Self::ldx, Self::lax, /* A */
	/* B */      Self::bcs, Self::lda,  Self::nop,  Self::lax,  Self::ldy, Self::lda, Self::ldx, Self::lax, Self::clv, Self::lda, Self::tsx, Self::lax, Self::ldy, Self::lda, Self::ldx, Self::lax, /* B */
	/* C */      Self::cpy, Self::cmp,  Self::nop,  Self::dcp,  Self::cpy, Self::cmp, Self::dec, Self::dcp, Self::iny, Self::cmp, Self::dex, Self::nop, Self::cpy, Self::cmp, Self::dec, Self::dcp, /* C */
	/* D */      Self::bne, Self::cmp,  Self::nop,  Self::dcp,  Self::nop, Self::cmp, Self::dec, Self::dcp, Self::cld, Self::cmp, Self::nop, Self::dcp, Self::nop, Self::cmp, Self::dec, Self::dcp, /* D */
	/* E */      Self::cpx, Self::sbc,  Self::nop,  Self::isb,  Self::cpx, Self::sbc, Self::inc, Self::isb, Self::inx, Self::sbc, Self::nop, Self::sbc, Self::cpx, Self::sbc, Self::inc, Self::isb, /* E */
	/* F */      Self::beq, Self::sbc,  Self::nop,  Self::isb,  Self::nop, Self::sbc, Self::inc, Self::isb, Self::sed, Self::sbc, Self::nop, Self::isb, Self::nop, Self::sbc, Self::inc, Self::isb  /* F */
        ];

        Self {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            status: StatusRegister(0),
            addrtable,
            optable,
            penaltyaddr: false,
            penaltyop: false,
            sp: 0xFF,
            clockticks: 0,
        }
    }

    pub fn reset(&mut self, bus: &mut T) {
        let lo = bus.cpu_read(0xFFFC) as u16;
        let hi = bus.cpu_read(0xFFFD) as u16;
        self.pc = (hi << 8) | lo;

        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFD;
        self.status.0 |= 0x20;
    }

    pub fn nmi6502(&mut self, bus: &mut T) {
        self.push_stack16(self.pc, bus);
        self.push_stack8(self.status.0, bus);
        self.status.set_interrupt_inhibit(true);
        self.pc = (bus.cpu_read(0xFFFA) as u16) | ((bus.cpu_read(0xFFFB) as u16) << 8);
    }

    pub fn step(&mut self, bus: &mut T) {
        let opcode = self.read8(bus);

        let addrmode = self.addrtable[opcode as usize];
        let op = self.optable[opcode as usize];
        self.clockticks = TICKTABLE[opcode as usize];

        self.penaltyaddr = false;
        self.penaltyop = false;

        if opcode == 0xFC {
            self.penaltyop = true; // Special NOP
        }

        let operand = (addrmode)(self, bus);
        (op)(self, &operand, bus);

        if self.penaltyop && self.penaltyaddr {
            self.clockticks += 1;
        }
    }

    fn get_value(&mut self, operand: &OperandType, bus: &mut T) -> u8 {
        match operand {
            OperandType::Implied => panic!("Implied, no value to get"),
            OperandType::Immediate(v) => *v,
            OperandType::Accumulator => self.a,
            OperandType::Memory(addr) => bus.cpu_read(*addr),
            OperandType::Relative(_) => panic!("Relative should be computed elsewhere"),
        }
    }

    fn put_value(&mut self, value: u8, operand: &OperandType, bus: &mut T) {
        match operand {
            OperandType::Implied => panic!("Implied, no value to set"),
            OperandType::Immediate(_) => panic!("Cannot set immediate value"),
            OperandType::Accumulator => self.a = value,
            OperandType::Memory(ea) => bus.cpu_write(*ea, value),
            OperandType::Relative(_) => panic!("Cannot write relative values"),
        }
    }

    fn signcalc(&mut self, value: u8) {
        self.status.set_negative((value & 0x80) != 0);
    }

    fn overflowcalc(&mut self, n: u16, m: u8, o: u16) {
        self.status
            .set_overflow(((n ^ (m as u16)) & (n ^ o) & 0x0080) != 0);
    }

    fn zerocalc(&mut self, value: u8) {
        self.status.set_zero(value == 0);
    }

    fn push_stack8(&mut self, value: u8, bus: &mut T) {
        bus.cpu_write(0x100 + (self.sp as u16), value);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn push_stack16(&mut self, value: u16, bus: &mut T) {
        self.push_stack8(((value >> 8) & 0xFF) as u8, bus);
        self.push_stack8((value & 0xFF) as u8, bus);
    }

    fn pull_stack8(&mut self, bus: &mut T) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        bus.cpu_read(0x100 + (self.sp as u16))
    }

    fn pull_stack16(&mut self, bus: &mut T) -> u16 {
        let lo = self.pull_stack8(bus) as u16;
        let hi = self.pull_stack8(bus) as u16;
        (hi << 8) | lo
    }

    fn adc(&mut self, operand: &OperandType, bus: &mut T) {
        self.penaltyop = true;

        let value = self.get_value(operand, bus) as u16;
        let result = (self.a as u16) + value + if self.status.carry() { 1 } else { 0 };
        
        self.status.set_carry(result > 0xFF);
        self.zerocalc(result as u8);
        self.overflowcalc(result, self.a, value);
        self.signcalc(result as u8);

        self.a = result as u8;
    }

    fn and(&mut self, operand: &OperandType, bus: &mut T) {
        self.penaltyop = true;

        let value = self.get_value(operand, bus);
        self.a &= value;

        self.signcalc(self.a);
        self.zerocalc(self.a);
    }

    fn asl(&mut self, operand: &OperandType, bus: &mut T) {
        let value = self.get_value(operand, bus);
        let result = value << 1;

        self.status.set_carry((value & 0x80) != 0);
        self.zerocalc(result);
        self.signcalc(result);

        self.put_value(result, operand, bus);
    }

    fn branch(&mut self, operand: &OperandType, _bus: &mut T) {
        if let OperandType::Relative(disp) = operand {
            let oldpc = self.pc;
            self.pc = self.pc.wrapping_add_signed(*disp as i16);
            if (oldpc & 0xFF00) != (self.pc & 0xFF00) {
                self.clockticks += 2;
            } else {
                self.clockticks += 1;
            }
        }
    }

    fn bcc(&mut self, operand: &OperandType, bus: &mut T) {
        if !self.status.carry() {
            self.branch(operand, bus);
        }
    }

    fn bcs(&mut self, operand: &OperandType, bus: &mut T) {
        if self.status.carry() {
            self.branch(operand, bus);
        }
    }

    fn beq(&mut self, operand: &OperandType, bus: &mut T) {
        if self.status.zero() {
            self.branch(operand, bus);
        }
    }

    fn bit(&mut self, operand: &OperandType, bus: &mut T) {
        let value = self.get_value(operand, bus);
        let result = self.a & value;

        self.zerocalc(result);
        self.status.set_negative((value & 0x80) == 0x80);
        self.status.set_overflow((value & 0x40) == 0x40);
    }

    fn bmi(&mut self, operand: &OperandType, bus: &mut T) {
        if self.status.negative() {
            self.branch(operand, bus);
        }
    }

    fn bne(&mut self, operand: &OperandType, bus: &mut T) {
        if !self.status.zero() {
            self.branch(operand, bus);
        }
    }

    fn bpl(&mut self, operand: &OperandType, bus: &mut T) {
        if !self.status.negative() {
            self.branch(operand, bus);
        }
    }

    fn brk(&mut self, _operand: &OperandType, bus: &mut T) {
        self.push_stack16(self.pc + 1, bus);
        self.push_stack8(self.status.0 | 0x10, bus);
        self.status.set_interrupt_inhibit(true);

        let lo = bus.cpu_read(0xFFFE) as u16;
        let hi = bus.cpu_read(0xFFFF) as u16;
        self.pc = (hi << 8) | lo;
    }

    fn bvc(&mut self, operand: &OperandType, bus: &mut T) {
        if !self.status.overflow() {
            self.branch(operand, bus);
        }
    }

    fn bvs(&mut self, operand: &OperandType, bus: &mut T) {
        if self.status.overflow() {
            self.branch(operand, bus);
        }
    }

    fn clc(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.status.set_carry(false);
    }

    fn cld(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.status.set_decimal(false);
    }

    fn cli(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.status.set_interrupt_inhibit(false);
    }

    fn clv(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.status.set_overflow(false);
    }

    fn compare(&mut self, register: u8, operand: &OperandType, bus: &mut T) {
        let value = self.get_value(operand, bus);

        if register >= value {
            self.status.set_carry(true);
        } else {
            self.status.set_carry(false);
        }

        if register == value {
            self.status.set_zero(true);
        } else {
            self.status.set_zero(false);
        }

        let result = register.wrapping_sub(value);
        self.signcalc(result);
    }

    fn cmp(&mut self, operand: &OperandType, bus: &mut T) {
        self.penaltyop = true;
        self.compare(self.a, operand, bus);
    }

    fn cpx(&mut self, operand: &OperandType, bus: &mut T) {
        self.compare(self.x, operand, bus);
    }

    fn cpy(&mut self, operand: &OperandType, bus: &mut T) {
        self.compare(self.y, operand, bus);
    }

    fn dec(&mut self, operand: &OperandType, bus: &mut T) {
        let value = self.get_value(operand, bus);
        let result = value.wrapping_sub(1);

        self.zerocalc(result);
        self.signcalc(result);
        self.put_value(result, operand, bus);
    }

    fn dex(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.x = self.x.wrapping_sub(1);
        self.zerocalc(self.x);
        self.signcalc(self.x);
    }

    fn dey(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.y = self.y.wrapping_sub(1);
        self.zerocalc(self.y);
        self.signcalc(self.y);
    }

    fn eor(&mut self, operand: &OperandType, bus: &mut T) {
        self.penaltyop = true;
        let value = self.get_value(operand, bus);
        let result = self.a ^ value;

        self.zerocalc(result);
        self.signcalc(result);

        self.a = result;
    }

    fn inc(&mut self, operand: &OperandType, bus: &mut T) {
        let value = self.get_value(operand, bus);
        let result = value.wrapping_add(1);

        self.zerocalc(result);
        self.signcalc(result);

        self.put_value(result, operand, bus);
    }

    fn inx(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.x = self.x.wrapping_add(1);
        self.zerocalc(self.x);
        self.signcalc(self.x);
    }

    fn iny(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.y = self.y.wrapping_add(1);
        self.zerocalc(self.y);
        self.signcalc(self.y);
    }

    fn jmp(&mut self, operand: &OperandType, _bus: &mut T) {
        match operand {
            OperandType::Implied => panic!("JMP is not implied"),
            OperandType::Immediate(_) => panic!("JMP is not an immediate 8-bit value"),
            OperandType::Accumulator => panic!("JMP does not touch the accumulator"),
            OperandType::Memory(ea) => self.pc = *ea,
            OperandType::Relative(_) => panic!("JMP does not do relative addresses"),
        }
    }

    fn jsr(&mut self, operand: &OperandType, bus: &mut T) {
        self.push_stack16(self.pc - 1, bus);
        match operand {
            OperandType::Implied => panic!("JSR is not implied"),
            OperandType::Immediate(_) => panic!("JSR is not an immediate 8-bit value"),
            OperandType::Accumulator => panic!("JSR does not touch the accumulator"),
            OperandType::Memory(ea) => self.pc = *ea,
            OperandType::Relative(_) => panic!("JSR does not do relative addresses"),
        }
    }

    fn lda(&mut self, operand: &OperandType, bus: &mut T) {
        self.penaltyop = true;
        self.a = self.get_value(operand, bus);

        self.zerocalc(self.a);
        self.signcalc(self.a);
    }

    fn ldx(&mut self, operand: &OperandType, bus: &mut T) {
        self.penaltyop = true;
        self.x = self.get_value(operand, bus);

        self.zerocalc(self.x);
        self.signcalc(self.x);
    }

    fn ldy(&mut self, operand: &OperandType, bus: &mut T) {
        self.penaltyop = true;
        self.y = self.get_value(operand, bus);

        self.zerocalc(self.y);
        self.signcalc(self.y);
    }

    fn lsr(&mut self, operand: &OperandType, bus: &mut T) {
        let value = self.get_value(operand, bus);
        let result = value >> 1;

        self.status.set_carry((value & 1) != 0);
        self.zerocalc(result);
        self.signcalc(result);

        self.put_value(result, operand, bus);
    }

    fn nop(&mut self, _operand: &OperandType, _bus: &mut T) {}

    fn ora(&mut self, operand: &OperandType, bus: &mut T) {
        self.penaltyop = true;
        let value = self.get_value(operand, bus);
        let result = self.a | value;

        self.zerocalc(result);
        self.signcalc(result);

        self.a = result;
    }

    fn pha(&mut self, _operand: &OperandType, bus: &mut T) {
        self.push_stack8(self.a, bus);
    }

    fn php(&mut self, _operand: &OperandType, bus: &mut T) {
        self.push_stack8(self.status.0 | 0x10, bus);
    }

    fn pla(&mut self, _operand: &OperandType, bus: &mut T) {
        self.a = self.pull_stack8(bus);

        self.zerocalc(self.a);
        self.signcalc(self.a);
    }

    fn plp(&mut self, _operand: &OperandType, bus: &mut T) {
        self.status.0 = self.pull_stack8(bus) | 0x20;
    }

    fn rol(&mut self, operand: &OperandType, bus: &mut T) {
        let value = self.get_value(operand, bus);
        let mut result = value << 1;

        if self.status.carry() {
            result |= 1
        }

        self.status.set_carry((value & 0x80) != 0);
        self.zerocalc(result);
        self.signcalc(result);

        self.put_value(result, operand, bus);
    }

    fn ror(&mut self, operand: &OperandType, bus: &mut T) {
        let value = self.get_value(operand, bus);
        let mut result = value >> 1;
        if self.status.carry() {
            result |= 0x80;
        }

        self.status.set_carry((value & 1) != 0);
        self.zerocalc(result);
        self.signcalc(result);

        self.put_value(result, operand, bus);
    }

    fn rti(&mut self, _operand: &OperandType, bus: &mut T) {
        self.status.0 = self.pull_stack8(bus);
        self.pc = self.pull_stack16(bus);
    }

    fn rts(&mut self, _operand: &OperandType, bus: &mut T) {
        self.pc = self.pull_stack16(bus) + 1;
    }

    fn sbc(&mut self, operand: &OperandType, bus: &mut T) {
        self.penaltyop = true;
        let value = (self.get_value(operand, bus) as u16) ^ 0x00FF;
        let result = (self.a as u16) + value + if self.status.carry() { 1 } else { 0 };

        self.status.set_carry(result > 0xFF);
        self.zerocalc(result as u8);
        self.overflowcalc(result, self.a, value);
        self.signcalc(result as u8);

        self.a = result as u8;
    }

    fn sec(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.status.set_carry(true);
    }

    fn sed(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.status.set_decimal(true);
    }

    fn sei(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.status.set_interrupt_inhibit(true);
    }

    fn sta(&mut self, operand: &OperandType, bus: &mut T) {
        self.put_value(self.a, operand, bus);
    }

    fn stx(&mut self, operand: &OperandType, bus: &mut T) {
        self.put_value(self.x, operand, bus);
    }

    fn sty(&mut self, operand: &OperandType, bus: &mut T) {
        self.put_value(self.y, operand, bus);
    }

    fn tax(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.x = self.a;

        self.zerocalc(self.x);
        self.signcalc(self.x);
    }

    fn tay(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.y = self.a;

        self.zerocalc(self.y);
        self.signcalc(self.y);
    }

    fn tsx(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.x = self.sp;
        self.zerocalc(self.x);
        self.signcalc(self.x);
    }

    fn txa(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.a = self.x;
        self.zerocalc(self.a);
        self.signcalc(self.a);
    }

    fn txs(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.sp = self.x;
    }

    fn tya(&mut self, _operand: &OperandType, _bus: &mut T) {
        self.a = self.y;
        self.zerocalc(self.a);
        self.signcalc(self.a);
    }

    // Not indented opcodes

    fn lax(&mut self, operand: &OperandType, bus: &mut T) {
        self.lda(operand, bus);
        self.ldx(operand, bus);
    }

    fn sax(&mut self, operand: &OperandType, bus: &mut T) {
        self.sta(operand, bus);
        self.stx(operand, bus);
        self.put_value(self.a & self.x, operand, bus);
        if self.penaltyop && self.penaltyaddr {
            self.clockticks -= 1;
        }
    }

    fn dcp(&mut self, operand: &OperandType, bus: &mut T) {
        self.dec(operand, bus);
        self.cmp(operand, bus);
        if self.penaltyop && self.penaltyaddr {
            self.clockticks -= 1;
        }
    }

    fn isb(&mut self, operand: &OperandType, bus: &mut T) {
        self.inc(operand, bus);
        self.sbc(operand, bus);
        if self.penaltyop && self.penaltyaddr {
            self.clockticks -= 1;
        }
    }

    fn slo(&mut self, operand: &OperandType, bus: &mut T) {
        self.asl(operand, bus);
        self.ora(operand, bus);
        if self.penaltyop && self.penaltyaddr {
            self.clockticks -= 1;
        }
    }

    fn rla(&mut self, operand: &OperandType, bus: &mut T) {
        self.rol(operand, bus);
        self.and(operand, bus);
        if self.penaltyop && self.penaltyaddr {
            self.clockticks -= 1;
        }
    }

    fn sre(&mut self, operand: &OperandType, bus: &mut T) {
        self.lsr(operand, bus);
        self.eor(operand, bus);
        if self.penaltyop && self.penaltyaddr {
            self.clockticks -= 1;
        }
    }

    fn rra(&mut self, operand: &OperandType, bus: &mut T) {
        self.ror(operand, bus);
        self.adc(operand, bus);
        if self.penaltyop && self.penaltyaddr {
            self.clockticks -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::bus::Bus;

    use super::MOS6502;

    struct SimpleMem {
        memory: Vec<u8>,
    }

    impl Bus for SimpleMem {
        fn cpu_read(&mut self, address: u16) -> u8 {
            self.memory[address as usize]
        }

        fn cpu_write(&mut self, address: u16, value: u8) {
            self.memory[address as usize] = value;
        }
    }

    #[test]
    fn test_reset() {
        let mut device = SimpleMem {
            memory: vec![0xea; 256 * 256],
        };

        let mut cpu = MOS6502::new();
        cpu.reset(&mut device);

        assert_eq!(cpu.pc, 0xEAEA);
    }

    #[test]
    fn test_step() {
        let mut device = SimpleMem {
            memory: vec![0xea; 256 * 256],
        };

        let mut cpu = MOS6502::new();
        cpu.reset(&mut device);

        cpu.step(&mut device);
        assert_eq!(cpu.pc, 0xEAEB);
    }

    #[test]
    fn dorman_tests() {
        let memory = std::fs::read("dorman/6502_functional_test.bin").unwrap();
        let mut device = SimpleMem { memory };

        let mut cpu = MOS6502::new();
        cpu.reset(&mut device);
        cpu.pc = 0x400;

        for _ in 0..100000000 {
            if cpu.pc == 0x336d {
                break;
            }
            cpu.step(&mut device);
        }

        assert_eq!(cpu.pc, 0x336d);
    }
}
