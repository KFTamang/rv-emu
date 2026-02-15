use crate::{bus::Bus, interrupt::*};
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
                self.desc_addr = value & 0xFFFFFFFF; // lower 32 bits
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

/// Access the disk via virtio. This function performs DMA against *guest physical memory*.
/// It must not call Bus::load/store that may re-enter virtio/MMIO routing; use *_memory variants.
///
/// Assumptions:
/// - desc_addr/avail_addr/used_addr are guest-physical addresses of the vring structures.
/// - VRING_DESC_SIZE is 16 bytes.
/// - DESC_NUM is the ring size (NUM).
pub fn disk_access(&mut self) {
    if self.queue_notify == 9999 {
        return;
    }
    self.queue_notify = 9999; // reset notify

    let mut bus = self.bus.as_ref().expect("No bus").borrow_mut();

    // Layout (legacy virtio ring):
    // desc  = pages
    // avail = pages + 0x40
    // used  = pages + 0x1000 (4096)
    let desc_addr = self.desc_addr;
    let avail_addr = self.avail_addr;
    let used_addr = self.used_addr;

    // ---- Read avail.idx and select the latest entry ----
    // struct VRingAvail { u16 flags; u16 idx; u16 ring[NUM]; ... }
    let avail_idx = match bus.load_memory(avail_addr + 2, 16) {
        Ok(v) => v as u16,
        Err(_) => return, // can't read -> nothing we can do safely
    };
    if avail_idx == 0 {
        return; // nothing submitted yet
    }

    // Process the most recently published ring entry.
    // (xv6 submits one request at a time; this is sufficient for now)
    let ring_pos = ((avail_idx - 1) as u64) % (DESC_NUM as u64);
    let head = bus
        .load_memory(avail_addr + 4 + ring_pos * 2, 16)
        .expect("failed to read avail.ring entry") as u16;

    // ---- Descriptor 0 ----
    // struct VRingDesc { u64 addr; u32 len; u16 flags; u16 next; }
    let desc0 = desc_addr + VRING_DESC_SIZE * (head as u64);
    let addr0 = bus
        .load_memory(desc0 + 0, 64)
        .expect("failed to read desc0.addr");
    let _len0 = bus
        .load_memory(desc0 + 8, 32)
        .expect("failed to read desc0.len") as u32;
    let _flags0 = bus
        .load_memory(desc0 + 12, 16)
        .expect("failed to read desc0.flags") as u16;
    let next0 = bus
        .load_memory(desc0 + 14, 16)
        .expect("failed to read desc0.next") as u16;

    // ---- Descriptor 1 ----
    let desc1 = desc_addr + VRING_DESC_SIZE * (next0 as u64);
    let addr1 = bus
        .load_memory(desc1 + 0, 64)
        .expect("failed to read desc1.addr");
    let len1 = bus
        .load_memory(desc1 + 8, 32)
        .expect("failed to read desc1.len") as u32;
    let flags1 = bus
        .load_memory(desc1 + 12, 16)
        .expect("failed to read desc1.flags") as u16;
    let next1 = bus
        .load_memory(desc1 + 14, 16)
        .expect("failed to read desc1.next") as u16;

    // ---- Descriptor 2 (status) ----
    let desc2 = desc_addr + VRING_DESC_SIZE * (next1 as u64);
    let addr2 = bus
        .load_memory(desc2 + 0, 64)
        .expect("failed to read desc2.addr");
    let _len2 = bus
        .load_memory(desc2 + 8, 32)
        .expect("failed to read desc2.len") as u32;
    let _flags2 = bus
        .load_memory(desc2 + 12, 16)
        .expect("failed to read desc2.flags") as u16;

    // ---- Read virtio_blk_outhdr.sector ----
    // struct virtio_blk_outhdr { u32 type; u32 reserved; u64 sector; }
    let blk_sector = bus.load_memory(addr0 + 8, 64).expect(&format!(
        "failed to read virtio_blk_outhdr.sector: addr0=0x{:x} (sector@0x{:x})",
        addr0,
        addr0 + 8
    ));

    info!("virtio: head={} desc=0x{:x} avail=0x{:x} used=0x{:x}", head, desc_addr, avail_addr, used_addr);
    info!("virtio: addr0=0x{:x} addr1=0x{:x} len1=0x{:x} flags1=0x{:x} addr2=0x{:x} sector={}",
      addr0, addr1, len1, flags1, addr2, blk_sector);

    // flags1 bit1 == VIRTQ_DESC_F_WRITE (device writes to buffer)
    let device_writes = (flags1 & 2) != 0;

    if !device_writes {
        // Guest -> Disk (write): device reads from guest buffer (addr1..addr1+len1)
        let mut buffer = Vec::with_capacity(len1 as usize);
        for i in 0..(len1 as u64) {
            let b = bus
                .load_memory(addr1 + i, 8)
                .expect(&format!(
                    "failed DMA read: guest addr=0x{:x}",
                    addr1 + i
                )) as u8;
            buffer.push(b);
        }
        for (i, b) in buffer.into_iter().enumerate() {
            let disk_index = blk_sector * 512 + (i as u64);
            self.disk[disk_index as usize] = b;
        }
    } else {
        // Disk -> Guest (read): device writes to guest buffer (addr1..addr1+len1)
        info!("Reading from disk sector: {}", blk_sector);
        for i in 0..(len1 as u64) {
            let b = self.read_disk(blk_sector * 512 + i) as u64;
            bus.store_memory(addr1 + i, 8, b)
                .expect("failed DMA write to guest memory");
        }
    }

    // ---- Write status byte (1 byte) ----
    // xv6 expects status=0 on success.
    bus.store_memory(addr2, 8, 0)
        .expect("failed to write status byte");

    // ---- Update used ring ----
    // struct VRingUsed { u16 flags; u16 idx; struct { u32 id; u32 len; } elems[NUM]; }
    let used_idx = bus
        .load_memory(used_addr + 2, 16)
        .unwrap_or(0) as u16;

    let used_pos = (used_idx as u64) % (DESC_NUM as u64);
    let used_elem = used_addr + 4 + used_pos * 8;

    // id = head descriptor index, len = number of bytes written (for block device reads),
    // for writes len is typically 0 or len1 depending on implementation; xv6 doesn't rely heavily on len.
    bus.store_memory(used_elem + 0, 32, head as u64)
        .expect("failed to write used.elems[].id");
    bus.store_memory(used_elem + 4, 32, len1 as u64)
        .expect("failed to write used.elems[].len");

    // bump used.idx
    bus.store_memory(used_addr + 2, 16, (used_idx.wrapping_add(1)) as u64)
        .expect("failed to write used.idx");
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
