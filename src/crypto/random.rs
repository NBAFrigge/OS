use crate::net::interface::NETWORK_INTERFACE;
use lazy_static::lazy_static;
use spin::Mutex;

pub struct EntropyPool {
    pub pool: u64,
}

impl EntropyPool {
    pub const fn new() -> Self {
        EntropyPool {
            pool: 0xACE1B2C3D4E5F6A7,
        }
    }

    pub fn init(&mut self) {
        let mac_addr = NETWORK_INTERFACE
            .lock()
            .hw_addr
            .unwrap_or([0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE]);

        let mut seed: u64 = 0;
        let mut i = 0;
        while i < 6 {
            seed |= (mac_addr[i] as u64) << (i * 8);
            i += 1;
        }

        seed ^= unsafe { core::arch::x86_64::_rdtsc() };
        if seed == 0 {
            seed = 0xACE1B2C3D4E5F6A7;
        }
        self.pool = seed;
    }

    pub fn add_noise(&mut self, noise: u64) {
        self.pool = self.pool.rotate_left(13) ^ noise;
    }
}

lazy_static! {
    pub static ref GLOBAL_ENTROPY: Mutex<EntropyPool> = Mutex::new(EntropyPool::new());
}

fn xor_shift() -> u64 {
    let mut entropy = GLOBAL_ENTROPY.lock();
    let mut x = entropy.pool;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    entropy.pool = x;
    x
}

pub fn generate_u64() -> u64 {
    xor_shift()
}

pub fn generate_u32() -> u32 {
    xor_shift() as u32
}

pub fn generate_u16() -> u16 {
    xor_shift() as u16
}
