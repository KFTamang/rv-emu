use crate::cpu::CPU_FREQUENCY;
use crate::interrupt::*;
use log::trace;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

#[derive(Serialize, Deserialize)]
pub struct CsrSnapshot {
    #[serde(with = "BigArray")]
    csr: [u64; 4096],
}

pub struct Csr {
    csr: [u64; 4096],
    cycle: Rc<RefCell<u64>>,
    interrupt_list: Rc<RefCell<BTreeSet<Interrupt>>>,
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
    pub fn new(interrupt_list: Rc<RefCell<BTreeSet<Interrupt>>>, cycle: Rc<RefCell<u64>>) -> Self {
        Self {
            csr: [0; 4096],
            interrupt_list,
            cycle,
        }
    }

    pub fn to_snapshot(&self) -> CsrSnapshot {
        CsrSnapshot { csr: self.csr }
    }

    pub fn from_snapshot(
        snapshot: CsrSnapshot,
        interrupt_list: Rc<RefCell<BTreeSet<Interrupt>>>,
        cycle: Rc<RefCell<u64>>,
    ) -> Self {
        Self {
            csr: snapshot.csr,
            interrupt_list,
            cycle,
        }
    }

    pub fn load_csrs(&self, addr: usize) -> u64 {
        match addr {
            SSTATUS => self.csr[MSTATUS] & SSTATUS_MASK,
            SIE => self.csr[MIE] & self.csr[MIDELEG],
            SIP => {
                let mut sip = 0 as u64;
                let interrupts = self.interrupt_list.borrow();
                for interrupt in interrupts.iter() {
                    sip |= interrupt.bit_code() | INTERRUPT_BIT;
                }
                sip & self.csr[MIDELEG]
            },
            TIME => self.get_time_ms() * TIMER_FREQ / 1000,
            MIP => {
                let mut mip = 0 as u64;
                let interrupts = self.interrupt_list.borrow();
                for interrupt in interrupts.iter() {
                    mip |= interrupt.bit_code() | INTERRUPT_BIT;
                }
                mip
            },
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
                // self.set_timer_interrupt(val);
            }
            _ => {
                self.csr[addr] = val;
            }
        }
    }

    pub fn set_mstatus_bit(&mut self, val: u64, mask: u64, bit: u64) {
        let mut current = self.load_csrs(MSTATUS);
        current &= !mask;
        current |= ((val as u64) << bit) & mask;
        self.store_csrs(MSTATUS, current);
    }

    pub fn get_mstatus_bit(&self, mask: u64, bit: u64) -> u64 {
        let status = self.load_csrs(MSTATUS);
        (status & mask) >> bit
    }

    pub fn set_sstatus_bit(&mut self, val: u64, mask: u64, bit: u64) {
        let mut current = self.load_csrs(SSTATUS);
        current &= !mask;
        current |= ((val as u64) << bit) & mask;
        self.store_csrs(SSTATUS, current);
    }

    pub fn get_sstatus_bit(&self, mask: u64, bit: u64) -> u64 {
        let status = self.load_csrs(SSTATUS);
        (status & mask) >> bit
    }

    fn get_time_ms(&self) -> u64 {
        *self.cycle.borrow() * 1000 / CPU_FREQUENCY
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

impl Drop for Csr {
    fn drop(&mut self) {
        // self.duration_sender.send(Option::None).unwrap();
        // if let Some(thread) = self.timer_thread.take() {
        //     thread.join().unwrap();
        // }
    }
}
