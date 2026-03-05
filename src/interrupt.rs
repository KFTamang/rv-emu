use crate::cpu::*;
use crate::csr::*;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use core::panic;
use std::process::exit;

pub const INTERRUPT_BIT: u64 = 1 << 63;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Interrupt {
    SupervisorSoftwareInterrupt,
    MachineSoftwareInterrupt,
    SupervisorTimerInterrupt,
    MachineTimerInterrupt,
    SupervisorExternalInterrupt,
    MachineExternalInterrupt,
}

impl PartialOrd for Interrupt {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Interrupt {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.code().cmp(&other.code())
    }
}

impl Interrupt {
    pub fn code(&self) -> u64 {
        match self {
            Interrupt::SupervisorSoftwareInterrupt => 1,
            Interrupt::MachineSoftwareInterrupt => 3,
            Interrupt::SupervisorTimerInterrupt => 5,
            Interrupt::MachineTimerInterrupt => 7,
            Interrupt::SupervisorExternalInterrupt => 9,
            Interrupt::MachineExternalInterrupt => 11,
        }
    }
    pub fn bit_code(&self) -> u64 {
        INTERRUPT_BIT | (1 << self.code())
    }
    pub fn take_trap(&mut self, cpu: &mut Cpu) {
        let cause = INTERRUPT_BIT | self.code();
        let target_mode = self.get_trap_mode(cpu);
        debug!(
            "Taking trap for interrupt: {:?}, cause: 0x{:x}, target mode: {}",
            self,
            cause,
            target_mode.unwrap()
        );
        match target_mode {
            Ok(M_MODE) => {
                cpu.csr.store_csrs(MEPC, cpu.pc);
                cpu.csr.store_csrs(MCAUSE, cause);
                cpu.csr.set_mstatus_bit(cpu.mode, MASK_MPP, BIT_MPP);
                let mie = MASK_MIE & cpu.csr.load_csrs(MSTATUS, cpu.cycle, &cpu.interrupt_list);
                cpu.csr
                    .set_mstatus_bit(if mie > 0 { 1 } else { 0 }, MASK_MPIE, BIT_MPIE);
                cpu.csr.set_mstatus_bit(0, MASK_MIE, MASK_MIE);
                cpu.mode = target_mode.unwrap();
                let mtvec = cpu.csr.load_csrs(MTVEC, cpu.cycle, &cpu.interrupt_list);
                debug!("MEPC is 0x{:x}", cpu.csr.load_csrs(MEPC, cpu.cycle, &cpu.interrupt_list));
                debug!("MCAUSE is 0x{:x}", cpu.csr.load_csrs(MCAUSE, cpu.cycle, &cpu.interrupt_list));
                debug!("MSTATUS is 0x{:x}", cpu.csr.load_csrs(MSTATUS, cpu.cycle, &cpu.interrupt_list));
                debug!("MTVEC is 0x{:x}", mtvec);
                debug!("enter M mode");
                match mtvec & 0x3 {
                    0x0 => {
                        cpu.pc = (mtvec & 0xffff_ffff_ffff_fffc);
                    }
                    0x1 => {}
                    _ => {
                        error!("Interrupt Error, this should not be reached!");
                        exit(1);
                    }
                }
            }
            Ok(S_MODE) => {
                cpu.csr.store_csrs(SEPC, cpu.pc);
                cpu.csr.store_csrs(SCAUSE, cause);
                cpu.csr.set_sstatus_bit(cpu.mode, MASK_SPP, BIT_SPP);
                let sie = MASK_SIE & cpu.csr.load_csrs(SSTATUS, cpu.cycle, &cpu.interrupt_list);
                cpu.csr
                    .set_sstatus_bit(if sie > 0 { 1 } else { 0 }, MASK_SPIE, BIT_SPIE);
                cpu.csr.set_sstatus_bit(0, MASK_SIE, BIT_SIE);
                cpu.mode = target_mode.unwrap();
                let stvec = cpu.csr.load_csrs(STVEC, cpu.cycle, &cpu.interrupt_list);
                debug!("SEPC is 0x{:x}", cpu.csr.load_csrs(SEPC, cpu.cycle, &cpu.interrupt_list));
                debug!("SCAUSE is 0x{:x}", cpu.csr.load_csrs(SCAUSE, cpu.cycle, &cpu.interrupt_list));
                debug!("SSTATUS is 0x{:x}", cpu.csr.load_csrs(SSTATUS, cpu.cycle, &cpu.interrupt_list));
                debug!("STVEC is 0x{:x}", stvec);
                debug!("enter S mode");
                match stvec & 0x3 {
                    0x0 => {
                        cpu.pc = (stvec & 0xffff_ffff_ffff_fffc);
                    }
                    0x1 => {}
                    _ => {
                        error!("Interrupt Error, this should not be reached!");
                        exit(1);
                    }
                }
            }
            _ => {
                error!("Interrupt Error, this should not be reached!");
                error!("pc=0x{:x}", cpu.pc);
                error!("current mode: {:?}, target_mode: {:?}", cpu.mode, target_mode);
                error!("CSR dump");
                error!("{}", cpu.csr.dump());
                panic!("Interrupt Error, this should not be reached!");
            }
        }
        debug!("Interrupt:{:?} occurred!", self);
    }
    pub fn get_trap_mode(&self, cpu: &Cpu) -> Result<u64, ()> {
        let bit_i = self.bit_code();
        let mideleg = cpu.csr.load_csrs(MIDELEG, cpu.cycle, &cpu.interrupt_list);
        let destined_mode = if (bit_i & mideleg) == 0 {
            M_MODE
        } else {
            S_MODE
        };

        let current_mode = cpu.mode;
        match destined_mode {
            M_MODE => {
                let mip = cpu.csr.load_csrs(MIP, cpu.cycle, &cpu.interrupt_list);
                let mie = cpu.csr.load_csrs(MIE, cpu.cycle, &cpu.interrupt_list);
                let mstatus = cpu.csr.load_csrs(MSTATUS, cpu.cycle, &cpu.interrupt_list);
                if (mip & mie & bit_i) == 0 {
                    return Err(());
                }
                if current_mode < M_MODE {
                    return Ok(M_MODE);
                }
                if mstatus & MASK_MIE != 0 {
                    return Ok(M_MODE);
                }
                return Err(());
            }
            S_MODE => {
                let sip = cpu.csr.load_csrs(SIP, cpu.cycle, &cpu.interrupt_list);
                let sie = cpu.csr.load_csrs(SIE, cpu.cycle, &cpu.interrupt_list);
                let sstatus = cpu.csr.load_csrs(SSTATUS, cpu.cycle, &cpu.interrupt_list);

                if current_mode == M_MODE {
                    return Err(());
                }
                if (sip & sie & bit_i) == 0 {
                    return Err(());
                }
                if (sstatus & MASK_SIE) != 0 {
                    return Ok(S_MODE);
                }
                return Err(());
            }
            _ => {
                return Err(());
            }
        }
    }
}

#[allow(unused)]
#[derive(Debug)]
pub enum Exception {
    InstructionAddressMissaligned,
    InstructionAccessFault,
    IllegalInstruction(u32),
    BreakPoint,
    LoadAddressMissaligned,
    LoadAccessFault,
    StoreAMOAddressMisaligned,
    StoreAMOAccessFault,
    EnvironmentalCallFromUMode,
    EnvironmentalCallFromSMode,
    EnvironmentalCallFromMMode,
    InstructionPageFault(u32),
    LoadPageFault(u32),
    StoreAMOPageFault(u32),
}

impl Exception {
    pub fn code(&self) -> u64 {
        match self {
            Exception::InstructionAddressMissaligned => 0,
            Exception::InstructionAccessFault => 1,
            Exception::IllegalInstruction(_) => 2,
            Exception::BreakPoint => 3,
            Exception::LoadAddressMissaligned => 4,
            Exception::LoadAccessFault => 5,
            Exception::StoreAMOAddressMisaligned => 6,
            Exception::StoreAMOAccessFault => 7,
            Exception::EnvironmentalCallFromUMode => 8,
            Exception::EnvironmentalCallFromSMode => 9,
            Exception::EnvironmentalCallFromMMode => 11,
            Exception::InstructionPageFault(_) => 12,
            Exception::LoadPageFault(_) => 13,
            Exception::StoreAMOPageFault(_) => 15,
        }
    }

    pub fn bit_code(&self) -> u64 {
        1 << self.code()
    }

    pub fn take_trap(&self, cpu: &mut Cpu) {
        let cause = self.code();
        let target_mode = self.get_target_mode(cpu);
        let xtval = match self {
            Exception::InstructionPageFault(v) => *v,
            Exception::LoadPageFault(v) => *v,
            Exception::StoreAMOPageFault(v) => *v,
            _ => 0,
        } as u64;
        match target_mode {
            M_MODE => {
                cpu.csr.store_csrs(MEPC, cpu.pc);
                cpu.csr.store_csrs(MCAUSE, cause);
                cpu.csr.set_mstatus_bit(cpu.mode, MASK_MPP, BIT_MPP);
                let mie = MASK_MIE & cpu.csr.load_csrs(MSTATUS, cpu.cycle, &cpu.interrupt_list);
                cpu.csr
                    .set_mstatus_bit(if mie > 0 { 1 } else { 0 }, MASK_MPIE, BIT_MPIE);
                cpu.csr.set_mstatus_bit(0, MASK_MIE, MASK_MIE);
                cpu.csr.store_csrs(MTVAL, xtval);
                cpu.mode = target_mode;
                let mtvec = cpu.csr.load_csrs(MTVEC, cpu.cycle, &cpu.interrupt_list);
                debug!("mtvec is 0x{:x}", mtvec);
                debug!("enter M mode");
                match mtvec & 0x3 {
                    0x0 => {
                        cpu.pc = (mtvec & 0xffff_ffff_ffff_fffc).wrapping_sub(4);
                    }
                    0x1 => {}
                    _ => {
                        error!("Exception Error, this should not be reached!");
                        exit(1);
                    }
                }
            }
            S_MODE => {
                cpu.csr.store_csrs(SEPC, cpu.pc);
                cpu.csr.store_csrs(SCAUSE, cause);
                cpu.csr.set_sstatus_bit(cpu.mode, MASK_SPP, BIT_SPP);
                let sie = MASK_SIE & cpu.csr.load_csrs(SSTATUS, cpu.cycle, &cpu.interrupt_list);
                cpu.csr
                    .set_sstatus_bit(if sie > 0 { 1 } else { 0 }, MASK_SPIE, BIT_SPIE);
                cpu.csr.set_sstatus_bit(0, MASK_SIE, BIT_SIE);
                cpu.csr.store_csrs(STVAL, xtval);
                cpu.mode = target_mode;
                let stvec = cpu.csr.load_csrs(STVEC, cpu.cycle, &cpu.interrupt_list);
                debug!("stvec is 0x{:x}", stvec);
                debug!("enter S mode");
                match stvec & 0x3 {
                    0x0 => {
                        cpu.pc = (stvec & 0xffff_ffff_ffff_fffc).wrapping_sub(4);
                    }
                    0x1 => {}
                    _ => {
                        error!("Exception Error, this should not be reached!");
                        exit(1);
                    }
                }
            }
            _ => {
                error!("Exception Error, this should not be reached!");
                error!("pc=0x{:x}", cpu.pc);
                error!("current mode: {:?}, target_mode: {:?}", cpu.mode, target_mode);
                error!("CSR dump");
                error!("{}", cpu.csr.dump());
                panic!("Exception Error, this should not be reached!");
            }
        }
        info!("Exception:{} occurred!", self.code());
    }

    fn get_target_mode(&self, cpu: &mut Cpu) -> u64 {
        let exception_bit = self.bit_code();
        let medeleg = cpu.csr.load_csrs(MEDELEG, cpu.cycle, &cpu.interrupt_list);
        if (cpu.mode < M_MODE) && ((exception_bit & medeleg) != 0) {
            S_MODE
        } else {
            M_MODE
        }
    }
}
