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

    pub fn take_trap(&self, cpu: &mut Cpu) {
        let exception_code = self.exception_code();
        let target_mode = self.get_target_mode(cpu);
        match target_mode {
            M_MODE => {
                cpu.csr.store_csrs(MEPC, cpu.pc);
                cpu.csr.store_csrs(MCAUSE, exception_code);
                cpu.csr.set_mstatus_mpp(cpu.mode);
                let mie = MASK_MIE & cpu.csr.load_csrs(MSTATUS);
                cpu.csr.set_mstatus_mpie(if mie > 0 { 1 } else { 0 });
                cpu.csr.set_mstatus_mie(0);

                let mtvec = cpu.csr.load_csrs(MTVEC);
                println!("mtvec is {}", mtvec);
                match mtvec & 0x3 {
                    0x0 => {
                        cpu.pc = (mtvec & 0xfffffffc).wrapping_sub(4);
                    }
                    0x1 => {}
                    _ => {
                        println!("Exception Error, this should not be reached!");
                        exit(1);
                    }
                }
            }
            _ => {}
        }
        println!("Exception:{} occurred!", self.exception_code());
    }

    fn get_target_mode(&self, cpu: &mut Cpu) -> u32 {
        return M_MODE;
    }
}
