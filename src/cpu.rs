use crate::bus::*;
use crate::clint::*;
use crate::csr::*;
use crate::dram::*;
use crate::interrupt::*;

use log::{debug, error, info, trace};

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::cmp;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

const REG_NUM: usize = 32;
pub const M_MODE: u64 = 0b11;
pub const S_MODE: u64 = 0b01;
pub const U_MODE: u64 = 0b00;

pub const CPU_FREQUENCY: u64 = 20_000_000; // 20MHz

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
    pub interrupt_list: Vec<DelayedInterrupt>,
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
    interrupt_list: Arc<Mutex<Vec<DelayedInterrupt>>>,
    address_translation_cache: std::collections::HashMap<u64, u64>,
}

impl Cpu {
    pub fn new(binary: Vec<u8>, base_addr: u64, _dump_count: u64) -> Self {
        let mut regs = [0; 32];
        regs[2] = DRAM_SIZE;
        let interrupt_list = Arc::new(Mutex::new(Vec::new()));
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
            interrupt_list: self.interrupt_list.lock().unwrap().to_vec(),
            address_translation_cache: self.address_translation_cache.clone(),
        }
    }

    pub fn from_snapshot(snapshot: CpuSnapshot) -> Self {
        let interrupt_list = Arc::new(Mutex::new(snapshot.interrupt_list));
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

    pub fn fetch(&mut self) -> Result<u64, ()> {
        let index = self.pc as usize;
        match self.load(index as u64, 32) {
            Ok(inst) => Ok(inst),
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
        debug!("waiting for interrupt");
        debug!("registers dump:");
        debug!("{}", self.dump_registers());
        debug!("CSR dump:");
        debug!("{}", self.csr.dump());

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

        for interrupt in Interrupt::PRIORITY_ORDER.iter() {
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
                        // "add"
                        self.regs[rd] = self.regs[rs1].wrapping_add(self.regs[rs2]);
                    }
                    (0x0, 0x20) => {
                        // "sub"
                        self.regs[rd] = self.regs[rs1].wrapping_sub(self.regs[rs2]);
                    }
                    (0x1, 0x0) => {
                        // "sll"
                        let shamt = self.regs[rs2] & 0x1f;
                        self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                    }
                    (0x2, 0x0) => {
                        // "slt"
                        self.regs[rd] = if (rs1 as i64) < (rs2 as i64) { 1 } else { 0 }
                    }
                    (0x3, 0x0) => {
                        // "sltu"
                        self.regs[rd] = if (rs1 as u64) < (rs2 as u64) { 1 } else { 0 }
                    }
                    (0x4, 0x0) => {
                        // "xor"
                        self.regs[rd] = self.regs[rs1] ^ self.regs[rs2];
                    }
                    (0x5, 0x0) => {
                        // "srl"
                        let shamt = self.regs[rs2] & 0x1f;
                        self.regs[rd] = self.regs[rs1] as u64 >> shamt;
                    }
                    (0x5, 0x20) => {
                        // "sra"
                        let shamt = self.regs[rs2] & 0x1f;
                        self.regs[rd] = (self.regs[rs1] as i64 as u64) >> shamt;
                    }
                    (0x6, 0x0) => {
                        // "or"
                        self.regs[rd] = self.regs[rs1] | self.regs[rs2];
                    }
                    (0x7, 0x0) => {
                        // "and"
                        self.regs[rd] = self.regs[rs1] & self.regs[rs2];
                    }
                    (0x0, 0x1) => {
                        // "mul"
                        self.regs[rd] = self.regs[rs1].wrapping_mul(self.regs[rs2]);
                    }
                    (0x1, 0x1) => {
                        // "mulh"
                        let mul = (self.regs[rs1] as i64 as i128)
                            .wrapping_mul(self.regs[rs2] as i64 as i128);
                        self.regs[rd] = (mul >> 64) as u64;
                    }
                    (0x2, 0x1) => {
                        // "mulhsu"
                        let mul = (self.regs[rs1] as i64 as i128)
                            .wrapping_mul(self.regs[rs2] as u128 as i128);
                        self.regs[rd] = (mul >> 64) as u64;
                    }
                    (0x3, 0x1) => {
                        // "mulhu"
                        let mul = (self.regs[rs1] as u128).wrapping_mul(self.regs[rs2] as u128);
                        self.regs[rd] = (mul >> 64) as u64;
                    }
                    (0x4, 0x1) => {
                        // "div"
                        self.regs[rd] = self.regs[rs1] / self.regs[rs2];
                    }
                    (0x5, 0x1) => {
                        // "divu"
                        self.regs[rd] = ((self.regs[rs1] as i64) / (self.regs[rs2] as i64)) as u64;
                    }
                    (0x6, 0x1) => {
                        // "rem"
                        self.regs[rd] = self.regs[rs1] % self.regs[rs2];
                    }
                    (0x7, 0x1) => {
                        // "remu"
                        self.regs[rd] = ((self.regs[rs1] as i64) % (self.regs[rs2] as i64)) as u64;
                    }
                    (_, _) => {
                        error!("This should not be reached!");
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
                        // "addi"
                        self.regs[rd] = self.regs[rs1].wrapping_add(imm);
                    }
                    0x2 => {
                        // "slti"
                        let result = if (self.regs[rs1] as i32 as i64) < (imm as i64) {
                            1
                        } else {
                            0
                        };
                        self.regs[rd] = result;
                    }
                    0x3 => {
                        // "sltiu"
                        let result = if (self.regs[rs1] as i32 as i64 as u64) < imm {
                            1
                        } else {
                            0
                        };
                        self.regs[rd] = result;
                    }
                    0x4 => {
                        // "xori"
                        let val = ((self.regs[rs1] as i32) ^ (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x6 => {
                        // "ori"
                        let val = ((self.regs[rs1] as i32) | (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x7 => {
                        // "andi"
                        let val = ((self.regs[rs1] as i32) & (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x1 => {
                        // "slli"
                        let shamt = (imm & 0x3f) as u64;
                        self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                    }
                    0x5 => {
                        // "srli/
                        let shamt = (imm & 0x3f) as u64;
                        let logical_shift = imm >> 5;
                        if logical_shift == 0 {
                            self.regs[rd] = (self.regs[rs1] as u64) >> shamt;
                        } else {
                            self.regs[rd] = ((self.regs[rs1] as i64) >> shamt) as u64;
                        }
                    }
                    _ => {
                        error!("This should not be reached!");
                        error!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
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
                        // "lb"
                        let val = self.load(addr, 8)?;
                        self.regs[rd] = val as i8 as i64 as u64;
                    }
                    0x1 => {
                        // "lh"
                        let val = self.load(addr, 16)?;
                        self.regs[rd] = val as i16 as i64 as u64;
                    }
                    0x2 => {
                        // "lw"
                        let val = self.load(addr, 32)?;
                        self.regs[rd] = val as i32 as i64 as u64;
                    }
                    0x3 => {
                        // "ld"
                        let val = self.load(addr, 64)?;
                        self.regs[rd] = val;
                    }
                    0x4 => {
                        // "lbu"
                        let val = self.load(addr, 8)?;
                        self.regs[rd] = val;
                    }
                    0x5 => {
                        // "lhu"
                        let val = self.load(addr, 16)?;
                        self.regs[rd] = val;
                    }
                    0x6 => {
                        // "lwu"
                        let val = self.load(addr, 32)?;
                        self.regs[rd] = val;
                    }
                    _ => {
                        error!("This should not be reached!");
                        error!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
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
                // "s?",
                match funct3 {
                    0x0 => self.store(addr, 8, self.regs[rs2])?,
                    0x1 => self.store(addr, 16, self.regs[rs2])?,
                    0x2 => self.store(addr, 32, self.regs[rs2])?,
                    0x3 => self.store(addr, 64, self.regs[rs2])?,
                    _ => {
                        error!("This should not be reached!");
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
                // "jal"
                self.regs[rd] = self.pc.wrapping_add(4);
                self.pc = self.pc.wrapping_add(imm).wrapping_sub(4); // subtract 4 because 4 will be added
                self.mark_as_dest(rd);
                Ok(())
            }
            0x67 => {
                match funct3 {
                    0x0 => {
                        let imm = ((inst as i32 as i64) >> 20) as u64;
                        // "jalr"
                        let return_addr = self.pc.wrapping_add(4);
                        let next_pc = self.regs[rs1].wrapping_add(imm).wrapping_sub(4);
                        // subtract 4 because 4 will be added
                        self.regs[rd] = return_addr;
                        self.pc = next_pc;
                    }
                    _ => {
                        error!("This should not be reached!");
                        error!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
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
                        // "addiw"
                        let src = self.regs[rs1] as i32;
                        let val = src.wrapping_add(imm);
                        self.regs[rd] = val as i64 as u64;
                    }
                    (0x1, 0x0) => {
                        // slliw
                        // I-type format
                        let shamt = ((inst as u32) >> 20) & 0x1f;
                        // "slliw"
                        let src = self.regs[rs1] as u32;
                        let val = src << shamt;
                        self.regs[rd] = val as i32 as i64 as u64;
                    }
                    (0x5, 0x0) => {
                        // srliw
                        // I-type format
                        let shamt = ((inst as u32) >> 20) & 0x1f;
                        // "srliw"
                        let src = self.regs[rs1] as u32;
                        let val = src >> shamt;
                        self.regs[rd] = val as i32 as i64 as u64;
                    }
                    (0x5, 0x20) => {
                        // sraiw
                        // I-type format
                        let shamt = ((inst as u32) >> 20) & 0x1f;
                        // "sraiw"
                        let src = self.regs[rs1] as i32;
                        let val = src >> shamt;
                        self.regs[rd] = val as i64 as u64;
                    }
                    _ => {
                        error!("This should not be reached!");
                        error!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
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
                        // "beq"
                        if self.regs[rs1] == self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x1 => {
                        // "bne"
                        if self.regs[rs1] != self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x4 => {
                        // "blt"
                        if (self.regs[rs1] as i64) < (self.regs[rs2] as i64) {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x5 => {
                        // "bge"
                        if (self.regs[rs1] as i64) >= (self.regs[rs2] as i64) {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x6 => {
                        // "bltu"
                        if self.regs[rs1] < self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x7 => {
                        // "bgeu"
                        if self.regs[rs1] >= self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    _ => {
                        error!("This should not be reached!");
                        error!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
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
                        // "addw"
                        let add_val = (self.regs[rs1] as i32).wrapping_add(self.regs[rs2] as i32);
                        self.regs[rd] = add_val as i64 as u64;
                    }
                    (0x0, 0x20) => {
                        // "subw"
                        let add_val = (self.regs[rs1] as i32).wrapping_sub(self.regs[rs2] as i32);
                        self.regs[rd] = add_val as i64 as u64;
                    }
                    (0x1, 0x0) => {
                        // "sllw"
                        let shamt = (self.regs[rs2] as u64) & 0x1f;
                        self.regs[rd] = ((self.regs[rs1] as u32) << shamt) as u64;
                    }
                    (0x5, 0x0) => {
                        // "srlw"
                        let shamt = (self.regs[rs2] as u64) & 0x1f;
                        self.regs[rd] = ((self.regs[rs1] as u32) >> shamt) as u64;
                    }
                    (0x5, 0x20) => {
                        // "sraw"
                        let shamt = (self.regs[rs2] as u64) & 0x1f;
                        self.regs[rd] = ((self.regs[rs1] as i32) >> shamt) as i64 as u64;
                    }
                    (0x0, 0x1) => {
                        // "mulw"
                        let mul = (self.regs[rs2] as u32) * (self.regs[rs2] as u32);
                        self.regs[rd] = mul as i32 as i64 as u64;
                    }
                    (0x4, 0x1) => {
                        // "divw"
                        let rem = (self.regs[rs2] as u32) / (self.regs[rs2] as u32);
                        self.regs[rd] = rem as u64;
                    }
                    (0x5, 0x1) => {
                        // "divuw"
                        let rem = (self.regs[rs2] as i32) / (self.regs[rs2] as i32);
                        self.regs[rd] = rem as i64 as u64;
                    }
                    (0x6, 0x1) => {
                        // "remw"
                        let rem = (self.regs[rs2] as i32) % (self.regs[rs2] as i32);
                        self.regs[rd] = rem as i64 as u64;
                    }
                    (0x7, 0x1) => {
                        // "remuw"
                        let rem = (self.regs[rs2] as u32) % (self.regs[rs2] as u32);
                        self.regs[rd] = rem as u64;
                    }
                    _ => {
                        error!("This should not be reached!");
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
                // "lui"
                self.regs[rd] = imm;
                self.mark_as_dest(rd);
                Ok(())
            }
            0x17 => {
                let imm = inst & 0xfffff000;
                // "auipc"
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
                        // "ecall"
                        Exception::EnvironmentalCallFromMMode.take_trap(self);
                    }
                    (0x0, 0x0, 0x1) => {
                        // "ebreak"
                    }
                    (0x0, 0x8, 0x2) => {
                        // "sret"
                        self.return_from_trap();
                    }
                    (0x0, 0x8, 0x5) => {
                        // "wfi"
                        self.wait_for_interrupt();
                    }
                    (0x0, 0x18, 0x2) => {
                        // "mret"
                        self.return_from_trap();
                    }
                    (0x1, _, _) => {
                        // "csrrw"
                        if rd != 0 {
                            self.regs[rd] = self.csr.load_csrs(csr) as u64;
                        }
                        self.csr.store_csrs(csr, self.regs[rs1]);
                    }
                    (0x2, _, _) => {
                        // "csrrs"
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, self.regs[rs1] | old_val);
                        }
                    }
                    (0x3, _, _) => {
                        // "csrrc"
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, self.regs[rs1] & !old_val);
                        }
                    }
                    (0x5, _, _) => {
                        // "csrrwi"
                        if rd != 0 {
                            self.regs[rd] = self.csr.load_csrs(csr);
                        }
                        self.csr.store_csrs(csr, uimm as u64);
                    }
                    (0x6, _, _) => {
                        // "csrrsi"
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, uimm as u64 | old_val);
                        }
                    }
                    (0x7, _, _) => {
                        // "csrrci"
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, uimm as u64 & !old_val);
                        }
                    }
                    (0x0, 0x9, _) => {
                        // "sfence.
                        self.address_translation_cache.clear();
                    }
                    (_, _, _) => {
                        error!("Unsupported CSR instruction!");
                        error!("pc = 0x{:x}, funct3:{}, funct7:{}", self.pc, funct3, funct7);
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
                        // "amoswap.
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
                        // "amoadd.
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
                        // "amoxor.
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
                        // "amoand.
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
                        // "amoor.
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
                    }
                    (0x14, 0x1) => {
                        // "amomax.
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
                    }
                    (0x1c, 0x1) => {
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
                    }
                    (0x2, 0x3) => {
                        // "amoswap.
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
                        // "amoadd.
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
                        // "amoxor.
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
                        // "amoand.
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
                        // "amoor.
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
                        // "amomin.
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
                        // "amomax.
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
                        // "amominu.
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
                        // "amomaxu.
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
                error!("not implemented yet!");
                error!("pc=0x{:x}", self.pc);
                error!("inst:{inst:b}");
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

        let result = self.execute(inst as u32).map_err(|mut e| e.take_trap(self));
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
        let mut interrupt_list_vec = self.interrupt_list.lock().unwrap();

        // Pend interrupts with cycle == 0 and add them to the pending list
        let mut to_pend = Vec::new();
        interrupt_list_vec.retain(|delayed_interrupt| {
            if delayed_interrupt.cycle > 0 {
                true
            } else {
                to_pend.push(delayed_interrupt.interrupt);
                false
            }
        });

        for delayed_interrupt in interrupt_list_vec.iter_mut() {
            trace!("Delayed Interrupt: {:?} ", delayed_interrupt);
            delayed_interrupt.cycle -= 1;
        }

        drop(interrupt_list_vec); // Release the lock before calling self methods

        for interrupt in to_pend {
            info!("Pend Interrupt: {:?} ", interrupt);
            self.set_pending_interrupt(interrupt);
        }

        // Update Supervisor Timer Interrupt pending status
        // If the current time count is greater than STIMECMP, set the pending status
        // Otherwise, clear the pending status
        let stimecmp = self.csr.load_csrs(STIMECMP);
        let current_counter = *self.cycle.borrow() * TIMER_FREQ / CPU_FREQUENCY;
        let mut xip = self.csr.load_csrs(MIP);
        let sti_bit = Interrupt::SupervisorTimerInterrupt.bit_code() & !INTERRUPT_BIT;
        // info!(
        //     "stimecmp: {}, current_counter: {}, xip: {:b}",
        //     stimecmp, current_counter, xip
        // );
        if (stimecmp > 0) && (current_counter >= stimecmp) {
            if xip & sti_bit == 0 {
                debug!(
                    "Setting Supervisor Timer Interrupt pending: stimecmp:{}, current_counter:{}",
                    stimecmp, current_counter
                );
                xip |= Interrupt::SupervisorTimerInterrupt.bit_code();
                self.csr.store_csrs(MIP, xip);
            }
        } else {
            if xip & sti_bit != 0 {
                debug!("Clearing Supervisor Timer Interrupt pending");
                if xip & sti_bit & !INTERRUPT_BIT != 0 {
                    // There are other pending interrupts, so we need to restore other pending bits
                    xip &= !sti_bit;
                } else {
                    // No other pending interrupts, clear the MIP register
                    xip = 0;
                }
                self.csr.store_csrs(MIP, xip);
            }
        }
    }
}
