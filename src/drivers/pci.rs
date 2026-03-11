// TODO: Implement BAR
use alloc::vec::Vec;
use x86_64::instructions::port::Port;

#[derive(Clone, Copy)]
pub struct PciAddress {
    config_address: u32,
    id_register: u32,
    class_register: u32,
}

impl PciAddress {
    pub fn new(bus: u32, device: u32, function: u32, offset: u32) -> Self {
        let mut addr = PciAddress {
            config_address: 1 << 31,
            id_register: 0xFFFFFFFF,
            class_register: 0,
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
        let mut addr_port = Port::<u32>::new(0xCF8);
        let mut data_port = Port::<u32>::new(0xCFC);
        unsafe {
            addr_port.write(addr.raw());
            self.id_register = data_port.read();
        }
    }

    pub fn read_class_register(&mut self) {
        let addr = PciAddress::new(self.bus(), self.device(), self.function(), 8);
        let mut addr_port = Port::<u32>::new(0xCF8);
        let mut data_port = Port::<u32>::new(0xCFC);
        unsafe {
            addr_port.write(addr.raw());
            self.class_register = data_port.read();
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
                    list.push(addr);
                }
            }
        }
        list
    }
}
