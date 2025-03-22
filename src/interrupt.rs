use crate::cpu::*;
use crate::csr::*;
use std::process::exit;
use log::info;

const INTERRUPT_BIT: u64 = 1 << 63;

#[derive(Clone, Copy, Debug)]
pub enum Interrupt {
    SupervisorSoftwareInterrupt,
    MachineSoftwareInterrupt,
    SupervisorTimerInterrupt,
    MachineTimerInterrupt,
    SupervisorExternalInterrupt,
    MachineExternalInterrupt,
}

impl Interrupt {
    pub const PRIORITY_ORDER: [Interrupt; 6] = [
        Interrupt::MachineExternalInterrupt,    //MEI
        Interrupt::MachineSoftwareInterrupt,    //MSI
        Interrupt::MachineTimerInterrupt,       //MTI
        Interrupt::SupervisorExternalInterrupt, //SEI
        Interrupt::SupervisorSoftwareInterrupt, //SSI
        Interrupt::SupervisorTimerInterrupt,    //STI
    ]; 

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
        let exception_code = self.bit_code();
        let target_mode = self.get_trap_mode(cpu);
        match target_mode {
            Ok(M_MODE) => {
                cpu.csr.store_csrs(MEPC, cpu.pc);
                cpu.csr.store_csrs(MCAUSE, exception_code);
                cpu.csr.set_mstatus_bit(cpu.mode, MASK_MPP, BIT_MPP);
                let mie = MASK_MIE & cpu.csr.load_csrs(MSTATUS);
                cpu.csr
                    .set_mstatus_bit(if mie > 0 { 1 } else { 0 }, MASK_MPIE, BIT_MPIE);
                cpu.csr.set_mstatus_bit(0, MASK_MIE, MASK_MIE);

                let mtvec = cpu.csr.load_csrs(MTVEC);
                info!("MEPC is 0x{:x}", cpu.csr.load_csrs(MEPC));
                info!("MCAUSE is 0x{:x}", cpu.csr.load_csrs(MCAUSE));
                info!("MSTATUS is 0x{:x}", cpu.csr.load_csrs(MSTATUS));
                info!("MTVEC is 0x{:x}", mtvec);
                info!("enter M mode");
                match mtvec & 0x3 {
                    0x0 => {
                        cpu.pc = (mtvec & 0xfffffffc).wrapping_sub(4);
                    }
                    0x1 => {}
                    _ => {
                        info!("Exception Error, this should not be reached!");
                        exit(1);
                    }
                }
            }
            Ok(S_MODE) => {
                cpu.csr.store_csrs(SEPC, cpu.pc);
                cpu.csr.store_csrs(SCAUSE, exception_code);
                cpu.csr.set_sstatus_bit(cpu.mode, MASK_SPP, BIT_SPP);
                let sie = MASK_SIE & cpu.csr.load_csrs(SSTATUS);
                cpu.csr
                    .set_sstatus_bit(if sie > 0 { 1 } else { 0 }, MASK_SPIE, BIT_SPIE);
                cpu.csr.set_sstatus_bit(0, MASK_SIE, BIT_SIE);

                let stvec = cpu.csr.load_csrs(STVEC);
                info!("stvec is 0x{:x}", stvec);
                info!("enter S mode");
                match stvec & 0x3 {
                    0x0 => {
                        cpu.pc = (stvec & 0xfffffffc).wrapping_sub(4);
                    }
                    0x1 => {}
                    _ => {
                        info!("Exception Error, this should not be reached!");
                        exit(1);
                    }
                }
            }
            _ => {}
        }
        info!("Exception:{:?} occurred!", self);
    }
    pub fn get_trap_mode(&self, cpu: &mut Cpu) -> Result<u64, ()> {
        // An interrupt i will be taken
        // (a)if bit i is set in both mip and mie,
        // (b)and if interrupts are globally enabled.
        // By default, M-mode interrupts are globally enabled
        // (b-1)if the hart’s current privilege mode is less than M,
        // (b-2)or if the current privilege mode is M and the MIE bit in the mstatus register is set.
        // (c)If bit i in mideleg is set, however, interrupts are considered to be globally enabled
        // if the hart’s current privilege mode equals the delegated privilege mode and that mode’s interrupt enable bit (xIE in mstatus for mode x) is set,
        // or if the current privilege mode is less than the delegated privilege mode.
        let bit_i = self.bit_code();
        let mideleg = cpu.csr.load_csrs(MIDELEG);
        let destined_mode = if (bit_i & mideleg) == 0 {
            M_MODE
        } else {
            S_MODE
        };

        match destined_mode {
            M_MODE => {
                let mstatus = cpu.csr.load_csrs(MSTATUS);
                if !(((cpu.mode == M_MODE) && (mstatus & MASK_MIE != 0)) || (cpu.mode < M_MODE)) {
                    return Err(());
                }
                let mip = cpu.csr.load_csrs(MIP);
                let mie = cpu.csr.load_csrs(MIE);
                if !((mie & bit_i != 0) && (mip & bit_i != 0)) {
                    return Err(());
                }
            }
            S_MODE => {
                let sstatus = cpu.csr.load_csrs(SSTATUS);
                if !(((cpu.mode == S_MODE) && (sstatus & MASK_SIE != 0)) || (cpu.mode < S_MODE)) {
                    return Err(());
                }
                let sip = cpu.csr.load_csrs(SIP);
                let sie = cpu.csr.load_csrs(SIE);
                if !((sie & bit_i != 0) && (sip & bit_i != 0)) {
                    return Err(());
                }
            }
            _ => {
                return Err(());
            }
        }
        Ok(destined_mode)
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

    pub fn take_trap(&mut self, cpu: &mut Cpu) {
        let exception_code = self.code();
        let target_mode = self.get_target_mode(cpu);
        match target_mode {
            M_MODE => {
                cpu.csr.store_csrs(MEPC, cpu.pc);
                cpu.csr.store_csrs(MCAUSE, exception_code);
                cpu.csr.set_mstatus_bit(cpu.mode, MASK_MPP, BIT_MPP);
                let mie = MASK_MIE & cpu.csr.load_csrs(MSTATUS);
                cpu.csr
                    .set_mstatus_bit(if mie > 0 { 1 } else { 0 }, MASK_MPIE, BIT_MPIE);
                cpu.csr.set_mstatus_bit(0, MASK_MIE, MASK_MIE);

                let mtvec = cpu.csr.load_csrs(MTVEC);
                info!("mtvec is {}\n", mtvec);
                info!("enter M mode\n");
                match mtvec & 0x3 {
                    0x0 => {
                        cpu.pc = (mtvec & 0xfffffffc).wrapping_sub(4);
                    }
                    0x1 => {}
                    _ => {
                        info!("Exception Error, this should not be reached!");
                        exit(1);
                    }
                }
            }
            S_MODE => {
                cpu.csr.store_csrs(SEPC, cpu.pc);
                cpu.csr.store_csrs(SCAUSE, exception_code);
                cpu.csr.set_sstatus_bit(cpu.mode, MASK_SPP, BIT_SPP);
                let sie = MASK_SIE & cpu.csr.load_csrs(SSTATUS);
                cpu.csr
                    .set_sstatus_bit(if sie > 0 { 1 } else { 0 }, MASK_SPIE, BIT_SPIE);
                cpu.csr.set_sstatus_bit(0, MASK_SIE, BIT_SIE);

                let stvec = cpu.csr.load_csrs(STVEC);
                info!("stvec is 0x{:x}", stvec);
                info!("enter S mode");
                match stvec & 0x3 {
                    0x0 => {
                        cpu.pc = (stvec & 0xfffffffc).wrapping_sub(4);
                    }
                    0x1 => {}
                    _ => {
                        info!("Exception Error, this should not be reached!");
                        exit(1);
                    }
                }
            }
            _ => {}
        }
        info!("Exception:{} occurred!", self.code());
    }

    fn get_target_mode(&self, cpu: &mut Cpu) -> u64 {
        let exception_bit = self.bit_code();
        let medeleg = cpu.csr.load_csrs(MEDELEG);
        if (cpu.mode < M_MODE) && ((exception_bit & medeleg) != 0) {
            S_MODE
        } else {
            M_MODE
        }
    }
}
