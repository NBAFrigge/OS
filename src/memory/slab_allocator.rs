use core::arch::x86_64::CpuidResult;

use crate::memory::buddy_allocator::PAGE_SIZE;

const MAX_ORDERS: usize = 10;

pub struct Slab {
    pub start_addr: *mut u8,
    pub num_allocated_objects: usize,
    pub first_free_slot: *mut u8,
    pub next: *mut Slab,
    pub prev: *mut Slab,
}
#[derive(Clone, Copy)]
pub struct SlabCache {
    pub object_size: usize,
    pub num_objects_per_slab: usize,
    pub slabs_full: *mut Slab,
    pub slabs_partial: *mut Slab,
    pub slabs_empty: *mut Slab,
}

impl SlabCache {
    fn default() -> Self {
        SlabCache {
            object_size: 0,
            num_objects_per_slab: 0,
            slabs_full: core::ptr::null_mut(),
            slabs_partial: core::ptr::null_mut(),
            slabs_empty: core::ptr::null_mut(),
        }
    }
}

pub struct SlabManager {
    pub slab_list: [SlabCache; MAX_ORDERS],
}

impl SlabManager {
    pub fn new() -> Self {
        let mut slab_manager = SlabManager {
            slab_list: [SlabCache::default(); MAX_ORDERS],
        };
        let free_space = PAGE_SIZE - size_of::<Slab>();
        for order in 0..MAX_ORDERS {
            slab_manager.slab_list[order].object_size = 1 << order;
            slab_manager.slab_list[order].num_objects_per_slab = free_space / (1 << order);
        }
        slab_manager
    }

    pub unsafe fn alloc(&mut self, size: u64) -> *mut u8 {
        let mut order = 0;
        while 1 << order < size {
            order += 1;
        }

        let cache = &mut self.slab_list[order];

        if !cache.slabs_partial.is_null() {
            let slab = cache.slabs_partial;
            let pointer = (*slab).first_free_slot;
            (*slab).first_free_slot = *(pointer as *mut *mut u8);
            (*slab).num_allocated_objects += 1;
            if (*slab).num_allocated_objects == cache.num_objects_per_slab {
                cache.slabs_partial = (*slab).next;
                if !cache.slabs_partial.is_null() {
                    (*cache.slabs_partial).prev = core::ptr::null_mut();
                }
                (*slab).next = cache.slabs_full;
                (*slab).prev = core::ptr::null_mut();
                if !cache.slabs_full.is_null() {
                    (*cache.slabs_full).prev = slab;
                }
                cache.slabs_full = slab;
            }
            return pointer;
        }

        if !cache.slabs_empty.is_null() {
            let slab = cache.slabs_empty;
            cache.slabs_empty = (*slab).next;
            if !cache.slabs_empty.is_null() {
                (*cache.slabs_empty).prev = core::ptr::null_mut();
            }
            (*slab).next = cache.slabs_partial;
            (*slab).prev = core::ptr::null_mut();
            if !cache.slabs_partial.is_null() {
                (*cache.slabs_partial).prev = slab;
            }
            cache.slabs_partial = slab;

            let pointer = (*slab).first_free_slot;
            (*slab).first_free_slot = *(pointer as *mut *mut u8);
            (*slab).num_allocated_objects += 1;
            return pointer;
        }

        core::ptr::null_mut()
    }

    pub unsafe fn dealloc(&mut self, mut ptr: *mut u8, size: u64) {
        let mut order = 0;
        while 1 << order < size {
            order += 1;
        }

        let cache = &mut self.slab_list[order];
        let slab = (ptr as usize & !(PAGE_SIZE - 1)) as *mut Slab;
        (*slab).num_allocated_objects -= 1;

        *(ptr as *mut *mut u8) = (*slab).first_free_slot;
        (*slab).first_free_slot = ptr;

        let next = (*slab).next;
        let prev = (*slab).prev;

        if !(*slab).next.is_null() {
            (*(*slab).next).prev = prev;
            (*slab).next = core::ptr::null_mut();
        }
        if !(*slab).prev.is_null() {
            (*(*slab).prev).next = next;
            (*slab).prev = core::ptr::null_mut();
        }

        if (*slab).num_allocated_objects == cache.num_objects_per_slab - 1 {
            if cache.slabs_full == slab {
                cache.slabs_full = next;
            }

            if cache.slabs_partial.is_null() {
                cache.slabs_partial = slab;
                return;
            }

            (*slab).next = cache.slabs_partial;
            (*cache.slabs_partial).prev = slab;
            cache.slabs_partial = slab;
        } else if (*slab).num_allocated_objects > 0 {
            if cache.slabs_partial == slab {
                cache.slabs_partial = next;
            }
            if cache.slabs_partial.is_null() {
                cache.slabs_partial = slab;
                return;
            }
            (*slab).next = cache.slabs_partial;
            (*cache.slabs_partial).prev = slab;
            cache.slabs_partial = slab;
        } else {
            if cache.slabs_partial == slab {
                cache.slabs_partial = next;
            }

            if cache.slabs_empty.is_null() {
                cache.slabs_empty = slab;
                return;
            }
            (*slab).next = cache.slabs_empty;
            (*cache.slabs_empty).prev = slab;
            cache.slabs_empty = slab;
        }
    }
}
