use crate::cpu::CPU_FREQUENCY;
use crate::interrupt::*;
use log::trace;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::collections::BTreeSet;

#[derive(Serialize, Deserialize)]
pub struct CsrSnapshot {
    #[serde(with = "BigArray")]
    pub csr: [u64; 4096],
}

pub struct Csr {
    csr: [u64; 4096],
}

pub const SSTATUS: usize = 0x100;
pub const SIE: usize = 0x104;
pub const STVEC: usize = 0x105;
pub const SEPC: usize = 0x141;
pub const SCAUSE: usize = 0x142;
pub const STVAL: usize = 0x143;
pub const SIP: usize = 0x144;
// Sstc extension for supervisor timer registers
pub const STIMECMP: usize = 0x14D;

pub const SATP: usize = 0x180;

pub const MSTATUS: usize = 0x300;
pub const MEDELEG: usize = 0x302;
pub const MIDELEG: usize = 0x303;
pub const MIE: usize = 0x304;
pub const MTVEC: usize = 0x305;
pub const MEPC: usize = 0x341;
pub const MCAUSE: usize = 0x342;
pub const MTVAL: usize = 0x343;
pub const MIP: usize = 0x344;

pub const TIME: usize = 0xc01;

pub const BIT_SXL: u64 = 34;
pub const BIT_TSR: u64 = 22;
pub const BIT_TW: u64 = 21;
pub const BIT_TVM: u64 = 20;
pub const BIT_MPRV: u64 = 17;
pub const BIT_MPP: u64 = 11;
pub const BIT_MPIE: u64 = 7;
pub const BIT_MIE: u64 = 3;
pub const BIT_SPP: u64 = 8;
pub const BIT_SPIE: u64 = 5;
pub const BIT_SIE: u64 = 1;

pub const MASK_SXL: u64 = 0b11 << BIT_SXL;
pub const MASK_TSR: u64 = 0b1 << BIT_TSR;
pub const MASK_TW: u64 = 0b1 << BIT_TW;
pub const MASK_TVM: u64 = 0b1 << BIT_TVM;
pub const MASK_SPP: u64 = 0b11 << BIT_SPP;
pub const MASK_SPIE: u64 = 0b1 << BIT_SPIE;
pub const MASK_SIE: u64 = 0b1 << BIT_SIE;
pub const MASK_MPRV: u64 = 0b1 << BIT_MPRV;
pub const MASK_MPP: u64 = 0b11 << BIT_MPP;
pub const MASK_MPIE: u64 = 0b1 << BIT_MPIE;
pub const MASK_MIE: u64 = 0b1 << BIT_MIE;
const SSTATUS_MASK: u64 = !(MASK_SXL
    | MASK_TSR
    | MASK_TSR
    | MASK_TW
    | MASK_TVM
    | MASK_MPRV
    | MASK_MPP
    | MASK_MPIE
    | MASK_MIE);

pub const TIMER_FREQ: u64 = 10000000; // 10 MHz

impl Csr {
    pub fn new() -> Self {
        Self { csr: [0; 4096] }
    }

    pub fn to_snapshot(&self) -> CsrSnapshot {
        CsrSnapshot { csr: self.csr }
    }

    pub fn from_snapshot(snapshot: CsrSnapshot) -> Self {
        Self { csr: snapshot.csr }
    }

    pub fn load_csrs(&self, addr: usize, cycle: u64, interrupts: &BTreeSet<Interrupt>) -> u64 {
        match addr {
            SSTATUS => self.csr[MSTATUS] & SSTATUS_MASK,
            SIE => self.csr[MIE] & self.csr[MIDELEG],
            SIP => {
                let mut sip = 0u64;
                for interrupt in interrupts.iter() {
                    sip |= interrupt.bit_code() | INTERRUPT_BIT;
                }
                sip & self.csr[MIDELEG]
            }
            TIME => cycle * 1000 / CPU_FREQUENCY * TIMER_FREQ / 1000,
            MIP => {
                let mut mip = 0u64;
                for interrupt in interrupts.iter() {
                    mip |= interrupt.bit_code() | INTERRUPT_BIT;
                }
                mip
            }
            _ => self.csr[addr],
        }
    }

    pub fn store_csrs(&mut self, addr: usize, val: u64) {
        trace!("store: addr:{:#x}, val:{:#x}", addr, val);
        match addr {
            SSTATUS => {
                self.csr[MSTATUS] = val & SSTATUS_MASK;
            }
            SIE => {
                self.csr[MIE] = val & self.csr[MIDELEG];
            }
            SIP => {
                self.csr[MIP] = val & self.csr[MIDELEG];
            }
            STIMECMP => {
                self.csr[STIMECMP] = val;
            }
            _ => {
                self.csr[addr] = val;
            }
        }
    }

    pub fn set_mstatus_bit(&mut self, val: u64, mask: u64, bit: u64) {
        let mut current = self.csr[MSTATUS];
        current &= !mask;
        current |= ((val as u64) << bit) & mask;
        self.store_csrs(MSTATUS, current);
    }

    pub fn get_mstatus_bit(&self, mask: u64, bit: u64) -> u64 {
        let status = self.csr[MSTATUS];
        (status & mask) >> bit
    }

    pub fn set_sstatus_bit(&mut self, val: u64, mask: u64, bit: u64) {
        let mut current = self.csr[MSTATUS] & SSTATUS_MASK;
        current &= !mask;
        current |= ((val as u64) << bit) & mask;
        self.store_csrs(SSTATUS, current);
    }

    pub fn get_sstatus_bit(&self, mask: u64, bit: u64) -> u64 {
        let status = self.csr[MSTATUS] & SSTATUS_MASK;
        (status & mask) >> bit
    }

    pub fn dump(&self) -> String {
        let mut result = String::new();
        for i in 0..4096 {
            if i % 16 == 0 {
                result.push_str(&format!("\n{:#x} ", i));
            }
            result.push_str(&format!("{:#x} ", self.csr[i]));
        }
        result.push_str("\n");
        result
    }
}
