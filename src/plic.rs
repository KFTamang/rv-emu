use crate::interrupt::*;

pub struct Plic {
    start_addr: u64,
    size: u64,
}

impl Plic {
    pub fn new(_start_addr: u64, _size: u64) -> Plic {
        Self {
            start_addr: _start_addr,
            size: _size,
        }
    }

    pub fn is_accessible(&self, addr: u64) -> bool {
        (addr >= self.start_addr) & (addr < self.start_addr + self.size)
    }

    pub fn load(&self, addr: u64, size: u64) -> Result<u64, Exception> {
        Ok(0x0)
    }

    pub fn store(&mut self, addr: u64, size: u64, value: u64) -> Result<(), Exception> {
        Ok(())
    }
}
