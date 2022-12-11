use crate::dram::*;
use crate::interrupt::*;

pub struct Bus {
    dram: Dram,
}

impl Bus {
    pub fn new(code: Vec<u8>, base_addr: u64) -> Bus {
        Self {
            dram: Dram::new(code, base_addr),
        }
    }

    pub fn load(&self, addr: u64, size: u64) -> Result<u64, Exception> {
        if self.dram.dram_base <= addr {
            return self.dram.load(addr, size);
        }
        Err(Exception::LoadAccessFault)
    }

    pub fn store(&mut self, addr: u64, size: u64, value: u64) -> Result<(), Exception> {
        if self.dram.dram_base <= addr {
            return self.dram.store(addr, size, value);
        }
        Err(Exception::StoreAMOAccessFault)
    }
}
