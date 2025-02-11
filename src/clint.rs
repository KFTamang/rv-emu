use crate::interrupt::*;
use log::{debug, info};

const FREQUENCY: u64 = 10000000; // clock frequency: 10MHz
const MTIMECMP: usize = 0x4000;
const MTIME: usize = 0xBFF8;

pub struct Clint {
    start_addr: u64,
    size: u64,
    registers: Vec<u64>,
    // a thread to emulate the timer interrupt
    // timer thread triggers an interrupt after a certain time
    // as a function call
    thread: Option<std::thread::JoinHandle<()>>,
    pub pend_interrupt: Option<Box<dyn Fn(Interrupt)>>,
}

impl Clint {
    pub fn new(_start_addr: u64, _size: u64) -> Clint {
        Self {
            start_addr: _start_addr,
            size: _size, // size is in bytes, but we store u64
            registers: vec![0; (_size / 8) as usize],
            thread: None,
            pend_interrupt: None,
        }
    }

    pub fn is_accessible(&self, addr: u64) -> bool {
        (addr >= self.start_addr) && (addr < self.start_addr + self.size)
    }

    pub fn load(&self, _addr: u64, _size: u64) -> Result<u64, Exception> {
        info!("clint: load: addr: {:#x}", _addr);
        Ok(0x0)
    }

    pub fn store(&mut self, _addr: u64, _size: u64, _value: u64) -> Result<(), Exception> {
        info!("clint: store: addr: {:#x}, value: {:#x}", _addr, _value);
        Ok(())
    }
}
