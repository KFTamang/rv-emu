use std::vec;

use crate::interrupt::*;
use serde::{Deserialize, Serialize};

const INTERRUPT_SOURCE_PRIORITIES: u64 = 0x000000;
const INTERRUPT_PENDING_BITS: u64 = 0x001000;
const INTERRUPT_ENABLES: u64 = 0x002000;
const PRIORITY_THRESHOLDS: u64 = 0x200000;
const CLAIM_COMPLETE: u64 = 0x200004;

#[derive(Clone, Serialize, Deserialize)]
pub struct Plic {
    start_addr: u64,
    size: u64,
    regs: Vec<u64>,
}

impl Plic {
    pub fn new(_start_addr: u64, _size: u64) -> Plic {
        Self {
            start_addr: _start_addr,
            size: _size,
            regs: vec![0; _size as usize / 8],
        }
    }

    pub fn is_accessible(&self, addr: u64) -> bool {
        (addr >= self.start_addr) && (addr < self.start_addr + self.size)
    }

    pub fn load(&self, _addr: u64, _size: u64) -> Result<u64, Exception> {
        Ok(0x0)
    }

    pub fn store(&mut self, _addr: u64, _size: u64, _value: u64) -> Result<(), Exception> {
        Ok(())
    }
}
