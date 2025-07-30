pub struct Bus {
    pub dram: Dram,
    pub uart: Uart,
    pub plic: Plic,
    pub virtio: Option<Rc<RefCell<Virtio>>>,
}
pub struct Emu {
    pub cpu: Cpu,
    pub bus: Rc<RefCell<Bus>>,
    pub virtio: Rc<RefCell<Virtio>>,
}

pub struct Cpu {
    pub regs: [u64; 32],
    pub pc: u64,
    pub bus: Rc<RefCell<Bus>>,
    pub csr: Csr,
}

impl Emu {
    pub fn run(&mut self, mut poll_incoming_data: impl FnMut() -> bool) -> RunEvent {
        let mut block_cache = std::collections::HashMap::<u64, BasicBlock>::new();

        while self.cpu.pc != 0 {
            self.cpu.trap_interrupt();
            self.virtio
                .borrow_mut()
                .disk_access();
            let pc = self.cpu.pc;
            let block = block_cache.entry(pc).or_insert_with(|| self.cpu.build_basic_block());
            self.cpu.run_block(block);
        }
    }
}

impl Bus {
    pub fn new(
        code: Vec<u8>,
        base_addr: u64,
        interrupt_list: Rc<RefCell<BTreeSet<Interrupt>>>,
    ) -> Rc<RefCell<Bus>> {
        let plic = Plic::new(0xc000000, interrupt_list.clone());
        let uart_notificator = plic.get_interrupt_notificator(ExternalInterrupt::UartInput);
        let bus_rc = Rc::new(RefCell::new(Self {
            plic,
            dram: Dram::new(code, base_addr),
            uart: Uart::new(0x10000000, uart_notificator),
            virtio: None,
        }));
        bus_rc
    }

    pub fn load(&mut self, addr: u64, size: u64) -> Result<u64, Exception> {
        if self.dram.dram_base <= addr {
            let ret_val = self.dram.load(addr, size);
            return ret_val; 
        }
        let mut virtio = self.virtio
            .as_ref()
            .expect("No virtio bus")
            .borrow_mut();
        if virtio.is_accessible(addr) {
            let ret_val = virtio.load(addr, size);
            return ret_val;
        }
        Err(Exception::LoadAccessFault)
    }