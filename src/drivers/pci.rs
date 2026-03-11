use alloc::vec::Vec;
use x86_64::instructions::port::Port;

#[derive(Clone, Copy)]
pub struct PciAddress(u32, u32);

impl PciAddress {
    pub fn new(bus: u32, device: u32, function: u32, offset: u32) -> Self {
        let mut addr = PciAddress(1 << 31, 0xFFFFFFFF);
        addr.set_bus(bus);
        addr.set_device(device);
        addr.set_function(function);
        addr.set_offset(offset);
        addr
    }

    pub fn bus(&self) -> u32 {
        (self.0 >> 16) & 0xFF
    }

    pub fn device(&self) -> u32 {
        (self.0 >> 11) & 0x1F
    }

    pub fn function(&self) -> u32 {
        (self.0 >> 8) & 0x07
    }

    pub fn offset(&self) -> u32 {
        self.0 & 0xFC
    }

    pub fn raw(&self) -> u32 {
        self.0
    }

    pub fn set_bus(&mut self, bus: u32) {
        self.0 = (self.0 & !(0xFF << 16)) | ((bus & 0xFF) << 16);
    }

    pub fn set_device(&mut self, device: u32) {
        self.0 = (self.0 & !(0x1F << 11)) | ((device & 0x1F) << 11);
    }

    pub fn set_function(&mut self, function: u32) {
        self.0 = (self.0 & !(0x07 << 8)) | ((function & 0x07) << 8);
    }

    pub fn set_offset(&mut self, offset: u32) {
        self.0 = (self.0 & !0xFC) | (offset & 0xFC);
    }

    pub fn set_register_id(&mut self) {
        let mut addr_port = Port::<u32>::new(0xCF8);
        let mut data_port = Port::<u32>::new(0xCFC);
        unsafe {
            addr_port.write(self.raw());
            self.1 = data_port.read();
        };
    }

    pub fn get_vendor_id(self) -> u32 {
        self.1 & 0xFFFF
    }

    pub fn get_device_id(self) -> u32 {
        (self.1 >> 16) & 0xFFFF
    }

    pub fn list_all() -> Vec<Self> {
        let mut list = Vec::new();

        for bus in 0..256 {
            for device in 0..32 {
                let mut addr = PciAddress::new(bus, device, 0, 0);
                addr.set_register_id();
                if addr.get_vendor_id() != 0xFFFF {
                    list.push(addr);
                }
            }
        }

        list
    }
}

