use crate::dram::*;
use crate::interrupt::*;
use crate::plic::*;
use crate::uart::*;
use crate::virtio::*;
use log::debug;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub struct Bus {
    pub dram: Dram,
    pub uart: Uart,
    pub plic: Plic,
    pub virtio: Option<Virtio>,
}

impl Bus {
    pub fn new(code: Vec<u8>, base_addr: u64) -> Bus {
        let plic = Plic::new(0xc000000);
        let uart_notificator = plic.get_interrupt_notificator(ExternalInterrupt::UartInput);
        let virtio_notificator = plic.get_interrupt_notificator(ExternalInterrupt::VirtioDiskIO);
        Bus {
            plic,
            dram: Dram::new(code, base_addr),
            uart: Uart::new(0x10000000, uart_notificator),
            virtio: Some(Virtio::new(0x10001000, virtio_notificator)),
        }
    }

    pub fn load(&mut self, addr: u64, size: u64) -> Result<u64, Exception> {
        if self.dram.dram_base <= addr {
            return self.dram.load(addr, size);
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
        if let Some(ref virtio) = self.virtio {
            if virtio.is_accessible(addr) {
                let ret_val = virtio.load(addr, size);
                debug!(
                    "load virtio addr:{:x}, size:{}, value:{}(0x{:x})",
                    addr,
                    size,
                    ret_val.as_ref().unwrap(),
                    ret_val.as_ref().unwrap()
                );
                return ret_val;
            }
        }
        debug!(
            "Error while load operation: accessing 0x{:x}, size:{}",
            addr, size
        );
        Err(Exception::LoadAccessFault)
    }

    /// Load from a PLIC address, passing the CPU's interrupt list for CLAIM_COMPLETE handling.
    pub fn plic_load(
        &mut self,
        addr: u64,
        size: u64,
        interrupts: &mut BTreeSet<Interrupt>,
    ) -> Result<u64, Exception> {
        let ret_val = self.plic.load(addr, size, interrupts);
        debug!(
            "load plic addr:{:x}, size:{}, value:{}(0x{:x})",
            addr,
            size,
            ret_val.as_ref().unwrap(),
            ret_val.as_ref().unwrap()
        );
        ret_val
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
        if let Some(ref mut virtio) = self.virtio {
            if virtio.is_accessible(addr) {
                return virtio.store(addr, size, value);
            }
        }
        debug!(
            "Error while store operation: accessing 0x{:x}, size:{}, value:{}(0x{:x})",
            addr, size, value, value
        );
        Err(Exception::StoreAMOAccessFault)
    }

    /// Process pending virtio disk DMA requests.
    pub fn process_virtio(&mut self) {
        if self.virtio.as_ref().map_or(false, |v| v.has_pending_work()) {
            if let Some(mut virtio) = self.virtio.take() {
                virtio.disk_access(&mut self.dram);
                self.virtio = Some(virtio);
            }
        }
    }

    /// Forward pending peripheral interrupts into the CPU's interrupt list.
    pub fn process_pending_interrupts(&mut self, interrupts: &mut BTreeSet<Interrupt>) {
        self.plic.process_pending_interrupts(interrupts);
    }

    #[allow(unused)]
    pub fn dump(&self, path: &str) {
        self.dram.dump(path);
    }

    pub fn from_snapshot(dram: Dram, uart_snap: UartSnapshot, plic_snap: PlicSnapshot) -> Self {
        let plic = Plic::from_snapshot(plic_snap);
        let uart_notificator = plic.get_interrupt_notificator(ExternalInterrupt::UartInput);
        Bus {
            dram,
            uart: Uart::from_snapshot(uart_snap, uart_notificator),
            plic,
            virtio: None,
        }
    }
}
