use x86_64::{
    registers::control::Cr3,
    structures::paging::{page_table, OffsetPageTable},
    VirtAddr,
};

pub fn init(offset: u64) -> OffsetPageTable<'static> {
    let pml4_addr = Cr3::read().0.start_address().as_u64() + offset;
    let page_table_ptr = pml4_addr as *mut page_table::PageTable;

    unsafe {
        let level_4_table = &mut *page_table_ptr;
        OffsetPageTable::new(level_4_table, VirtAddr::new(offset))
    }
}
