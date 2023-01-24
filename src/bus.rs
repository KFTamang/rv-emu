use crate::dram::*;
use crate::clint::*;
use crate::uart::*;
use crate::interrupt::*;
use crate::plic::*;
use crate::virtio::*;

pub struct Bus {
    dram: Dram,
    clint: Clint,
    uart: Uart,
    plic: Plic,
    virtio: Virtio,
}

impl Bus {
    pub fn new(code: Vec<u8>, base_addr: u64) -> Bus {
        Self {
            dram: Dram::new(code, base_addr),
            clint: Clint::new(0x2000000, 0x10000),
            uart: Uart::new(0x10000000, 0x100),
            plic: Plic::new(0xc000000, 0x4000000),
            virtio: Virtio::new(0x10001000, 0x1000),
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
            return self.uart.load(addr, size);
        }
        if self.plic.is_accessible(addr) {
            return self.plic.load(addr, size);
        }
        if self.virtio.is_accessible(addr) {
            return self.virtio.load(addr, size);
        }
        eprintln!("Error while load operation: accessing 0x{:x}, size:{}", addr, size);
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
            return self.uart.store(addr, size, value);
        }
        if self.plic.is_accessible(addr) {
            return self.plic.store(addr, size, value);
        }
        if self.virtio.is_accessible(addr) {
            return self.virtio.store(addr, size, value);
        }
        eprintln!("Error while store operation: accessing 0x{:x}, size:{}, value:{}(0x{:x})", addr, size, value, value);
        Err(Exception::StoreAMOAccessFault)
    }

    pub fn dump(&self, path: &str) {
        self.dram.dump(path);
    }
}
