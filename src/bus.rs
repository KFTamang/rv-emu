use crate::dram::*;
use crate::interrupt::*;
use crate::plic::*;
use crate::uart::*;
use crate::virtio::*;
use log::debug;
use log::info;

pub struct Bus {
    dram: Dram,
    uart: Uart,
    plic: Plic,
    virtio: Virtio,
}

impl Bus {
    pub fn new(code: Vec<u8>, base_addr: u64) -> Bus {
        Self {
            dram: Dram::new(code, base_addr),
            uart: Uart::new(0x10000000, 0x100),
            plic: Plic::new(0xc000000, 0x4000000),
            virtio: Virtio::new(0x10001000, 0x1000),
        }
    }

    pub fn load(&self, addr: u64, size: u64) -> Result<u64, Exception> {
        if self.dram.dram_base <= addr {
            let ret_val = self.dram.load(addr, size);
            return ret_val;
        }
        info!("load addr:{:x}, size:{}", addr, size);
        if self.uart.is_accessible(addr) {
            let ret_val = self.uart.load(addr, size);
            debug!(
                "load uart addr:{:x}, size:{}, value:{}(0x{:x})",
                addr,
                size,
                ret_val.as_ref().unwrap(),
                ret_val.as_ref().unwrap()
            );
            return ret_val;
        }
        if self.plic.is_accessible(addr) {
            let ret_val = self.plic.load(addr, size);
            debug!(
                "load plic addr:{:x}, size:{}, value:{}(0x{:x})",
                addr,
                size,
                ret_val.as_ref().unwrap(),
                ret_val.as_ref().unwrap()
            );
            return ret_val;
        }
        if self.virtio.is_accessible(addr) {
            let ret_val = self.virtio.load(addr, size);
            debug!(
                "load virtio addr:{:x}, size:{}, value:{}(0x{:x})",
                addr,
                size,
                ret_val.as_ref().unwrap(),
                ret_val.as_ref().unwrap()
            );
            return ret_val;
        }
        debug!(
            "Error while load operation: accessing 0x{:x}, size:{}",
            addr, size
        );
        Err(Exception::LoadAccessFault)
    }

    pub fn store(&mut self, addr: u64, size: u64, value: u64) -> Result<(), Exception> {
        if self.dram.dram_base <= addr {
            return self.dram.store(addr, size, value);
        }
        info!(
            "store addr:{:x}, size:{}, value:{}(0x{:x})",
            addr, size, value, value
        );
        if self.uart.is_accessible(addr) {
            return self.uart.store(addr, size, value);
        }
        if self.plic.is_accessible(addr) {
            return self.plic.store(addr, size, value);
        }
        if self.virtio.is_accessible(addr) {
            return self.virtio.store(addr, size, value);
        }
        debug!(
            "Error while store operation: accessing 0x{:x}, size:{}, value:{}(0x{:x})",
            addr, size, value, value
        );
        Err(Exception::StoreAMOAccessFault)
    }

    #[allow(unused)]
    pub fn dump(&self, path: &str) {
        self.dram.dump(path);
    }
}
