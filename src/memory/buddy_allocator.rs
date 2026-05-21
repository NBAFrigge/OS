use core::alloc::{GlobalAlloc, Layout};
use spin::Mutex;

pub const PAGE_SIZE: usize = 4096;
pub const MAX_ORDERS: usize = 11;

#[repr(C)]
pub struct FreeBlockNode {
    pub next: *mut FreeBlockNode,
    pub prev: *mut FreeBlockNode,
}

pub struct BuddyAllocator {
    pub free_lists: [*mut FreeBlockNode; MAX_ORDERS],
    pub memory_start: u64,
    pub memory_end: u64,
    pub total_free_bytes: u64,
}

impl BuddyAllocator {
    pub const fn new() -> Self {
        Self {
            free_lists: [core::ptr::null_mut(); MAX_ORDERS],
            memory_start: 0,
            memory_end: 0,
            total_free_bytes: 0,
        }
    }

    pub fn init(&mut self, start: u64, end: u64) {
        self.memory_start = start;
        self.memory_end = end;
        self.total_free_bytes = end - start;

        let mut current = start;
        for i in (0..MAX_ORDERS).rev() {
            let block_size = (PAGE_SIZE << i) as u64;
            if current + block_size <= end {
                let node = current as *mut FreeBlockNode;
                unsafe {
                    (*node).next = self.free_lists[i];
                    (*node).prev = core::ptr::null_mut();
                    if !self.free_lists[i].is_null() {
                        (*self.free_lists[i]).prev = node;
                    }
                }
                self.free_lists[i] = node;
                current += block_size;
            }
        }
    }

    pub unsafe fn alloc(&mut self, size: u64) -> *mut u8 {
        if size > self.total_free_bytes {
            return core::ptr::null_mut();
        }

        let mut order = 0;
        while (PAGE_SIZE << order) < size as usize {
            order += 1;
            if order >= MAX_ORDERS {
                return core::ptr::null_mut();
            }
        }

        let mut i = order;
        while i < MAX_ORDERS && self.free_lists[i].is_null() {
            i += 1;
        }
        if i >= MAX_ORDERS {
            return core::ptr::null_mut();
        }

        let candidate = self.free_lists[i] as *mut u8;
        self.free_lists[i] = (*(candidate as *mut FreeBlockNode)).next;
        if !self.free_lists[i].is_null() {
            (*self.free_lists[i]).prev = core::ptr::null_mut();
        }

        while i > order {
            let second_half = candidate.add(PAGE_SIZE << (i - 1)) as *mut FreeBlockNode;
            (*second_half).next = self.free_lists[i - 1];
            (*second_half).prev = core::ptr::null_mut();
            if !self.free_lists[i - 1].is_null() {
                (*self.free_lists[i - 1]).prev = second_half;
            }
            self.free_lists[i - 1] = second_half;
            i -= 1;
        }

        self.total_free_bytes -= (PAGE_SIZE << order) as u64;
        candidate
    }

    pub unsafe fn dealloc(&mut self, mut ptr: *mut u8, size: u64) {
        let mut order = 0;
        while (PAGE_SIZE << order) < size as usize {
            order += 1;
            if order >= MAX_ORDERS {
                return;
            }
        }

        for i in order..MAX_ORDERS {
            let buddy_ptr = (ptr as usize ^ (PAGE_SIZE << i)) as *mut u8;
            let mut potential_buddy = self.free_lists[i];

            while potential_buddy as *mut u8 != buddy_ptr && !potential_buddy.is_null() {
                potential_buddy = (*potential_buddy).next
            }

            if potential_buddy.is_null() {
                let node = ptr as *mut FreeBlockNode;
                (*node).next = self.free_lists[i];
                if !self.free_lists[i].is_null() {
                    (*self.free_lists[i]).prev = node;
                }
                (*node).prev = core::ptr::null_mut();
                self.free_lists[i] = node;
                self.total_free_bytes += (PAGE_SIZE << order) as u64;
                return;
            } else {
                let node = potential_buddy as *mut FreeBlockNode;
                if !(*node).prev.is_null() {
                    (*(*node).prev).next = (*node).next;
                } else {
                    self.free_lists[i] = (*node).next;
                }
                if !(*node).next.is_null() {
                    (*(*node).next).prev = (*node).prev;
                }
                ptr = ptr.min(potential_buddy as *mut u8);
            }
        }

        let node = ptr as *mut FreeBlockNode;
        (*node).next = self.free_lists[MAX_ORDERS - 1];
        if !self.free_lists[MAX_ORDERS - 1].is_null() {
            (*self.free_lists[MAX_ORDERS - 1]).prev = node;
        }
        (*node).prev = core::ptr::null_mut();
        self.free_lists[MAX_ORDERS - 1] = node;
        self.total_free_bytes += (PAGE_SIZE << order) as u64;
    }
}

unsafe impl Send for BuddyAllocator {}

pub struct LockedBuddy(Mutex<BuddyAllocator>);

impl LockedBuddy {
    pub const fn new() -> Self {
        LockedBuddy(Mutex::new(BuddyAllocator::new()))
    }

    pub fn lock(&self) -> spin::MutexGuard<BuddyAllocator> {
        self.0.lock()
    }
}

unsafe impl GlobalAlloc for LockedBuddy {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0.lock().alloc(layout.size() as u64)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.lock().dealloc(ptr, layout.size() as u64)
    }
}

#[global_allocator]
pub static ALLOCATOR: LockedBuddy = LockedBuddy::new();
