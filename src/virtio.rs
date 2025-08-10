use crate::{bus::Bus, interrupt::*, dram::Dram};
use log::info;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

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
    start_addr: u64,
    id: u8,
    driver_features: u64,
    page_size: u64,
    queue_sel: u64,
    queue_num: u64,
    queue_pfn: u64,
    queue_notify: u64,
    desc_addr: u64,
    avail_addr: u64,
    used_addr: u64,
    status: u64,
    disk: Vec<u8>,
}

pub struct Virtio {
    start_addr: u64,
    notificator: Box<dyn Fn() + Send + Sync>,
    bus: Option<Rc<RefCell<Bus>>>,
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
    pub fn new(_start_addr: u64, notificator: Box<dyn Fn() + Send + Sync>) -> Virtio {
        Self {
            start_addr: _start_addr,
            notificator,
            bus: None,
            id: 0,
            driver_features: 0,
            page_size: 0,
            queue_sel: 0,
            queue_num: 0,
            queue_pfn: 0,
            desc_addr: 0,
            avail_addr: 0,
            used_addr: 0,
            queue_notify: 9999, // TODO: what is the correct initial value?
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
            VIRTIO_MMIO_DEVICE_FEATURES => 0, // TODO: what should it return?
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
                // Notify the virtio device that a queue is ready.
                info!("virtio: queue notify called with value: {}", value);
                if value != 9999 {
                    (self.notificator)();
                }
            }
            VIRTIO_MMIO_STATUS => self.status = value,
            VIRTIO_MMIO_QUEUE_DESC_LOW => {
                self.desc_addr = value;
            }
            VIRTIO_MMIO_QUEUE_DESC_HIGH => {
                self.desc_addr |= value << 32;
            }
            VIRTIO_MMIO_DRIVER_DESC_LOW => {
                self.avail_addr = value;
            }
            VIRTIO_MMIO_DRIVER_DESC_HIGH => {
                self.avail_addr |= value << 32;
            }
            VIRTIO_MMIO_DEVICE_DESC_LOW => {
                self.used_addr = value;
            }
            VIRTIO_MMIO_DEVICE_DESC_HIGH => {
                self.used_addr |= value << 32;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn set_bus(&mut self, bus: Rc<RefCell<Bus>>) {
        self.bus = Some(bus);
    }

    /// Set the binary in the virtio disk.
    pub fn set_disk_image(&mut self, binary: Vec<u8>) {
        self.disk.extend(binary.iter().cloned());
    }

    fn read_disk(&self, addr: u64) -> u8 {
        self.disk[addr as usize]
    }

    fn write_disk(&mut self, addr: u64, value: u8) {
        self.disk[addr as usize] = value
    }

    /// Access the disk via virtio. This is an associated function which takes a `cpu` object to
    /// read and write with a memory directly (DMA).
    pub fn disk_access(&mut self) {
        if self.queue_notify == 9999 {
            return;
        }

        let mut bus = self.bus.as_ref().expect("No bus").borrow_mut();

        // See more information in
        // https://github.com/mit-pdos/xv6-riscv/blob/riscv/kernel/virtio_disk.c

        // the spec says that legacy block operations use three
        // descriptors: one for type/reserved/sector, one for
        // the data, one for a 1-byte status result.

        // desc = pages -- num * VRingDesc
        // avail = pages + 0x40 -- 2 * uint16, then num * uint16
        // used = pages + 4096 -- 2 * uint16, then num * vRingUsedElem
        let desc_addr = self.desc_addr;
        let avail_addr = self.avail_addr;
        let used_addr = self.used_addr;

        // avail[0] is flags
        // avail[1] tells the device how far to look in avail[2...].
        info!("virtio: disk access, desc_addr: {:x}, avail_addr: {:x}, used_addr: {:x}",
            desc_addr, avail_addr, used_addr);
        let offset = bus.load(avail_addr.wrapping_add(2), 16)
            .unwrap_or(0) as u64;
        // avail[2...] are desc[] indices the device should process.
        // we only tell device the first index in our chain of descriptors.
        let index = bus.load(avail_addr.wrapping_add(offset % DESC_NUM).wrapping_add(2), 16)
            .expect("failed to read index");

        // Read `VRingDesc`, virtio descriptors.
        let desc_addr0 = desc_addr + VRING_DESC_SIZE * index;
        let addr0 = bus.load(desc_addr0, 64)
            .expect("failed to read an address field in a descriptor");
        // Add 14 because of `VRingDesc` structure.
        // struct VRingDesc {
        //   uint64 addr;
        //   uint32 len;
        //   uint16 flags;
        //   uint16 next
        // };
        // The `next` field can be accessed by offset 14 (8 + 4 + 2) bytes.
        let next0 = bus.load(desc_addr0.wrapping_add(14), 16)
            .expect("failed to read a next field in a descripor");

        // Read `VRingDesc` again, virtio descriptors.
        let desc_addr1 = desc_addr + VRING_DESC_SIZE * next0;
        let addr1 = bus.load(desc_addr1, 64)
            .expect("failed to read an address field in a descriptor");
        let len1 = bus.load(desc_addr1.wrapping_add(8), 32)
            .expect("failed to read a length field in a descriptor");
        let flags1 = bus.load(desc_addr1.wrapping_add(12), 16)
            .expect("failed to read a flags field in a descriptor");

        // Read `virtio_blk_outhdr`. Add 8 because of its structure.
        // struct virtio_blk_outhdr {
        //   uint32 type;
        //   uint32 reserved;
        //   uint64 sector;
        // } buf0;
        let blk_sector = bus.load(addr0.wrapping_add(8), 64)
            .expect("failed to read a sector field in a virtio_blk_outhdr");

        // Write to a device if the second bit `flag1` is set.
        match (flags1 & 2) == 0 {
            true => {
                // Read memory data and write it to a disk directly (DMA).
                let mut buffer = Vec::with_capacity(len1 as usize);
                for i in 0..len1 as u64 {
                    let data = bus.load(addr1 + i, 8)
                        .expect("failed to read from memory") as u8;
                    buffer.push(data);
                }
                for (i, data) in buffer.into_iter().enumerate() {
                    self.write_disk(blk_sector * 512 + i as u64, data);
                }
            }
            false => {
                // Read disk data and write it to memory directly (DMA).
                for i in 0..len1 as u64 {
                    let data = self.read_disk(blk_sector * 512 + i) as u64;
                    bus.store(addr1 + i, 8, data)
                        .expect("failed to write to memory");
                }
            }
        };

        // Write id to `UsedArea`. Add 2 because of its structure.
        // struct UsedArea {
        //   uint16 flags;
        //   uint16 id;
        //   struct VRingUsedElem elems[NUM];
        // };
        self.id = self.id.wrapping_add(1);
        let new_id = self.id as u64;
        bus.store(used_addr.wrapping_add(2), 16, new_id % 8)
            .expect("failed to write to memory");
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

    pub fn from_snapshot(snapshot: VirtioSnapshot, notificator: Box<dyn Fn() + Send + Sync>) -> Self {
        Self {
            start_addr: snapshot.start_addr,
            notificator,
            bus: None,
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
