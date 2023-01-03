use crate::cpu::*;
use crate::csr::*;
use std::process::exit;

pub struct Interrupt {
    pending_interrupt: Option<u32>,
}

impl Interrupt {
    pub fn new() -> Self {
        Self {
            pending_interrupt: None,
        }
    }

    pub fn get_pending_interrupt(&self) -> Option<u32> {
        self.pending_interrupt
    }

    pub fn cause_interrupt(&mut self, csr: &mut Csr, i: u32) {
        self.pending_interrupt = Some(i);
        let val = csr.load_csrs(MSTATUS) | (1u64 << i);
        csr.store_csrs(MSTATUS, val);
        let cause = (1u64 << 63) | i as u64;
        csr.store_csrs(MCAUSE, cause);
    }
}

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
    fn exception_code(&self) -> u64 {
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

    pub fn take_trap(&mut self, cpu: &mut Cpu) {
        let exception_code = self.exception_code();
        let target_mode = self.get_target_mode(cpu);
        match target_mode {
            M_MODE => {
                cpu.csr.store_csrs(MEPC, cpu.pc);
                cpu.csr.store_csrs(MCAUSE, exception_code);
                cpu.csr.set_mstatus_bit(cpu.mode, MASK_MPP, BIT_MPP);
                let mie = MASK_MIE & cpu.csr.load_csrs(MSTATUS);
                cpu.csr.set_mstatus_bit(if mie > 0 { 1 } else { 0 }, MASK_MPIE, BIT_MPIE);
                cpu.csr.set_mstatus_bit(0, MASK_MIE, MASK_MIE);

                let mtvec = cpu.csr.load_csrs(MTVEC);
                cpu.log(format!("mtvec is {}\n", mtvec));
                eprintln!("enter M mode\n");
                match mtvec & 0x3 {
                    0x0 => {
                        cpu.pc = (mtvec & 0xfffffffc).wrapping_sub(4);
                    }
                    0x1 => {}
                    _ => {
                        cpu.log(format!("Exception Error, this should not be reached!"));
                        exit(1);
                    }
                }
            },
            S_MODE => {
                cpu.csr.store_csrs(SEPC, cpu.pc);
                cpu.csr.store_csrs(SCAUSE, exception_code);
                cpu.csr.set_sstatus_bit(cpu.mode, MASK_SPP, BIT_SPP);
                let sie = MASK_SIE & cpu.csr.load_csrs(SSTATUS);
                cpu.csr.set_sstatus_bit(if sie > 0 { 1 } else { 0 }, MASK_SPIE, BIT_SPIE);
                cpu.csr.set_sstatus_bit(0, MASK_SIE, BIT_SIE);

                let stvec = cpu.csr.load_csrs(STVEC);
                cpu.log(format!("stvec is {}", stvec));
                eprintln!("enter S mode");
                match stvec & 0x3 {
                    0x0 => {
                        cpu.pc = (stvec & 0xfffffffc).wrapping_sub(4);
                    }
                    0x1 => {}
                    _ => {
                        cpu.log(format!("Exception Error, this should not be reached!"));
                        exit(1);
                    }
                }
            }
            _ => {}
        }
        cpu.log(format!("Exception:{} occurred!", self.exception_code()));
    }

    fn get_target_mode(&self, cpu: &mut Cpu) -> u64 {
        return M_MODE;
    }
}
