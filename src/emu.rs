use crate::cpu::*;
use crate::instruction::*;
use crate::bus::*;
use crate::interrupt::*;
use crate::virtio::*;
use crate::plic::ExternalInterrupt;

use bincode;
use log::info;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::collections::BTreeSet;
use std::cell::RefCell;
use std::rc::Rc;

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
    pub bus: Rc<RefCell<Bus>>,
    pub virtio: Rc<RefCell<Virtio>>,
    pub cycle: u64,
    pub snapshot_interval: u64,
}

#[derive(Serialize, Deserialize)]
pub struct EmuSnapshot {
    pub cpu: CpuSnapshot,
    pub bus: BusSnapshot,
    pub virtio: VirtioSnapshot,
    pub cycle: u64,
}

impl Emu {
    pub fn new(binary: Vec<u8>, base_addr: u64, _dump_count: u64, _snapshot_interval: u64) -> Self {
        let interrupt_list = Rc::new(RefCell::new(BTreeSet::<Interrupt>::new()));
        let bus = Bus::new(binary.clone(), base_addr, interrupt_list.clone());
        let virtio = Rc::new(
            RefCell::new(
                Virtio::new(
                    0x10001000, 
                    bus.borrow().plic.get_interrupt_notificator(ExternalInterrupt::VirtioDiskIO),
                )
            )
        );
        bus.borrow_mut().virtio = Some(Rc::clone(&virtio));
        virtio.borrow_mut().set_bus(Rc::clone(&bus));
        Self {
            breakpoints: vec![0; 32 as usize],
            exec_mode: ExecMode::Continue,
            cpu: Cpu::new(Rc::clone(&bus), base_addr, _dump_count as u64, interrupt_list),
            bus: Rc::clone(&bus),
            virtio: Rc::clone(&virtio),
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

                let mut block_cache = std::collections::HashMap::<u64, BasicBlock>::new();
                let mut last_cycle_before_snapshot: u64 = 0;
                while !poll_incoming_data() {
                    self.cpu.trap_interrupt();
                    {
                        self.virtio.borrow_mut().disk_access();
                    }
                    let pc = self.cpu.pc;
                    let block = block_cache.entry(pc).or_insert_with(|| self.cpu.build_basic_block());
                    let cycle = self.cpu.run_block(block);
                    last_cycle_before_snapshot += cycle;
                    if last_cycle_before_snapshot > self.snapshot_interval {
                        let path = std::path::PathBuf::from(format!("log/snapshot_{}.bin", self.cycle));
                        self.save_snapshot(path.clone());
                        info!("Snapshot saved to {}", path.clone().display());
                        last_cycle_before_snapshot %= self.snapshot_interval;
                    }
                    self.cycle += cycle;
                }

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
        let virtio = &mut self.virtio.as_ref().borrow_mut();
        virtio.set_disk_image(disk_image);
    }

    pub fn to_snapshot(&self) -> EmuSnapshot {
        EmuSnapshot {
            cpu: self.cpu.to_snapshot(),
            bus: self.bus.borrow().to_snapshot(),
            cycle: self.cycle,
            virtio: self.virtio.borrow().to_snapshot(),
        }
    }

    pub fn from_snapshot(snapshot: EmuSnapshot) -> Self {
        let interrupt_list = Rc::new(RefCell::new(BTreeSet::<Interrupt>::new()));
        let bus = Rc::new(RefCell::new(Bus::from_snapshot(snapshot.bus, interrupt_list.clone())));
        let virtio = Rc::new(
            RefCell::new(
                Virtio::from_snapshot(snapshot.virtio,
                    bus.borrow().plic.get_interrupt_notificator(ExternalInterrupt::VirtioDiskIO)
                )
            )
        );
        let mut cpu = Cpu::from_snapshot(snapshot.cpu, Rc::clone(&bus));
        cpu.bus = Rc::clone(&bus);
        bus.borrow_mut().virtio = Some(Rc::clone(&virtio));
        virtio.borrow_mut().set_bus(Rc::clone(&bus));
        info!("emu is made from snapshot!");
        Self {
            breakpoints: vec![0; 32 as usize],
            exec_mode: ExecMode::Continue,
            cpu: cpu,
            bus: Rc::clone(&bus),
            virtio: Rc::clone(&virtio),
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
