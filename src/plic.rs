use std::collections::BTreeSet;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::rc::Rc;
use std::cell::RefCell;
use std::vec;

use crate::interrupt::{Interrupt, Exception};
use serde::{Deserialize, Serialize};

use log::{error, info};

const PLIC_SIZE: u64 = 0x4000000;

const INTERRUPT_SOURCE_PRIORITIES: u64 = 0x000000;
const INTERRUPT_PENDING_BITS: u64 = 0x001000;
const INTERRUPT_ENABLES: u64 = 0x002000;
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
    UartInput = 10,
}

impl ExternalInterrupt {
    pub fn id(&self) -> u64 {
        match self {
            ExternalInterrupt::UartInput => 10,
        }
    }
}

pub struct Plic {
    start_addr: u64,
    regs: Vec<u32>,
    receiver: Receiver<ExternalInterrupt>,
    sender: Sender<ExternalInterrupt>,
    interrupt_list: Rc<RefCell<BTreeSet<Interrupt>>>,
    external_interrupt_list: BTreeSet<ExternalInterrupt>,
}

impl Plic {
    pub fn new(_start_addr: u64, interrupt_list: Rc<RefCell<BTreeSet<Interrupt>>>) -> Plic {
        let (sender, receiver) = channel();
        Self {
            start_addr: _start_addr,
            regs: vec![0; PLIC_SIZE as usize / 8],
            sender: sender,
            receiver: receiver,
            interrupt_list: interrupt_list,
            external_interrupt_list: BTreeSet::new(),
        }
    }

    pub fn get_interrupt_notificator(&self, id: ExternalInterrupt) -> Box<dyn Fn() + Send + Sync> {
        // This function should return a closure that notifies the PLIC of an interrupt of ID `id`.
        let sender_clone = self.sender.clone();
        Box::new(move || {
            info!("Notifying PLIC of interrupt ID: {:?}", id);
            if let Err(e) = sender_clone.send(id) {
                error!("Failed to send interrupt notification: {}", e);
            }
        })
    }

    pub fn process_pending_interrupts(&mut self) {
        while let Ok(interrupt) = self.receiver.try_recv() {
            info!("Processing interrupt ID: {:?}", interrupt);
            self.interrupt_list.borrow_mut().insert(Interrupt::SupervisorExternalInterrupt);
            self.regs[INTERRUPT_PENDING_BITS as usize / 4] |= 1 << interrupt.id();
            info!("Updated pending bits for interrupt ID: {:?}", interrupt);
            self.external_interrupt_list.insert(interrupt.clone());
        }
    }

    pub fn is_accessible(&self, addr: u64) -> bool {
        (addr >= self.start_addr) && (addr < self.start_addr + PLIC_SIZE)
    }

    pub fn load(&mut self, _addr: u64, _size: u64) -> Result<u64, Exception> {
        let relative_addr = _addr - self.start_addr;
        match relative_addr {
            CLAIM_COMPLETE => {
                // Search for the highest priority pending interrupt
                let max_interrupt = self.external_interrupt_list
                    .iter()
                    .max_by_key(|&interrupt| {
                        self.regs[INTERRUPT_SOURCE_PRIORITIES as usize / 4 + interrupt.id() as usize]
                    })
                    .cloned();
                let result = if let Some(interrupt) = max_interrupt {
                    let id = interrupt.id();
                    // Clear the pending bit for this interrupt
                    self.regs[INTERRUPT_PENDING_BITS as usize / 4] &= !(1 << id);
                    // Remove the interrupt from the external list
                    self.external_interrupt_list.remove(&interrupt);
                    // Clear the interrupt pending bit in the interrupt list
                    self.interrupt_list.borrow_mut().remove(&Interrupt::SupervisorExternalInterrupt);
                    // Return the ID of the claimed interrupt
                    Ok(id as u64)
                } else {
                    // No pending interrupts, return 0
                    Ok(0)
                };

                // Clear pending bit of External Interrupt
                if self.external_interrupt_list.is_empty() {
                    
                }
                result
            },
            _ => Ok(self.regs[(relative_addr / 4) as usize] as u64),
        }
    }

    pub fn store(&mut self, _addr: u64, _size: u64, _value: u64) -> Result<(), Exception> {
        Ok(())
    }

    pub fn from_snapshot(
        snapshot: PlicSnapshot,
        interrupt_list: Rc<RefCell<BTreeSet<Interrupt>>>,
    ) -> Plic {
        let (sender, receiver) = channel();
        Plic {
            start_addr: snapshot.start_addr,
            regs: snapshot.regs,
            interrupt_list,
            receiver: receiver,
            sender: sender,
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
