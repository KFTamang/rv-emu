use crate::interrupt::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Uart {
    start_addr: u64,
    size: u64,
}

#[allow(unused)]
const REG_RHR_THR: u64 = 0;
#[allow(unused)]
const REG_IER: u64 = 1;
#[allow(unused)]
const REG_FCR_ISR: u64 = 2;
#[allow(unused)]
const REG_LCR: u64 = 3;
#[allow(unused)]
const REG_MCR: u64 = 4;
const REG_LSR: u64 = 5;
#[allow(unused)]
const REG_MSR: u64 = 6;
#[allow(unused)]
const REG_SPR: u64 = 7;

#[allow(unused)]
const RECEIVE_DATA_READY: u64 = 1 << 0;
#[allow(unused)]
const OVERRUN_ERROR: u64 = 1 << 1;
#[allow(unused)]
const PARITY_ERROR: u64 = 1 << 2;
#[allow(unused)]
const FRAMING_ERROR: u64 = 1 << 3;
#[allow(unused)]
const BREAK_INTERRUPT: u64 = 1 << 4;
const TRANSMIT_HOLDING_EMPTY: u64 = 1 << 5;
const TRANSMIT_EMPTY: u64 = 1 << 6;
#[allow(unused)]
const FIFO_ERROR: u64 = 1 << 7;

impl Uart {
    pub fn new(_start_addr: u64, _size: u64) -> Uart {
        Self {
            start_addr: _start_addr,
            size: _size,
        }
    }

    pub fn is_accessible(&self, addr: u64) -> bool {
        (addr >= self.start_addr) && (addr < self.start_addr + self.size)
    }

    pub fn load(&self, addr: u64, size: u64) -> Result<u64, Exception> {
        if size != 8 {
            return Err(Exception::LoadAccessFault);
        }
        let actual_addr = addr - self.start_addr;
        match actual_addr {
            REG_LSR => {
                // returns TRANSMIT_EMPTY | TRANSMIT_HOLDING_EMPTY,
                // assuming infinitely fast UART, with FIFO being always empty
                Ok(TRANSMIT_EMPTY | TRANSMIT_HOLDING_EMPTY)
            }
            _ => Ok(0x0),
        }
    }

    pub fn store(&mut self, addr: u64, size: u64, value: u64) -> Result<(), Exception> {
        if size != 8 {
            return Err(Exception::LoadAccessFault);
        }
        let actual_addr = addr - self.start_addr;
        match actual_addr {
            REG_RHR_THR => {
                print!("{}", value as u8 as char);
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
