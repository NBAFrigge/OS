#![allow(dead_code)]

use core::sync::atomic::Ordering;

use alloc::vec::Vec;
use lazy_static::lazy_static;
use x86_64::{
    instructions::port::Port,
    structures::{idt::InterruptStackFrame, paging::FrameAllocator},
};

use crate::{
    apic::{apic::send_eoi, io_apic::add_redirect},
    drivers::pci::PciAddress,
    idt::interrupt::add_handler,
    kinfo,
    memory::{
        frame_allocator::FRAME_ALLOCATOR, memory::PHYSICAL_MEMORY_OFFSET,
    },
    net::dispatcher::poll_network,
    sync::IrqMutex,
};

pub const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
pub const VIRTIO_NET_DEVICE_ID: u16 = 0x1000;

pub const REG_DEVICE_FEATURES: u16 = 0x00;
pub const REG_DRIVER_FEATURES: u16 = 0x04;
pub const REG_QUEUE_ADDRESS: u16 = 0x08;
pub const REG_QUEUE_SIZE: u16 = 0x0C;
pub const REG_QUEUE_SELECT: u16 = 0x0E;
pub const REG_QUEUE_NOTIFY: u16 = 0x10;
pub const REG_DEVICE_STATUS: u16 = 0x12;
pub const REG_ISR_STATUS: u16 = 0x13;
pub const REG_CONFIG_MAC: u16 = 0x14;

pub const STATUS_ACKNOWLEDGE: u8 = 1;
pub const STATUS_DRIVER: u8 = 2;
pub const STATUS_DRIVER_OK: u8 = 4;
pub const STATUS_FEATURES_OK: u8 = 8;
pub const STATUS_FAILED: u8 = 0x80;

pub const VIRTIO_NET_F_CSUM: u32 = 1 << 0;
pub const VIRTIO_NET_F_MAC: u32 = 1 << 5;
pub const VIRTIO_NET_F_MRG_RXBUF: u32 = 1 << 15;
pub const VIRTIO_NET_F_STATUS: u32 = 1 << 16;

pub const VIRTQ_DESC_F_NEXT: u16 = 1;
pub const VIRTQ_DESC_F_WRITE: u16 = 2;
pub const VIRTQ_DESC_F_INDIRECT: u16 = 4;

pub const VIRTQ_AVAIL_F_NO_INTERRUPT: u16 = 1;
pub const VIRTQ_USED_F_NO_NOTIFY: u16 = 1;

pub const RX_QUEUE: u16 = 0;
pub const TX_QUEUE: u16 = 1;

pub const QUEUE_SIZE: usize = 1024;
pub const RX_BUFFER_SIZE: usize = 2048;
pub const VIRTIO_NET_HDR_LEN: usize = 10;
pub const VIRTIO_NET_HDR_GSO_NONE: u8 = 0;

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct VirtqDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

#[repr(C)]
pub struct VirtqAvail {
    pub flags: u16,
    pub idx: u16,
    pub ring: [u16; QUEUE_SIZE],
    pub used_event: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VirtqUsedElem {
    pub id: u32,
    pub len: u32,
}

#[repr(C)]
pub struct VirtqUsed {
    pub flags: u16,
    pub idx: u16,
    pub ring: [VirtqUsedElem; QUEUE_SIZE],
    pub avail_event: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VirtioNetHeader {
    pub flags: u8,
    pub gso_type: u8,
    pub hdr_len: u16,
    pub gso_size: u16,
    pub csum_start: u16,
    pub csum_offset: u16,
}

pub struct Virtqueue {
    pub desc: &'static mut [VirtqDesc],
    pub avail: &'static mut VirtqAvail,
    pub used: &'static mut VirtqUsed,
    pub size: u16,
    pub free_head: u16,
    pub num_free: u16,
    pub last_used_idx: u16,
    pub buffers: Vec<&'static mut [u8]>,
    pub region_phys: u64,
}

impl Virtqueue {
    fn new(qsz: u16, offset: u64) -> Virtqueue {
        let qsz = qsz as usize;
        let desc_size = 16 * qsz;
        let avail_size = 6 + 2 * qsz;
        let used_off = (desc_size + avail_size + 4095) & !4095;
        // Allocate for the MAX ring (QUEUE_SIZE) so the fixed-size
        // [_; QUEUE_SIZE] avail/used structs always fit, even when the device
        // negotiates a smaller qsz. The desc/avail/used *offsets* above stay
        // qsz-based so they match the device's legacy layout.
        let max_used_off =
            (16 * QUEUE_SIZE + 6 + 2 * QUEUE_SIZE + 4095) & !4095;
        let max_total = max_used_off + 6 + 8 * QUEUE_SIZE;
        let pages = (max_total + 4095) / 4096;

        let mut guard = FRAME_ALLOCATOR.lock();
        let allocator = guard.as_mut().expect("frame allocator not init");

        let first = allocator
            .allocate_frame()
            .expect("out of memory for virtqueue");
        let region_phys = first.start_address().as_u64();

        for i in 1..pages {
            let frame = allocator
                .allocate_frame()
                .expect("out of memory for virtqueue");
            let phys = frame.start_address().as_u64();
            assert_eq!(
                phys,
                region_phys + (i as u64) * 4096,
                "virtqueue frames not physically contiguous"
            );
        }

        let region_virt = region_phys + offset;
        unsafe {
            core::ptr::write_bytes(region_virt as *mut u8, 0, pages * 4096)
        }

        let desc = unsafe {
            core::slice::from_raw_parts_mut(region_virt as *mut VirtqDesc, qsz)
        };
        let avail = unsafe {
            &mut *((region_virt + desc_size as u64) as *mut VirtqAvail)
        };
        let used = unsafe {
            &mut *((region_virt + used_off as u64) as *mut VirtqUsed)
        };

        let mut buffers: Vec<&'static mut [u8]> = Vec::with_capacity(qsz);

        for i in 0..qsz {
            let frame = allocator
                .allocate_frame()
                .expect("oom for virtqueue buffer");
            let buf_phys = frame.start_address().as_u64();
            let buf_virt = buf_phys + offset;

            desc[i].addr = buf_phys;
            desc[i].len = 2048;
            desc[i].next = if i + 1 < qsz { (i + 1) as u16 } else { 0 };
            buffers.push(unsafe {
                core::slice::from_raw_parts_mut(buf_virt as *mut u8, 2048)
            });
        }

        drop(guard);

        Virtqueue {
            desc,
            avail,
            used,
            size: qsz as u16,
            free_head: 0,
            num_free: qsz as u16,
            last_used_idx: 0,
            buffers,
            region_phys,
        }
    }

    fn alloc_desc(&mut self) -> Option<u16> {
        if self.num_free == 0 {
            return None;
        }
        let idx = self.free_head;
        self.free_head = self.desc[idx as usize].next;
        self.num_free -= 1;
        Some(idx)
    }

    fn free_desc(&mut self, idx: u16) {
        self.desc[idx as usize].next = self.free_head;
        self.free_head = idx;
        self.num_free += 1;
    }

    fn push_avail(&mut self, desc_idx: u16) {
        let slot = self.avail.idx as usize % self.size as usize;
        self.avail.ring[slot] = desc_idx;
        core::sync::atomic::compiler_fence(Ordering::SeqCst);
        self.avail.idx = self.avail.idx.wrapping_add(1);
    }

    fn pop_used(&mut self) -> Option<(u16, u32)> {
        let used_idx = unsafe { core::ptr::read_volatile(&self.used.idx) };
        if used_idx == self.last_used_idx {
            return None;
        }

        let element =
            self.used.ring[self.last_used_idx as usize % self.size as usize];
        self.last_used_idx = self.last_used_idx.wrapping_add(1);

        Some((element.id as u16, element.len))
    }
}

pub struct VirtioNet {
    pub iobase: u16,
    pub mac: [u8; 6],
    pub rx: Virtqueue,
    pub tx: Virtqueue,
}

extern "x86-interrupt" fn virtio_handler(_stack_frame: InterruptStackFrame) {
    if let Some(iobase) = VIRTIO_NET_DRIVER.lock().as_ref().map(|d| d.iobase) {
        unsafe {
            let _ = Port::<u8>::new(iobase + REG_ISR_STATUS).read();
        }
    }
    poll_network();
    unsafe {
        send_eoi();
    }
}

impl VirtioNet {
    pub fn init() -> Result<Self, &'static str> {
        let offset = PHYSICAL_MEMORY_OFFSET.load(Ordering::Relaxed);
        let devices = PciAddress::list_all();
        let mut nic = devices
            .into_iter()
            .find(|d| {
                d.get_vendor_id() == VIRTIO_VENDOR_ID
                    && d.get_device_id() == VIRTIO_NET_DEVICE_ID
            })
            .ok_or("VirtioNet not found")?;

        nic.enable_bus_mastering();
        let irq = nic.get_IRQ();
        kinfo!(
            "virtio-net io-apic input={} pin={} slot={}",
            irq,
            nic.get_interrupt_pin(),
            nic.device()
        );
        unsafe {
            add_redirect(irq, irq + 32);
        }
        add_handler((32 + irq) as usize, virtio_handler);
        let bar0 = nic.get_bar(0).ok_or("virtio BAR0 not found")?;
        let iobase = bar0.address as u16;

        // Handshake
        unsafe {
            Port::<u8>::new(iobase + REG_DEVICE_STATUS).write(0);
            Port::<u8>::new(iobase + REG_DEVICE_STATUS)
                .write(STATUS_ACKNOWLEDGE);
            Port::<u8>::new(iobase + REG_DEVICE_STATUS)
                .write(STATUS_ACKNOWLEDGE | STATUS_DRIVER);
        }

        Self::negotiate_features(iobase);

        let mut mac = [0u8; 6];
        unsafe {
            for i in 0..6 {
                mac[i] =
                    Port::<u8>::new(iobase + REG_CONFIG_MAC + i as u16).read();
            }
        }

        // Set up the virtqueues (this writes each queue's PFN to the device).
        let rx = Self::setup_queue(iobase, RX_QUEUE, offset);
        let tx = Self::setup_queue(iobase, TX_QUEUE, offset);

        unsafe {
            Port::<u8>::new(iobase + REG_DEVICE_STATUS)
                .write(STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_DRIVER_OK);
        }

        let mut dev = VirtioNet {
            iobase,
            mac,
            rx,
            tx,
        };
        dev.fill_rx();

        kinfo!("virtio-net iobase {:#x} mac {:02x?}", iobase, mac);
        Ok(dev)
    }

    fn negotiate_features(iobase: u16) -> u32 {
        unsafe {
            let features =
                Port::<u32>::new(iobase + REG_DEVICE_FEATURES).read();
            let accepted = features & VIRTIO_NET_F_MAC;
            Port::<u32>::new(iobase + REG_DRIVER_FEATURES).write(accepted);
            accepted
        }
    }

    fn setup_queue(iobase: u16, index: u16, offset: u64) -> Virtqueue {
        unsafe {
            Port::<u16>::new(iobase + REG_QUEUE_SELECT).write(index);
            let qsz = Port::<u16>::new(iobase + REG_QUEUE_SIZE).read();
            assert!(
                qsz.is_power_of_two() && qsz as usize <= QUEUE_SIZE,
                "virtqueue size {} unsupported (max {})",
                qsz,
                QUEUE_SIZE
            );

            let vq = Virtqueue::new(qsz, offset);
            Port::<u32>::new(iobase + REG_QUEUE_ADDRESS)
                .write((vq.region_phys >> 12) as u32);
            vq
        }
    }

    fn fill_rx(&mut self) {
        for i in 0..self.rx.size as usize {
            self.rx.desc[i].flags = VIRTQ_DESC_F_WRITE;
            self.rx.avail.ring[i] = i as u16;
        }
        core::sync::atomic::compiler_fence(Ordering::SeqCst);
        self.rx.avail.idx = self.rx.size;
        core::sync::atomic::compiler_fence(Ordering::SeqCst);
        self.notify(RX_QUEUE);
    }

    fn notify(&self, queue: u16) {
        unsafe {
            Port::<u16>::new(self.iobase + REG_QUEUE_NOTIFY).write(queue);
        }
    }

    pub fn send(&mut self, frame: &[u8]) -> bool {
        self.reclaim_tx();
        if frame.len() > 2048 - VIRTIO_NET_HDR_LEN {
            return false;
        }

        let idx = match self.tx.alloc_desc() {
            Some(i) => i,
            None => return false,
        };

        self.tx.buffers[idx as usize][..VIRTIO_NET_HDR_LEN].fill(0);
        self.tx.buffers[idx as usize]
            [VIRTIO_NET_HDR_LEN..VIRTIO_NET_HDR_LEN + frame.len()]
            .copy_from_slice(frame);

        self.tx.desc[idx as usize].len =
            (VIRTIO_NET_HDR_LEN + frame.len()) as u32;
        self.tx.desc[idx as usize].flags = 0;
        self.tx.push_avail(idx);
        self.notify(TX_QUEUE);

        true
    }

    pub fn receive(&mut self) -> Option<&[u8]> {
        let (id, len) = self.rx.pop_used()?;
        self.rx.push_avail(id);
        self.notify(RX_QUEUE);
        Some(&self.rx.buffers[id as usize][VIRTIO_NET_HDR_LEN..len as usize])
    }

    fn reclaim_tx(&mut self) {
        while let Some((id, _)) = self.tx.pop_used() {
            self.tx.free_desc(id);
        }
    }
}

lazy_static! {
    pub static ref VIRTIO_NET_DRIVER: IrqMutex<Option<VirtioNet>> =
        IrqMutex::new(None);
}
