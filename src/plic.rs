use std::vec;
use std::sync::{Arc, Mutex};

use crate::interrupt::*;
use serde::{Deserialize, Serialize};

const PLIC_SIZE: u64 = 0x4000000;

const INTERRUPT_SOURCE_PRIORITIES: u64 = 0x000000;
const INTERRUPT_PENDING_BITS: u64 = 0x001000;
const INTERRUPT_ENABLES: u64 = 0x002000;
const PRIORITY_THRESHOLDS: u64 = 0x200000;
const CLAIM_COMPLETE: u64 = 0x200004;

#[derive(Clone, Serialize, Deserialize)]
pub struct PlicSnapshot {
    pub start_addr: u64,
    pub regs: Vec<u64>,
}

pub struct Plic {
    start_addr: u64,
    regs: Vec<u64>,
    interrupt_list: Arc<Mutex<Vec<DelayedInterrupt>>>,
}

impl Plic {
    pub fn new(_start_addr: u64, interrupt_list: Arc<Mutex<Vec<DelayedInterrupt>>>) -> Plic {
        Self {
            start_addr: _start_addr,
            regs: vec![0; PLIC_SIZE as usize / 8],
            interrupt_list,
        }
    }

    pub fn is_accessible(&self, addr: u64) -> bool {
        (addr >= self.start_addr) && (addr < self.start_addr + PLIC_SIZE)
    }

    pub fn load(&self, _addr: u64, _size: u64) -> Result<u64, Exception> {
        Ok(0x0)
    }

    pub fn store(&mut self, _addr: u64, _size: u64, _value: u64) -> Result<(), Exception> {
        Ok(())
    }

    
    pub fn from_snapshot(snapshot: PlicSnapshot, interrupt_list: Arc<Mutex<Vec<DelayedInterrupt>>>) -> Plic {
        Plic {
            start_addr: snapshot.start_addr,
            regs: snapshot.regs,
            interrupt_list,
        }
    }

    pub fn to_snapshot(&self) -> PlicSnapshot {
        PlicSnapshot {
            start_addr: self.start_addr,
            regs: self.regs.clone(),
        }
    }
}
