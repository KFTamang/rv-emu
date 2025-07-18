use crate::bus::*;
use crate::clint::*;
use crate::csr::*;
use crate::dram::*;
use crate::interrupt::*;
use crate::instruction::*;

use log::{debug, error, info, trace};

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::cmp;
use std::rc::Rc;
use std::collections::BTreeSet;

const REG_NUM: usize = 32;
pub const M_MODE: u64 = 0b11;
pub const S_MODE: u64 = 0b01;
pub const U_MODE: u64 = 0b00;

pub const CPU_FREQUENCY: u64 = 10_000_000; // 10MHz

#[derive(PartialEq)]
enum AccessMode {
    Load,
    Store,
}

fn bit(integer: u64, bit: u64) -> u64 {
    (integer >> bit) & 0x1
}

#[derive(Serialize, Deserialize)]
pub struct CpuSnapshot {
    pub regs: [u64; 32],
    pub pc: u64,
    pub bus: BusSnapshot,
    pub csr: CsrSnapshot,
    pub mode: u64,
    pub cycle: u64,
    pub clint: Clint,
    pub interrupt_list: BTreeSet<Interrupt>,
    pub address_translation_cache: std::collections::HashMap<u64, u64>,
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
    dump_interval: u64,
    inst_string: String,
    pub cycle: Rc<RefCell<u64>>,
    clint: Clint,
    interrupt_list: Rc<RefCell<BTreeSet<Interrupt>>>,
    address_translation_cache: std::collections::HashMap<u64, u64>,
}

impl Cpu {
    pub fn new(binary: Vec<u8>, base_addr: u64, _dump_count: u64) -> Self {
        let mut regs = [0; 32];
        regs[2] = DRAM_SIZE;
        let interrupt_list = Rc::new(RefCell::new(BTreeSet::<Interrupt>::new()));
        let bus = Bus::new(binary, base_addr, interrupt_list.clone());
        let cycle = Rc::new(RefCell::new(0u64));
        let csr = Csr::new(interrupt_list.clone(), cycle.clone());
        Self {
            regs,
            pc: base_addr,
            bus,
            csr,
            dest: REG_NUM,
            src1: REG_NUM,
            src2: REG_NUM,
            mode: M_MODE,
            dump_count: _dump_count,
            dump_interval: _dump_count,
            inst_string: String::from(""),
        clint: Clint::new(0x200_0000, 0x10000),
            cycle,
            interrupt_list,
            address_translation_cache: std::collections::HashMap::new(),
        }
    }

    pub fn to_snapshot(&self) -> CpuSnapshot {
        CpuSnapshot {
            regs: self.regs,
            pc: self.pc,
            bus: self.bus.to_snapshot(),
            csr: self.csr.to_snapshot(),
            mode: self.mode,
            cycle: *self.cycle.borrow(),
            clint: self.clint.clone(),
            interrupt_list: self.interrupt_list.borrow().clone(),
            address_translation_cache: self.address_translation_cache.clone(),
        }
    }

    pub fn from_snapshot(snapshot: CpuSnapshot) -> Self {
        let interrupt_list = Rc::new(RefCell::new(snapshot.interrupt_list));
        let cycle = Rc::new(RefCell::new(snapshot.cycle));
        let mut cpu = Self {
            regs: snapshot.regs,
            pc: snapshot.pc,
            bus: Bus::from_snapshot(snapshot.bus, interrupt_list.clone()),
            csr: Csr::from_snapshot(snapshot.csr, interrupt_list.clone(), cycle.clone()),
            dest: REG_NUM,
            src1: REG_NUM,
            src2: REG_NUM,
            mode: snapshot.mode,
            dump_count: 0,
            dump_interval: 0,
            inst_string: String::from(""),
            clint: snapshot.clint,
            cycle,
            interrupt_list: interrupt_list,
            address_translation_cache: snapshot.address_translation_cache,
        };
        cpu.clear_reg_marks();
        cpu
    }

    pub fn fetch(&mut self) -> Result<u32, ()> {
        let index = self.pc as usize;
        match self.load(index as u64, 32) {
            Ok(inst) => Ok(inst as u32),
            Err(_) => Err(()),
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
        trace!("Load access to 0x{:x}", va);
        match self.translate(va, AccessMode::Load) {
            Ok(pa) => {
                if self.clint.is_accessible(pa) {
                    self.clint.load(pa, size)
                } else {
                    self.bus.load(pa, size)
                }
            }
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
            }
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
        // store address translation cache by a unit of 4kB
        let va_cache_entry = va >> 12; // 4kB aligned
        if self.address_translation_cache.contains_key(&va_cache_entry) {
            let pa_base = *self.address_translation_cache.get(&va_cache_entry).unwrap() << 12;
            return Ok(pa_base | (va & 0xfff));
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
                    return match acc_mode {
                        AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
                        AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
                    };
                }
                if (r == 1) || (x == 1) {
                    break;
                }
                ppn = (pte >> 10) & 0xfff_ffff_ffff;
                i = i - 1;
                if i < 0 {
                    return match acc_mode {
                        AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
                        AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
                    };
                }
            } else {
                return Err(Exception::LoadPageFault(va as u32));
            }
        }
        let a = bit(pte, 6);
        let d = bit(pte, 7);
        if (a == 0) || ((d == 0) && (acc_mode == AccessMode::Store)) {
            self.bus.store(pt_addr, 64, pte | (1 << 6))?;
        }
        let pa = match i {
            0 => ((pte << 2) & 0xfffffffffff000) | (va & 0x00000fff),
            1 => ((pte << 2) & 0xffffffffe00000) | (va & 0x001fffff),
            2 => ((pte << 2) & 0xffffffc0000000) | (va & 0x3fffffff),
            _ => panic!("something goes wrong at MMU! va: 0x{:x}, Level: {}", va, i),
        };
        self.address_translation_cache.insert(va >> 12, pa >> 12); // store 4kB aligned
        Ok(pa)
    }

    fn wait_for_interrupt(&mut self) {
        // wait for a message that notifies an interrupt on the interrupt channel
        trace!("waiting for interrupt");
        trace!("registers dump:");
        trace!("{}", self.dump_registers());
        trace!("CSR dump:");
        trace!("{}", self.csr.dump());

        // loop {
        //     // check for interrupts
        //     self.bus.plic.process_pending_interrupts();

        //     // check and pend all the delayed interrupts
        //     self.update_pending_interrupts();

        //     if let Some(mut interrupt) = self.get_interrupt_to_take() {
        //         info!("wake up from waiting for interrupt");
        //         debug!("Interrupt: {:?} taken", interrupt);
        //         debug!("{}", self.csr.dump());
        //         interrupt.take_trap(self);
        //     }

        //     // sleep for a while to avoid busy waiting
        //     std::thread::sleep(std::time::Duration::from_millis(10));
        // }
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

        debug!("Interrupt {:?} is set", interrupt);
        debug!("xIP: {:0b}", new_xip);
        debug!("xIE: {:0b}", self.csr.load_csrs(MIE));
        debug!("{}", self.csr.dump());
    }

    // get the takable pending interrupt with the highest priority
    pub fn get_interrupt_to_take(&mut self) -> Option<Interrupt> {
        // An interrupt i will be taken
        // (a)if bit i is set in both mip and mie,
        // (b)and if interrupts are globally enabled.
        // By default, M-mode interrupts are globally enabled
        // (b-1)if the hart’s current privilege mode is less than M,
        // (b-2)or if the current privilege mode is M and the MIE bit in the mstatus register is set.
        // (c)If bit i in mideleg is set, however, interrupts are considered to be globally enabled
        // if the hart’s current privilege mode equals the delegated privilege mode and that mode’s interrupt enable bit (xIE in mstatus for mode x) is set,
        // or if the current privilege mode is less than the delegated privilege mode.

        // early return if no interrupt is set
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
        if xip & xie == 0 {
            return None;
        }

        let interrupt_list = self.interrupt_list.borrow(); 
        for interrupt in interrupt_list.iter() {
            if let Ok(destined_mode) = interrupt.get_trap_mode(self) {
                info!(
                    "interrupt: {:?}, destined mode: {}, current mode: {}",
                    interrupt, destined_mode, self.mode
                );
                if destined_mode >= self.mode {
                    return Some(*interrupt);
                }
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
        debug!("{}", self.csr.dump());
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
                debug!("back to privilege {} from machine mode", pp);
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
                debug!("back to privilege {} from supervisor mode", pp);
            }
            _ => {
                panic!("m/sret from U_MODE\n");
            }
        }
        debug!("return from trap");
        debug!("csr dump");
        debug!("{}", self.csr.dump());
    }

    pub fn execute(&mut self, inst: DecodedInstr) -> Result<(), Exception> {
        // let opcode = inst & 0x7f;
        // let rd = ((inst >> 7) & 0x1f) as usize;
        // let rs1 = ((inst >> 15) & 0x1f) as usize;
        // let rs2 = ((inst >> 20) & 0x1f) as usize;
        // let funct3 = ((inst >> 12) & 0x7) as usize;
        // let funct7 = ((inst >> 25) & 0x7f) as usize;

        self.clear_reg_marks();
        match inst {
            DecodedInstr::Add{rd, rs1, rs2} => {
                // "add"
                self.regs[rd] = self.regs[rs1].wrapping_add(self.regs[rs2]);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sub{rd, rs1, rs2} => {
                // "sub"
                self.regs[rd] = self.regs[rs1].wrapping_sub(self.regs[rs2]);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sll{rd, rs1, rs2} => {
                // "sll"
                let shamt = self.regs[rs2] & 0x1f;
                self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Slt{rd, rs1, rs2} => {
                // "slt"
                self.regs[rd] = if (rs1 as i64) < (rs2 as i64) { 1 } else { 0 };
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sltu{rd, rs1, rs2} => {
                // "sltu"
                self.regs[rd] = if (rs1 as u64) < (rs2 as u64) { 1 } else { 0 };
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Xor{rd, rs1, rs2} => {
                // "xor"
                self.regs[rd] = self.regs[rs1] ^ self.regs[rs2];
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Srl{rd, rs1, rs2} => {
                // "srl"
                let shamt = self.regs[rs2] & 0x1f;
                self.regs[rd] = self.regs[rs1] as u64 >> shamt;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sra{rd, rs1, rs2} => {
                // "sra"
                let shamt = self.regs[rs2] & 0x1f;
                self.regs[rd] = (self.regs[rs1] as i64 as u64) >> shamt;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Or{rd, rs1, rs2} => {
                // "or"
                self.regs[rd] = self.regs[rs1] | self.regs[rs2];
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::And{rd, rs1, rs2} => {
                // "and"
                self.regs[rd] = self.regs[rs1] & self.regs[rs2];
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Mul{rd, rs1, rs2} => {
                // "mul"
                self.regs[rd] = self.regs[rs1].wrapping_mul(self.regs[rs2]);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Mulh{rd, rs1, rs2} => {
                // "mulh"
                let mul = (self.regs[rs1] as i64 as i128)
                    .wrapping_mul(self.regs[rs2] as i64 as i128);
                self.regs[rd] = (mul >> 64) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Mulhsu{rd, rs1, rs2} => {
                // "mulhsu"
                let mul = (self.regs[rs1] as i64 as i128)
                    .wrapping_mul(self.regs[rs2] as u128 as i128);
                self.regs[rd] = (mul >> 64) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Mulhu{rd, rs1, rs2} => {
                // "mulhu"
                let mul = (self.regs[rs1] as u128).wrapping_mul(self.regs[rs2] as u128);
                self.regs[rd] = (mul >> 64) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Div{rd, rs1, rs2} => {
                // "div"
                self.regs[rd] = self.regs[rs1] / self.regs[rs2];
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Divu{rd, rs1, rs2} => {
                // "divu"
                self.regs[rd] = ((self.regs[rs1] as i64) / (self.regs[rs2] as i64)) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Rem{rd, rs1, rs2} => {
                // "rem"
                self.regs[rd] = self.regs[rs1] % self.regs[rs2];
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Remu{rd, rs1, rs2} => {
                // "remu"
                self.regs[rd] = ((self.regs[rs1] as i64) % (self.regs[rs2] as i64)) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Addi{rd, rs1, imm} => {
                // "addi"
                self.regs[rd] = self.regs[rs1].wrapping_add(imm as u64);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Slti{rd, rs1, imm} => {
                // "slti"
                let result = if (self.regs[rs1] as i32 as i64) < (imm as i64) {
                    1
                } else {
                    0
                };
                self.regs[rd] = result;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Sltiu{rd, rs1, imm} => {
                // "sltiu"
                let result = if (self.regs[rs1] as i32 as u32) < imm {
                    1
                } else {
                    0
                };
                self.regs[rd] = result;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Xori{rd, rs1, imm} => {
                // "xori"
                let val = ((self.regs[rs1] as i32) ^ (imm as i32)) as u64;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Ori{rd, rs1, imm} => {
                // "ori"
                let val = ((self.regs[rs1] as i32) | (imm as i32)) as u64;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Andi{rd, rs1, imm} => {
                // "andi"
                let val = ((self.regs[rs1] as i32) & (imm as i32)) as u64;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Slli{rd, rs1, imm} => {
                // "slli"
                let shamt = (imm & 0x3f) as u64;
                self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Srli{rd, rs1, imm} => {
                // "srli/
                let shamt = (imm & 0x3f) as u64;
                let logical_shift = imm >> 5;
                if logical_shift == 0 {
                    self.regs[rd] = (self.regs[rs1] as u64) >> shamt;
                } else {
                    self.regs[rd] = ((self.regs[rs1] as i64) >> shamt) as u64;
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lb {rd, rs1, imm} => {
                // "lb"
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(addr, 8)?;
                self.regs[rd] = val as i8 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lh {rd, rs1, imm} => {
                // "lh"
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(addr, 16)?;
                self.regs[rd] = val as i16 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lw {rd, rs1, imm} => {
                // "lw"
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(addr, 32)?;
                self.regs[rd] = val as i32 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Ld {rd, rs1, imm} => {
                // "ld"
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(addr, 64)?;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lbu {rd, rs1, imm} => {
                // "lbu"
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(addr, 8)?;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lhu {rd, rs1, imm} => {
                // "lhu"
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(addr, 16)?;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lwu {rd, rs1, imm} => {
                // "lwu"
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(addr, 32)?;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Sb { rd, rs1, rs2, imm } => {
                // store instructions
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                self.store(addr, 8, self.regs[rs2])?;
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sh { rd, rs1, rs2, imm } => {
                // store instructions
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                self.store(addr, 16, self.regs[rs2])?;
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sw { rd, rs1, rs2, imm } => {
                // store instructions
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                self.store(addr, 32, self.regs[rs2])?;
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sd { rd, rs1, rs2, imm } => {
                // store instructions
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                self.store(addr, 64, self.regs[rs2])?;
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Jal { rd, imm } => {
                // jal
                self.regs[rd] = self.pc.wrapping_add(4);
                self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4); // subtract 4 because 4 will be added
                self.mark_as_dest(rd);
                Ok(())
            }
            DecodedInstr::Jalr {rd, rs1, imm} => {
                // "jalr"
                let return_addr = self.pc.wrapping_add(4);
                let next_pc = self.regs[rs1].wrapping_add(imm as u64).wrapping_sub(4);
                // subtract 4 because 4 will be added
                self.regs[rd] = return_addr;
                self.pc = next_pc;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Addiw {rd, rs1, imm } => {
                // addiw
                let src = self.regs[rs1] as i32;
                let val = src.wrapping_add(imm as i32);
                self.regs[rd] = val as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Slliw {rd, rs1, imm } => {
                // slliw
                let src = self.regs[rs1] as u32;
                let val = src << imm;
                self.regs[rd] = val as i32 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Srliw {rd, rs1, imm } => {
                // srliw
                let src = self.regs[rs1] as u32;
                let val = src >> imm;
                self.regs[rd] = val as i32 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Sraiw {rd, rs1, imm } => {
                // sraiw
                let src = self.regs[rs1] as i32;
                let val = src >> imm;
                self.regs[rd] = val as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Beq {rd, rs1, rs2, imm } => {
                // "beq"
                if self.regs[rs1] == self.regs[rs2] {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Bne {rd, rs1, rs2, imm } => {
                // "bne"
                if self.regs[rs1] != self.regs[rs2] {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Blt {rd, rs1, rs2, imm } => {
                // "blt"
                if (self.regs[rs1] as i64) < (self.regs[rs2] as i64) {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Bge {rd, rs1, rs2, imm } => {
                // "bge"
                if (self.regs[rs1] as i64) >= (self.regs[rs2] as i64) {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Bltu {rd, rs1, rs2, imm } => {
                // "bltu"
                if self.regs[rs1] < self.regs[rs2] {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Bgeu {rd, rs1, rs2, imm } => {
                // "bgeu"
                if self.regs[rs1] >= self.regs[rs2] {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Addw {rd, rs1, rs2} => {
                // "addw"
                let add_val = (self.regs[rs1] as i32).wrapping_add(self.regs[rs2] as i32);
                self.regs[rd] = add_val as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Subw {rd, rs1, rs2}  => {
                // "subw"
                let add_val = (self.regs[rs1] as i32).wrapping_sub(self.regs[rs2] as i32);
                self.regs[rd] = add_val as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sllw {rd, rs1, rs2} => {
                // "sllw"
                let shamt = (self.regs[rs2] as u64) & 0x1f;
                self.regs[rd] = ((self.regs[rs1] as u32) << shamt) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Srlw {rd, rs1, rs2} => {
                // "srlw"
                let shamt = (self.regs[rs2] as u64) & 0x1f;
                self.regs[rd] = ((self.regs[rs1] as u32) >> shamt) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sraw {rd, rs1, rs2}  => {
                // "sraw"
                let shamt = (self.regs[rs2] as u64) & 0x1f;
                self.regs[rd] = ((self.regs[rs1] as i32) >> shamt) as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Mulw {rd, rs1, rs2} => {
                // "mulw"
                let mul = (self.regs[rs2] as u32) * (self.regs[rs2] as u32);
                self.regs[rd] = mul as i32 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Divw {rd, rs1, rs2} => {
                // "divw"
                let rem = (self.regs[rs2] as u32) / (self.regs[rs2] as u32);
                self.regs[rd] = rem as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Divuw {rd, rs1, rs2} => {
                // "divuw"
                let rem = (self.regs[rs2] as i32) / (self.regs[rs2] as i32);
                self.regs[rd] = rem as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Remw {rd, rs1, rs2} => {
                // "remw"
                let rem = (self.regs[rs2] as i32) % (self.regs[rs2] as i32);
                self.regs[rd] = rem as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Remuw {rd, rs1, rs2} => {
                // "remuw"
                let rem = (self.regs[rs2] as u32) % (self.regs[rs2] as u32);
                self.regs[rd] = rem as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Lui { rd, imm } => {
                // "lui"
                self.regs[rd] = imm;
                self.mark_as_dest(rd);
                Ok(())
            }
            DecodedInstr::Auipc { rd, imm } => {
                // "auipc"
                self.regs[rd] = imm.wrapping_add(self.pc as u32) as u64;
                self.mark_as_dest(rd);
                Ok(())
            }
            DecodedInstr::Ecall => {
                Exception::EnvironmentalCallFromMMode.take_trap(self);
            }
            DecodedInstr::Ebreak => {
                // Optional: implement EBREAK behavior
                Ok(())
            }
            DecodedInstr::Sret | DecodedInstr::Mret => {
                self.return_from_trap();
                Ok(())
            }
            DecodedInstr::Wfi => {
                self.wait_for_interrupt();
                Ok(())
            }
            DecodedInstr::Csrrw { rd, rs1, imm } => {
                if rd != 0 {
                    self.regs[rd] = self.csr.load_csrs(imm) as u64;
                }
                self.csr.store_csrs(imm, self.regs[rs1]);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(()) 
            }
            DecodedInstr::Csrrs { rd, rs1, imm } => {
                let old = self.csr.load_csrs(imm) as u64;
                self.regs[rd] = old;
                if rs1 != 0 {
                    self.csr.store_csrs(imm, self.regs[rs1] | old);
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Csrrc { rd, rs1, imm } => {
                let old = self.csr.load_csrs(imm) as u64;
                self.regs[rd] = old;
                if rs1 != 0 {
                    self.csr.store_csrs(imm, self.regs[rs1] & !old);
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(()) 
            }
            DecodedInstr::Csrrwi { rd, rs1, imm } => {
                if rd != 0 {
                    self.regs[rd] = self.csr.load_csrs(imm);
                }
                self.csr.store_csrs(imm, rs1 as u64);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(()) 
            }
            DecodedInstr::Csrrsi { rd, rs1, imm } => {
                let old = self.csr.load_csrs(imm) as u64;
                self.regs[rd] = old;
                if rs1 != 0 {
                    self.csr.store_csrs(imm, rs1 as u64 | old);
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(()) 
            }
            DecodedInstr::Csrrci { rd, rs1, imm } => {
                let old = self.csr.load_csrs(imm) as u64;
                self.regs[rd] = old;
                if rs1 != 0 {
                    self.csr.store_csrs(imm, rs1 as u64 & !old);
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(()) 
            }
            DecodedInstr::Fence => {
                self.address_translation_cache.clear();
                Ok(())
            }
            DecodedInstr::Fence => {
                // 実際には no-op（または memory ordering のために記録する）
                Ok(())
            }
            DecodedInstr::Amoswap { rd, rs1, rs2 } => {
                let addr = self.regs[rs1];
                let val = self.load(addr, 32)?;              // メモリからロード
                let src = self.regs[rs2];
                self.regs[rd] = val;                             // rd に old val
                self.regs[rs2] = val;                            // swap
                self.store(addr, 32, src)?;                   // 書き戻し
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(()) 
            }
            DecodedInstr::Amoadd { rd, rs1, rs2 } => {
                let addr = self.regs[rs1];
                let val = self.load(addr, 32)?;
                let result = val.wrapping_add(self.regs[rs2]);
                self.regs[rd] = val;
                self.store(addr, 32, result)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(()) 
            }
            DecodedInstr::Amoxor { rd, rs1, rs2 } => {
                let addr = self.regs[rs1];
                let val = self.load(addr, 32)?;
                let result = val ^ self.regs[rs2];
                self.regs[rd] = val;
                self.store(addr, 32, result)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(()) 
            }
            DecodedInstr::Amoand { rd, rs1, rs2 } => {
                let addr = self.regs[rs1];
                let val = self.load(addr, 32)?;
                let result = val & self.regs[rs2];
                self.regs[rd] = val;
                self.store(addr, 32, result)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(()) 
            }
            DecodedInstr::Amoor { rd, rs1, rs2 } => {
                let addr = self.regs[rs1];
                let val = self.load(addr, 32)?;
                let result = val | self.regs[rs2];
                self.regs[rd] = val;
                self.store(addr, 32, result)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(()) 
            }
            DecodedInstr::Amomin { rd, rs1, rs2 } => {
                // "amomin.
                let addr = self.regs[rs1];
                let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                let src_value = self.regs[rs2];
                // store loaded value to dest register
                self.regs[rd] = loaded_value;
                // binary operation: singed min
                let result = cmp::min(loaded_value as i64, src_value as i64) as u64;
                // store operation result
                self.store(addr, 32, result)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(()) 
            }
            DecodedInstr::Amomax { rd, rs1, rs2 } => {
                // "amomax.
                let addr = self.regs[rs1];
                let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                let src_value = self.regs[rs2];
                // store loaded value to dest register
                self.regs[rd] = loaded_value;
                // binary operation: signed max
                let result = cmp::max(loaded_value as i64, src_value as i64) as u64;
                // store operation result
                self.store(addr, 32, result)?;                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(()) 
            }
            DecodedInstr::Amominu { rd, rs1, rs2 } => {
                // "amominu.
                let addr = self.regs[rs1];
                let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                let src_value = self.regs[rs2];
                // store loaded value to dest register
                self.regs[rd] = loaded_value;
                // binary operation: unsigned min
                let result = cmp::min(loaded_value, src_value);
                // store operation result
                self.store(addr, 32, result)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(()) 
            }
            DecodedInstr::Amomaxu { rd, rs1, rs2 } => {
                // "amomaxu.
                let addr = self.regs[rs1];
                let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                let src_value = self.regs[rs2];
                // store loaded value to dest register
                self.regs[rd] = loaded_value;
                // binary operation: unsigned max
                let result = cmp::max(loaded_value, src_value);
                // store operation result
                self.store(addr, 32, result)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(()) 
            }
            DecodedInstr::IllegalInstruction => {
                return Err(Exception::IllegalInstruction(inst));
            }        
        }
    }

    pub fn dump_registers(&mut self) -> String {
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
        output
    }

    pub fn step_run(&mut self) -> u64 {
        trace!("pc={:>#18x}", self.pc);
        *self.cycle.borrow_mut() += 1;
        if *self.cycle.borrow() % 1000000 == 0 {
            debug!("Cycle: {}", self.cycle.borrow());
        }

        // check for interrupts
        self.bus.plic.process_pending_interrupts();

        // check and pend all the delayed interrupts
        self.update_pending_interrupts();

        if let Some(mut interrupt) = self.get_interrupt_to_take() {
            debug!("Interrupt: {:?} taken", interrupt);
            debug!("{}", self.csr.dump());
            interrupt.take_trap(self);
        }

        let inst = match self.fetch() {
            Ok(inst) => inst,
            Err(_) => return 0x0,
        };

        let decoded_inst =  DecodedInstr::decode(inst);

        let result = self.execute(decoded_inst).map_err(|mut e| e.take_trap(self));
        if let Err(e) = result {
            error!("Execution failed!");
            error!("Exception: {:?}", e);
            error!("pc=0x{:x}", self.pc);
            error!("inst:{:b}", inst);
            error!("{}", self.dump_registers());
            error!("{}", self.csr.dump());
        }
        self.regs[0] = 0;

        self.pc = self.pc.wrapping_add(4);

        if self.dump_count > 0 {
            self.dump_count -= 1;
            if self.dump_count == 0 {
                self.dump_count = self.dump_interval;
                info!("{}", self.dump_registers());
                debug!("CSR: {}", self.csr.dump());
            }
        }

        if self.pc == 0 {
            info!("{}", self.dump_registers());
            info!("Program finished!");
            std::process::exit(0);
        }
        self.pc
    }

    fn update_pending_interrupts(&mut self) {
        // Update Supervisor Timer Interrupt pending status
        // If the current time count is greater than STIMECMP, set the pending status
        // Otherwise, clear the pending status
        let stimecmp = self.csr.load_csrs(STIMECMP);
        let current_counter = *self.cycle.borrow() * TIMER_FREQ / CPU_FREQUENCY;
        if current_counter % 10000 == 0 {
            if current_counter % 1000000 == 0 {
                debug!(
                    "stimecmp: {}, current_counter: {}",
                    stimecmp, current_counter
                );
            }
            if (stimecmp > 0) && (current_counter >= stimecmp) {
                self.interrupt_list
                    .borrow_mut()
                    .insert(Interrupt::SupervisorTimerInterrupt);
            } else {
                self.interrupt_list
                    .borrow_mut()
                    .remove(&Interrupt::SupervisorTimerInterrupt);
            }
        }
    }
}
