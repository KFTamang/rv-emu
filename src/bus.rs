use crate::dram::*;
use crate::interrupt::*;
use crate::plic::*;
use crate::uart::*;
use crate::virtio::*;
use log::debug;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Serialize, Deserialize)]
pub struct BusSnapshot {
    pub dram: Dram,
    pub uart: UartSnapshot,
    pub plic: PlicSnapshot,
    pub virtio: VirtioSnapshot,
}

pub struct Bus {
    pub dram: Dram,
    pub uart: Uart,
    pub plic: Plic,
    pub virtio: Option<Virtio>,
}

impl Bus {
    pub fn new(
        code: Vec<u8>,
        base_addr: u64,
        interrupt_list: Rc<RefCell<BTreeSet<Interrupt>>>,
    ) -> Bus {
        let plic = Plic::new(0xc000000, interrupt_list.clone());
        let uart_notificator = plic.get_interrupt_notificator(ExternalInterrupt::UartInput);
        let mut bus = Self {
            plic,
            dram: Dram::new(code, base_addr),
            uart: Uart::new(0x10000000, uart_notificator),
            virtio: None,
        };
        let virtio = Rc::new(
            RefCell::new(
                Virtio::new(
                    0x10001000, 
                    bus.plic.get_interrupt_notificator(ExternalInterrupt::VirtioDiskIO),
                )
            )
        );
        virtio.borrow_mut().set_bus(Rc::new(RefCell::new(bus.clone())));
          
        bus.virtio = Some(virtio.borrow().clone());
        bus
    }

    pub fn load(&mut self, addr: u64, size: u64) -> Result<u64, Exception> {
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
        let mut virtio = self.virtio.as_mut().expect("No virtio bus");
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
        let mut virtio = self.virtio.as_mut().expect("No virtio bus");
        if virtio.is_accessible(addr) {
            return virtio.store(addr, size, value);
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

    pub fn to_snapshot(&self) -> BusSnapshot {
        BusSnapshot {
            dram: self.dram.clone(),
            uart: self.uart.to_snapshot(),
            plic: self.plic.to_snapshot(),
            virtio: self.virtio.as_ref().expect("No virtio bus").to_snapshot(),
        }
    }
    pub fn from_snapshot(
        snapshot: BusSnapshot,
        interrupt_list: Rc<RefCell<BTreeSet<Interrupt>>>,
    ) -> Self {
        let plic = Plic::from_snapshot(snapshot.plic, interrupt_list.clone());
        let uart_notificator = plic.get_interrupt_notificator(ExternalInterrupt::UartInput);
        let virtio_notificator = plic.get_interrupt_notificator(ExternalInterrupt::VirtioDiskIO);
        Self {
            dram: snapshot.dram,
            uart: Uart::from_snapshot(
                snapshot.uart,
                uart_notificator,
            ),
            plic,
            virtio: Some(Virtio::from_snapshot(
                snapshot.virtio,
                virtio_notificator,
            )),
        }
    }
}
