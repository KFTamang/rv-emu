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

pub const CPU_FREQUENCY: u64 = 200_000_000; // 200MHz

#[derive(PartialEq)]
enum AccessMode {
    Fetch,
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
    pub csr: CsrSnapshot,
    pub mode: u64,
    pub cycle: u64,
    pub clint: Clint,
    pub interrupt_list: BTreeSet<Interrupt>,
    pub address_translation_cache: std::collections::HashMap<(u64,u64,u64), u64>, // (satp_ppn, asid, va_page) -> pa_page
}

pub struct Cpu {
    pub regs: [u64; 32],
    pub pc: u64,
    pub bus: Rc<RefCell<Bus>>,
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
    address_translation_cache: std::collections::HashMap<(u64,u64,u64), u64>,
    block_cache: std::collections::HashMap<u64, BasicBlock>,
}

impl Cpu {
    pub fn new(bus: Rc<RefCell<Bus>>, base_addr: u64, _dump_count: u64, interrupt_list: Rc<RefCell<BTreeSet<Interrupt>>>) -> Self {
        let mut regs = [0; 32];
        regs[2] = DRAM_SIZE;
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
            block_cache: std::collections::HashMap::new(),
        }
    }

    pub fn to_snapshot(&self) -> CpuSnapshot {
        CpuSnapshot {
            regs: self.regs,
            pc: self.pc,
            csr: self.csr.to_snapshot(),
            mode: self.mode,
            cycle: *self.cycle.borrow(),
            clint: self.clint.clone(),
            interrupt_list: self.interrupt_list.borrow().clone(),
            address_translation_cache: self.address_translation_cache.clone(),
        }
    }

    pub fn from_snapshot(snapshot: CpuSnapshot, bus: Rc<RefCell<Bus>>) -> Self {
        let interrupt_list = Rc::new(RefCell::new(snapshot.interrupt_list));
        let cycle = Rc::new(RefCell::new(snapshot.cycle));
        let mut cpu = Self {
            regs: snapshot.regs,
            pc: snapshot.pc,
            bus: bus,
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
            block_cache: std::collections::HashMap::new(),
        };
        cpu.clear_reg_marks();
        cpu
    }

    pub fn fetch(&mut self, addr: u64) -> Result<u32, Exception> {
        match self.translate(addr, AccessMode::Fetch) {
            Ok(pa) => {
                self.bus
                .borrow_mut()
                .load(pa, 32)
                .map(|v| v as u32)
            }
            Err(e) => Err(e),
        }
    }

    pub fn set_dump_count(&mut self, count: u64) {
        self.dump_count = count;
        self.dump_interval = count;
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

    pub fn load(&mut self, va: u64, size: u64) -> Result<u64, Exception> {
        trace!("Load access to 0x{:x}", va);
        match self.translate(va, AccessMode::Load) {
            Ok(pa) => {
                if self.clint.is_accessible(pa) {
                    self.clint.load(pa, size)
                } else {
                    self.bus.as_ref()
                        .borrow_mut()
                        .load(pa, size)
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn store(&mut self, va: u64, size: u64, value: u64) -> Result<(), Exception> {
        match self.translate(va, AccessMode::Store) {
            Ok(pa) => {
                if self.clint.is_accessible(pa) {
                    self.clint.store(pa, size, value)
                } else {
                    self.bus.as_ref()
                        .borrow_mut()
                        .store(pa, size, value)
                }
            }
            Err(e) => Err(e),
        }
    }

/// SV39 page-table walk + permission check + A/D handling + simple TLB keyed by (satp_ppn, asid, va_page).
///
/// Notes:
/// - This is RV64 Sv39 only (satp.mode == 8). Other modes return page fault (or Ok(va) for Bare).
/// - Implements canonical VA check.
/// - Implements leaf detection and misaligned-superpage checks.
/// - Implements basic R/W/X + U checks; SUM/MXR are left as TODO hooks (depends on how you model S/U).
/// - Implements A/D as hardware-updated bits (common for emulators). If you want "software-managed A/D",
///   change the A/D section to raise page fault instead of writing PTE.
/// - TLB is flushed when satp changes (you should also call mmu.sfence_vma(...) from your SFENCE.VMA).
fn translate(&mut self, va: u64, acc: AccessMode) -> Result<u64, Exception> {
    const PAGESIZE: u64 = 4096;
    const PTESIZE: u64 = 8;

    // ---- satp decode (Sv39) ----
    let satp = self.csr.load_csrs(SATP);
    let mode = (satp >> 60) & 0xF; // [63:60]
    let asid = (satp >> 44) & 0xFFFF; // [59:44]
    let satp_ppn = satp & ((1u64 << 44) - 1); // [43:0]

    // Bare
    if mode == 0 {
        return Ok(va);
    }
    // Only Sv39 supported here
    if mode != 8 {
        return match acc {
            AccessMode::Fetch => Err(Exception::InstructionPageFault(va as u32)),
            AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
            AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
        };
    }

    // ---- canonical VA check for Sv39 ----
    // VA[63:39] must all equal VA[38]
    let sign = (va >> 38) & 1;
    let upper = va >> 39;
    if (sign == 0 && upper != 0) || (sign == 1 && upper != ((1u64 << 25) - 1)) {
        return match acc {
            AccessMode::Fetch => Err(Exception::InstructionPageFault(va as u32)),
            AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
            AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
        };
    }

    // ---- TLB lookup (keyed by satp_ppn + asid + va_page) ----
    // You must clear/flush this cache on SFENCE.VMA and/or satp writes.
    let va_page = va >> 12;
    let tlb_key = (satp_ppn, asid, va_page);
    if let Some(&pa_page) = self.address_translation_cache.get(&tlb_key) {
        return Ok((pa_page << 12) | (va & 0xFFF));
    }

    // ---- VPN parts ----
    let vpn0 = (va >> 12) & 0x1FF;
    let vpn1 = (va >> 21) & 0x1FF;
    let vpn2 = (va >> 30) & 0x1FF;
    let vpn = [vpn0, vpn1, vpn2];

    // ---- walk ----
    // a = root page table physical address
    let mut a = satp_ppn * PAGESIZE;
    let mut level: i32 = 2; // Sv39: levels 2,1,0

    let mut pte_addr: u64 = 0;
    let mut pte: u64 = 0;

    loop {
        pte_addr = a + vpn[level as usize] * PTESIZE;
        pte = self
            .bus
            .borrow_mut()
            .load(pte_addr, 64)
            .map_err(|_| match acc {
                AccessMode::Fetch => Exception::InstructionPageFault(va as u32),
                AccessMode::Load => Exception::LoadPageFault(va as u32),
                AccessMode::Store => Exception::StoreAMOPageFault(va as u32),
            })?;

        let v = bit(pte, 0);
        let r = bit(pte, 1);
        let w = bit(pte, 2);
        let x = bit(pte, 3);
        let u = bit(pte, 4);
        // let g = bit(pte, 5);
        let a_bit = bit(pte, 6);
        let d_bit = bit(pte, 7);

        // Invalid PTE or reserved combo
        if v == 0 || (r == 0 && w == 1) {
            return match acc {
                AccessMode::Fetch => Err(Exception::InstructionPageFault(va as u32)),
                AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
                AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
            };
        }

        let is_leaf = (r == 1) || (x == 1);
        if is_leaf {
            // ---- permission check ----
            // NOTE: You likely also need S/U privilege, SUM/MXR, etc.
            match acc {
                AccessMode::Fetch => {
                    if x == 0 {
                        return Err(Exception::InstructionPageFault(va as u32));
                    }
                }
                AccessMode::Load => {
                    // MXR: if sstatus.MXR=1 and x=1 then load may be allowed; TODO if you model MXR.
                    if r == 0 {
                        return Err(Exception::LoadPageFault(va as u32));
                    }
                }
                AccessMode::Store => {
                    if w == 0 {
                        return Err(Exception::StoreAMOPageFault(va as u32));
                    }
                }
            }

            // U bit check (if you model current privilege)
            // Here we assume you have self.priv_level() returning enum {User, Supervisor, Machine}.
            // If not, replace with your own check or remove.
            if self.mode == U_MODE && u == 0 {
                return match acc {
                    AccessMode::Fetch => Err(Exception::InstructionPageFault(va as u32)),
                    AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
                    AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
                };
            }

            // ---- A/D bits (hardware-updated model) ----
            // Set A on any access; set D on store.
            let mut new_pte = pte;
            if a_bit == 0 {
                new_pte |= 1 << 6;
            }
            if matches!(acc, AccessMode::Store) && d_bit == 0 {
                new_pte |= 1 << 7;
            }
            if new_pte != pte {
                self.bus
                    .borrow_mut()
                    .store(pte_addr, 64, new_pte)
                    .map_err(|_| match acc {
                        AccessMode::Fetch => Exception::InstructionPageFault(va as u32),
                        AccessMode::Load => Exception::LoadPageFault(va as u32),
                        AccessMode::Store => Exception::StoreAMOPageFault(va as u32),
                    })?;
                pte = new_pte;
            }

            // ---- misaligned superpage check ----
            // If leaf at level=2 (1GiB), PTE.PPN[1:0] must be 0.
            // If leaf at level=1 (2MiB), PTE.PPN[0] must be 0.
            let ppn0 = (pte >> 10) & 0x1FF;
            let ppn1 = (pte >> 19) & 0x1FF;
            let ppn2 = (pte >> 28) & 0x3FF_FFFF; // remaining bits
            if level == 2 {
                if ppn0 != 0 || ppn1 != 0 {
                    return match acc {
                        AccessMode::Fetch => Err(Exception::InstructionPageFault(va as u32)),
                        AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
                        AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
                    };
                }
            } else if level == 1 {
                if ppn0 != 0 {
                    return match acc {
                        AccessMode::Fetch => Err(Exception::InstructionPageFault(va as u32)),
                        AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
                        AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
                    };
                }
            }

            // ---- physical address composition ----
            let page_off = va & 0xFFF;
            let pa: u64 = match level {
                // 4KiB page: PA = {PPN2,PPN1,PPN0,off}
                0 => {
                    let ppn = (pte >> 10) & ((1u64 << 44) - 1);
                    (ppn << 12) | page_off
                }
                // 2MiB page: PA = {PPN2,PPN1, VPN0, off}
                1 => {
                    let ppn = ((ppn2 << 18) | (ppn1 << 9) | vpn0) & ((1u64 << 44) - 1);
                    (ppn << 12) | page_off
                }
                // 1GiB page: PA = {PPN2, VPN1, VPN0, off}
                2 => {
                    let ppn = ((ppn2 << 18) | (vpn1 << 9) | vpn0) & ((1u64 << 44) - 1);
                    (ppn << 12) | page_off
                }
                _ => unreachable!(),
            };

            // Update TLB entry at 4KiB granularity (ok even for superpages, but less effective).
            self.address_translation_cache.insert(tlb_key, pa >> 12);

            return Ok(pa);
        }

        // Non-leaf: next level
        if level == 0 {
            return match acc {
                AccessMode::Fetch => Err(Exception::InstructionPageFault(va as u32)),
                AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
                AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
            };
        }

        let next_ppn = (pte >> 10) & ((1u64 << 44) - 1);
        a = next_ppn * PAGESIZE;
        level -= 1;
    }
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

    fn return_from_machine_trap(&mut self) {
        // mstatus.MIE <- mstatus.MPIE(=1)
        // U-modeに遷移する
        // mstatus.MPIE <~ 1 [always]
        // mstatus.MPP <~ 00(U-mode) [always]
        // pc(program counter) <~ mepc CSR
        info!("mret instruction from mode {}", self.mode);
        debug!("{}", self.dump_registers());
        debug!("{}", self.csr.dump());
        // machine mode is guaranteed here
        let pp = self.csr.get_mstatus_bit(MASK_MPP, BIT_MPP);
        let pie = self.csr.get_mstatus_bit(MASK_MPIE, BIT_MPIE);
        let previous_pc = self.csr.load_csrs(MEPC);
        self.csr.set_mstatus_bit(pie, MASK_MIE, BIT_MIE);
        self.csr.set_mstatus_bit(0b1, MASK_MPIE, BIT_MPIE);
        self.csr.set_mstatus_bit(U_MODE, MASK_MPP, BIT_MPP);
        self.pc = previous_pc.wrapping_sub(4); // subtract 4 to cancel out addition in main loop
        self.mode = pp;
        info!("back to privilege {} from machine mode by mret", pp);
        debug!("return from trap");
        debug!("PC: 0x{:x}", previous_pc);
        debug!("csr dump");
        debug!("{}", self.csr.dump());
    }

    fn return_from_supervisor_trap(&mut self) {
        // mstatus.MIE <- mstatus.MPIE(=1)
        // U-modeに遷移する
        // mstatus.MPIE <~ 1 [always]
        // mstatus.MPP <~ 00(U-mode) [always]
        // pc(program counter) <~ mepc CSR
        debug!("sret instruction from mode {}", self.mode);
        debug!("{}", self.dump_registers());
        debug!("{}", self.csr.dump());
        let pp = self.csr.get_sstatus_bit(MASK_SPP, BIT_SPP);
        let pie = self.csr.get_sstatus_bit(MASK_SPIE, BIT_SPIE);
        let previous_pc = self.csr.load_csrs(SEPC);
        self.csr.set_sstatus_bit(pie, MASK_SIE, BIT_SIE);
        self.csr.set_sstatus_bit(0b1, MASK_SPIE, BIT_SPIE);
        self.csr.set_sstatus_bit(U_MODE, MASK_SPP, BIT_SPP);
        self.pc = previous_pc.wrapping_sub(4); // subtract 4 to cancel out addition in main loop
        self.mode = pp;
        info!("back to privilege {} from supervisor mode by sret", pp);
        debug!("return from trap");
        debug!("PC: 0x{:x}", previous_pc);
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
            DecodedInstr::Add{ raw: _, rd, rs1, rs2} => {
                // "add"
                self.regs[rd] = self.regs[rs1].wrapping_add(self.regs[rs2]);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sub{ raw: _, rd, rs1, rs2} => {
                // "sub"
                self.regs[rd] = self.regs[rs1].wrapping_sub(self.regs[rs2]);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sll{ raw: _, rd, rs1, rs2} => {
                // "sll"
                let shamt = self.regs[rs2] & 0x1f;
                self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Slt{ raw: _, rd, rs1, rs2} => {
                // "slt"
                self.regs[rd] = if (rs1 as i64) < (rs2 as i64) { 1 } else { 0 };
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sltu{ raw: _, rd, rs1, rs2} => {
                // "sltu"
                self.regs[rd] = if (rs1 as u64) < (rs2 as u64) { 1 } else { 0 };
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Xor{ raw: _, rd, rs1, rs2} => {
                // "xor"
                self.regs[rd] = self.regs[rs1] ^ self.regs[rs2];
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Srl{ raw: _, rd, rs1, rs2} => {
                // "srl"
                let shamt = self.regs[rs2] & 0x1f;
                self.regs[rd] = self.regs[rs1] as u64 >> shamt;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sra{ raw: _, rd, rs1, rs2} => {
                // "sra"
                let shamt = self.regs[rs2] & 0x1f;
                self.regs[rd] = (self.regs[rs1] as i64 as u64) >> shamt;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Or{ raw: _, rd, rs1, rs2} => {
                // "or"
                self.regs[rd] = self.regs[rs1] | self.regs[rs2];
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::And{ raw: _, rd, rs1, rs2} => {
                // "and"
                self.regs[rd] = self.regs[rs1] & self.regs[rs2];
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Mul{ raw: _, rd, rs1, rs2} => {
                // "mul"
                self.regs[rd] = self.regs[rs1].wrapping_mul(self.regs[rs2]);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Mulh{ raw: _, rd, rs1, rs2} => {
                // "mulh"
                let mul = (self.regs[rs1] as i64 as i128)
                    .wrapping_mul(self.regs[rs2] as i64 as i128);
                self.regs[rd] = (mul >> 64) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Mulhsu{ raw: _, rd, rs1, rs2} => {
                // "mulhsu"
                let mul = (self.regs[rs1] as i64 as i128)
                    .wrapping_mul(self.regs[rs2] as u128 as i128);
                self.regs[rd] = (mul >> 64) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Mulhu{ raw: _, rd, rs1, rs2} => {
                // "mulhu"
                let mul = (self.regs[rs1] as u128).wrapping_mul(self.regs[rs2] as u128);
                self.regs[rd] = (mul >> 64) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Div{ raw: _, rd, rs1, rs2} => {
                // "div"
                self.regs[rd] = self.regs[rs1] / self.regs[rs2];
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Divu{ raw: _, rd, rs1, rs2} => {
                // "divu"
                self.regs[rd] = ((self.regs[rs1] as i64) / (self.regs[rs2] as i64)) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Rem{ raw: _, rd, rs1, rs2} => {
                // "rem"
                self.regs[rd] = self.regs[rs1] % self.regs[rs2];
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Remu{ raw: _, rd, rs1, rs2} => {
                // "remu"
                self.regs[rd] = ((self.regs[rs1] as i64) % (self.regs[rs2] as i64)) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Addi{ raw: _, rd, rs1, imm} => {
                // "addi"
                self.regs[rd] = self.regs[rs1].wrapping_add(imm);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Slti{ raw: _, rd, rs1, imm} => {
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
            DecodedInstr::Sltiu{ raw: _, rd, rs1, imm} => {
                // "sltiu"
                let result = if (self.regs[rs1] as i32 as i64 as u64) < imm {
                    1
                } else {
                    0
                };
                self.regs[rd] = result;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Xori{ raw: _, rd, rs1, imm} => {
                // "xori"
                let val = ((self.regs[rs1] as i32) ^ (imm as i32)) as u64;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Ori{ raw: _, rd, rs1, imm} => {
                // "ori"
                let val = ((self.regs[rs1] as i32) | (imm as i32)) as u64;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Andi{ raw: _, rd, rs1, imm} => {
                // "andi"
                let val = ((self.regs[rs1] as i32) & (imm as i32)) as u64;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Slli{ raw: _, rd, rs1, imm} => {
                // "slli"
                let shamt = (imm & 0x3f) as u64;
                self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Srli{ raw: _, rd, rs1, imm} => {
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
            DecodedInstr::Lb { raw: _, rd, rs1, imm} => {
                // "lb"
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(addr, 8)?;
                self.regs[rd] = val as i8 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lh { raw: _, rd, rs1, imm} => {
                // "lh"
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(addr, 16)?;
                self.regs[rd] = val as i16 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lw { raw: _, rd, rs1, imm} => {
                // "lw"
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(addr, 32)?;
                self.regs[rd] = val as i32 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Ld { raw: _, rd, rs1, imm} => {
                // "ld"
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(addr, 64)?;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lbu { raw: _, rd, rs1, imm} => {
                // "lbu"
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(addr, 8)?;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lhu { raw: _, rd, rs1, imm} => {
                // "lhu"
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(addr, 16)?;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lwu { raw: _, rd, rs1, imm} => {
                // "lwu"
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(addr, 32)?;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Sb { raw: _, rs1, rs2, imm } => {
                // store instructions
                let addr = self.regs[rs1].wrapping_add(imm as i32 as i64 as u64);
                self.store(addr, 8, self.regs[rs2])?;
                self.mark_as_src1(rs1);
                self.mark_as_dest(rs2);
                Ok(())
            }
            DecodedInstr::Sh { raw: _, rs1, rs2, imm } => {
                // store instructions
                let addr = self.regs[rs1].wrapping_add(imm as i32 as i64 as u64);
                self.store(addr, 16, self.regs[rs2])?;
                self.mark_as_src1(rs1);
                self.mark_as_dest(rs2);
                Ok(())
            }
            DecodedInstr::Sw { raw: _, rs1, rs2, imm } => {
                // store instructions
                let addr = self.regs[rs1].wrapping_add(imm as i32 as i64 as u64);
                self.store(addr, 32, self.regs[rs2])?;
                self.mark_as_src1(rs1);
                self.mark_as_dest(rs2);
                Ok(())
            }
            DecodedInstr::Sd { raw: _, rs1, rs2, imm } => {
                // store instructions
                let addr = self.regs[rs1].wrapping_add(imm as i32 as i64 as u64);
                self.store(addr, 64, self.regs[rs2])?;
                self.mark_as_src1(rs1);
                self.mark_as_dest(rs2);
                Ok(())
            }
            DecodedInstr::Jal { raw: _, rd, imm } => {
                // jal
                self.regs[rd] = self.pc.wrapping_add(4);
                self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4); // subtract 4 because 4 will be added
                self.mark_as_dest(rd);
                Ok(())
            }
            DecodedInstr::Jalr { raw: _, rd, rs1, imm} => {
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
            DecodedInstr::Addiw { raw: _, rd, rs1, imm } => {
                // addiw
                let src = self.regs[rs1] as i32;
                let val = src.wrapping_add(imm as i32);
                self.regs[rd] = val as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Slliw { raw: _, rd, rs1, shamt } => {
                // slliw
                let src = self.regs[rs1] as u32;
                let val = src << shamt;
                self.regs[rd] = val as i32 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Srliw { raw: _, rd, rs1, shamt } => {
                // srliw
                let src = self.regs[rs1] as u32;
                let val = src >> shamt;
                self.regs[rd] = val as i32 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Sraiw { raw: _, rd, rs1, shamt } => {
                // sraiw
                let src = self.regs[rs1] as i32;
                let val = src >> shamt;
                self.regs[rd] = val as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Beq { raw: _, rs1, rs2, imm } => {
                // "beq"
                if self.regs[rs1] == self.regs[rs2] {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Bne { raw: _, rs1, rs2, imm } => {
                // "bne"
                if self.regs[rs1] != self.regs[rs2] {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Blt { raw: _, rs1, rs2, imm } => {
                // "blt"
                if (self.regs[rs1] as i64) < (self.regs[rs2] as i64) {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Bge { raw: _, rs1, rs2, imm } => {
                // "bge"
                if (self.regs[rs1] as i64) >= (self.regs[rs2] as i64) {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Bltu { raw: _, rs1, rs2, imm } => {
                // "bltu"
                if self.regs[rs1] < self.regs[rs2] {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Bgeu { raw: _, rs1, rs2, imm } => {
                // "bgeu"
                if self.regs[rs1] >= self.regs[rs2] {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Addw { raw: _, rd, rs1, rs2} => {
                // "addw"
                let add_val = (self.regs[rs1] as i32).wrapping_add(self.regs[rs2] as i32);
                self.regs[rd] = add_val as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Subw { raw: _, rd, rs1, rs2}  => {
                // "subw"
                let add_val = (self.regs[rs1] as i32).wrapping_sub(self.regs[rs2] as i32);
                self.regs[rd] = add_val as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sllw { raw: _, rd, rs1, rs2} => {
                // "sllw"
                let shamt = (self.regs[rs2] as u64) & 0x1f;
                self.regs[rd] = ((self.regs[rs1] as u32) << shamt) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Srlw { raw: _, rd, rs1, rs2} => {
                // "srlw"
                let shamt = (self.regs[rs2] as u64) & 0x1f;
                self.regs[rd] = ((self.regs[rs1] as u32) >> shamt) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sraw { raw: _, rd, rs1, rs2}  => {
                // "sraw"
                let shamt = (self.regs[rs2] as u64) & 0x1f;
                self.regs[rd] = ((self.regs[rs1] as i32) >> shamt) as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Mulw { raw: _, rd, rs1, rs2} => {
                // "mulw"
                let mul = (self.regs[rs2] as u32) * (self.regs[rs2] as u32);
                self.regs[rd] = mul as i32 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Divw { raw: _, rd, rs1, rs2} => {
                // "divw"
                let rem = (self.regs[rs2] as u32) / (self.regs[rs2] as u32);
                self.regs[rd] = rem as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Divuw { raw: _, rd, rs1, rs2} => {
                // "divuw"
                let rem = (self.regs[rs2] as i32) / (self.regs[rs2] as i32);
                self.regs[rd] = rem as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Remw { raw: _, rd, rs1, rs2} => {
                // "remw"
                let rem = (self.regs[rs2] as i32) % (self.regs[rs2] as i32);
                self.regs[rd] = rem as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Remuw { raw: _, rd, rs1, rs2} => {
                // "remuw"
                let rem = (self.regs[rs2] as u32) % (self.regs[rs2] as u32);
                self.regs[rd] = rem as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Lui { raw: _, rd, imm } => {
                // "lui"
                self.regs[rd] = imm as u64;
                self.mark_as_dest(rd);
                Ok(())
            }
            DecodedInstr::Auipc { raw: _, rd, imm } => {
                // "auipc"
                self.regs[rd] = self.pc + imm;
                self.mark_as_dest(rd);
                Ok(())
            }
            DecodedInstr::Ecall { raw: _ } => {
                info!("ecall instruction from mode {}", self.mode);
                match self.mode {
                    M_MODE => Exception::EnvironmentalCallFromMMode.take_trap(self),
                    S_MODE => Exception::EnvironmentalCallFromSMode.take_trap(self),
                    U_MODE => Exception::EnvironmentalCallFromUMode.take_trap(self),
                    _ => panic!("ecall is executed with mode: {}", self.mode),
                }
                Ok(())
            }
            DecodedInstr::Ebreak { raw: _ } => {
                // Optional: implement EBREAK behavior
                Ok(())
            }
            DecodedInstr::Sret { raw } => {
                if self.mode < S_MODE {
                    return Err(Exception::IllegalInstruction(raw));
                }
                self.return_from_supervisor_trap();
                Ok(())
            }
            DecodedInstr::Mret { raw } => {
                if self.mode < M_MODE {
                    return Err(Exception::IllegalInstruction(raw));
                }
                self.return_from_machine_trap();
                Ok(())
            }
            DecodedInstr::Wfi { raw: _ } => {
                self.wait_for_interrupt();
                Ok(())
            }
            DecodedInstr::Csrrw { raw: _, rd, rs1, csr } => {
                if rd != 0 {
                    self.regs[rd] = self.csr.load_csrs(csr) as u64;
                }
                self.csr.store_csrs(csr, self.regs[rs1]);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(()) 
            }
            DecodedInstr::Csrrs { raw: _, rd, rs1, csr } => {
                let old = self.csr.load_csrs(csr) as u64;
                self.regs[rd] = old;
                if rs1 != 0 {
                    self.csr.store_csrs(csr, self.regs[rs1] | old);
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Csrrc { raw: _, rd, rs1, csr } => {
                let old = self.csr.load_csrs(csr) as u64;
                self.regs[rd] = old;
                if rs1 != 0 {
                    self.csr.store_csrs(csr, self.regs[rs1] & !old);
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(()) 
            }
            DecodedInstr::Csrrwi { raw: _, rd, rs1, csr, uimm } => {
                if rd != 0 {
                    self.regs[rd] = self.csr.load_csrs(csr);
                }
                self.csr.store_csrs(csr, uimm as u64);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(()) 
            }
            DecodedInstr::Csrrsi { raw: _, rd, rs1, csr, uimm } => {
                let old_val = self.csr.load_csrs(csr) as u64;
                self.regs[rd] = old_val;
                if rs1 != 0 {
                    self.csr.store_csrs(csr, uimm as u64 | old_val);
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(()) 
            }
            DecodedInstr::Csrrci { raw: _, rd, rs1, csr, uimm } => {
                let old_val = self.csr.load_csrs(csr) as u64;
                self.regs[rd] = old_val;
                if rs1 != 0 {
                    self.csr.store_csrs(csr, uimm as u64 & !old_val);
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(()) 
            }
            DecodedInstr::Sfence { raw: _ } => {
                self.address_translation_cache.clear();
                self.block_cache.clear();
                Ok(())
            }
            DecodedInstr::Fence { raw: _ } => {
                // 実際には no-op（または memory ordering のために記録する）
                self.address_translation_cache.clear();
                self.block_cache.clear();
                Ok(())
            }
            DecodedInstr::Amoswap { raw: _, rd, rs1, rs2 } => {
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
            DecodedInstr::Amoadd { raw: _, rd, rs1, rs2 } => {
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
            DecodedInstr::Amoxor { raw: _, rd, rs1, rs2 } => {
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
            DecodedInstr::Amoand { raw: _, rd, rs1, rs2 } => {
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
            DecodedInstr::Amoor { raw: _, rd, rs1, rs2 } => {
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
            DecodedInstr::Amomin { raw: _, rd, rs1, rs2 } => {
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
            DecodedInstr::Amomax { raw: _, rd, rs1, rs2 } => {
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
            DecodedInstr::Amominu { raw: _, rd, rs1, rs2 } => {
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
            DecodedInstr::Amomaxu { raw: _, rd, rs1, rs2 } => {
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
            DecodedInstr::IllegalInstruction {inst } => {
                Err(Exception::IllegalInstruction(inst))
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

    pub fn trap_interrupt(&mut self) {
        *self.cycle.borrow_mut() += 1;
        if *self.cycle.borrow() % 1000000 == 0 {
            debug!("Cycle: {}", self.cycle.borrow());
        }

        // check for interrupts
        self.bus.as_ref()
            .borrow_mut()
            .plic.process_pending_interrupts();

        // check and pend all the delayed interrupts
        self.update_pending_interrupts();

        if let Some(mut interrupt) = self.get_interrupt_to_take() {
            debug!("Interrupt: {:?} taken", interrupt);
            debug!("{}", self.csr.dump());
            interrupt.take_trap(self);
        }
    }

    pub fn build_basic_block(&mut self) -> Result<BasicBlock, Exception> {
        // Build a basic block for the current instruction
        let mut instrs = Vec::new();
        let mut pc = self.pc;

        if self.block_cache.contains_key(&pc) {
            return Ok(self.block_cache.get(&pc).unwrap().clone());
        }

        loop {
            let inst = match self.fetch(pc) {
                Ok(inst) => inst,
                Err(e) => {
                    error!("Failed to fetch instruction at pc={:x}: {:?}", pc, e);
                    if instrs.is_empty() {
                        // If we cannot fetch the first instruction, return an empty block
                        return Err(Exception::InstructionAccessFault);
                    }
                    break;
                }
            };
            let decoded_inst = DecodedInstr::decode(inst);
            instrs.push(decoded_inst.clone());

            if decoded_inst.is_branch() || decoded_inst.is_jump()  || decoded_inst.is_illegal(){
                break;
            }

            pc = pc.wrapping_add(4);
        }
        
        let block = BasicBlock {
            start_pc: self.pc,
            end_pc: pc,
            instrs,
        };
        self.block_cache.insert(self.pc, block.clone());
        Ok(block)
    }

    pub fn run_block(&mut self, block: &BasicBlock) -> u64 {
        self.pc = block.start_pc;
        let mut cycle: u64 = 0;
        info!("Block execution: 0x{:x} to 0x{:x}", block.start_pc, block.end_pc);
        for instr in &block.instrs {
            let result = self.execute(instr.clone());
            if let Err(e) = result {
                error!("Execution failed in block at pc={:x}: {:?}, mode={}", self.pc, e, self.mode);
                e.take_trap(self);
                self.pc = self.pc.wrapping_add(4);
                break;
            }
            self.regs[0] = 0; // x0 is always zero
            self.pc = self.pc.wrapping_add(4);
            *self.cycle.borrow_mut() += 1;
            if self.dump_count > 0 {
                self.dump_count -= 1;
                if self.dump_count == 0 {
                    self.dump_count = self.dump_interval;
                    debug!("Block executed up to pc={:x}, cycle={}", self.pc, *self.cycle.borrow());
                    debug!("{}", self.dump_registers());
                    debug!("CSR: {}", self.csr.dump());
                }
            }
            cycle += 1;
        }
        cycle
    }

    pub fn step_run(&mut self) -> u64 {
        trace!("pc={:>#18x}", self.pc);

        self.trap_interrupt();

        let inst = match self.fetch(self.pc) {
            Ok(inst) => inst,
            Err(_) => return 0x0,
        };

        let decoded_inst =  DecodedInstr::decode(inst);

        let result = self.execute(decoded_inst).map_err(|e| e.take_trap(self));
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
