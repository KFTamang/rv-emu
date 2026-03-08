mod execute;
mod mmu;

use crate::bus::*;
use crate::clint::*;
use crate::csr::*;
use crate::dram::*;
use crate::instruction::*;
use crate::interrupt::*;

use log::{debug, error, info, trace};

use serde::{Deserialize, Serialize};
use std::cmp;
use std::collections::BTreeSet;
use std::sync::Arc;

const REG_NUM: usize = 32;
pub const M_MODE: u64 = 0b11;
pub const S_MODE: u64 = 0b01;
pub const U_MODE: u64 = 0b00;

pub const CPU_FREQUENCY: u64 = 200_000_000; // 200MHz

#[derive(PartialEq)]
pub(crate) enum AccessMode {
    Fetch,
    Load,
    Store,
}

pub(crate) fn bit(integer: u64, bit: u64) -> u64 {
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
    pub address_translation_cache: std::collections::HashMap<(u64, u64, u64), u64>,
}

pub struct Cpu {
    pub regs: [u64; 32],
    pub pc: u64,
    pub csr: Csr,
    pub(crate) dest: usize,
    pub(crate) src1: usize,
    pub(crate) src2: usize,
    pub mode: u64,
    pub(crate) dump_count: u64,
    pub(crate) dump_interval: u64,
    pub(crate) inst_string: String,
    pub cycle: u64,
    pub(crate) clint: Clint,
    pub interrupt_list: BTreeSet<Interrupt>,
    pub(crate) address_translation_cache: std::collections::HashMap<(u64, u64, u64), u64>,
    pub(crate) block_cache: std::collections::HashMap<u64, Arc<BasicBlock>>,
}

impl Cpu {
    pub fn new(base_addr: u64, dump_count: u64) -> Self {
        let mut regs = [0; 32];
        regs[2] = DRAM_SIZE;
        Self {
            regs,
            pc: base_addr,
            csr: Csr::new(),
            dest: REG_NUM,
            src1: REG_NUM,
            src2: REG_NUM,
            mode: M_MODE,
            dump_count,
            dump_interval: dump_count,
            inst_string: String::from(""),
            clint: Clint::new(0x200_0000, 0x10000),
            cycle: 0,
            interrupt_list: BTreeSet::new(),
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
            cycle: self.cycle,
            clint: self.clint.clone(),
            interrupt_list: self.interrupt_list.clone(),
            address_translation_cache: self.address_translation_cache.clone(),
        }
    }

    pub fn from_snapshot(snapshot: CpuSnapshot) -> Self {
        let mut cpu = Self {
            regs: snapshot.regs,
            pc: snapshot.pc,
            csr: Csr::from_snapshot(snapshot.csr),
            dest: REG_NUM,
            src1: REG_NUM,
            src2: REG_NUM,
            mode: snapshot.mode,
            dump_count: 0,
            dump_interval: 0,
            inst_string: String::from(""),
            clint: snapshot.clint,
            cycle: snapshot.cycle,
            interrupt_list: snapshot.interrupt_list,
            address_translation_cache: snapshot.address_translation_cache,
            block_cache: std::collections::HashMap::new(),
        };
        cpu.clear_reg_marks();
        cpu
    }

    pub fn fetch(&mut self, bus: &mut Bus, addr: u64) -> Result<u32, Exception> {
        match self.translate(bus, addr, AccessMode::Fetch) {
            Ok(pa) => bus.load(pa, 32).map(|v| v as u32),
            Err(e) => Err(e),
        }
    }

    pub fn set_dump_count(&mut self, count: u64) {
        self.dump_count = count;
        self.dump_interval = count;
    }

    pub(crate) fn mark_as_dest(&mut self, reg: usize) {
        self.dest = reg;
    }

    pub(crate) fn mark_as_src1(&mut self, reg: usize) {
        self.src1 = reg;
    }

    pub(crate) fn mark_as_src2(&mut self, reg: usize) {
        self.src2 = reg;
    }

    pub(crate) fn clear_reg_marks(&mut self) {
        self.dest = REG_NUM;
        self.src1 = REG_NUM;
        self.src2 = REG_NUM;
    }

    pub fn load(&mut self, bus: &mut Bus, va: u64, size: u64) -> Result<u64, Exception> {
        trace!("Load access to 0x{:x}", va);
        match self.translate(bus, va, AccessMode::Load) {
            Ok(pa) => {
                if self.clint.is_accessible(pa) {
                    self.clint.load(pa, size)
                } else if bus.plic.is_accessible(pa) {
                    bus.plic_load(pa, size, &mut self.interrupt_list)
                } else {
                    bus.load(pa, size)
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn store(
        &mut self,
        bus: &mut Bus,
        va: u64,
        size: u64,
        value: u64,
    ) -> Result<(), Exception> {
        match self.translate(bus, va, AccessMode::Store) {
            Ok(pa) => {
                if self.clint.is_accessible(pa) {
                    self.clint.store(pa, size, value)
                } else {
                    bus.store(pa, size, value)
                }
            }
            Err(e) => Err(e),
        }
    }

    // get the takable pending interrupt with the highest priority
    pub fn get_interrupt_to_take(&mut self) -> Option<Interrupt> {
        let xip = if self.mode == M_MODE {
            self.csr.load_csrs(MIP, self.cycle, &self.interrupt_list)
        } else {
            self.csr.load_csrs(SIP, self.cycle, &self.interrupt_list)
        };
        let xie = if self.mode == M_MODE {
            self.csr.load_csrs(MIE, self.cycle, &self.interrupt_list)
        } else {
            self.csr.load_csrs(SIE, self.cycle, &self.interrupt_list)
        };
        if xip & xie == 0 {
            return None;
        }

        // Collect to avoid borrow conflict when calling get_trap_mode(self)
        let candidates: Vec<Interrupt> = self.interrupt_list.iter().cloned().collect();
        for interrupt in &candidates {
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

    pub(crate) fn return_from_machine_trap(&mut self) {
        info!("mret instruction from mode {}", self.mode);
        debug!("{}", self.dump_registers());
        debug!("{}", self.csr.dump());
        let pp = self.csr.get_mstatus_bit(MASK_MPP, BIT_MPP);
        let pie = self.csr.get_mstatus_bit(MASK_MPIE, BIT_MPIE);
        let previous_pc = self.csr.load_csrs(MEPC, self.cycle, &self.interrupt_list);
        self.csr.set_mstatus_bit(pie, MASK_MIE, BIT_MIE);
        self.csr.set_mstatus_bit(0b1, MASK_MPIE, BIT_MPIE);
        self.csr.set_mstatus_bit(U_MODE, MASK_MPP, BIT_MPP);
        self.pc = previous_pc.wrapping_sub(4);
        self.mode = pp;
        info!("back to privilege {} from machine mode by mret", pp);
        debug!("return from trap");
        debug!("PC: 0x{:x}", previous_pc);
        debug!("csr dump");
        debug!("{}", self.csr.dump());
    }

    pub(crate) fn return_from_supervisor_trap(&mut self) {
        debug!("sret instruction from mode {}", self.mode);
        debug!("{}", self.dump_registers());
        debug!("{}", self.csr.dump());
        let pp = self.csr.get_sstatus_bit(MASK_SPP, BIT_SPP);
        let pie = self.csr.get_sstatus_bit(MASK_SPIE, BIT_SPIE);
        let previous_pc = self.csr.load_csrs(SEPC, self.cycle, &self.interrupt_list);
        self.csr.set_sstatus_bit(pie, MASK_SIE, BIT_SIE);
        self.csr.set_sstatus_bit(0b1, MASK_SPIE, BIT_SPIE);
        self.csr.set_sstatus_bit(U_MODE, MASK_SPP, BIT_SPP);
        self.pc = previous_pc.wrapping_sub(4);
        self.mode = pp;
        info!("back to privilege {} from supervisor mode by sret", pp);
        debug!("return from trap");
        debug!("PC: 0x{:x}", previous_pc);
        debug!("csr dump");
        debug!("{}", self.csr.dump());
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

    pub fn trap_interrupt(&mut self, bus: &mut Bus) {
        self.cycle += 1;
        if self.cycle % 1000000 == 0 {
            debug!("Cycle: {}", self.cycle);
        }

        bus.plic
            .process_pending_interrupts(&mut self.interrupt_list);

        self.update_pending_interrupts();

        if let Some(mut interrupt) = self.get_interrupt_to_take() {
            debug!("Interrupt: {:?} taken", interrupt);
            debug!("{}", self.csr.dump());
            interrupt.take_trap(self);
        }
    }

    pub fn build_basic_block(&mut self, bus: &mut Bus) -> Result<Arc<BasicBlock>, Exception> {
        let pc = self.pc;

        if let Some(block) = self.block_cache.get(&pc) {
            return Ok(Arc::clone(block));
        }

        let mut instrs = Vec::with_capacity(16);
        let mut cur_pc = pc;

        loop {
            let inst = match self.fetch(bus, cur_pc) {
                Ok(inst) => inst,
                Err(e) => {
                    error!("Failed to fetch instruction at pc={:x}: {:?}", cur_pc, e);
                    if instrs.is_empty() {
                        return Err(e);
                    }
                    break;
                }
            };
            let decoded_inst = DecodedInstr::decode(inst);
            let is_end = decoded_inst.is_building_block_end();
            instrs.push(decoded_inst);
            if is_end {
                break;
            }
            cur_pc = cur_pc.wrapping_add(4);
        }

        let block = Arc::new(BasicBlock {
            start_pc: pc,
            end_pc: cur_pc,
            instrs,
        });
        self.block_cache.insert(pc, Arc::clone(&block));
        Ok(block)
    }

    pub fn run_block(&mut self, bus: &mut Bus, block: &BasicBlock) -> u64 {
        self.pc = block.start_pc;
        let mut cycle: u64 = 0;
        trace!(
            "Block execution: 0x{:x} to 0x{:x}",
            block.start_pc,
            block.end_pc
        );
        for instr in &block.instrs {
            let result = self.execute(bus, instr.clone());
            if let Err(e) = result {
                error!(
                    "Execution failed in block at pc={:x}: {:?}, mode={}",
                    self.pc, e, self.mode
                );
                e.take_trap(self);
                self.pc = self.pc.wrapping_add(4);
                break;
            }
            self.regs[0] = 0;
            self.pc = self.pc.wrapping_add(4);
            self.cycle += 1;
            if self.dump_count > 0 {
                self.dump_count -= 1;
                if self.dump_count == 0 {
                    self.dump_count = self.dump_interval;
                    debug!(
                        "Block executed up to pc={:x}, cycle={}",
                        self.pc, self.cycle
                    );
                    debug!("{}", self.dump_registers());
                    debug!("CSR: {}", self.csr.dump());
                }
            }
            cycle += 1;
        }
        cycle
    }

    pub fn step_run(&mut self, bus: &mut Bus) -> u64 {
        trace!("pc={:>#18x}", self.pc);

        self.trap_interrupt(bus);

        let inst = match self.fetch(bus, self.pc) {
            Ok(inst) => inst,
            Err(_) => return 0x0,
        };

        let decoded_inst = DecodedInstr::decode(inst);

        let result = self
            .execute(bus, decoded_inst)
            .map_err(|e| e.take_trap(self));
        if let Err(e) = result {
            error!("Execution failed!");
            error!("Exception: {:?}", e);
            error!("pc=0x{:x}", self.pc);
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
        let stimecmp = self
            .csr
            .load_csrs(STIMECMP, self.cycle, &self.interrupt_list);
        let current_counter = self.cycle * TIMER_FREQ / CPU_FREQUENCY;
        if current_counter % 10000 == 0 {
            if current_counter % 1000000 == 0 {
                debug!(
                    "stimecmp: {}, current_counter: {}",
                    stimecmp, current_counter
                );
            }
            if (stimecmp > 0) && (current_counter >= stimecmp) {
                self.interrupt_list
                    .insert(Interrupt::SupervisorTimerInterrupt);
            } else {
                self.interrupt_list
                    .remove(&Interrupt::SupervisorTimerInterrupt);
            }
        }
    }
}
