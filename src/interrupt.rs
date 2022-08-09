use crate::csr::*;

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
}