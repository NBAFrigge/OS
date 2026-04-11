use alloc::vec;
use alloc::vec::Vec;
use core::ptr::{read_volatile, write_volatile};
use lazy_static::lazy_static;
use spin::mutex::Mutex;

use crate::drivers::pci::PciAddress;
use crate::memory::memory::PHYSICAL_MEMORY_OFFSET;
use core::sync::atomic::Ordering;

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
const REG_TDH: u32 = 0x3810;
const REG_TDT: u32 = 0x3818;
const REG_TCTL: u32 = 0x0400;

const RX_DESC_COUNT: usize = 32;
const TX_DESC_COUNT: usize = 32;
const BUFFER_SIZE: usize = 2048;

const CTRL_RST: u32 = 1 << 26;
const RCTL_EN: u32 = 1 << 1;
const RCTL_BAM: u32 = 1 << 15;
const RCTL_BSIZE_2048: u32 = 0 << 16;
const TCTL_EN: u32 = 1 << 1;
const TCTL_PSP: u32 = 1 << 3;

pub struct E1000 {
    pub base_addr: u64,
    pub mac: [u8; 6],

    pub rx_ring: Vec<RxDescriptor>,
    pub rx_buffers: Vec<Vec<u8>>,
    pub rx_tail: u32,

    pub tx_ring: Vec<TxDescriptor>,
    pub tx_buffers: Vec<Vec<u8>>,
    pub tx_tail: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct RxDescriptor {
    pub addr: u64,
    pub length: u16,
    pub checksum: u16,
    pub status: u8,
    pub errors: u8,
    pub special: u16,
}

#[repr(C, packed)]
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
        let nic = devices
            .iter()
            .find(|d| d.get_vendor_id() == 0x8086 && d.get_device_id() == 0x100E)
            .ok_or("E1000 not found")?;

        let bar0 = nic.get_bar(0).ok_or("BAR0 not found")?;
        let base_addr = bar0.address + offset;

        unsafe {
            let ctrl = read_volatile((base_addr + REG_CTRL as u64) as *const u32);
            write_volatile((base_addr + REG_CTRL as u64) as *mut u32, ctrl | CTRL_RST);
            while read_volatile((base_addr + REG_CTRL as u64) as *const u32) & CTRL_RST != 0 {}
        }

        // disable interrupt
        unsafe {
            write_volatile((base_addr + REG_IMC as u64) as *mut u32, 0xFFFFFFFF);
        }

        // set mac
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

        // init RX
        let mut rx_ring: Vec<RxDescriptor> = Vec::with_capacity(RX_DESC_COUNT);
        let mut rx_buffers: Vec<Vec<u8>> = Vec::with_capacity(RX_DESC_COUNT);

        for _ in 0..RX_DESC_COUNT {
            let buffer = vec![0u8; BUFFER_SIZE];
            let phys_addr = buffer.as_ptr() as u64 - offset;
            rx_ring.push(RxDescriptor {
                addr: phys_addr,
                length: 0,
                checksum: 0,
                status: 0,
                errors: 0,
                special: 0,
            });
            rx_buffers.push(buffer);
        }

        unsafe {
            let ring_phys = rx_ring.as_ptr() as u64 - offset;
            write_volatile((base_addr + REG_RDBAL as u64) as *mut u32, ring_phys as u32);
            write_volatile(
                (base_addr + REG_RDBAH as u64) as *mut u32,
                (ring_phys >> 32) as u32,
            );
            write_volatile(
                (base_addr + REG_RDLEN as u64) as *mut u32,
                (RX_DESC_COUNT * core::mem::size_of::<RxDescriptor>()) as u32,
            );
            write_volatile((base_addr + REG_RDH as u64) as *mut u32, 0);
            write_volatile(
                (base_addr + REG_RDT as u64) as *mut u32,
                (RX_DESC_COUNT - 1) as u32,
            );
            write_volatile(
                (base_addr + REG_RCTL as u64) as *mut u32,
                RCTL_EN | RCTL_BAM | RCTL_BSIZE_2048,
            );
        }

        // init TX
        let mut tx_ring: Vec<TxDescriptor> = Vec::with_capacity(TX_DESC_COUNT);
        let mut tx_buffers: Vec<Vec<u8>> = Vec::with_capacity(TX_DESC_COUNT);

        for _ in 0..TX_DESC_COUNT {
            let buffer = vec![0u8; BUFFER_SIZE];
            let phys_addr = buffer.as_ptr() as u64 - offset;
            tx_ring.push(TxDescriptor {
                addr: phys_addr,
                length: 0,
                cso: 0,
                cmd: 0,
                status: 0,
                css: 0,
                special: 0,
            });
            tx_buffers.push(buffer);
        }

        unsafe {
            let ring_phys = tx_ring.as_ptr() as u64 - offset;
            write_volatile((base_addr + REG_TDBAL as u64) as *mut u32, ring_phys as u32);
            write_volatile(
                (base_addr + REG_TDBAH as u64) as *mut u32,
                (ring_phys >> 32) as u32,
            );
            write_volatile(
                (base_addr + REG_TDLEN as u64) as *mut u32,
                (TX_DESC_COUNT * core::mem::size_of::<TxDescriptor>()) as u32,
            );
            write_volatile((base_addr + REG_TDH as u64) as *mut u32, 0);
            write_volatile((base_addr + REG_TDT as u64) as *mut u32, 0);
            write_volatile(
                (base_addr + REG_TCTL as u64) as *mut u32,
                TCTL_EN | TCTL_PSP,
            );
        }

        Ok(E1000 {
            base_addr,
            mac,
            rx_ring,
            rx_buffers,
            rx_tail: (RX_DESC_COUNT - 1) as u32,
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
}

lazy_static! {
    pub static ref E1000_DRIVER: Mutex<Option<E1000>> = Mutex::new(None);
}
