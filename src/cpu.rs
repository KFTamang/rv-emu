use crate::bus::*;
use crate::csr::*;
use crate::dram::*;
use crate::clint::*;
use crate::interrupt::*;

use log::{info, debug};

use std::cmp;
use std::sync::{Arc, mpsc};

const REG_NUM: usize = 32;
pub const M_MODE: u64 = 0b11;
pub const S_MODE: u64 = 0b10;
pub const U_MODE: u64 = 0b00;

#[derive(PartialEq)]
enum AccessMode {
    Load,
    Store,
}

fn bit(integer: u64, bit: u64) -> u64 {
    (integer >> bit) & 0x1
}

pub struct Cpu {
    pub regs: [u64; 32],
    pub pc: u64,
    pub bus: Bus,
    pub csr: Csr,
    dest: usize,
    src1: usize,
    src2: usize,
    pub mode: u64,
    dump_count: u64,
    inst_string: String,
    interrupt_receiver: mpsc::Receiver<Interrupt>,
    clint: Clint,
}

impl Cpu {
    pub fn new(
        binary: Vec<u8>,
        base_addr: u64,
        _dump_count: u64,
    ) -> Self {
        let mut regs = [0; 32];
        regs[2] = DRAM_SIZE;
        let (interrupt_sender, interrupt_receiver) = mpsc::channel();

        Self {
            regs,
            pc: base_addr,
            bus: Bus::new(binary, base_addr),
            csr: Csr::new(Arc::new(interrupt_sender.clone())),
            dest: REG_NUM,
            src1: REG_NUM,
            src2: REG_NUM,
            mode: M_MODE,
            dump_count: _dump_count,
            inst_string: String::from(""),
            clint: Clint::new(0x200_0000, 0x10000, Arc::new(interrupt_sender)),
            interrupt_receiver: interrupt_receiver,
        }
    }

    pub fn fetch(&mut self) -> Result<u64, ()> {
        let index = self.pc as usize;
        match self.load(index as u64, 32) {
            Ok(inst) => Ok(inst),
            Err(_) => Err(()),
        }
    }

    fn print_inst_r(&mut self, name: &str, rd: usize, rs1: usize, rs2: usize) {
        if self.dump_count > 0 {
            self.inst_string = format!(
                "{:>#x} : {}, dest:{}, rs1:{}, rs2:{}\n",
                self.pc, name, rd, rs1, rs2
            );
        }
    }

    fn print_inst_i(&mut self, name: &str, rd: usize, rs1: usize, imm: u64) {
        if self.dump_count > 0 {
            self.inst_string = format!(
                "{:>#x} : {}, rd:{}, rs1:{}, imm:{}({:>#x})\n",
                self.pc, name, rd, rs1, imm as i32, imm as i32
            );
        }
    }

    fn print_inst_s(&mut self, name: &str, rs1: usize, rs2: usize, imm: u64) {
        if self.dump_count > 0 {
            self.inst_string = format!(
                "{:>#x} : {}, offset:{}, base:{}, src:{}\n",
                self.pc, name, imm as i64, rs1, rs2
            );
        }
    }

    fn print_inst_b(&mut self, name: &str, rs1: usize, rs2: usize, imm: u64) {
        if self.dump_count > 0 {
            self.inst_string = format!(
                "{:>#x} : {}, rs1:{}, rs2:{}, offset:{}\n",
                self.pc, name, rs1, rs2, imm as i64
            );
        }
    }

    fn print_inst_j(&mut self, name: &str, rd: usize, imm: u64) {
        if self.dump_count > 0 {
            self.inst_string = format!(
                "{:>#x} : {}, dest:{}, offset:{}({:>#x})\n",
                self.pc, name, rd, imm as i64, imm as i64
            );
        }
    }

    fn print_inst_csr(&mut self, name: &str, rd: usize, rs1: usize, csr: u64) {
        if self.dump_count > 0 {
            self.inst_string = format!(
                "{:>#x} : {}, dest:{}, rs1:{}, csr:{}({:>#x})\n",
                self.pc, name, rd, rs1, csr, csr
            );
        }
    }

    fn print_inst_csri(&mut self, name: &str, rd: usize, csr: u64, uimm: u64) {
        if self.dump_count > 0 {
            self.inst_string = format!(
                "{:>#x} : {}, dest:{}, csr:{}({:>#x}), uimm:{}({:>#x})\n",
                self.pc, name, rd, csr, csr, uimm, uimm
            );
        }
    }

    fn mark_as_dest(&mut self, reg: usize) {
        self.dest = reg;
    }

    fn mark_as_src1(&mut self, reg: usize) {
        self.src1 = reg;
    }

    fn mark_as_src2(&mut self, reg: usize) {
        self.src2 = reg;
    }

    fn clear_reg_marks(&mut self) {
        self.dest = REG_NUM;
        self.src1 = REG_NUM;
        self.src2 = REG_NUM;
    }

    fn load(&mut self, va: u64, size: u64) -> Result<u64, Exception> {
        debug!("Load access to 0x{:x}", va);
        match self.translate(va, AccessMode::Load) {
            Ok(pa) => {
                debug!("Physical address :0x{:x}", pa);
                if self.clint.is_accessible(pa) {
                    debug!("Access to CLINT");
                    self.clint.load(pa, size)
                } else {
                    debug!("Access to bus");
                    self.bus.load(pa, size)
                }
            },
            Err(e) => Err(e),
        }
    }

    fn store(&mut self, va: u64, size: u64, value: u64) -> Result<(), Exception> {
        match self.translate(va, AccessMode::Store) {
            Ok(pa) => {
                if self.clint.is_accessible(pa) {
                    self.clint.store(pa, size, value)
                } else {
                    self.bus.store(pa, size, value)
                }
            },
            Err(e) => Err(e),
        }
    }

    fn translate(&mut self, va: u64, acc_mode: AccessMode) -> Result<u64, Exception> {
        const PAGESIZE: u64 = 4096;
        const PTESIZE: u64 = 8; // 64bit
        const LEVEL: u64 = 3;
        let satp = self.csr.load_csrs(SATP);
        let mode = satp >> 63;
        let _asid = (satp >> 22) & 0x1ff;
        if mode == 0 {
            return Ok(va);
        }
        let vpn = [(va >> 12) & 0x1ff, (va >> 21) & 0x1ff, (va >> 30) & 0x1ff];
        let mut pt_addr = 0;
        let mut i = (LEVEL - 1) as i64;
        let mut pte = 0;
        let mut ppn = satp & 0xfff_ffff_ffff;
        while i >= 0 {
            pt_addr = ppn * PAGESIZE + vpn[i as usize] * PTESIZE;
            if let Ok(val) = self.bus.load(pt_addr, 64) {
                pte = val;
                let v = bit(pte, 0);
                let r = bit(pte, 1);
                let w = bit(pte, 2);
                let x = bit(pte, 3);
                let _u = bit(pte, 4);
                let _g = bit(pte, 5);
                if (v == 0) || ((r == 0) && (w == 1)) {
                    return Err(Exception::LoadPageFault(va as u32));
                }
                if (r == 1) || (x == 1) {
                    break;
                }
                ppn = (pte >> 10) & 0xfff_ffff_ffff;
                i = i - 1;
            } else {
                return Err(Exception::LoadPageFault(va as u32));
            }
        }
        let a = bit(pte, 6);
        let d = bit(pte, 7);
        if (a == 0) || ((d == 0) && (acc_mode == AccessMode::Store)) {
            self.bus.store(pt_addr, 64, pte | (1 << 6))?;
        }
        match i {
            0 => Ok(((pte << 2) & 0xfffffffffff000) | (va & 0x00000fff)),
            1 => Ok(((pte << 2) & 0xffffffffe00000) | (va & 0x001fffff)),
            2 => Ok(((pte << 2) & 0xffffffc0000000) | (va & 0x3fffffff)),
            _ => panic!("something goes wrong at MMU!"),
        }
    }

    fn wait_for_interrupt(&mut self) {
        // wait for a message that notifies an interrupt on the interrupt channel
        info!("waiting for interrupt");
        let interrupt = self.interrupt_receiver.recv().unwrap();
        self.set_pending_interrupt(interrupt);
    }

    fn set_pending_interrupt(&mut self, interrupt: Interrupt) {
        let xip = if self.mode == M_MODE {
            self.csr.load_csrs(MIP)
        } else {
            self.csr.load_csrs(SIP)
        };
        let new_xip = xip | interrupt.bit_code();
        if self.mode == M_MODE {
            self.csr.store_csrs(MIP, new_xip);
        } else {
            self.csr.store_csrs(SIP, new_xip);
        };

        info!("Interrupt {:?} is set, xIP: {:b}, xIE: {:0b}", interrupt, new_xip, self.csr.load_csrs(MIE));
    }

    // get the takable pending interrupt with the highest priority 
    pub fn get_interrupt_to_take(&self) -> Option<Interrupt> {
        // An interrupt i will be taken
        // (a)if bit i is set in both mip and mie,
        // (b)and if interrupts are globally enabled.
        // By default, M-mode interrupts are globally enabled
        // (b-1)if the hart’s current privilege mode is less than M,
        // (b-2)or if the current privilege mode is M and the MIE bit in the mstatus register is set.
        // (c)If bit i in mideleg is set, however, interrupts are considered to be globally enabled
        // if the hart’s current privilege mode equals the delegated privilege mode and that mode’s interrupt enable bit (xIE in mstatus for mode x) is set,
        // or if the current privilege mode is less than the delegated privilege mode.
        let xip = if self.mode == M_MODE {
            self.csr.load_csrs(MIP)
        } else {
            self.csr.load_csrs(SIP)
        };
        let xie = if self.mode == M_MODE {
            self.csr.load_csrs(MIE)
        } else {
            self.csr.load_csrs(SIE)
        };
        let pending_interrupt = xip & xie;
        for interrupt in Interrupt::PRIORITY_ORDER.iter() {
            if (pending_interrupt & interrupt.bit_code()) != 0 {
                // check if the interrupt is globally enabled
                if self.mode != M_MODE {
                    return Some(*interrupt);
                } else {
                    //TODO: check if the interrupt is delegated
                }
                return Some(*interrupt);
            }
        }
        None
    }

    #[allow(unused)]
    fn trap(&mut self) {
        // trap process here

        // store program counter
        self.csr.store_csrs(MEPC, self.pc);

        // prepare mstatus
        let prev_mstatus = self.csr.load_csrs(MSTATUS);
        let mut new_mstatus = prev_mstatus;
        new_mstatus &= !MASK_MIE; // clear mstatus.MIE
        new_mstatus &= !MASK_MPP; // clear mstatus.MPP for writing new value
        new_mstatus |= (self.mode as u64) << 11; // write current mode to mstatus.MPP
        if (prev_mstatus & MASK_MIE) != 0 {
            // set previous MIE to MPIE
            new_mstatus |= MASK_MPIE;
        } else {
            new_mstatus &= !MASK_MPIE;
        }
        self.csr.store_csrs(MSTATUS, new_mstatus);

        // transition to M_MODE
        self.mode = M_MODE;

        self.pc = self.csr.load_csrs(MTVEC) & !(0b11);
    }

    fn return_from_trap(&mut self) {
        // mstatus.MIE <- mstatus.MPIE(=1)
        // U-modeに遷移する
        // mstatus.MPIE <~ 1 [always]
        // mstatus.MPP <~ 00(U-mode) [always]
        // pc(program counter) <~ mepc CSR
        match self.mode {
            M_MODE => {
                let pp = self.csr.get_mstatus_bit(MASK_MPP, BIT_MPP);
                let pie = self.csr.get_mstatus_bit(MASK_MPIE, BIT_MPIE);
                let previous_pc = self.csr.load_csrs(MEPC);
                self.csr.set_mstatus_bit(pie, MASK_MIE, BIT_MIE);
                self.csr.set_mstatus_bit(0b1, MASK_MPIE, BIT_MPIE);
                self.csr.set_mstatus_bit(U_MODE, MASK_MPP, BIT_MPP);
                self.pc = previous_pc.wrapping_sub(4); // subtract 4 to cancel out addition in main loop
                self.mode = pp;
                info!("back to privilege {} from machine mode", pp);
            }
            S_MODE => {
                let pp = self.csr.get_sstatus_bit(MASK_SPP, BIT_SPP);
                let pie = self.csr.get_sstatus_bit(MASK_SPIE, BIT_SPIE);
                let previous_pc = self.csr.load_csrs(SEPC);
                self.csr.set_sstatus_bit(pie, MASK_SIE, BIT_SIE);
                self.csr.set_sstatus_bit(0b1, MASK_SPIE, BIT_SPIE);
                self.csr.set_sstatus_bit(U_MODE, MASK_SPP, BIT_SPP);
                self.pc = previous_pc.wrapping_sub(4); // subtract 4 to cancel out addition in main loop
                self.mode = pp;
                info!("back to privilege {} from supervisor mode", pp);
            }
            _ => {
                panic!("m/sret from U_MODE\n");
            }
        }
    }

    pub fn execute(&mut self, inst: u32) -> Result<(), Exception> {
        let opcode = inst & 0x7f;
        let rd = ((inst >> 7) & 0x1f) as usize;
        let rs1 = ((inst >> 15) & 0x1f) as usize;
        let rs2 = ((inst >> 20) & 0x1f) as usize;
        let funct3 = ((inst >> 12) & 0x7) as usize;
        let funct7 = ((inst >> 25) & 0x7f) as usize;

        self.clear_reg_marks();

        match opcode {
            0x33 => {
                match (funct3, funct7) {
                    (0x0, 0x0) => {
                        self.print_inst_r("add", rd, rs1, rs2);
                        self.regs[rd] = self.regs[rs1].wrapping_add(self.regs[rs2]);
                    }
                    (0x0, 0x20) => {
                        self.print_inst_r("sub", rd, rs1, rs2);
                        self.regs[rd] = self.regs[rs1].wrapping_sub(self.regs[rs2]);
                    }
                    (0x1, 0x0) => {
                        self.print_inst_r("sll", rd, rs1, rs2);
                        let shamt = self.regs[rs2] & 0x1f;
                        self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                    }
                    (0x2, 0x0) => {
                        self.print_inst_r("slt", rd, rs1, rs2);
                        self.regs[rd] = if (rs1 as i64) < (rs2 as i64) { 1 } else { 0 }
                    }
                    (0x3, 0x0) => {
                        self.print_inst_r("sltu", rd, rs1, rs2);
                        self.regs[rd] = if (rs1 as u64) < (rs2 as u64) { 1 } else { 0 }
                    }
                    (0x4, 0x0) => {
                        self.print_inst_r("xor", rd, rs1, rs2);
                        self.regs[rd] = self.regs[rs1] ^ self.regs[rs2];
                    }
                    (0x5, 0x0) => {
                        self.print_inst_r("srl", rd, rs1, rs2);
                        let shamt = self.regs[rs2] & 0x1f;
                        self.regs[rd] = self.regs[rs1] as u64 >> shamt;
                    }
                    (0x5, 0x20) => {
                        self.print_inst_r("sra", rd, rs1, rs2);
                        let shamt = self.regs[rs2] & 0x1f;
                        self.regs[rd] = (self.regs[rs1] as i64 as u64) >> shamt;
                    }
                    (0x6, 0x0) => {
                        self.print_inst_r("or", rd, rs1, rs2);
                        self.regs[rd] = self.regs[rs1] | self.regs[rs2];
                    }
                    (0x7, 0x0) => {
                        self.print_inst_r("and", rd, rs1, rs2);
                        self.regs[rd] = self.regs[rs1] & self.regs[rs2];
                    }
                    (0x0, 0x1) => {
                        self.print_inst_r("mul", rd, rs1, rs2);
                        self.regs[rd] = self.regs[rs1].wrapping_mul(self.regs[rs2]);
                    }
                    (0x1, 0x1) => {
                        self.print_inst_r("mulh", rd, rs1, rs2);
                        let mul = (self.regs[rs1] as i64 as i128)
                            .wrapping_mul(self.regs[rs2] as i64 as i128);
                        self.regs[rd] = (mul >> 64) as u64;
                    }
                    (0x2, 0x1) => {
                        self.print_inst_r("mulhsu", rd, rs1, rs2);
                        let mul = (self.regs[rs1] as i64 as i128)
                            .wrapping_mul(self.regs[rs2] as u128 as i128);
                        self.regs[rd] = (mul >> 64) as u64;
                    }
                    (0x3, 0x1) => {
                        self.print_inst_r("mulhu", rd, rs1, rs2);
                        let mul = (self.regs[rs1] as u128).wrapping_mul(self.regs[rs2] as u128);
                        self.regs[rd] = (mul >> 64) as u64;
                    }
                    (0x4, 0x1) => {
                        self.print_inst_r("div", rd, rs1, rs2);
                        self.regs[rd] = self.regs[rs1] / self.regs[rs2];
                    }
                    (0x5, 0x1) => {
                        self.print_inst_r("divu", rd, rs1, rs2);
                        self.regs[rd] = ((self.regs[rs1] as i64) / (self.regs[rs2] as i64)) as u64;
                    }
                    (0x6, 0x1) => {
                        self.print_inst_r("rem", rd, rs1, rs2);
                        self.regs[rd] = self.regs[rs1] % self.regs[rs2];
                    }
                    (0x7, 0x1) => {
                        self.print_inst_r("remu", rd, rs1, rs2);
                        self.regs[rd] = ((self.regs[rs1] as i64) % (self.regs[rs2] as i64)) as u64;
                    }
                    (_, _) => {
                        info!("This should not be reached!");
                        info!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(Exception::IllegalInstruction(inst));
                    }
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            0x13 => {
                let imm = (inst as i32 as i64 >> 20) as u64;
                match funct3 {
                    0x0 => {
                        self.print_inst_i("addi", rd, rs1, imm);
                        self.regs[rd] = self.regs[rs1].wrapping_add(imm);
                    }
                    0x2 => {
                        self.print_inst_i("slti", rd, rs1, imm);
                        let result = if (self.regs[rs1] as i32 as i64) < (imm as i64) {
                            1
                        } else {
                            0
                        };
                        self.regs[rd] = result;
                    }
                    0x3 => {
                        self.print_inst_i("sltiu", rd, rs1, imm);
                        let result = if (self.regs[rs1] as i32 as i64 as u64) < imm {
                            1
                        } else {
                            0
                        };
                        self.regs[rd] = result;
                    }
                    0x4 => {
                        self.print_inst_i("xori", rd, rs1, imm);
                        let val = ((self.regs[rs1] as i32) ^ (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x6 => {
                        self.print_inst_i("ori", rd, rs1, imm);
                        let val = ((self.regs[rs1] as i32) | (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x7 => {
                        self.print_inst_i("andi", rd, rs1, imm);
                        let val = ((self.regs[rs1] as i32) & (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x1 => {
                        self.print_inst_i("slli", rd, rs1, imm);
                        let shamt = (imm & 0x3f) as u64;
                        self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                    }
                    0x5 => {
                        self.print_inst_i("srli/srai", rd, rs1, imm);
                        let shamt = (imm & 0x3f) as u64;
                        let logical_shift = imm >> 5;
                        if logical_shift == 0 {
                            self.regs[rd] = (self.regs[rs1] as u64) >> shamt;
                        } else {
                            self.regs[rd] = ((self.regs[rs1] as i64) >> shamt) as u64;
                        }
                    }
                    _ => {
                        info!("This should not be reached!");
                        info!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(Exception::IllegalInstruction(inst));
                    }
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            0x03 => {
                // load instructions
                // load a value stored at addr, where addr is RS1 + imm
                let imm = ((inst as i32 as i64) >> 20) as u64;
                let addr = self.regs[rs1].wrapping_add(imm);
                match funct3 {
                    0x0 => {
                        self.print_inst_i("lb", rd, rs1, imm);
                        let val = self.load(addr, 8)?;
                        self.regs[rd] = val as i8 as i64 as u64;
                    }
                    0x1 => {
                        self.print_inst_i("lh", rd, rs1, imm);
                        let val = self.load(addr, 16)?;
                        self.regs[rd] = val as i16 as i64 as u64;
                    }
                    0x2 => {
                        self.print_inst_i("lw", rd, rs1, imm);
                        let val = self.load(addr, 32)?;
                        self.regs[rd] = val as i32 as i64 as u64;
                    }
                    0x3 => {
                        self.print_inst_i("ld", rd, rs1, imm);
                        let val = self.load(addr, 64)?;
                        self.regs[rd] = val;
                    }
                    0x4 => {
                        self.print_inst_i("lbu", rd, rs1, imm);
                        let val = self.load(addr, 8)?;
                        self.regs[rd] = val;
                    }
                    0x5 => {
                        self.print_inst_i("lhu", rd, rs1, imm);
                        let val = self.load(addr, 16)?;
                        self.regs[rd] = val;
                    }
                    0x6 => {
                        self.print_inst_i("lwu", rd, rs1, imm);
                        let val = self.load(addr, 32)?;
                        self.regs[rd] = val;
                    }
                    _ => {
                        info!("This should not be reached!");
                        info!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(Exception::IllegalInstruction(inst));
                    }
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            0x23 => {
                // store instructions
                let imm = (((inst & 0xfe000000) as i32 as i64 >> 20) as u64)
                    | ((inst >> 7) & 0x1f) as u64;
                let addr = self.regs[rs1].wrapping_add(imm);
                self.print_inst_s("s?", rs1, rs2, imm);
                match funct3 {
                    0x0 => self.store(addr, 8, self.regs[rs2])?,
                    0x1 => self.store(addr, 16, self.regs[rs2])?,
                    0x2 => self.store(addr, 32, self.regs[rs2])?,
                    0x3 => self.store(addr, 64, self.regs[rs2])?,
                    _ => {
                        info!("This should not be reached!");
                        info!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(Exception::IllegalInstruction(inst));
                    }
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            0x6f => {
                // jal
                let imm = ((inst & 0x80000000) as i32 as i64 >> 11) as u64
                    | ((inst & 0x7fe00000) as u64) >> 20
                    | ((inst & 0x100000) as u64) >> 9
                    | ((inst & 0xff000) as u64);
                self.print_inst_j("jal", rd, imm);
                self.regs[rd] = self.pc.wrapping_add(4);
                self.pc = self.pc.wrapping_add(imm).wrapping_sub(4); // subtract 4 because 4 will be added
                self.mark_as_dest(rd);
                Ok(())
            }
            0x67 => {
                match funct3 {
                    0x0 => {
                        let imm = ((inst as i32 as i64) >> 20) as u64;
                        self.print_inst_i("jalr", rd, rs1, imm);
                        let return_addr = self.pc.wrapping_add(4);
                        let next_pc = self.regs[rs1].wrapping_add(imm).wrapping_sub(4);
                        // subtract 4 because 4 will be added
                        self.regs[rd] = return_addr;
                        self.pc = next_pc;
                    }
                    _ => {
                        info!("This should not be reached!");
                        info!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(Exception::IllegalInstruction(inst));
                    }
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            0x1b => {
                match (funct3, funct7) {
                    (0x0, _) => {
                        // addiw
                        // I-type format
                        let imm = (inst as i32) >> 20;
                        self.print_inst_i("addiw", rd, rs1, imm as u32 as u64);
                        let src = self.regs[rs1] as i32;
                        let val = src.wrapping_add(imm);
                        self.regs[rd] = val as i64 as u64;
                    }
                    (0x1, 0x0) => {
                        // slliw
                        // I-type format
                        let shamt = ((inst as u32) >> 20) & 0x1f;
                        self.print_inst_i("slliw", rd, rs1, shamt as u64);
                        let src = self.regs[rs1] as u32;
                        let val = src << shamt;
                        self.regs[rd] = val as i32 as i64 as u64;
                    }
                    (0x5, 0x0) => {
                        // srliw
                        // I-type format
                        let shamt = ((inst as u32) >> 20) & 0x1f;
                        self.print_inst_i("srliw", rd, rs1, shamt as u64);
                        let src = self.regs[rs1] as u32;
                        let val = src >> shamt;
                        self.regs[rd] = val as i32 as i64 as u64;
                    }
                    (0x5, 0x20) => {
                        // sraiw
                        // I-type format
                        let shamt = ((inst as u32) >> 20) & 0x1f;
                        self.print_inst_i("sraiw", rd, rs1, shamt as u64);
                        let src = self.regs[rs1] as i32;
                        let val = src >> shamt;
                        self.regs[rd] = val as i64 as u64;
                    }
                    _ => {
                        info!("This should not be reached!");
                        info!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(Exception::IllegalInstruction(inst));
                    }
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            0x63 => {
                // branch instructions
                let imm = ((inst & 0x80000000) as i32 as i64 >> 19) as u64
                    | ((inst & 0x7e000000) as u64) >> 20
                    | ((inst & 0xf00) as u64) >> 7
                    | ((inst & 0x80) as u64) << 4;
                match funct3 {
                    0x0 => {
                        self.print_inst_b("beq", rs1, rs2, imm);
                        if self.regs[rs1] == self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x1 => {
                        self.print_inst_b("bne", rs1, rs2, imm);
                        if self.regs[rs1] != self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x4 => {
                        self.print_inst_b("blt", rs1, rs2, imm);
                        if (self.regs[rs1] as i64) < (self.regs[rs2] as i64) {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x5 => {
                        self.print_inst_b("bge", rs1, rs2, imm);
                        if (self.regs[rs1] as i64) >= (self.regs[rs2] as i64) {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x6 => {
                        self.print_inst_b("bltu", rs1, rs2, imm);
                        if self.regs[rs1] < self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x7 => {
                        self.print_inst_b("bgeu", rs1, rs2, imm);
                        if self.regs[rs1] >= self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    _ => {
                        info!("This should not be reached!");
                        info!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(Exception::IllegalInstruction(inst));
                    }
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            0x3b => {
                match (funct3, funct7) {
                    (0x0, 0x0) => {
                        self.print_inst_r("addw", rd, rs1, rs2);
                        let add_val = (self.regs[rs1] as i32).wrapping_add(self.regs[rs2] as i32);
                        self.regs[rd] = add_val as i64 as u64;
                    }
                    (0x0, 0x20) => {
                        self.print_inst_r("subw", rd, rs1, rs2);
                        let add_val = (self.regs[rs1] as i32).wrapping_sub(self.regs[rs2] as i32);
                        self.regs[rd] = add_val as i64 as u64;
                    }
                    (0x1, 0x0) => {
                        self.print_inst_r("sllw", rd, rs1, rs2);
                        let shamt = (self.regs[rs2] as u64) & 0x1f;
                        self.regs[rd] = ((self.regs[rs1] as u32) << shamt) as u64;
                    }
                    (0x5, 0x0) => {
                        self.print_inst_r("srlw", rd, rs1, rs2);
                        let shamt = (self.regs[rs2] as u64) & 0x1f;
                        self.regs[rd] = ((self.regs[rs1] as u32) >> shamt) as u64;
                    }
                    (0x5, 0x20) => {
                        self.print_inst_r("sraw", rd, rs1, rs2);
                        let shamt = (self.regs[rs2] as u64) & 0x1f;
                        self.regs[rd] = ((self.regs[rs1] as i32) >> shamt) as i64 as u64;
                    }
                    (0x0, 0x1) => {
                        self.print_inst_r("mulw", rd, rs1, rs2);
                        let mul = (self.regs[rs2] as u32) * (self.regs[rs2] as u32);
                        self.regs[rd] = mul as i32 as i64 as u64;
                    }
                    (0x4, 0x1) => {
                        self.print_inst_r("divw", rd, rs1, rs2);
                        let rem = (self.regs[rs2] as u32) / (self.regs[rs2] as u32);
                        self.regs[rd] = rem as u64;
                    }
                    (0x5, 0x1) => {
                        self.print_inst_r("divuw", rd, rs1, rs2);
                        let rem = (self.regs[rs2] as i32) / (self.regs[rs2] as i32);
                        self.regs[rd] = rem as i64 as u64;
                    }
                    (0x6, 0x1) => {
                        self.print_inst_r("remw", rd, rs1, rs2);
                        let rem = (self.regs[rs2] as i32) % (self.regs[rs2] as i32);
                        self.regs[rd] = rem as i64 as u64;
                    }
                    (0x7, 0x1) => {
                        self.print_inst_r("remuw", rd, rs1, rs2);
                        let rem = (self.regs[rs2] as u32) % (self.regs[rs2] as u32);
                        self.regs[rd] = rem as u64;
                    }
                    _ => {
                        info!("This should not be reached!");
                        return Err(Exception::IllegalInstruction(inst));
                    }
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            0x37 => {
                let imm = (inst & 0xfffff000) as i32 as i64 as u64;
                self.print_inst_j("lui", rd, imm);
                self.regs[rd] = imm;
                self.mark_as_dest(rd);
                Ok(())
            }
            0x17 => {
                let imm = inst & 0xfffff000;
                self.print_inst_j("auipc", rd, imm as u64);
                self.regs[rd] = imm.wrapping_add(self.pc as u32) as u64;
                self.mark_as_dest(rd);
                Ok(())
            }
            0x73 => {
                let csr = ((inst as u32) >> 20) as usize;
                let uimm = ((inst & 0xf8000) as u32) >> 15;
                let imm = (inst as i32 as i64 >> 20) as u64;
                match (funct3, funct7, rs2) {
                    (0x0, 0x0, 0x0) => {
                        self.print_inst_i("ecall", rd, rs1, imm);
                        Exception::EnvironmentalCallFromMMode.take_trap(self);
                    }
                    (0x0, 0x0, 0x1) => {
                        self.print_inst_i("ebreak", rd, rs1, imm);
                    }
                    (0x0, 0x8, 0x2) => {
                        self.print_inst_i("sret", rd, rs1, imm);
                        self.return_from_trap();
                    }
                    (0x0, 0x8, 0x5) => {
                        self.print_inst_i("wfi", rd, rs1, imm);
                        self.wait_for_interrupt();
                    }                    
                    (0x0, 0x18, 0x2) => {
                        self.print_inst_i("mret", rd, rs1, imm);
                        self.return_from_trap();
                    }
                    (0x1, _, _) => {
                        self.print_inst_csr("csrrw", rd, rs1, csr as u64);
                        if rd != 0 {
                            self.regs[rd] = self.csr.load_csrs(csr) as u64;
                        }
                        self.csr.store_csrs(csr, self.regs[rs1]);
                    }
                    (0x2, _, _) => {
                        self.print_inst_csr("csrrs", rd, rs1, csr as u64);
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, self.regs[rs1] | old_val);
                        }
                    }
                    (0x3, _, _) => {
                        self.print_inst_csr("csrrc", rd, rs1, csr as u64);
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, self.regs[rs1] & !old_val);
                        }
                    }
                    (0x5, _, _) => {
                        self.print_inst_csri("csrrwi", rd, csr as u64, uimm as u64);
                        if rd != 0 {
                            self.regs[rd] = self.csr.load_csrs(csr);
                        }
                        self.csr.store_csrs(csr, uimm as u64);
                    }
                    (0x6, _, _) => {
                        self.print_inst_csri("csrrsi", rd, csr as u64, uimm as u64);
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, uimm as u64 | old_val);
                        }
                    }
                    (0x7, _, _) => {
                        self.print_inst_csri("csrrci", rd, csr as u64, uimm as u64);
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, uimm as u64 & !old_val);
                        }
                    }
                    (0x0, 0x9, _) => {
                        self.print_inst_r("sfence.vma", rd, rs1, rs2);
                    }
                    (_, _, _) => {
                        info!("Unsupported CSR instruction!");
                        info!(
                            "pc = 0x{:x}, funct3:{}, funct7:{}",
                            self.pc, funct3, funct7
                        );
                        return Err(Exception::IllegalInstruction(inst));
                    }
                }
                Ok(())
            }
            0x0f => {
                self.inst_string = format!("pc=0x{:x}\nfence(do nothing)\n", self.pc);
                Ok(())
            }
            0x2f => {
                // Atomic Operation instructions
                let funct5 = funct7 >> 2;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                match (funct3, funct5) {
                    (0x2, 0x1) => {
                        self.print_inst_r("amoswap.w", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: swap
                        self.regs[rs2] = loaded_value;
                        let result = src_value;
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x0, 0x1) => {
                        self.print_inst_r("amoadd.w", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: add
                        let result = loaded_value.wrapping_add(src_value);
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x4, 0x1) => {
                        self.print_inst_r("amoxor.w", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: xor
                        let result = loaded_value ^ src_value;
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0xc, 0x1) => {
                        self.print_inst_r("amoand.w", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: and
                        let result = loaded_value & src_value;
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x8, 0x1) => {
                        self.print_inst_r("amoor.w", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: or
                        let result = loaded_value | src_value;
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x10, 0x1) => {
                        self.print_inst_r("amomin.w", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: singed min
                        let result = cmp::min(loaded_value as i64, src_value as i64) as u64;
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x14, 0x1) => {
                        self.print_inst_r("amomax.w", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: signed max
                        let result = cmp::max(loaded_value as i64, src_value as i64) as u64;
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x18, 0x1) => {
                        self.print_inst_r("amominu.w", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: unsigned min
                        let result = cmp::min(loaded_value, src_value);
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x1c, 0x1) => {
                        self.print_inst_r("amomaxu.w", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: unsigned max
                        let result = cmp::max(loaded_value, src_value);
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x2, 0x3) => {
                        self.print_inst_r("amoswap.d", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: swap
                        self.regs[rs2] = loaded_value;
                        let result = src_value;
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0x0, 0x3) => {
                        self.print_inst_r("amoadd.d", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: add
                        let result = loaded_value.wrapping_add(src_value);
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0x4, 0x3) => {
                        self.print_inst_r("amoxor.d", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: xor
                        let result = loaded_value ^ src_value;
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0xc, 0x3) => {
                        self.print_inst_r("amoand.d", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: and
                        let result = loaded_value & src_value;
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0x8, 0x3) => {
                        self.print_inst_r("amoor.d", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: or
                        let result = loaded_value | src_value;
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0x10, 0x3) => {
                        self.print_inst_r("amomin.d", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: signed min
                        let result = cmp::min(loaded_value as i64, src_value as i64) as u64;
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0x14, 0x3) => {
                        self.print_inst_r("amomax.d", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: signed max
                        let result = cmp::max(loaded_value as i64, src_value as i64) as u64;
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0x18, 0x3) => {
                        self.print_inst_r("amominu.d", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: unsigned min
                        let result = cmp::min(loaded_value, src_value);
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0x1c, 0x3) => {
                        self.print_inst_r("amomaxu.d", rd, rs1, rs2);
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: unsigned max
                        let result = cmp::max(loaded_value, src_value);
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    _ => {
                        return Err(Exception::IllegalInstruction(inst));
                    }
                }
                Ok(())
            }
            _ => {
                info!("not implemented yet!");
                info!("pc=0x{:x}", self.pc);
                info!("inst:{inst:b}");
                return Err(Exception::IllegalInstruction(inst));
            }
        }
    }

    pub fn dump_registers(&mut self) {
        let abi = [
            "zero", " ra ", " sp ", " gp ", " tp ", " t0 ", " t1 ", " t2 ", " s0 ", " s1 ", " a0 ",
            " a1 ", " a2 ", " a3 ", " a4 ", " a5 ", " a6 ", " a7 ", " s2 ", " s3 ", " s4 ", " s5 ",
            " s6 ", " s7 ", " s8 ", " s9 ", " s10", " s11", " t3 ", " t4 ", " t5 ", " t6 ",
        ];
        let mut output = format!("pc={:>#18x}\n{}", self.pc, self.inst_string);
        const SEQ_RED: &str = "\x1b[91m";
        const SEQ_GREEN: &str = "\x1b[92m";
        const SEQ_CLEAR: &str = "\x1b[0m";
        for i in 0..32 {
            output = format!(
                "{}{}",
                output,
                format!(
                    "{}x{:02}({})={:>#18x}{}{}",
                    if i == self.dest {
                        SEQ_RED
                    } else if (i == self.src1) || (i == self.src2) {
                        SEQ_GREEN
                    } else {
                        ""
                    },
                    i,
                    abi[i],
                    self.regs[i],
                    if (i == self.dest) || (i == self.src1) || (i == self.src2) {
                        SEQ_CLEAR
                    } else {
                        ""
                    },
                    if i % 4 == 3 { "\n" } else { ", " }
                )
            )
        }
        info!("{}", output);
        info!("----\n");
    }

    pub fn step_run(&mut self) -> u64 {

        // recieve all the interrupt messages
        while let Some(interrupt) = self.interrupt_receiver.try_recv().ok() {
            info!("Interrupt: {:?} received", interrupt);
            self.set_pending_interrupt(interrupt);
        }

        if let Some(mut interrupt) = self.get_interrupt_to_take() {
            info!("Interrupt: {:?} taken", interrupt);
            interrupt.take_trap(self);
        }

        let inst = match self.fetch() {
            Ok(inst) => inst,
            Err(_) => return 0x0,
        };

        self.execute(inst as u32)
            .map_err(|mut e| e.take_trap(self))
            .expect("Execution failed!\n");
        self.regs[0] = 0;

        self.pc = self.pc.wrapping_add(4);

        if self.pc == 0 {
            self.dump_registers();
            info!("Program finished!\n");
        }
        self.pc
    }
}