use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::vec;

use crate::interrupt::{self, *};
use serde::{Deserialize, Serialize};

use log::{error, info};

const PLIC_SIZE: u64 = 0x4000000;

const INTERRUPT_SOURCE_PRIORITIES: u64 = 0x000000;
const INTERRUPT_PENDING_BITS: u64 = 0x001000;
const INTERRUPT_ENABLES: u64 = 0x002000;
const PRIORITY_THRESHOLDS: u64 = 0x200000;
const CLAIM_COMPLETE: u64 = 0x200004;

#[derive(Clone, Serialize, Deserialize)]
pub struct PlicSnapshot {
    pub start_addr: u64,
    pub regs: Vec<u32>,
}

pub struct Plic {
    start_addr: u64,
    regs: Vec<u32>,
    receiver: Receiver<u64>,
    sender: Sender<u64>,
    interrupt_list: Arc<Mutex<Vec<DelayedInterrupt>>>,
}

impl Plic {
    pub fn new(_start_addr: u64, interrupt_list: Arc<Mutex<Vec<DelayedInterrupt>>>) -> Plic {
        let (sender, receiver) = channel();
        Self {
            start_addr: _start_addr,
            regs: vec![0; PLIC_SIZE as usize / 8],
            sender: sender,
            receiver: receiver,
            interrupt_list: interrupt_list,
        }
    }

    pub fn get_interrupt_notificator(&self, id: u64) -> Box<dyn Fn() + Send + Sync> {
        // This function should return a closure that notifies the PLIC of an interrupt of ID `id`.
        let sender_clone = self.sender.clone();
        Box::new(move || {
            info!("Notifying PLIC of interrupt ID: {}", id);
            if let Err(e) = sender_clone.send(id as u64) {
                error!("Failed to send interrupt notification: {}", e);
            }
        })
    }

    pub fn process_pending_interrupts(&mut self) {
        while let Ok(interrupt_id) = self.receiver.try_recv() {
            info!("Processing interrupt ID: {}", interrupt_id);
            self.interrupt_list.lock().unwrap().push(DelayedInterrupt {
                interrupt: Interrupt::MachineExternalInterrupt,
                cycle: 0,
            });
            self.regs[INTERRUPT_PENDING_BITS as usize / 4] |= 1 << interrupt_id;
            info!("Updated pending bits for interrupt ID: {}", interrupt_id);
        }
    }

    pub fn is_accessible(&self, addr: u64) -> bool {
        (addr >= self.start_addr) && (addr < self.start_addr + PLIC_SIZE)
    }

    pub fn load(&self, _addr: u64, _size: u64) -> Result<u64, Exception> {
        Ok(0x0)
    }

    pub fn store(&mut self, _addr: u64, _size: u64, _value: u64) -> Result<(), Exception> {
        Ok(())
    }

    pub fn from_snapshot(
        snapshot: PlicSnapshot,
        interrupt_list: Arc<Mutex<Vec<DelayedInterrupt>>>,
    ) -> Plic {
        let (sender, receiver) = channel();
        Plic {
            start_addr: snapshot.start_addr,
            regs: snapshot.regs,
            interrupt_list,
            receiver: receiver,
            sender: sender,
        }
    }

    pub fn to_snapshot(&self) -> PlicSnapshot {
        PlicSnapshot {
            start_addr: self.start_addr,
            regs: self.regs.clone(),
        }
    }
}
