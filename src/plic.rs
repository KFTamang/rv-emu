use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use crate::interrupt::{Exception, Interrupt};
use serde::{Deserialize, Serialize};

use log::info;

const PLIC_SIZE: u64 = 0x4000000;

const INTERRUPT_SOURCE_PRIORITIES: u64 = 0x000000;
const INTERRUPT_PENDING_BITS: u64 = 0x001000;
#[allow(unused)]
const INTERRUPT_ENABLES: u64 = 0x002000;
#[allow(unused)]
const PRIORITY_THRESHOLDS: u64 = 0x200000;
const CLAIM_COMPLETE: u64 = 0x201004;

#[derive(Clone, Serialize, Deserialize)]
pub struct PlicSnapshot {
    pub start_addr: u64,
    pub regs: Vec<u32>,
    pub external_interrupt_list: BTreeSet<ExternalInterrupt>,
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, Copy)]
pub enum ExternalInterrupt {
    VirtioDiskIO = 1,
    UartInput = 10,
}

impl ExternalInterrupt {
    pub fn id(&self) -> u64 {
        match self {
            ExternalInterrupt::VirtioDiskIO => 1,
            ExternalInterrupt::UartInput => 10,
        }
    }
}

pub struct Plic {
    start_addr: u64,
    regs: Vec<u32>,
    pending_queue: Arc<Mutex<Vec<ExternalInterrupt>>>,
    external_interrupt_list: BTreeSet<ExternalInterrupt>,
}

impl Plic {
    pub fn new(start_addr: u64) -> Plic {
        Self {
            start_addr,
            regs: vec![0; PLIC_SIZE as usize / 8],
            pending_queue: Arc::new(Mutex::new(Vec::new())),
            external_interrupt_list: BTreeSet::new(),
        }
    }

    pub fn get_interrupt_notificator(&self, id: ExternalInterrupt) -> Box<dyn Fn() + Send + Sync> {
        let queue = Arc::clone(&self.pending_queue);
        Box::new(move || {
            info!("Notifying PLIC of interrupt ID: {:?}", id);
            queue.lock().unwrap().push(id);
        })
    }

    pub fn process_pending_interrupts(&mut self, interrupts: &mut BTreeSet<Interrupt>) {
        let pending: Vec<ExternalInterrupt> =
            self.pending_queue.lock().unwrap().drain(..).collect();
        for interrupt in pending {
            info!("Processing interrupt ID: {:?}", interrupt);
            interrupts.insert(Interrupt::SupervisorExternalInterrupt);
            self.regs[INTERRUPT_PENDING_BITS as usize / 4] |= 1 << interrupt.id();
            info!("Updated pending bits for interrupt ID: {:?}", interrupt);
            self.external_interrupt_list.insert(interrupt);
        }
    }

    pub fn is_accessible(&self, addr: u64) -> bool {
        (addr >= self.start_addr) && (addr < self.start_addr + PLIC_SIZE)
    }

    pub fn load(
        &mut self,
        addr: u64,
        _size: u64,
        interrupts: &mut BTreeSet<Interrupt>,
    ) -> Result<u64, Exception> {
        let relative_addr = addr - self.start_addr;
        match relative_addr {
            CLAIM_COMPLETE => {
                let max_interrupt = self
                    .external_interrupt_list
                    .iter()
                    .max_by_key(|&interrupt| {
                        self.regs
                            [INTERRUPT_SOURCE_PRIORITIES as usize / 4 + interrupt.id() as usize]
                    })
                    .cloned();
                if let Some(interrupt) = max_interrupt {
                    let id = interrupt.id();
                    self.regs[INTERRUPT_PENDING_BITS as usize / 4] &= !(1 << id);
                    self.external_interrupt_list.remove(&interrupt);
                    if self.external_interrupt_list.is_empty() {
                        interrupts.remove(&Interrupt::SupervisorExternalInterrupt);
                    }
                    Ok(id as u64)
                } else {
                    Ok(0)
                }
            }
            _ => Ok(self.regs[(relative_addr / 4) as usize] as u64),
        }
    }

    pub fn store(&mut self, _addr: u64, _size: u64, _value: u64) -> Result<(), Exception> {
        Ok(())
    }

    pub fn from_snapshot(snapshot: PlicSnapshot) -> Plic {
        Plic {
            start_addr: snapshot.start_addr,
            regs: snapshot.regs,
            pending_queue: Arc::new(Mutex::new(Vec::new())),
            external_interrupt_list: snapshot.external_interrupt_list,
        }
    }

    pub fn to_snapshot(&self) -> PlicSnapshot {
        PlicSnapshot {
            start_addr: self.start_addr,
            regs: self.regs.clone(),
            external_interrupt_list: self.external_interrupt_list.clone(),
        }
    }
}
