#[derive(Debug, Clone, Copy)]
pub enum BarType {
    Memory32,
    Memory64,
    Io,
}

#[derive(Debug, Clone, Copy)]
pub struct Bar {
    pub address: u64,
    pub size: usize,
    pub bar_type: BarType,
    pub prefetchable: bool,
}

impl Bar {
    pub fn new(
        address: u64,
        size: usize,
        bar_type: BarType,
        prefetchable: bool,
    ) -> Self {
        Bar {
            address,
            size,
            bar_type,
            prefetchable,
        }
    }
}
