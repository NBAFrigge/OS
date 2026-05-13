use crate::drivers::pci::PciAddress;
use crate::memory::frame_allocator::FRAME_ALLOCATOR;
use crate::memory::memory::PHYSICAL_MEMORY_OFFSET;
use alloc::vec;
use alloc::vec::Vec;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::Ordering;
use lazy_static::lazy_static;
use spin::mutex::Mutex;
use x86_64::structures::paging::FrameAllocator;

const REG_CTRL: u32 = 0x0000;
const REG_IMC: u32 = 0x00D8;
const REG_EERD: u32 = 0x0014;
const REG_RAL: u32 = 0x5400;
const REG_RAH: u32 = 0x5404;
const REG_RDBAL: u32 = 0x2800;
const REG_RDBAH: u32 = 0x2804;
const REG_RDLEN: u32 = 0x2808;
const REG_RDH: u32 = 0x2810;
const REG_RDT: u32 = 0x2818;
const REG_RCTL: u32 = 0x0100;
const REG_TDBAL: u32 = 0x3800;
const REG_TDBAH: u32 = 0x3804;
const REG_TDLEN: u32 = 0x3808;
const REG_TDH: u64 = 0x3810;
const REG_TDT: u64 = 0x3818;
const REG_TCTL: u32 = 0x0400;

const RX_DESC_COUNT: usize = 32;
const TX_DESC_COUNT: usize = 32;
const BUFFER_SIZE: usize = 2048;

const CTRL_RST: u32 = 1 << 26;
const RCTL_EN: u32 = 1 << 1;
const RCTL_SBP: u32 = 1 << 2;
const RCTL_UPE: u32 = 1 << 3;
const RCTL_MPE: u32 = 1 << 4;
const RCTL_BAM: u32 = 1 << 15;
const RCTL_BSIZE_2048: u32 = 0 << 16;
const TCTL_EN: u32 = 1 << 1;
const TCTL_PSP: u32 = 1 << 3;

const CMD_EOP_IFCS_RS: u8 = 0b00001011;

pub struct E1000 {
    pub base_addr: u64,
    pub mac: [u8; 6],
    pub rx_ring: &'static mut [RxDescriptor],
    pub rx_buffers: Vec<&'static mut [u8]>,
    pub rx_next: usize,
    pub tx_ring: &'static mut [TxDescriptor],
    pub tx_buffers: Vec<&'static mut [u8]>,
    pub tx_tail: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct RxDescriptor {
    pub addr: u64,
    pub length: u16,
    pub checksum: u16,
    pub status: u8,
    pub errors: u8,
    pub special: u16,
}

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct TxDescriptor {
    pub addr: u64,
    pub length: u16,
    pub cso: u8,
    pub cmd: u8,
    pub status: u8,
    pub css: u8,
    pub special: u16,
}

impl E1000 {
    pub fn init() -> Result<Self, &'static str> {
        let offset = PHYSICAL_MEMORY_OFFSET.load(Ordering::Relaxed);
        let devices = PciAddress::list_all();
        let mut nic = devices
            .into_iter()
            .find(|d| d.get_vendor_id() == 0x8086 && d.get_device_id() == 0x100E)
            .ok_or("E1000 not found")?;

        nic.enable_bus_mastering();

        let bar0 = nic.get_bar(0).ok_or("BAR0 not found")?;
        let base_addr = bar0.address + offset;

        unsafe {
            let ctrl = read_volatile((base_addr + REG_CTRL as u64) as *const u32);
            write_volatile((base_addr + REG_CTRL as u64) as *mut u32, ctrl | CTRL_RST);
            while read_volatile((base_addr + REG_CTRL as u64) as *const u32) & CTRL_RST != 0 {}

            let mut ctrl_after_reset = read_volatile((base_addr + REG_CTRL as u64) as *const u32);
            ctrl_after_reset |= (1 << 6) | (1 << 5);
            write_volatile((base_addr + REG_CTRL as u64) as *mut u32, ctrl_after_reset);

            for i in 0..128 {
                write_volatile((base_addr + 0x5200 + (i * 4)) as *mut u32, 0);
            }

            write_volatile((base_addr + REG_IMC as u64) as *mut u32, 0xFFFFFFFF);
        }

        let mac = unsafe { Self::read_mac(base_addr) };

        unsafe {
            let ral = (mac[0] as u32)
                | ((mac[1] as u32) << 8)
                | ((mac[2] as u32) << 16)
                | ((mac[3] as u32) << 24);
            let rah = (mac[4] as u32) | ((mac[5] as u32) << 8) | (1 << 31);
            write_volatile((base_addr + REG_RAL as u64) as *mut u32, ral);
            write_volatile((base_addr + REG_RAH as u64) as *mut u32, rah);
        }

        let mut allocator_guard = crate::memory::frame_allocator::FRAME_ALLOCATOR.lock();
        let allocator = allocator_guard.as_mut().expect("Frame allocator not init");

        let mut get_page = || -> (u64, u64) {
            let frame = allocator.allocate_frame().expect("Out of memory for NIC");
            let phys = frame.start_address().as_u64();
            let virt = phys + offset;
            (phys, virt)
        };

        let (rx_ring_phys, rx_ring_virt) = get_page();
        let rx_ring = unsafe {
            core::slice::from_raw_parts_mut(rx_ring_virt as *mut RxDescriptor, RX_DESC_COUNT)
        };
        let mut rx_buffers = Vec::with_capacity(RX_DESC_COUNT);

        for i in 0..RX_DESC_COUNT {
            let (buf_phys, buf_virt) = get_page();
            rx_ring[i] = RxDescriptor {
                addr: buf_phys,
                length: 0,
                checksum: 0,
                status: 0,
                errors: 0,
                special: 0,
            };
            let buf_slice = unsafe { core::slice::from_raw_parts_mut(buf_virt as *mut u8, 2048) };
            rx_buffers.push(buf_slice);
        }

        unsafe {
            write_volatile(
                (base_addr + REG_RDBAL as u64) as *mut u32,
                rx_ring_phys as u32,
            );
            write_volatile(
                (base_addr + REG_RDBAH as u64) as *mut u32,
                (rx_ring_phys >> 32) as u32,
            );
            write_volatile(
                (base_addr + REG_RDLEN as u64) as *mut u32,
                (RX_DESC_COUNT * 16) as u32,
            );
            write_volatile((base_addr + REG_RDH as u64) as *mut u32, 0);
            write_volatile(
                (base_addr + REG_RDT as u64) as *mut u32,
                (RX_DESC_COUNT - 1) as u32,
            );
            write_volatile(
                (base_addr + REG_RCTL as u64) as *mut u32,
                RCTL_EN | RCTL_SBP | RCTL_UPE | RCTL_MPE | RCTL_BAM | RCTL_BSIZE_2048,
            );
        }

        let (tx_ring_phys, tx_ring_virt) = get_page();
        let tx_ring = unsafe {
            core::slice::from_raw_parts_mut(tx_ring_virt as *mut TxDescriptor, TX_DESC_COUNT)
        };
        let mut tx_buffers = Vec::with_capacity(TX_DESC_COUNT);

        for i in 0..TX_DESC_COUNT {
            let (buf_phys, buf_virt) = get_page();
            tx_ring[i] = TxDescriptor {
                addr: buf_phys,
                length: 0,
                cso: 0,
                cmd: 0,
                status: 0,
                css: 0,
                special: 0,
            };
            let buf_slice = unsafe { core::slice::from_raw_parts_mut(buf_virt as *mut u8, 2048) };
            tx_buffers.push(buf_slice);
        }

        unsafe {
            write_volatile(
                (base_addr + REG_TDBAL as u64) as *mut u32,
                tx_ring_phys as u32,
            );
            write_volatile(
                (base_addr + REG_TDBAH as u64) as *mut u32,
                (tx_ring_phys >> 32) as u32,
            );
            write_volatile(
                (base_addr + REG_TDLEN as u64) as *mut u32,
                (TX_DESC_COUNT * 16) as u32,
            );
            write_volatile((base_addr + REG_TDH as u64) as *mut u32, 0);
            write_volatile((base_addr + REG_TDT as u64) as *mut u32, 0);
            write_volatile(
                (base_addr + REG_TCTL as u64) as *mut u32,
                TCTL_EN | TCTL_PSP,
            );
        }

        drop(allocator_guard);

        Ok(E1000 {
            base_addr,
            mac,
            rx_ring,
            rx_buffers,
            rx_next: 0,
            tx_ring,
            tx_buffers,
            tx_tail: 0,
        })
    }
    unsafe fn read_mac(base_addr: u64) -> [u8; 6] {
        let mut mac = [0u8; 6];
        for i in 0..3u32 {
            write_volatile((base_addr + REG_EERD as u64) as *mut u32, (i << 8) | 0x1);
            let mut val;
            loop {
                val = read_volatile((base_addr + REG_EERD as u64) as *const u32);
                if val & (1 << 4) != 0 {
                    break;
                }
            }
            let word = (val >> 16) as u16;
            mac[i as usize * 2] = word as u8;
            mac[i as usize * 2 + 1] = (word >> 8) as u8;
        }
        mac
    }

    pub fn send(&mut self, frame: &[u8]) -> bool {
        if self.is_tx_ring_full() {
            return false;
        }
        self.tx_buffers[self.tx_tail as usize][..frame.len()].copy_from_slice(frame);
        self.tx_ring[self.tx_tail as usize].length = frame.len() as u16;
        self.tx_ring[self.tx_tail as usize].cmd = CMD_EOP_IFCS_RS;
        self.tx_ring[self.tx_tail as usize].status = 0;

        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);

        self.tx_tail = (self.tx_tail + 1) % TX_DESC_COUNT as u32;
        unsafe {
            write_volatile((self.base_addr + REG_TDT as u64) as *mut u32, self.tx_tail);
        }

        true
    }
    pub fn receive(&mut self) -> Option<&[u8]> {
        let status = unsafe { read_volatile(&self.rx_ring[self.rx_next].status as *const u8) };
        if status & 0x1 == 0 {
            return None;
        }

        let length = self.rx_ring[self.rx_next].length as usize;
        let data = &self.rx_buffers[self.rx_next][..length];

        self.rx_ring[self.rx_next].status = 0;
        let old_idx = self.rx_next;
        self.rx_next = (self.rx_next + 1) % RX_DESC_COUNT;

        unsafe {
            write_volatile(
                (self.base_addr + REG_RDT as u64) as *mut u32,
                old_idx as u32,
            );
        }

        Some(data)
    }

    fn is_tx_ring_full(&self) -> bool {
        let head = self.read_register(REG_TDH);
        let tail = self.read_register(REG_TDT);
        let next_tail = (tail + 1) % TX_DESC_COUNT as u32;

        next_tail == head
    }

    fn read_register(&self, offset: u64) -> u32 {
        unsafe {
            let addr = (self.base_addr + offset) as usize;
            core::ptr::read_volatile(addr as *const u32)
        }
    }
}

lazy_static! {
    pub static ref E1000_DRIVER: Mutex<Option<E1000>> = Mutex::new(None);
}
