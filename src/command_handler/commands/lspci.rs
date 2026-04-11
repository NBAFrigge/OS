use crate::drivers::pci;

pub fn cmd_lspci(args: &str) {
    let pci_list = pci::PciAddress::list_all();
    if pci_list.is_empty() {
        println!("PCI empty");
        return;
    }

    println!("BUS  DEV  FUN  ID_IDENTIFIER  TYPE");
    println!("-------------------------------------");

    for device in pci_list {
        println!(
            "{:02x}:{:02x}.{:01x} [{:04x}:{:04x}] {}",
            device.bus(),
            device.device(),
            device.function(),
            device.get_vendor_id(),
            device.get_device_id(),
            device.get_class_name(),
        );
    }
}
