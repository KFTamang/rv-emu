use crate::dram::*;
use crate::clint::*;
use crate::uart::*;
use crate::interrupt::*;

pub struct Bus {
    dram: Dram,
    clint: Clint,
    uart: Uart,
}

impl Bus {
    pub fn new(code: Vec<u8>, base_addr: u64) -> Bus {
        Self {
            dram: Dram::new(code, base_addr),
            clint: Clint::new(0x2000000, 0x10000),
            uart: Uart::new(0x10000000, 0x100),
        }
    }

    pub fn load(&self, addr: u64, size: u64) -> Result<u64, Exception> {
        if self.dram.dram_base <= addr {
            return self.dram.load(addr, size);
        }
        if self.clint.is_accessible(addr) {
            return self.clint.load(addr, size);
        }
        if self.uart.is_accessible(addr) {
            return self.clint.load(addr, size);
        }
        Err(Exception::LoadAccessFault)
    }

    pub fn store(&mut self, addr: u64, size: u64, value: u64) -> Result<(), Exception> {
        if self.dram.dram_base <= addr {
            return self.dram.store(addr, size, value);
        }
        if self.clint.is_accessible(addr) {
            return self.clint.store(addr, size, value);
        }
        if self.uart.is_accessible(addr) {
            return self.clint.store(addr, size, value);
        }
        Err(Exception::StoreAMOAccessFault)
    }
}
