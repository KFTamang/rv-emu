use crate::interrupt::*;
use log::{debug, info};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

const FREQUENCY: u64 = 10000000; // clock frequency: 10MHz
const MTIMECMP: usize = 0x4000;
const MTIME: usize = 0xBFF8;

pub struct Clint {
    start_addr: u64,
    size: u64,
    registers: Vec<u64>,
    // a thread to emulate the timer interrupt
    // timer thread triggers an interrupt after a certain time
    // as a function call
    thread: Option<std::thread::JoinHandle<()>>,
    condvar: Condvar,
    mutex: Mutex<bool>,
    sleep_duration: Duration,
    is_finished: bool,
    pub pend_interrupt: Option<Box<dyn Fn(Interrupt)>>,
}

impl Clint {
    pub fn new(_start_addr: u64, _size: u64) -> Clint {
        let myself = Self {
            start_addr: _start_addr,
            size: _size, // size is in bytes, but we store u64
            registers: vec![0; (_size / 8) as usize],
            thread: None,
            pend_interrupt: None,
            condvar: Condvar::new(),
            mutex: Mutex::new(false),
            sleep_duration: Duration::from_secs(1),
            is_finished: false,
        };

        let thread = std::thread::spawn(
            move || {
                myself.timer_thread();
            },
        );
        myself
    }

    fn timer_thread(&mut self) {

        let mut mutex = self.mutex.lock().unwrap();
        loop {
            // wait for a signal via condvar
            mutex = self.condvar.wait(mutex).unwrap();
            // sleep for duration
            debug!("timer thread: sleeping for: {:?}", self.sleep_duration);
            std::thread::sleep(self.sleep_duration);
            // trigger the interrupt
            self.pend_interrupt.unwrap()(Interrupt::MachineTimerInterrupt);
            debug!("timer thread: interrupt triggered Interrupt::MachineTimerInterrupt");
            if self.is_finished {
                break;
            }
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
        if offset == MTIMECMP {


        }
        Ok(())
    }

    fn set_mtimecmp(&mut self, value: u64) {
        let current_counter = self.registers[MTIME / 8];
        let target_counter = value;
        let wait_time_ms = ((target_counter - current_counter) * 1000 / FREQUENCY) as u64;
        info!("clint: store: mtimecmp: wait_time_ms: {}", wait_time_ms);

        {
            let mut mutex = self.mutex.lock().unwrap();
            self.sleep_duration = Duration::from_millis(wait_time_ms);
            self.condvar.notify_all();
        }
    }
}
