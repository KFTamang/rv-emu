use crate::interrupt::*;
use log::info;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::thread;

const UART_SIZE: u64 = 0x100; // size of the UART memory-mapped region

#[derive(Clone, Serialize, Deserialize)]
pub struct UartSnapshot {
    start_addr: u64,
}

pub struct Uart {
    start_addr: u64,
    interrupt_notifier: Arc<Box<dyn Fn() + Send + Sync>>,
    // The input_thread is an optional thread that can be used to read input from the user.
    input_thread: std::thread::JoinHandle<()>,
}

#[allow(unused)]
const REG_RHR_THR: u64 = 0;
#[allow(unused)]
const REG_IER: u64 = 1;
#[allow(unused)]
const REG_FCR_ISR: u64 = 2;
#[allow(unused)]
const REG_LCR: u64 = 3;
#[allow(unused)]
const REG_MCR: u64 = 4;
const REG_LSR: u64 = 5;
#[allow(unused)]
const REG_MSR: u64 = 6;
#[allow(unused)]
const REG_SPR: u64 = 7;

#[allow(unused)]
const RECEIVE_DATA_READY: u64 = 1 << 0;
#[allow(unused)]
const OVERRUN_ERROR: u64 = 1 << 1;
#[allow(unused)]
const PARITY_ERROR: u64 = 1 << 2;
#[allow(unused)]
const FRAMING_ERROR: u64 = 1 << 3;
#[allow(unused)]
const BREAK_INTERRUPT: u64 = 1 << 4;
const TRANSMIT_HOLDING_EMPTY: u64 = 1 << 5;
const TRANSMIT_EMPTY: u64 = 1 << 6;
#[allow(unused)]
const FIFO_ERROR: u64 = 1 << 7;

impl Uart {
    pub fn new(_start_addr: u64, interrupt_notifier: Box<dyn Fn() + Send + Sync>) -> Uart {
        let interrupt_notifier = Arc::new(interrupt_notifier);
        let interrupt_notifier_clone = Arc::clone(&interrupt_notifier);

        Self {
            start_addr: _start_addr,
            interrupt_notifier,
            input_thread: thread::spawn(move || {
                info!("Thread to take input...");
                let mut input = String::new();
                loop {
                    if std::io::stdin().read_line(&mut input).is_ok() {
                        // Call the interrupt notifier with the character read
                        (interrupt_notifier_clone)();
                        info!("input: {}", input.trim());
                        input.clear(); // Clear the input buffer for the next read
                    }
                }
            }),
        }
    }

    pub fn is_accessible(&self, addr: u64) -> bool {
        (addr >= self.start_addr) && (addr < self.start_addr + UART_SIZE)
    }

    pub fn load(&self, addr: u64, size: u64) -> Result<u64, Exception> {
        if size != 8 {
            return Err(Exception::LoadAccessFault);
        }
        let actual_addr = addr - self.start_addr;
        match actual_addr {
            REG_LSR => {
                // returns TRANSMIT_EMPTY | TRANSMIT_HOLDING_EMPTY,
                // assuming infinitely fast UART, with FIFO being always empty
                Ok(TRANSMIT_EMPTY | TRANSMIT_HOLDING_EMPTY)
            }
            _ => Ok(0x0),
        }
    }

    pub fn store(&mut self, addr: u64, size: u64, value: u64) -> Result<(), Exception> {
        if size != 8 {
            return Err(Exception::LoadAccessFault);
        }
        let actual_addr = addr - self.start_addr;
        match actual_addr {
            REG_RHR_THR => {
                print!("{}", value as u8 as char);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn from_snapshot(
        snapshot: UartSnapshot,
        interrupt_notifier: Box<dyn Fn() + Send + Sync + 'static>,
    ) -> Self {
        let interrupt_notifier = Arc::new(interrupt_notifier);
        let interrupt_notifier_clone = Arc::clone(&interrupt_notifier);

        Self {
            start_addr: snapshot.start_addr,
            interrupt_notifier,
            input_thread: thread::spawn(move || {
                loop {
                    let mut input = String::new();
                    if std::io::stdin().read_line(&mut input).is_ok() {
                        if let Some(ch) = input.chars().next() {
                            // Call the interrupt notifier with the character read
                            (interrupt_notifier_clone)();
                            info!("input: {}", ch);
                        }
                    }
                }
            }),
        }
    }

    pub fn to_snapshot(&self) -> UartSnapshot {
        UartSnapshot {
            start_addr: self.start_addr,
        }
    }
}
