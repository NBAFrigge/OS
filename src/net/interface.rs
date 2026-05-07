use lazy_static::lazy_static;
use spin::Mutex;

pub struct Interface {
    pub hw_addr: Option<[u8; 6]>,
    pub ip_addr: Option<[u8; 4]>,
    pub subnet_mask: Option<[u8; 4]>,
    pub gateway_ip: Option<[u8; 4]>,
}

impl Interface {
    pub fn is_local(&self, target_ip: [u8; 4]) -> bool {
        if let (Some(ip), Some(mask)) = (self.ip_addr, self.subnet_mask) {
            for i in 0..4 {
                if (ip[i] & mask[i]) != (target_ip[i] & mask[i]) {
                    return false;
                }
            }
            return true;
        }
        false
    }
}

lazy_static! {
    pub static ref NETWORK_INTERFACE: Mutex<Interface> = Mutex::new(Interface {
        hw_addr: None,
        ip_addr: None,
        subnet_mask: None,
        gateway_ip: None,
    });
}
