# rv-emu Architecture

## Current Structure

### Module Overview (4,702 lines total)

| Module | Lines | Role |
|--------|-------|------|
| `cpu.rs` | 1,536 | Execution, TLB, traps, CSR ops |
| `instruction.rs` | 596 | Instruction decoding (giant match) |
| `interrupt.rs` | 307 | Interrupt/exception types + take_trap |
| `emu.rs` | 434 | Orchestration, snapshot/restore |
| `virtio.rs` | 381 | Virtio block device |
| `debugger.rs` | 246 | GDB remote protocol |
| `main.rs` | 259 | CLI + ELF loader |
| `csr.rs` | 201 | CSR array (4096 registers) |
| `bus.rs` | 167 | MMIO address router |
| `uart.rs` | 171 | UART device (stdin/stdout) |
| `plic.rs` | 148 | Platform interrupt controller |
| `dram.rs` | 111 | 128 MB flat memory |
| `clint.rs` | 79 | Timer (stub, mostly dead) |

### Ownership and Shared State

```
Emu
├── cpu: Cpu
│   ├── bus: Rc<RefCell<Bus>>          ← shared with Emu
│   ├── csr: Csr
│   │   ├── cycle: Rc<RefCell<u64>>    ← shared with Cpu
│   │   └── interrupt_list: Rc<RefCell<BTreeSet<Interrupt>>>  ← shared
│   ├── interrupt_list: Rc<RefCell<BTreeSet<Interrupt>>>      ← same Rc
│   ├── block_cache: HashMap<u64, BasicBlock>  ← declared, never inserted
│   └── dest/src1/src2: usize                  ← written, never read
├── bus: Rc<RefCell<Bus>>
│   ├── dram: Dram
│   ├── uart: Uart
│   │   └── recv_buf: Arc<Mutex<VecDeque<u8>>>  ← crosses thread boundary
│   ├── plic: Plic
│   │   ├── sender: Sender<ExternalInterrupt>   ← mpsc channel
│   │   ├── receiver: Receiver<ExternalInterrupt>
│   │   ├── external_interrupt_list: BTreeSet<ExternalInterrupt>
│   │   └── interrupt_list: Rc<RefCell<BTreeSet<Interrupt>>>  ← same Rc
│   └── virtio: Option<Rc<RefCell<Virtio>>>     ← Option because added post-construction
├── virtio: Rc<RefCell<Virtio>>
└── cycle: u64
```

### Interrupt Routing (Current)

```
UART input thread
  → Arc<Box<dyn Fn()>> notificator
  → Sender<ExternalInterrupt>              ← mpsc channel (extra hop)
  → PLIC.process_pending_interrupts()      ← must be called explicitly
  → external_interrupt_list: BTreeSet
  → Rc<RefCell<BTreeSet<Interrupt>>>       ← shared interrupt_list
  → Cpu.trap_interrupt()                   ← polled each cycle
```

### Snapshot Layout (Current)

```
EmuSnapshot
├── cpu: CpuSnapshot         (regs, pc, csr[4096], mode, cycle, clint, interrupt_list, TLB)
├── bus: BusSnapshot         (dram, uart, plic)  ← Virtio excluded here
├── virtio: VirtioSnapshot   (registers + full disk image)  ← separate from bus
└── cycle: u64
```

### Key Problems

1. **`cpu.rs` is a God class** — mixes instruction execution, SV39 page-table walk,
   TLB management, trap/interrupt handling, and basic-block compilation in 1,536 lines.

2. **`Rc<RefCell<>>` fan-out** — fragile shared mutable state across 5+ owners.
   Runtime borrow panics are possible. The `Bus`↔`Virtio` circular dep required
   `Option<Rc<RefCell<Virtio>>>` as a workaround.

3. **Two-stage interrupt routing** — PLIC uses an mpsc channel that must be explicitly
   drained, adding indirection without benefit.

4. **Dead code**:
   - `block_cache` — HashMap declared, insertion commented out
   - `dest`/`src1`/`src2` fields on `Cpu` — written but never read
   - `DelayedInterrupt` struct — defined, never used
   - `Clint` — timer thread commented out; load/store are no-ops

5. **Dual cycle counters** — `Emu.cycle` and `Cpu.cycle` previously caused
   double-counting (fixed 2026-03-02, but the redundancy remains).

6. **Asymmetric snapshot** — `BusSnapshot` excludes Virtio; `VirtioSnapshot` is a
   sibling of `BusSnapshot` in `EmuSnapshot` rather than nested inside it.

---

## Proposed Structure

### Module Split

```
cpu.rs       ~400 lines   Cpu struct, registers, PC, mode, step()/run_block()
mmu.rs       ~250 lines   translate(), SV39 3-level walk, TLB cache
execute.rs   ~700 lines   execute_instruction() dispatch (moved from cpu.rs)
interrupt.rs  ~307 lines  unchanged: Exception/Interrupt types + take_trap
```

All other modules remain; dead code removed.

### Ownership (Proposed)

Replace `Rc<RefCell<>>` fan-out with explicit `&mut` passing:

```
Emu  (owns everything)
├── cpu: Cpu              ← owns regs, PC, mode, TLB, CSR
├── bus: Bus              ← plain ownership, no Rc
├── virtio: Virtio        ← plain ownership, no Rc, no Option
├── interrupts: InterruptState
└── cycle: u64            ← single source of truth
```

```rust
// Cpu no longer stores Bus; it receives it per call
impl Cpu {
    pub fn step(&mut self, bus: &mut Bus, interrupts: &mut InterruptState, cycle: u64) -> StepResult;
    pub fn load(&mut self, bus: &mut Bus, va: u64, size: u64) -> Result<u64, Exception>;
    pub fn store(&mut self, bus: &mut Bus, va: u64, size: u64, val: u64) -> Result<(), Exception>;
}

// CSR TIME derived from cycle passed in, not a shared Rc
impl Csr {
    pub fn load(&self, addr: u64, cycle: u64) -> u64;
}
```

Only the UART input thread boundary still needs `Arc<Mutex<>>` — isolated to `uart.rs`.

### Interrupt Routing (Proposed)

Remove the mpsc channel from PLIC. Use a single `InterruptState` protected by
`Arc<Mutex<>>` at the one real thread boundary (UART → Emu):

```
UART input thread
  → Arc<Mutex<InterruptState>>             ← single lock, no channel
  → InterruptState.external_pending.insert(UartInput)

Virtio (same thread as Emu)
  → &mut InterruptState                    ← no locking needed
  → InterruptState.external_pending.insert(VirtioDiskIO)

Emu main loop each cycle:
  → PLIC.process(&mut interrupts)          ← drain external_pending → pending
  → Cpu.trap_interrupt(&interrupts)
```

### Snapshot Layout (Proposed)

Flat, symmetric layout — all peripheral snapshots at the same level:

```
EmuSnapshot
├── cpu: CpuSnapshot        (regs, pc, csr[4096], mode, TLB)
├── dram: Dram              (moved out of BusSnapshot)
├── uart: UartSnapshot
├── plic: PlicSnapshot
├── virtio: VirtioSnapshot  (registers + disk image)
└── cycle: u64              (single cycle field)
```

### Dead Code to Remove

| Item | Location | Reason |
|------|----------|--------|
| `block_cache` | `cpu.rs` | Declared, insertion commented out |
| `dest`/`src1`/`src2` | `cpu.rs` | Written, never read |
| `DelayedInterrupt` | `interrupt.rs` | Defined, never used |
| `Clint` struct | `clint.rs` | Timer stub; can replace with inline `match` returning 0 |
| `Cpu.cycle: Rc<RefCell<u64>>` | `cpu.rs` | Replaced by `Emu.cycle` passed explicitly |

### Change Summary

| Concern | Current | Proposed |
|---------|---------|----------|
| cpu.rs size | 1,536 lines (God class) | ~400 lines + `mmu.rs` + `execute.rs` |
| Shared state | `Rc<RefCell<>>` to 5+ owners | `&mut` passing; `Arc<Mutex<>>` only at thread boundary |
| Bus↔Virtio | `Option<Rc<RefCell<>>>` | `Emu` owns both; `Bus` gets `&mut Virtio` when needed |
| Cycle counter | `Emu.cycle` + `Cpu.cycle` Rc | `Emu.cycle` only |
| Interrupt routing | mpsc channel → BTreeSet | Direct `Arc<Mutex<InterruptState>>` |
| Snapshot layout | `BusSnapshot` ≠ `VirtioSnapshot` | Flat `EmuSnapshot` |
| Dead code | `block_cache`, `dest/src1/src2`, `DelayedInterrupt`, `Clint` | Removed |
