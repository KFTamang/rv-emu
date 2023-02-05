use crate::cpu::*;
use crate::csr::*;
use std::process::exit;

const INTERRUPT_BIT:u64 = 1 << 63;

pub enum Interrupt {
    SupervisorSoftwareInterrupt,
    MachineSoftwareInterrupt,
    SupervisorTimerInterrupt,
    MachineTimerInterrupt,
    SupervisorExternalInterrupt,
    MachineExternalInterrupt,
}

impl Interrupt {
    fn exception_code(&self) -> u64 {
        match self {
            SupervisorSoftwareInterrupt =>  1 | INTERRUPT_BIT,
            MachineSoftwareInterrupt    =>  3 | INTERRUPT_BIT,
            SupervisorTimerInterrupt    =>  5 | INTERRUPT_BIT,
            MachineTimerInterrupt       =>  7 | INTERRUPT_BIT,
            SupervisorExternalInterrupt =>  9 | INTERRUPT_BIT,
            MachineExternalInterrupt    => 11 | INTERRUPT_BIT,        
        }
    }
    pub fn take_trap(&mut self, cpu: &mut Cpu) {

        let exception_code = self.exception_code();
        let target_mode = self.get_trap_mode(cpu);
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
    fn get_trap_mode(&self, cpu: &mut Cpu) -> Result<u64, ()> {
        // An interrupt i will trap to M-mode (causing the privilege mode to change to M-mode)
        // if all of the following are true: 
        // (a) either the current privilege mode is M and the MIE bit in the mstatus register is set,
        //  or the current privilege mode has less privilege than M-mode; 
        // (b) bit i is set in both mip and mie; and 
        // (c) if register mideleg exists, bit i is not set in mideleg.

        // (a)
        let mstatus = cpu.csr.load_csrs(MSTATUS);
        if !(((cpu.mode == M_MODE) && (mstatus & MASK_MIE != 0)) || (cpu.mode < M_MODE)) {
            return Err(());
        }

        // (b)
        let i = self.exception_code();
        let bit_i = 0b1 << i;
        let mip = cpu.csr.load_csrs(MIP);
        let mie = cpu.csr.load_csrs(MIE);
        if !((mie & bit_i != 0) && (mip & bit_i != 0)) {
            return Err(());
        }
        let mideleg = cpu.csr.load_csrs(MIDELEG);
        if (cpu.mode < M_MODE) && ((exception_bit & mideleg) != 0) {
            S_MODE
        } else {
            M_MODE
        }
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
        let bit_shift = self.exception_code();
        let exception_bit = 0b1 << bit_shift;
        let medeleg = cpu.csr.load_csrs(MEDELEG);
        if (cpu.mode < M_MODE) && ((exception_bit & medeleg) != 0) {
            S_MODE
        } else {
            M_MODE
        }
    }
}
