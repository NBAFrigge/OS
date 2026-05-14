use core::sync::atomic::Ordering;

use alloc::{collections::btree_map::BTreeMap, string::String};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{idt::interrupt::TICKS, kdebug};

pub struct DnsCacheEntry {
    pub ip: [u8; 4],
    pub expiration_tick: u32,
}

impl DnsCacheEntry {
    fn is_expired(&self) -> bool {
        let current_ticks = TICKS.load(Ordering::Relaxed) as u32;
        current_ticks >= self.expiration_tick
    }
}

pub fn get_from_cache(domain: &str) -> Option<[u8; 4]> {
    let mut cache = DNS_CACHE.lock();

    if let Some(entry) = cache.get(domain) {
        if entry.is_expired() {
            cache.remove(domain);
            kdebug!("DNS: Cache entry for {} expired", domain);
            return None;
        }
        return Some(entry.ip);
    }

    None
}

lazy_static! {
    pub static ref DNS_CACHE: Mutex<BTreeMap<String, DnsCacheEntry>> = Mutex::new(BTreeMap::new());
}
