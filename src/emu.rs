
use crate::cpu::*;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use log::info;
use bincode;

pub enum ExecMode {
    Step,
    Continue,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Event {
    DoneStep,
    Halted,
    Break,
}

pub enum RunEvent {
    IncomingData,
    Event(Event),
}

pub struct Emu {
    pub breakpoints: Vec<u64>,
    pub exec_mode: ExecMode,
    pub cpu: Cpu,
    pub cycle: u64,
}

impl Emu {
    pub fn new(binary: Vec<u8>, base_addr: u64, _dump_count: u64) -> Self {
        Self {
            breakpoints: vec![0; 32 as usize],
            exec_mode: ExecMode::Continue,
            cpu: Cpu::new(binary, base_addr, _dump_count as u64),
            cycle: 0,
        }
    }

    /// single-step the interpreter
    pub fn step(&mut self) -> Option<Event> {
        let pc = self.cpu.step_run();

        info!("PC: {:#x} Cycle: {}", pc, self.cycle);

        self.cycle += 1;
        if self.cycle % 10000 == 0 {
            let path = std::path::PathBuf::from(format!("/tmp/snapshot_{}.bin", self.cycle));
            self.save_snapshot(path.clone());
            info!("Snapshot saved to {}", path.clone().display());
        }
        
        if self.breakpoints.contains(&pc) {
            return Some(Event::Break);
        }

        None
    }

    pub fn run(&mut self, mut poll_incoming_data: impl FnMut() -> bool) -> RunEvent {
        match self.exec_mode {
            ExecMode::Step => RunEvent::Event(self.step().unwrap_or(Event::DoneStep)),
            ExecMode::Continue => {
                let mut cycles = 0;
                loop {
                    if cycles % 1024 == 0 {
                        // poll for incoming data
                        if poll_incoming_data() {
                            break RunEvent::IncomingData;
                        }
                    }
                    cycles += 1;

                    if let Some(event) = self.step() {
                        break RunEvent::Event(event);
                    };

                    if cycles % 1000000 == 0 {
                        let path = std::path::PathBuf::from(format!("/tmp/snapshot_{}.bin", cycles));
                        self.save_snapshot(path.clone());
                        info!("Snapshot saved to {}", path.clone().display());
                    }
                }
            }
        }
    }

    pub fn set_entry_point(&mut self, entry_addr: u64) {
        self.cpu.pc = entry_addr;
    }

    pub fn save_snapshot(&self, path: std::path::PathBuf) {
        let config = bincode::config::standard()
            .with_little_endian()
            .with_fixed_int_encoding();
        let mut file = File::create(path).expect("Unable to create file");
        let snapshot = self.cpu.to_snapshot();
        let data = bincode::serde::encode_to_vec(snapshot, config).expect("Unable to serialize snapshot");
        file.write_all(&data).expect("Unable to write data");
        // info!("Snapshot saved to {}", path);
    }

    pub fn from_snapshot(path: std::path::PathBuf, reg_dump_count: u64) -> Result<Emu, std::io::Error> {
        let config = bincode::config::standard()
            .with_little_endian()
            .with_fixed_int_encoding();
        let mut file = File::open(path).expect("Unable to open file");
        let mut data = Vec::new();
        file.read_to_end(&mut data).expect("Unable to read snapshot data");
        let snapshot: CpuSnapshot = bincode::serde::decode_from_slice(&data, config).expect("Unable to deserialize snapshot").0;
        let cpu = Cpu::from_snapshot(snapshot);
        // info!("Snapshot loaded from {}", path);
        Ok(Self {
            breakpoints: vec![0; 32 as usize],
            exec_mode: ExecMode::Continue,
            cpu: cpu,
            cycle: 0,
        })
    }
}
