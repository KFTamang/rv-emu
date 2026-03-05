use crate::dram::Dram;
use crate::interrupt::*;
use log::info;
use serde::{Deserialize, Serialize};

const VIRTIO_SIZE: u64 = 0x1000; // size of virtio mmio device
const VRING_DESC_SIZE: u64 = 16;
/// The number of virtio descriptors. It must be a power of two.
const DESC_NUM: u64 = 8;

// virtio mmio control registers, mapped starting at 0x10001000.
// from qemu virtio_mmio.h
const VIRTIO_MMIO_MAGIC_VALUE: usize = 0x000; // 0x74726976
const VIRTIO_MMIO_VERSION: usize = 0x004; // version; 1 is legacy
const VIRTIO_MMIO_DEVICE_ID: usize = 0x008; // device type; 1 is net, 2 is disk
const VIRTIO_MMIO_VENDOR_ID: usize = 0x00c; // 0x554d4551
const VIRTIO_MMIO_DEVICE_FEATURES: usize = 0x010;
const VIRTIO_MMIO_DRIVER_FEATURES: usize = 0x020;
const VIRTIO_MMIO_GUEST_PAGE_SIZE: usize = 0x028; // page size for PFN, write-only
const VIRTIO_MMIO_QUEUE_SEL: usize = 0x030; // select queue, write-only
const VIRTIO_MMIO_QUEUE_NUM_MAX: usize = 0x034; // max size of current queue, read-only
const VIRTIO_MMIO_QUEUE_NUM: usize = 0x038; // size of current queue, write-only
const VIRTIO_MMIO_QUEUE_PFN: usize = 0x040; // physical page number for queue, read/write
#[allow(dead_code)]
const VIRTIO_MMIO_QUEUE_READY: usize = 0x044; // ready bit
#[allow(dead_code)]
const VIRTIO_MMIO_QUEUE_NOTIFY: usize = 0x050; // write-only
#[allow(dead_code)]
const VIRTIO_MMIO_INTERRUPT_STATUS: usize = 0x060; // read-only
#[allow(dead_code)]
const VIRTIO_MMIO_INTERRUPT_ACK: usize = 0x064; // write-only
const VIRTIO_MMIO_STATUS: usize = 0x070; // read/write
const VIRTIO_MMIO_QUEUE_DESC_LOW: usize = 0x080; // physical address for descriptor table, write-only
const VIRTIO_MMIO_QUEUE_DESC_HIGH: usize = 0x084;
const VIRTIO_MMIO_DRIVER_DESC_LOW: usize = 0x090; // physical address for available ring, write-only
const VIRTIO_MMIO_DRIVER_DESC_HIGH: usize = 0x094;
const VIRTIO_MMIO_DEVICE_DESC_LOW: usize = 0x0a0; // physical address for used ring, write-only
const VIRTIO_MMIO_DEVICE_DESC_HIGH: usize = 0x0a4;

#[derive(Clone, Serialize, Deserialize)]
pub struct VirtioSnapshot {
    pub start_addr: u64,
    pub id: u8,
    pub driver_features: u64,
    pub page_size: u64,
    pub queue_sel: u64,
    pub queue_num: u64,
    pub queue_pfn: u64,
    pub queue_notify: u64,
    pub desc_addr: u64,
    pub avail_addr: u64,
    pub used_addr: u64,
    pub status: u64,
    pub disk: Vec<u8>,
}

pub struct Virtio {
    start_addr: u64,
    notificator: Box<dyn Fn() + Send + Sync>,
    id: u8,
    driver_features: u64,
    page_size: u64,
    queue_sel: u64,
    queue_num: u64,
    queue_pfn: u64,
    desc_addr: u64,
    avail_addr: u64,
    used_addr: u64,
    queue_notify: u64,
    status: u64,
    disk: Vec<u8>,
}

impl Virtio {
    pub fn new(start_addr: u64, notificator: Box<dyn Fn() + Send + Sync>) -> Virtio {
        Self {
            start_addr,
            notificator,
            id: 0,
            driver_features: 0,
            page_size: 0,
            queue_sel: 0,
            queue_num: 0,
            queue_pfn: 0,
            desc_addr: 0,
            avail_addr: 0,
            used_addr: 0,
            queue_notify: 9999,
            status: 0,
            disk: Vec::new(),
        }
    }

    pub fn is_accessible(&self, addr: u64) -> bool {
        (addr >= self.start_addr) && (addr < self.start_addr + VIRTIO_SIZE)
    }

    pub fn load(&self, addr: u64, size: u64) -> Result<u64, Exception> {
        let relative_addr = (addr - self.start_addr) as usize;
        let ret_val = match relative_addr {
            VIRTIO_MMIO_MAGIC_VALUE => 0x74726976,
            VIRTIO_MMIO_VERSION => 0x2,
            VIRTIO_MMIO_DEVICE_ID => 0x2,
            VIRTIO_MMIO_VENDOR_ID => 0x554d4551,
            VIRTIO_MMIO_DEVICE_FEATURES => 0,
            VIRTIO_MMIO_DRIVER_FEATURES => self.driver_features,
            VIRTIO_MMIO_QUEUE_NUM_MAX => 8,
            VIRTIO_MMIO_QUEUE_PFN => self.queue_pfn,
            VIRTIO_MMIO_STATUS => self.status,
            VIRTIO_MMIO_QUEUE_SEL => self.queue_sel,
            VIRTIO_MMIO_QUEUE_NUM => self.queue_num,
            VIRTIO_MMIO_GUEST_PAGE_SIZE => self.page_size,
            VIRTIO_MMIO_QUEUE_NOTIFY => self.queue_notify,
            VIRTIO_MMIO_QUEUE_DESC_LOW => self.desc_addr,
            VIRTIO_MMIO_QUEUE_DESC_HIGH => self.desc_addr >> 32,
            VIRTIO_MMIO_DRIVER_DESC_LOW => self.avail_addr,
            VIRTIO_MMIO_DRIVER_DESC_HIGH => self.avail_addr >> 32,
            VIRTIO_MMIO_DEVICE_DESC_LOW => self.used_addr,
            VIRTIO_MMIO_DEVICE_DESC_HIGH => self.used_addr >> 32,
            _ => 0,
        };
        info!(
            "virtio: load addr:{:x}(relative {:x}), size:{}, value:{}",
            addr, relative_addr, size, ret_val
        );
        Ok(ret_val)
    }

    pub fn store(&mut self, addr: u64, size: u64, value: u64) -> Result<(), Exception> {
        info!(
            "virtio: store addr:{:x}, size:{}, value:{}",
            addr, size, value
        );
        let relative_addr = (addr - self.start_addr) as usize;
        match relative_addr {
            VIRTIO_MMIO_DEVICE_FEATURES => self.driver_features = value,
            VIRTIO_MMIO_GUEST_PAGE_SIZE => self.page_size = value,
            VIRTIO_MMIO_QUEUE_SEL => self.queue_sel = value,
            VIRTIO_MMIO_QUEUE_NUM => self.queue_num = value,
            VIRTIO_MMIO_QUEUE_PFN => self.queue_pfn = value,
            VIRTIO_MMIO_QUEUE_NOTIFY => {
                self.queue_notify = value;
                info!("virtio: queue notify called with value: {}", value);
                if value != 9999 {
                    (self.notificator)();
                }
            }
            VIRTIO_MMIO_STATUS => self.status = value,
            VIRTIO_MMIO_QUEUE_DESC_LOW => {
                self.desc_addr = value & 0xFFFFFFFF;
            }
            VIRTIO_MMIO_QUEUE_DESC_HIGH => {
                self.desc_addr |= (value & 0xFFFFFFFF) << 32;
            }
            VIRTIO_MMIO_DRIVER_DESC_LOW => {
                self.avail_addr = value & 0xFFFFFFFF;
            }
            VIRTIO_MMIO_DRIVER_DESC_HIGH => {
                self.avail_addr |= (value & 0xFFFFFFFF) << 32;
            }
            VIRTIO_MMIO_DEVICE_DESC_LOW => {
                self.used_addr = value & 0xFFFFFFFF;
            }
            VIRTIO_MMIO_DEVICE_DESC_HIGH => {
                self.used_addr |= (value & 0xFFFFFFFF) << 32;
            }
            _ => {}
        }
        Ok(())
    }

    /// Set the binary in the virtio disk.
    pub fn set_disk_image(&mut self, binary: Vec<u8>) {
        self.disk.extend(binary.iter().cloned());
    }

    fn read_disk(&self, addr: u64) -> u8 {
        self.disk[addr as usize]
    }

    /// Access the disk via virtio. This function performs DMA against *guest physical memory*.
    /// Takes a mutable reference to Dram to read/write guest memory directly.
    pub fn disk_access(&mut self, dram: &mut Dram) {
        if self.queue_notify == 9999 {
            return;
        }
        self.queue_notify = 9999;

        let desc_addr = self.desc_addr;
        let avail_addr = self.avail_addr;
        let used_addr = self.used_addr;

        let avail_idx = match dram.load(avail_addr + 2, 16) {
            Ok(v) => v as u16,
            Err(_) => return,
        };
        if avail_idx == 0 {
            return;
        }

        let ring_pos = ((avail_idx - 1) as u64) % (DESC_NUM as u64);
        let head = dram
            .load(avail_addr + 4 + ring_pos * 2, 16)
            .expect("failed to read avail.ring entry") as u16;

        let desc0 = desc_addr + VRING_DESC_SIZE * (head as u64);
        let addr0 = dram.load(desc0 + 0, 64).expect("failed to read desc0.addr");
        let _len0 = dram.load(desc0 + 8, 32).expect("failed to read desc0.len") as u32;
        let _flags0 = dram
            .load(desc0 + 12, 16)
            .expect("failed to read desc0.flags") as u16;
        let next0 = dram
            .load(desc0 + 14, 16)
            .expect("failed to read desc0.next") as u16;

        let desc1 = desc_addr + VRING_DESC_SIZE * (next0 as u64);
        let addr1 = dram.load(desc1 + 0, 64).expect("failed to read desc1.addr");
        let len1 = dram.load(desc1 + 8, 32).expect("failed to read desc1.len") as u32;
        let flags1 = dram
            .load(desc1 + 12, 16)
            .expect("failed to read desc1.flags") as u16;
        let next1 = dram
            .load(desc1 + 14, 16)
            .expect("failed to read desc1.next") as u16;

        let desc2 = desc_addr + VRING_DESC_SIZE * (next1 as u64);
        let addr2 = dram.load(desc2 + 0, 64).expect("failed to read desc2.addr");
        let _len2 = dram.load(desc2 + 8, 32).expect("failed to read desc2.len") as u32;
        let _flags2 = dram
            .load(desc2 + 12, 16)
            .expect("failed to read desc2.flags") as u16;

        let blk_sector = dram.load(addr0 + 8, 64).expect(&format!(
            "failed to read virtio_blk_outhdr.sector: addr0=0x{:x} (sector@0x{:x})",
            addr0,
            addr0 + 8
        ));

        info!(
            "virtio: head={} desc=0x{:x} avail=0x{:x} used=0x{:x}",
            head, desc_addr, avail_addr, used_addr
        );
        info!(
            "virtio: addr0=0x{:x} addr1=0x{:x} len1=0x{:x} flags1=0x{:x} addr2=0x{:x} sector={}",
            addr0, addr1, len1, flags1, addr2, blk_sector
        );

        let device_writes = (flags1 & 2) != 0;

        if !device_writes {
            let mut buffer = Vec::with_capacity(len1 as usize);
            for i in 0..(len1 as u64) {
                let b = dram
                    .load(addr1 + i, 8)
                    .expect(&format!("failed DMA read: guest addr=0x{:x}", addr1 + i))
                    as u8;
                buffer.push(b);
            }
            for (i, b) in buffer.into_iter().enumerate() {
                let disk_index = blk_sector * 512 + (i as u64);
                self.disk[disk_index as usize] = b;
            }
        } else {
            info!("Reading from disk sector: {}", blk_sector);
            for i in 0..(len1 as u64) {
                let b = self.read_disk(blk_sector * 512 + i) as u64;
                dram.store(addr1 + i, 8, b)
                    .expect("failed DMA write to guest memory");
            }
        }

        dram.store(addr2, 8, 0)
            .expect("failed to write status byte");

        let used_idx = dram.load(used_addr + 2, 16).unwrap_or(0) as u16;

        let used_pos = (used_idx as u64) % (DESC_NUM as u64);
        let used_elem = used_addr + 4 + used_pos * 8;

        dram.store(used_elem + 0, 32, head as u64)
            .expect("failed to write used.elems[].id");
        dram.store(used_elem + 4, 32, len1 as u64)
            .expect("failed to write used.elems[].len");

        dram.store(used_addr + 2, 16, (used_idx.wrapping_add(1)) as u64)
            .expect("failed to write used.idx");
    }

    /// Returns a clone of the disk image (used in tests to verify preservation).
    pub fn disk_snapshot(&self) -> Vec<u8> {
        self.disk.clone()
    }

    pub fn to_snapshot(&self) -> VirtioSnapshot {
        VirtioSnapshot {
            start_addr: self.start_addr,
            id: self.id,
            driver_features: self.driver_features,
            page_size: self.page_size,
            queue_sel: self.queue_sel,
            queue_num: self.queue_num,
            queue_pfn: self.queue_pfn,
            queue_notify: self.queue_notify,
            desc_addr: self.desc_addr,
            avail_addr: self.avail_addr,
            used_addr: self.used_addr,
            status: self.status,
            disk: self.disk.clone(),
        }
    }

    pub fn from_snapshot(
        snapshot: VirtioSnapshot,
        notificator: Box<dyn Fn() + Send + Sync>,
    ) -> Self {
        Self {
            start_addr: snapshot.start_addr,
            notificator,
            id: snapshot.id,
            driver_features: snapshot.driver_features,
            page_size: snapshot.page_size,
            queue_sel: snapshot.queue_sel,
            queue_num: snapshot.queue_num,
            queue_pfn: snapshot.queue_pfn,
            queue_notify: snapshot.queue_notify,
            desc_addr: snapshot.desc_addr,
            avail_addr: snapshot.avail_addr,
            used_addr: snapshot.used_addr,
            status: snapshot.status,
            disk: snapshot.disk,
        }
    }
}
