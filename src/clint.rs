use crate::interrupt::*;
use log::info;
// use std::sync::{mpsc, Arc};
// use std::time::Duration;
use serde::{Deserialize, Serialize};

const FREQUENCY: u64 = 10000000; // clock frequency: 10MHz
const MTIMECMP: usize = 0x4000;
const MTIME: usize = 0xBFF8;

#[derive(Clone, Serialize, Deserialize)]
pub struct Clint {
    start_addr: u64,
    size: u64,
    registers: Vec<u64>,
    // a thread to emulate the timer interrupt
    // timer thread triggers an interrupt after a certain time
    // as a function call
    // thread: Option<std::thread::JoinHandle<()>>,
    // duration_sender: mpsc::Sender<Option<Duration>>,
}

impl Clint {
    pub fn new(
        _start_addr: u64,
        _size: u64,
        // _interrupt_sender: Arc<mpsc::Sender<Interrupt>>,
    ) -> Clint {
        // let (sender, receiver) = mpsc::channel();
        // let thread = std::thread::spawn(move || {
        //     Self::timer_thread(receiver, _interrupt_sender);
        // });
        Self {
            start_addr: _start_addr,
            size: _size, // size is in bytes, but we store u64
            registers: vec![0; (_size / 8) as usize],
            // thread: Some(thread),
            // duration_sender: sender,
        }
    }

    fn timer_thread(
        // receiver: mpsc::Receiver<Option<Duration>>,
        // interrupt_sender: Arc<mpsc::Sender<Interrupt>>,
    ) {
        loop {
            // let maybe_sleep_duration = receiver.recv().unwrap();
            // if let Some(duration) = maybe_sleep_duration {
            //     std::thread::sleep(duration);
            //     // trigger the interrupt
            //     interrupt_sender
            //         .send(Interrupt::MachineTimerInterrupt)
            //         .unwrap();
            //     debug!("timer thread: interrupt triggered Interrupt::MachineTimerInterrupt");
            // } else {
            //     break;
            // }
        }
    }

    pub fn is_accessible(&self, addr: u64) -> bool {
        (addr >= self.start_addr) && (addr < self.start_addr + self.size)
    }

    pub fn load(&self, _addr: u64, _size: u64) -> Result<u64, Exception> {
        info!("clint: load: addr: {:#x}", _addr);
        Ok(0x0)
    }

    pub fn store(&mut self, _addr: u64, _size: u64, _value: u64) -> Result<(), Exception> {
        info!("clint: store: addr: {:#x}, value: {:#x}", _addr, _value);
        let offset = (_addr - self.start_addr) as usize;
        self.registers[offset / 8] = _value;
        if offset == MTIMECMP {
            self.set_mtimecmp(_value);
        }
        Ok(())
    }

    fn set_mtimecmp(&mut self, value: u64) {
        let current_counter = self.registers[MTIME / 8];
        let target_counter = value;
        let wait_time_ms = ((target_counter - current_counter) * 1000 / FREQUENCY) as u64;
        info!("clint: store: mtimecmp: wait_time_ms: {}", wait_time_ms);

        // let sleep_duration = Option::Some(Duration::from_millis(wait_time_ms));
        // self.duration_sender.send(sleep_duration).unwrap();
    }
}

impl Drop for Clint {
    fn drop(&mut self) {
        // self.duration_sender.send(Option::None).unwrap();
        // if let Some(thread) = self.thread.take() {
        //     thread.join().unwrap();
        // }
    }
}
