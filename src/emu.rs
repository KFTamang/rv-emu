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
            breakpoints: Vec::new(),
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

    fn run_block_with_breakpoints(&mut self, block: &mut BasicBlock) -> u64 {
        for breakpoint in &self.breakpoints {
            if block.start_pc <= *breakpoint && *breakpoint < block.end_pc {
                block.end_pc = *breakpoint;
                break;
            }
        }
        self.cpu.run_block(block)
    }

    pub fn run(&mut self, mut poll_incoming_data: impl FnMut() -> bool) -> RunEvent {
        match self.exec_mode {
            ExecMode::Step => RunEvent::Event(self.step().unwrap_or(Event::DoneStep)),
            ExecMode::Continue => {

                let mut last_cycle_before_snapshot: u64 = 0;
                let mut cycle = 1;
                while !poll_incoming_data() {
                    self.cpu.trap_interrupt();
                    {
                        self.virtio.borrow_mut().disk_access();
                    }
                    match self.cpu.build_basic_block() {
                        Ok(mut block) => {
                            cycle = self.run_block_with_breakpoints(&mut block);
                        },
                        Err(exception) => {
                            exception.take_trap(&mut self.cpu);
                        }
                    }
                    last_cycle_before_snapshot += cycle;
                    if last_cycle_before_snapshot > self.snapshot_interval {
                        let path = std::path::PathBuf::from(format!("log/snapshot_{}.bin", self.cycle));
                        self.save_snapshot(path.clone());
                        info!("Snapshot saved to {}", path.clone().display());
                        last_cycle_before_snapshot %= self.snapshot_interval;
                    }
                    self.cycle += cycle;
                    if self.breakpoints.contains(&self.cpu.pc) {
                        return RunEvent::Event(Event::Break);
                    }
                }
                if poll_incoming_data() {
                    RunEvent::IncomingData
                }else {
                    RunEvent::Event(Event::DoneStep)
                }
            }
        }
    }

    pub fn run_for(&mut self, interation: u64) -> RunEvent {
        match self.exec_mode {
            ExecMode::Step => RunEvent::Event(self.step().unwrap_or(Event::DoneStep)),
            ExecMode::Continue => {

                let mut last_cycle_before_snapshot: u64 = 0;
                let mut cycle = 1;
                while self.cycle < interation {
                    self.cpu.trap_interrupt();
                    {
                        self.virtio.borrow_mut().disk_access();
                    }
                    match self.cpu.build_basic_block() {
                        Ok(mut block) => {
                            cycle = self.run_block_with_breakpoints(&mut block);
                        },
                        Err(exception) => {
                            exception.take_trap(&mut self.cpu);
                        }
                    }
                    last_cycle_before_snapshot += cycle;
                    if last_cycle_before_snapshot > self.snapshot_interval {
                        let path = std::path::PathBuf::from(format!("log/snapshot_{}.bin", self.cycle));
                        self.save_snapshot(path.clone());
                        info!("Snapshot saved to {}", path.clone().display());
                        last_cycle_before_snapshot %= self.snapshot_interval;
                    }
                    self.cycle += cycle;
                    if self.breakpoints.contains(&self.cpu.pc) {
                        return RunEvent::Event(Event::Break);
                    }
                }
                RunEvent::Event(Event::DoneStep)}
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
        // Restore the interrupt_list from the CPU snapshot so that both the bus
        // (PLIC/UART notificators) and the CPU share the same Rc.  Without this,
        // interrupts fired by peripherals after restore are invisible to the CPU.
        let interrupt_list = Rc::new(RefCell::new(snapshot.cpu.interrupt_list.clone()));
        let bus = Rc::new(RefCell::new(Bus::from_snapshot(snapshot.bus, interrupt_list.clone())));
        let virtio = Rc::new(
            RefCell::new(
                Virtio::from_snapshot(snapshot.virtio,
                    bus.borrow().plic.get_interrupt_notificator(ExternalInterrupt::VirtioDiskIO)
                )
            )
        );
        let mut cpu = Cpu::from_snapshot(snapshot.cpu, Rc::clone(&bus), interrupt_list);
        cpu.bus = Rc::clone(&bus);
        bus.borrow_mut().virtio = Some(Rc::clone(&virtio));
        virtio.borrow_mut().set_bus(Rc::clone(&bus));
        info!("emu is made from snapshot!");
        Self {
            breakpoints: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    fn make_emu(binary: Vec<u8>, base_addr: u64) -> Emu {
        let mut emu = Emu::new(binary, base_addr, 0, u64::MAX);
        emu.exec_mode = ExecMode::Continue;
        emu
    }

    /// Capture the CPU-visible state needed for reproducibility comparison.
    /// Returns (regs[0..32], pc, mode, csr[0..4096]).
    fn capture_state(emu: &Emu) -> ([u64; 32], u64, u64, Box<[u64; 4096]>) {
        let regs = emu.cpu.regs;
        let pc = emu.cpu.pc;
        let mode = emu.cpu.mode;
        let csr_snap = emu.cpu.csr.to_snapshot();
        let mut csr_arr = Box::new([0u64; 4096]);
        csr_arr.copy_from_slice(&csr_snap.csr);
        (regs, pc, mode, csr_arr)
    }

    fn assert_states_eq(
        a: &([u64; 32], u64, u64, Box<[u64; 4096]>),
        b: &([u64; 32], u64, u64, Box<[u64; 4096]>),
    ) {
        assert_eq!(a.0, b.0, "registers differ after snapshot resume");
        assert_eq!(a.1, b.1, "PC differs after snapshot resume");
        assert_eq!(a.2, b.2, "privilege mode differs after snapshot resume");
        // Compare non-time CSRs. TIME (0xc01) is wall-clock-derived in
        // load_csrs() and not stored in the array, so any stored mismatch is real.
        assert_eq!(a.3[..], b.3[..], "CSR array differs after snapshot resume");
    }

    // ---------------------------------------------------------------------------
    // Test 1: registers and CSRs are reproducible across snapshot / resume
    //
    // Uses apps/fib.bin – a simple flat binary linked at 0x0 that just
    // computes Fibonacci numbers in a loop.  No UART/PLIC/virtio is needed.
    // ---------------------------------------------------------------------------
    #[test]
    fn test_snapshot_registers_and_csrs_reproducible() {
        let binary = std::fs::read("apps/fib.bin")
            .expect("apps/fib.bin must exist; run `make build_apps` if missing");

        const SNAPSHOT_AT: u64 = 50000;
        const RUN_TOTAL: u64 = 200000;

        // -- Run 1: from scratch, snapshot mid-way, continue to end ----------
        let mut emu1 = make_emu(binary.clone(), 0);
        emu1.run_for(SNAPSHOT_AT);
        let mid_snapshot = emu1.to_snapshot();
        emu1.run_for(RUN_TOTAL);
        let state1 = capture_state(&emu1);

        // -- Run 2: restore mid-way snapshot, run to same total --------------
        let mut emu2 = Emu::from_snapshot(mid_snapshot);
        emu2.exec_mode = ExecMode::Continue;
        emu2.run_for(RUN_TOTAL);
        let state2 = capture_state(&emu2);

        assert_states_eq(&state1, &state2);
    }

    // ---------------------------------------------------------------------------
    // Test 2: virtio disk contents survive snapshot / restore
    //
    // Verifies that VirtioSnapshot correctly serialises and deserialises the
    // disk image so no disk data is silently lost or corrupted.
    // ---------------------------------------------------------------------------
    #[test]
    fn test_snapshot_virtio_disk_preserved() {
        let binary = std::fs::read("apps/fib.bin")
            .expect("apps/fib.bin must exist");

        // Build a recognisable synthetic disk image.
        let disk_size = 512 * 4; // 4 sectors
        let disk_image: Vec<u8> = (0..disk_size as u8).collect();

        let mut emu = make_emu(binary, 0);
        emu.set_disk_image(disk_image.clone());

        // Run briefly so there is some CPU state to preserve as well.
        emu.run_for(20);

        let snap = emu.to_snapshot();

        // Restore and compare disk byte-for-byte.
        let emu2 = Emu::from_snapshot(snap);
        let disk2 = emu2.virtio.borrow().disk_snapshot();
        assert_eq!(disk2, disk_image, "disk image corrupted through snapshot/restore");
    }

    // ---------------------------------------------------------------------------
    // Test 3: file round-trip – save to disk and load back, state preserved
    // ---------------------------------------------------------------------------
    #[test]
    fn test_snapshot_file_roundtrip() {
        let binary = std::fs::read("apps/fib.bin")
            .expect("apps/fib.bin must exist");

        let mut emu = make_emu(binary, 0);
        emu.run_for(30);
        let state_before = capture_state(&emu);

        // Save to a temp file.
        let path = std::path::PathBuf::from("log/test_snapshot_roundtrip.bin");
        std::fs::create_dir_all("log").ok();
        emu.save_snapshot(path.clone());

        // Load and compare.
        let emu2 = Emu::load_snapshot(path.clone()).expect("load_snapshot failed");
        let state_after = capture_state(&emu2);
        std::fs::remove_file(path).ok();

        assert_states_eq(&state_before, &state_after);
    }

    // ---------------------------------------------------------------------------
    // Test 4: XV6 + real disk image – registers and CSRs reproducible
    //
    // Loads apps/xv6-riscv/kernel/kernel (ELF) and fs.img, runs for
    // RUN_TOTAL instruction cycles with a snapshot taken at SNAPSHOT_AT.
    // The snapshot is restored and run to the same total; final CPU state must
    // be bit-identical.
    //
    // This test exercises the full emulator path including virtio disk I/O and
    // the PLIC interrupt path repaired in Emu::from_snapshot.
    //
    // Marked #[ignore] because it is slow in debug mode (~minutes).
    // Run with:  cargo test --release -- --ignored
    //        or: cargo test -- --ignored   (debug, expect several minutes)
    // ---------------------------------------------------------------------------
    #[test]
    #[ignore]
    fn test_xv6_snapshot_reproducible() {
        const BASE_ADDR: u64 = 0x8000_0000;
        const SNAPSHOT_AT: u64 = 1_000_000;
        const RUN_TOTAL: u64 = 3_000_000;

        // --- Load XV6 kernel ELF -------------------------------------------
        let mut code: Vec<u8> = Vec::new();
        let mut kernel_file = std::fs::File::open("apps/xv6-riscv/kernel/kernel")
            .expect("apps/xv6-riscv/kernel/kernel must exist");
        let entry = crate::load_elf(&mut code, &mut kernel_file, BASE_ADDR as usize)
            .expect("failed to load xv6 kernel ELF");

        let disk_image = std::fs::read("apps/xv6-riscv/fs.img")
            .expect("apps/xv6-riscv/fs.img must exist");

        // --- Run 1: from scratch, snapshot mid-way --------------------------
        let mut emu1 = Emu::new(code.clone(), BASE_ADDR, 0, u64::MAX);
        emu1.set_entry_point(entry);
        emu1.set_disk_image(disk_image.clone());
        emu1.exec_mode = ExecMode::Continue;

        emu1.run_for(SNAPSHOT_AT);
        let mid_snapshot = emu1.to_snapshot();
        emu1.run_for(RUN_TOTAL);
        let state1 = capture_state(&emu1);

        // --- Run 2: restore mid-way snapshot, run to same total -------------
        let mut emu2 = Emu::from_snapshot(mid_snapshot);
        emu2.exec_mode = ExecMode::Continue;
        emu2.run_for(RUN_TOTAL);
        let state2 = capture_state(&emu2);

        assert_states_eq(&state1, &state2);
    }
}
