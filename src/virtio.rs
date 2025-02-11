use crate::interrupt::*;
use log::{debug, info};

// virtio mmio control registers, mapped starting at 0x10001000.
// from qemu virtio_mmio.h
#[allow(dead_code)]
const VIRTIO_MMIO_MAGIC_VALUE: usize = 0x000; // 0x74726976
#[allow(dead_code)]
const VIRTIO_MMIO_VERSION: usize = 0x004; // version; 1 is legacy
#[allow(dead_code)]
const VIRTIO_MMIO_DEVICE_ID: usize = 0x008; // device type; 1 is net, 2 is disk
#[allow(dead_code)]
const VIRTIO_MMIO_VENDOR_ID: usize = 0x00c; // 0x554d4551
#[allow(dead_code)]
const VIRTIO_MMIO_DEVICE_FEATURES: usize = 0x010;
#[allow(dead_code)]
const VIRTIO_MMIO_DRIVER_FEATURES: usize = 0x020;
#[allow(dead_code)]
const VIRTIO_MMIO_GUEST_PAGE_SIZE: usize = 0x028; // page size for PFN, write-only
#[allow(dead_code)]
const VIRTIO_MMIO_QUEUE_SEL: usize = 0x030; // select queue, write-only
#[allow(dead_code)]
const VIRTIO_MMIO_QUEUE_NUM_MAX: usize = 0x034; // max size of current queue, read-only
#[allow(dead_code)]
const VIRTIO_MMIO_QUEUE_NUM: usize = 0x038; // size of current queue, write-only
#[allow(dead_code)]
const VIRTIO_MMIO_QUEUE_ALIGN: usize = 0x03c; // used ring alignment, write-only
#[allow(dead_code)]
const VIRTIO_MMIO_QUEUE_PFN: usize = 0x040; // physical page number for queue, read/write
#[allow(dead_code)]
const VIRTIO_MMIO_QUEUE_READY: usize = 0x044; // ready bit
#[allow(dead_code)]
const VIRTIO_MMIO_QUEUE_NOTIFY: usize = 0x050; // write-only
#[allow(dead_code)]
const VIRTIO_MMIO_INTERRUPT_STATUS: usize = 0x060; // read-only
#[allow(dead_code)]
const VIRTIO_MMIO_INTERRUPT_ACK: usize = 0x064; // write-only
#[allow(dead_code)]
const VIRTIO_MMIO_STATUS: usize = 0x070; // read/write

pub struct Virtio {
    start_addr: u64,
    size: u64,
    registers: Vec<u8>,
}

impl Virtio {
    pub fn new(_start_addr: u64, _size: u64) -> Virtio {
        let mut _registers = vec![0; _size as usize];

        _registers[VIRTIO_MMIO_MAGIC_VALUE + 0] = 0x76;
        _registers[VIRTIO_MMIO_MAGIC_VALUE + 1] = 0x69;
        _registers[VIRTIO_MMIO_MAGIC_VALUE + 2] = 0x72;
        _registers[VIRTIO_MMIO_MAGIC_VALUE + 3] = 0x74;
        _registers[VIRTIO_MMIO_VERSION] = 2;
        _registers[VIRTIO_MMIO_DEVICE_ID] = 2;
        _registers[VIRTIO_MMIO_VENDOR_ID + 0] = 0x51;
        _registers[VIRTIO_MMIO_VENDOR_ID + 1] = 0x45;
        _registers[VIRTIO_MMIO_VENDOR_ID + 2] = 0x4d;
        _registers[VIRTIO_MMIO_VENDOR_ID + 3] = 0x55;
        _registers[VIRTIO_MMIO_QUEUE_NUM_MAX] = 10;

        Self {
            start_addr: _start_addr,
            size: _size,
            registers: _registers,
        }
    }

    pub fn is_accessible(&self, addr: u64) -> bool {
        (addr >= self.start_addr) && (addr < self.start_addr + self.size)
    }

    pub fn load(&self, addr: u64, size: u64) -> Result<u64, Exception> {
        let relative_addr = (addr - self.start_addr) as usize;
        let ret_val = match size {
            8 => (self.registers[relative_addr + 0] << 0) as u64,
            16 => {
                ((self.registers[relative_addr + 0] as u64) << 0)
                    | ((self.registers[relative_addr + 1] as u64) << 8)
            }
            32 => {
                ((self.registers[relative_addr + 0] as u64) << 0)
                    | ((self.registers[relative_addr + 1] as u64) << 8)
                    | ((self.registers[relative_addr + 2] as u64) << 16)
                    | ((self.registers[relative_addr + 3] as u64) << 24)
            }
            64 => {
                ((self.registers[relative_addr + 0] as u64) << 0)
                    | ((self.registers[relative_addr + 1] as u64) << 8)
                    | ((self.registers[relative_addr + 2] as u64) << 16)
                    | ((self.registers[relative_addr + 3] as u64) << 24)
                    | ((self.registers[relative_addr + 4] as u64) << 32)
                    | ((self.registers[relative_addr + 5] as u64) << 40)
                    | ((self.registers[relative_addr + 6] as u64) << 48)
                    | ((self.registers[relative_addr + 7] as u64) << 56)
            }
            _ => {
                panic!("Invalid access size: {}", size)
            }
        };
        info!("virtio: load addr:{:x}(relative {:x}), size:{}, value:{}", addr, relative_addr, size, ret_val);
        Ok(ret_val)
    }

    pub fn store(&mut self, addr: u64, size: u64, value: u64) -> Result<(), Exception> {
        info!("virtio: store addr:{:x}, size:{}, value:{}", addr, size, value);
        let relative_addr = (addr - self.start_addr) as usize;
        match size {
            8 => self.registers[relative_addr + 0] = (value & 0xff) as u8,
            16 => {
                self.registers[relative_addr + 0] = ((value << 0) & 0xff) as u8;
                self.registers[relative_addr + 1] = ((value << 8) & 0xff) as u8;
            }
            32 => {
                self.registers[relative_addr + 0] = ((value << 0) & 0xff) as u8;
                self.registers[relative_addr + 1] = ((value << 8) & 0xff) as u8;
                self.registers[relative_addr + 2] = ((value << 16) & 0xff) as u8;
                self.registers[relative_addr + 3] = ((value << 24) & 0xff) as u8;
            }
            64 => {
                self.registers[relative_addr + 0] = ((value << 0) & 0xff) as u8;
                self.registers[relative_addr + 1] = ((value << 8) & 0xff) as u8;
                self.registers[relative_addr + 2] = ((value << 16) & 0xff) as u8;
                self.registers[relative_addr + 3] = ((value << 24) & 0xff) as u8;
                self.registers[relative_addr + 4] = ((value << 32) & 0xff) as u8;
                self.registers[relative_addr + 5] = ((value << 40) & 0xff) as u8;
                self.registers[relative_addr + 6] = ((value << 48) & 0xff) as u8;
                self.registers[relative_addr + 7] = ((value << 56) & 0xff) as u8;
            }
            _ => {}
        };
        debug!("register after store: 0x{} at 0x{:x}", self.registers[relative_addr], addr);
        Ok(())
    }
}
