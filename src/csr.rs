use crate::interrupt::*;
use log::{debug, info};
use std::sync::{Arc, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct Csr {
    csr: [u64; 4096],
    timer_thread: std::thread::JoinHandle<()>,
    duration_sender: mpsc::Sender<Option<Duration>>,
    initial_time: u64,
}

pub const SSTATUS: usize = 0x100;
pub const SIE: usize = 0x104;
pub const STVEC: usize = 0x105;
pub const SEPC: usize = 0x141;
pub const SCAUSE: usize = 0x142;
pub const SIP: usize = 0x144;
// Sstc extension for supervisor timer registers
pub const STIMECMP: usize = 0x14D;
pub const STIMECMPH: usize = 0x15D;

pub const SATP: usize = 0x180;

pub const MSTATUS: usize = 0x300;
pub const MEDELEG: usize = 0x302;
pub const MIDELEG: usize = 0x302;
pub const MIE: usize = 0x304;
pub const MTVEC: usize = 0x305;
pub const MEPC: usize = 0x341;
pub const MCAUSE: usize = 0x342;
pub const MIP: usize = 0x344;

pub const TIME: usize = 0xc01;

pub const BIT_SXL: u64 = 34;
pub const BIT_TSR: u64 = 22;
pub const BIT_TW: u64 = 21;
pub const BIT_TVM: u64 = 20;
pub const BIT_MPRV: u64 = 17;
pub const BIT_MPP: u64 = 11;
pub const BIT_MPIE: u64 = 7;
pub const BIT_MIE: u64 = 3;
pub const BIT_SPP: u64 = 8;
pub const BIT_SPIE: u64 = 5;
pub const BIT_SIE: u64 = 1;

pub const MASK_SXL: u64 = 0b11 << BIT_SXL;
pub const MASK_TSR: u64 = 0b1 << BIT_TSR;
pub const MASK_TW: u64 = 0b1 << BIT_TW;
pub const MASK_TVM: u64 = 0b1 << BIT_TVM;
pub const MASK_SPP: u64 = 0b11 << BIT_SPP;
pub const MASK_SPIE: u64 = 0b1 << BIT_SPIE;
pub const MASK_SIE: u64 = 0b1 << BIT_SIE;
pub const MASK_MPRV: u64 = 0b1 << BIT_MPRV;
pub const MASK_MPP: u64 = 0b11 << BIT_MPP;
pub const MASK_MPIE: u64 = 0b1 << BIT_MPIE;
pub const MASK_MIE: u64 = 0b1 << BIT_MIE;
const SSTATUS_MASK: u64 = !(MASK_SXL
    | MASK_TSR
    | MASK_TSR
    | MASK_TW
    | MASK_TVM
    | MASK_MPRV
    | MASK_MPP
    | MASK_MPIE
    | MASK_MIE);

pub const COUNT_PER_MS: u64 = 20; // 50 MHz

impl Csr {
    pub fn new(_interrupt_sender: Arc<mpsc::Sender<Interrupt>>) -> Self {
        let (sender, receiver) = mpsc::channel();
        let thread = std::thread::spawn(
            move || {
                Self::timer_thread(receiver, _interrupt_sender);
            },
        );
        Self {
            csr: [0; 4096],
               timer_thread: thread,
                duration_sender: sender,
                initial_time: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
        }
    }

    pub fn load_csrs(&self, addr: usize) -> u64 {
        // debug!("load: addr:{:#x}", addr);
        if addr == SSTATUS {
            self.csr[MSTATUS] & SSTATUS_MASK
        } else {
            self.csr[addr]
        }
    }

    pub fn store_csrs(&mut self, addr: usize, val: u64) {
        debug!("store: addr:{:#x}, val:{:#x}", addr, val);
        match addr {
            SSTATUS => {
                self.csr[MSTATUS] = val & SSTATUS_MASK;
            },
            STIMECMP => {
                self.csr[STIMECMP] = val;
                self.set_timer_interrupt(val);
            },
            _ => {
                self.csr[addr] = val;
            }
        }
    }

    pub fn set_mstatus_bit(&mut self, val: u64, mask: u64, bit: u64) {
        let mut current = self.load_csrs(MSTATUS);
        current &= !mask;
        current |= ((val as u64) << bit) & mask;
        self.store_csrs(MSTATUS, current);
    }

    pub fn get_mstatus_bit(&self, mask: u64, bit: u64) -> u64 {
        let status = self.load_csrs(MSTATUS);
        (status & mask) >> bit
    }

    pub fn set_sstatus_bit(&mut self, val: u64, mask: u64, bit: u64) {
        let mut current = self.load_csrs(SSTATUS);
        current &= !mask;
        current |= ((val as u64) << bit) & mask;
        self.store_csrs(SSTATUS, current);
    }

    pub fn get_sstatus_bit(&self, mask: u64, bit: u64) -> u64 {
        let status = self.load_csrs(SSTATUS);
        (status & mask) >> bit
    }

    fn get_time_ms(&self) -> u64 {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        current_time - self.initial_time
    }

    fn set_timer_interrupt(&self, comp_value: u64) {
        let time = self.get_time_ms();
        let comptime_ms = comp_value / COUNT_PER_MS;
        if comptime_ms >= time {
            let duration = Duration::from_millis(comp_value - time);
            self.duration_sender.send(Option::Some(duration)).unwrap();
        }
    }

    fn timer_thread(receiver: mpsc::Receiver<Option<Duration>>, interrupt_sender: Arc<mpsc::Sender<Interrupt>>) {
        info!("timer thread: started");
        loop {
            let maybe_sleep_duration = receiver.recv().unwrap();
            if let Some(duration) = maybe_sleep_duration {
                std::thread::sleep(duration);
                // trigger the interrupt
                interrupt_sender.send(Interrupt::MachineTimerInterrupt).unwrap();
                info!("timer thread: interrupt triggered Interrupt::MachineTimerInterrupt");
            } else {
                info!("timer thread: exiting");
                break;
            }
        }
    }
}
