use crate::cpu::*;
use bincode;
use log::info;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

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
    pub snapshot_interval: u64,
}

#[derive(Serialize, Deserialize)]
pub struct EmuSnapshot {
    pub cpu: CpuSnapshot,
    pub cycle: u64,
}

impl Emu {
    pub fn new(binary: Vec<u8>, base_addr: u64, _dump_count: u64, _snapshot_interval: u64) -> Self {
        Self {
            breakpoints: vec![0; 32 as usize],
            exec_mode: ExecMode::Continue,
            cpu: Cpu::new(binary, base_addr, _dump_count as u64),
            cycle: 0,
            snapshot_interval: _snapshot_interval,
        }
    }

    /// single-step the interpreter
    pub fn step(&mut self) -> Option<Event> {
        let pc = self.cpu.step_run();

        self.cycle += 1;
        if self.cycle % self.snapshot_interval == 0 {
            let path = std::path::PathBuf::from(format!("log/snapshot_{}.bin", self.cycle));
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
                self.cpu.block_run();
                if self.breakpoints.contains(&self.cpu.pc) {
                    RunEvent::Event(Event::Break)
                } else if poll_incoming_data() {
                    RunEvent::IncomingData
                } else {
                    RunEvent::Event(Event::DoneStep)
                }
            }
        }
    }

    pub fn set_entry_point(&mut self, entry_addr: u64) {
        self.cpu.pc = entry_addr;
    }

    pub fn set_disk_image(&mut self, disk_image: Vec<u8>) {
        if let Some(virtio) = &mut self.cpu.bus.virtio {
            virtio.set_disk_image(disk_image);
        } else {
            panic!("Virtio not initialized: No virtio device found in bus");
        }
    }

    pub fn to_snapshot(&self) -> EmuSnapshot {
        EmuSnapshot {
            cpu: self.cpu.to_snapshot(),
            cycle: self.cycle,
        }
    }

    pub fn from_snapshot(snapshot: EmuSnapshot) -> Self {
        let cpu = Cpu::from_snapshot(snapshot.cpu);
        Self {
            breakpoints: vec![0; 32 as usize],
            exec_mode: ExecMode::Continue,
            cpu: cpu,
            cycle: snapshot.cycle,
            snapshot_interval: 100000000,
        }
    }

    pub fn save_snapshot(&self, path: std::path::PathBuf) {
        let config = bincode::config::standard()
            .with_little_endian()
            .with_fixed_int_encoding();
        let mut file = File::create(path).expect("Unable to create file");
        let snapshot = self.to_snapshot();
        let data =
            bincode::serde::encode_to_vec(snapshot, config).expect("Unable to serialize snapshot");
        file.write_all(&data).expect("Unable to write data");
        // info!("Snapshot saved to {}", path);
    }

    pub fn load_snapshot(path: std::path::PathBuf) -> Result<Emu, std::io::Error> {
        let config = bincode::config::standard()
            .with_little_endian()
            .with_fixed_int_encoding();
        let mut file = File::open(path.clone()).expect("Unable to open file");
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .expect("Unable to read snapshot data");
        let snapshot: EmuSnapshot = bincode::serde::decode_from_slice(&data, config)
            .expect("Unable to deserialize snapshot")
            .0;
        let emu = Emu::from_snapshot(snapshot);
        Ok(emu)
    }
}
