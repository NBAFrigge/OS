#![allow(dead_code)]

use core::{ptr::from_raw_parts, sync::atomic::Ordering};

use alloc::vec::Vec;
use lazy_static::lazy_static;
use x86_64::{
    instructions::port::Port,
    structures::{idt::InterruptStackFrame, paging::FrameAllocator},
};

use crate::{
    apic::{apic::send_eoi, io_apic::add_redirect},
    drivers::pci::{self, PciAddress},
    idt::interrupt::add_handler,
    kinfo,
    memory::{
        frame_allocator::FRAME_ALLOCATOR, memory::PHYSICAL_MEMORY_OFFSET,
    },
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

pub const QUEUE_SIZE: usize = 256;
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
        let used_size = 6 + 8 * qsz;
        let total = used_off + used_size;
        let pages = (total + 4095) / 4096;

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
        todo!()
    }

    fn free_desc(&mut self, idx: u16) {
        todo!()
    }

    fn push_avail(&mut self, desc_idx: u16) {
        todo!()
    }

    fn pop_used(&mut self) -> Option<(u16, u32)> {
        todo!()
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
    unsafe {
        send_eoi();
    }
}

impl VirtioNet {
    pub fn init() -> Result<(), &'static str> {
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
        unsafe {
            add_redirect(irq, irq + 32);
        }
        let bar0 = nic.get_bar(0).ok_or("virtio BAR0 not found")?;
        let iobase = bar0.address as u16;
        add_handler((32 + irq) as usize, virtio_handler);

        let mut mac = [0u8; 6];
        unsafe {
            Port::<u8>::new(iobase + REG_DEVICE_STATUS).write(0);
            Port::<u8>::new(iobase + REG_DEVICE_STATUS)
                .write(STATUS_ACKNOWLEDGE);
            Port::<u8>::new(iobase + REG_DEVICE_STATUS)
                .write(STATUS_ACKNOWLEDGE | STATUS_DRIVER);

            let features =
                Port::<u32>::new(iobase + REG_DEVICE_FEATURES).read();
            Port::<u32>::new(iobase + REG_DRIVER_FEATURES)
                .write(features & VIRTIO_NET_F_MAC);

            for i in 0..6 {
                mac[i] =
                    Port::<u8>::new(iobase + REG_CONFIG_MAC + i as u16).read();
            }
        }

        kinfo!("virtio-net iobase {:#x} mac {:02x?}", iobase, mac);
        Ok(())
    }

    fn negotiate_features(iobase: u16) -> u32 {
        todo!()
    }

    fn setup_queue(iobase: u16, index: u16, offset: u64) -> Virtqueue {
        todo!()
    }

    fn fill_rx(&mut self) {
        todo!()
    }

    fn notify(&self, queue: u16) {
        todo!()
    }

    pub fn send(&mut self, frame: &[u8]) -> bool {
        todo!()
    }

    pub fn receive(&mut self) -> Option<&[u8]> {
        todo!()
    }

    fn reclaim_tx(&mut self) {
        todo!()
    }
}

lazy_static! {
    pub static ref VIRTIO_NET_DRIVER: IrqMutex<Option<VirtioNet>> =
        IrqMutex::new(None);
}
