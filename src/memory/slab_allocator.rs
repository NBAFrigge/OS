use core::alloc::{GlobalAlloc, Layout};

use spin::mutex::Mutex;

use crate::memory::buddy_allocator::{BUDDYALLOCATOR, PAGE_SIZE};

pub const MAX_ORDERS: usize = 10;

pub struct Slab {
    pub start_addr: *mut u8,
    pub num_allocated_objects: usize,
    pub first_free_slot: *mut u8,
    pub next: *mut Slab,
    pub prev: *mut Slab,
}

impl Slab {
    pub fn new(start_addr: *mut u8) -> Self {
        Self {
            start_addr,
            num_allocated_objects: 0,
            first_free_slot: (start_addr as usize + size_of::<Slab>()) as *mut u8,
            next: core::ptr::null_mut(),
            prev: core::ptr::null_mut(),
        }
    }
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
    const fn default() -> Self {
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
    pub const fn new() -> Self {
        Self {
            slab_list: [SlabCache::default(); MAX_ORDERS],
        }
    }

    pub fn init(&mut self) {
        let free_space = PAGE_SIZE - size_of::<Slab>();
        for order in 0..MAX_ORDERS {
            self.slab_list[order].object_size = if 1 << order >= size_of::<*mut u8>() {
                1 << order
            } else {
                size_of::<*mut u8>()
            };

            self.slab_list[order].num_objects_per_slab =
                free_space / (self.slab_list[order].object_size);
        }
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

        if cache.slabs_empty.is_null() {
            let start_addr = BUDDYALLOCATOR.lock().alloc(PAGE_SIZE as u64);
            let new_slab = Slab::new(start_addr);
            core::ptr::write(start_addr as *mut Slab, new_slab);
            cache.slabs_empty = start_addr as *mut Slab;
            for i in 0..cache.num_objects_per_slab - 1 {
                *((start_addr as usize + size_of::<Slab>() + cache.object_size * i)
                    as *mut *mut u8) =
                    (start_addr as usize + size_of::<Slab>() + cache.object_size * (i + 1))
                        as *mut u8;
            }
            *((start_addr as usize
                + size_of::<Slab>()
                + cache.object_size * (cache.num_objects_per_slab - 1))
                as *mut *mut u8) = core::ptr::null_mut();
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

    pub unsafe fn dealloc(&mut self, ptr: *mut u8, size: u64) {
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

unsafe impl Send for SlabManager {}

pub struct LockedSblab(Mutex<SlabManager>);

impl LockedSblab {
    pub const fn new() -> Self {
        LockedSblab(Mutex::new(SlabManager::new()))
    }

    pub fn lock(&self) -> spin::MutexGuard<SlabManager> {
        self.0.lock()
    }
}

unsafe impl GlobalAlloc for LockedSblab {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0.lock().alloc(layout.size() as u64)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.lock().dealloc(ptr, layout.size() as u64)
    }
}

pub static SLABALLOCATOR: LockedSblab = LockedSblab::new();
