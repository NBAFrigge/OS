use alloc::vec::Vec;
use x86_64::instructions::port::Port;

use crate::drivers::bar::{Bar, BarType};

const PCI_CONFIG_ADDR: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

const PCI_BAR_TYPE_IO: u32 = 0x1;
const PCI_BAR_MEM_TYPE_MASK: u32 = 0x6;
const PCI_BAR_MEM_TYPE_64: u32 = 0x4;
const PCI_BAR_PREFETCHABLE: u32 = 0x8;

const PCI_BAR_IO_ADDR_MASK: u32 = !0x3;
const PCI_BAR_MEM_ADDR_MASK: u32 = !0xF;

#[derive(Clone, Copy)]
pub struct PciAddress {
    config_address: u32,
    id_register: u32,
    class_register: u32,
    bars: [Option<Bar>; 6],
}

impl PciAddress {
    pub fn new(bus: u32, device: u32, function: u32, offset: u32) -> Self {
        let mut addr = PciAddress {
            config_address: 1 << 31,
            id_register: 0xFFFFFFFF,
            class_register: 0,
            bars: [None; 6],
        };
        addr.set_bus(bus);
        addr.set_device(device);
        addr.set_function(function);
        addr.set_offset(offset);
        addr
    }

    pub fn bus(&self) -> u32 {
        (self.config_address >> 16) & 0xFF
    }

    pub fn device(&self) -> u32 {
        (self.config_address >> 11) & 0x1F
    }

    pub fn function(&self) -> u32 {
        (self.config_address >> 8) & 0x07
    }

    pub fn offset(&self) -> u32 {
        self.config_address & 0xFC
    }

    pub fn raw(&self) -> u32 {
        self.config_address
    }

    pub fn set_bus(&mut self, bus: u32) {
        self.config_address = (self.config_address & !(0xFF << 16)) | ((bus & 0xFF) << 16);
    }

    pub fn set_device(&mut self, device: u32) {
        self.config_address = (self.config_address & !(0x1F << 11)) | ((device & 0x1F) << 11);
    }

    pub fn set_function(&mut self, function: u32) {
        self.config_address = (self.config_address & !(0x07 << 8)) | ((function & 0x07) << 8);
    }

    pub fn set_offset(&mut self, offset: u32) {
        self.config_address = (self.config_address & !0xFC) | (offset & 0xFC);
    }

    pub fn read_id_register(&mut self) {
        let addr = PciAddress::new(self.bus(), self.device(), self.function(), 0);
        let mut addr_port = Port::<u32>::new(PCI_CONFIG_ADDR);
        let mut data_port = Port::<u32>::new(PCI_CONFIG_DATA);
        unsafe {
            addr_port.write(addr.raw());
            self.id_register = data_port.read();
        }
    }

    pub fn read_class_register(&mut self) {
        let addr = PciAddress::new(self.bus(), self.device(), self.function(), 8);
        let mut addr_port = Port::<u32>::new(PCI_CONFIG_ADDR);
        let mut data_port = Port::<u32>::new(PCI_CONFIG_DATA);
        unsafe {
            addr_port.write(addr.raw());
            self.class_register = data_port.read();
        }
    }

    pub fn read_bar(&mut self) {
        let mut addr_port = Port::<u32>::new(PCI_CONFIG_ADDR);
        let mut data_port = Port::<u32>::new(PCI_CONFIG_DATA);
        let mut skip_next = false;

        for i in 0..6 {
            if skip_next {
                skip_next = false;
                continue;
            }

            let bar_offset = 0x10 + (i * 4);
            let addr = PciAddress::new(self.bus(), self.device(), self.function(), bar_offset);

            unsafe {
                addr_port.write(addr.raw());
                let original_bar = data_port.read();

                if original_bar == 0 {
                    continue;
                }

                data_port.write(0xFFFFFFFF);
                let response_mask = data_port.read();
                data_port.write(original_bar);

                if (original_bar & PCI_BAR_TYPE_IO) != 0 {
                    let address = (original_bar & PCI_BAR_IO_ADDR_MASK) as u64;
                    let size = (!(response_mask & PCI_BAR_IO_ADDR_MASK)).wrapping_add(1);

                    self.bars[i as usize] =
                        Some(Bar::new(address, size as usize, BarType::Io, false));
                } else {
                    let is_prefetchable = (original_bar & PCI_BAR_PREFETCHABLE) != 0;
                    let is_64bit = (original_bar & PCI_BAR_MEM_TYPE_MASK) == PCI_BAR_MEM_TYPE_64;

                    if !is_64bit {
                        let address = (original_bar & PCI_BAR_MEM_ADDR_MASK) as u64;
                        let size = (!(response_mask & PCI_BAR_MEM_ADDR_MASK)).wrapping_add(1);

                        self.bars[i as usize] = Some(Bar::new(
                            address,
                            size as usize,
                            BarType::Memory32,
                            is_prefetchable,
                        ));
                    } else {
                        let next_bar_offset = bar_offset + 4;
                        let next_addr = PciAddress::new(
                            self.bus(),
                            self.device(),
                            self.function(),
                            next_bar_offset,
                        );

                        addr_port.write(next_addr.raw());
                        let original_high = data_port.read();
                        data_port.write(0xFFFFFFFF);
                        let response_high = data_port.read();
                        data_port.write(original_high);

                        let address = ((original_bar & PCI_BAR_MEM_ADDR_MASK) as u64)
                            | ((original_high as u64) << 32);

                        let full_mask = ((response_mask & PCI_BAR_MEM_ADDR_MASK) as u64)
                            | ((response_high as u64) << 32);
                        let size = (!full_mask).wrapping_add(1);

                        self.bars[i as usize] = Some(Bar::new(
                            address,
                            size as usize,
                            BarType::Memory64,
                            is_prefetchable,
                        ));
                        skip_next = true;
                    }
                }
            }
        }
    }

    pub fn get_vendor_id(&self) -> u16 {
        (self.id_register & 0xFFFF) as u16
    }

    pub fn get_device_id(&self) -> u16 {
        ((self.id_register >> 16) & 0xFFFF) as u16
    }

    pub fn get_base_class(&self) -> u8 {
        ((self.class_register >> 24) & 0xFF) as u8
    }

    pub fn get_subclass(&self) -> u8 {
        ((self.class_register >> 16) & 0xFF) as u8
    }

    pub fn get_bar(&self, index: usize) -> Option<Bar> {
        self.bars[index]
    }

    pub fn get_class_name(&self) -> &'static str {
        match self.get_base_class() {
            0x00 => "Unclassified",
            0x01 => match self.get_subclass() {
                0x01 => "IDE Interface",
                0x06 => "SATA Controller",
                _ => "Mass Storage Controller",
            },
            0x02 => match self.get_subclass() {
                0x00 => "Ethernet Controller",
                _ => "Network Controller",
            },
            0x03 => match self.get_subclass() {
                0x00 => "VGA Compatible Controller",
                _ => "Display Controller",
            },
            0x04 => "Multimedia Controller",
            0x06 => match self.get_subclass() {
                0x00 => "Host Bridge",
                0x01 => "ISA Bridge",
                0x04 => "PCI-to-PCI Bridge",
                _ => "Bridge Device",
            },
            0x07 => "Communication Controller",
            0x0C => match self.get_subclass() {
                0x03 => "USB Controller",
                _ => "Serial Bus Controller",
            },
            _ => "Unknown Device",
        }
    }

    pub fn list_all() -> Vec<Self> {
        let mut list = Vec::new();

        for bus in 0..256 {
            for device in 0..32 {
                let mut addr = PciAddress::new(bus, device, 0, 0);
                addr.read_id_register();

                if addr.get_vendor_id() != 0xFFFF {
                    addr.read_class_register();
                    addr.read_bar();
                    list.push(addr);
                }
            }
        }
        list
    }

    pub fn enable_bus_mastering(&mut self) {
        let addr = PciAddress::new(self.bus(), self.device(), self.function(), 0x04);
        let mut addr_port = Port::<u32>::new(PCI_CONFIG_ADDR);
        let mut data_port = Port::<u32>::new(PCI_CONFIG_DATA);
        unsafe {
            addr_port.write(addr.raw());
            let command = data_port.read();
            data_port.write(command | (1 << 2));
        }
    }

    pub fn get_IRQ(self) -> u8 {
        let addr = PciAddress::new(self.bus(), self.device(), self.function(), 0x3C);
        let mut addr_port = Port::<u32>::new(PCI_CONFIG_ADDR);
        let mut data_port = Port::<u32>::new(PCI_CONFIG_DATA);
        unsafe {
            addr_port.write(addr.raw());
            data_port.read() as u8
        }
    }
}
