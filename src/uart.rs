use crate::interrupt::*;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::Read as _;
use std::sync::{Arc, Mutex};
use std::thread;

const UART_SIZE: u64 = 0x100; // size of the UART memory-mapped region

#[derive(Clone, Serialize, Deserialize)]
pub struct UartSnapshot {
    start_addr: u64,
    // recv_buf is intentionally excluded: pending input is transient and
    // cannot be serialised meaningfully.
}

pub struct Uart {
    start_addr: u64,
    interrupt_notifier: Arc<Box<dyn Fn() + Send + Sync>>,
    // Receive buffer written by the input thread, read by the emulator via RHR.
    recv_buf: Arc<Mutex<VecDeque<u8>>>,
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

fn spawn_input_thread(
    recv_buf: Arc<Mutex<VecDeque<u8>>>,
    interrupt_notifier: Arc<Box<dyn Fn() + Send + Sync>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        info!("UART input thread started");
        let stdin = std::io::stdin();
        let mut raw = stdin.lock();
        let mut buf = [0u8; 256];
        loop {
            match raw.read(&mut buf) {
                Ok(0) => break, // EOF – stop spinning
                Ok(n) => {
                    {
                        let mut q = recv_buf.lock().unwrap();
                        for &b in &buf[..n] {
                            q.push_back(b);
                        }
                    }
                    (interrupt_notifier)();
                    info!("UART: queued {} byte(s)", n);
                }
                Err(_) => break,
            }
        }
        info!("UART input thread exiting");
    })
}

impl Uart {
    pub fn new(_start_addr: u64, interrupt_notifier: Box<dyn Fn() + Send + Sync>) -> Uart {
        let interrupt_notifier = Arc::new(interrupt_notifier);
        let recv_buf = Arc::new(Mutex::new(VecDeque::<u8>::new()));

        let input_thread = spawn_input_thread(
            Arc::clone(&recv_buf),
            Arc::clone(&interrupt_notifier),
        );

        Self {
            start_addr: _start_addr,
            interrupt_notifier,
            recv_buf,
            input_thread,
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
            REG_RHR_THR => {
                // Pop the oldest character from the receive buffer.
                let ch = self.recv_buf.lock().unwrap().pop_front().unwrap_or(0);
                info!("UART RHR read: 0x{:02x} ('{}')", ch, ch as char);
                Ok(ch as u64)
            }
            REG_LSR => {
                // Bit 0 – RECEIVE_DATA_READY: set when there is data in the receive buffer.
                // Bits 5-6 – transmit side: always ready (infinitely fast transmitter).
                let has_data = !self.recv_buf.lock().unwrap().is_empty();
                let rx_ready = if has_data { RECEIVE_DATA_READY } else { 0 };
                Ok(TRANSMIT_EMPTY | TRANSMIT_HOLDING_EMPTY | rx_ready)
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
                use std::io::Write as _;
                print!("{}", value as u8 as char);
                let _ = std::io::stdout().flush();
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
        let recv_buf = Arc::new(Mutex::new(VecDeque::<u8>::new()));

        let input_thread = spawn_input_thread(
            Arc::clone(&recv_buf),
            Arc::clone(&interrupt_notifier),
        );

        Self {
            start_addr: snapshot.start_addr,
            interrupt_notifier,
            recv_buf,
            input_thread,
        }
    }

    pub fn to_snapshot(&self) -> UartSnapshot {
        UartSnapshot {
            start_addr: self.start_addr,
        }
    }
}
