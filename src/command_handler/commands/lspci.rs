use crate::drivers::pci;

pub fn cmd_lspci(args: &str) {
    let pci_list = pci::PciAddress::list_all();
    if pci_list.is_empty() {
        println!("PCI empty");
        return;
    }

    println!("BUS  DEV  FUN  ID_IDENTIFIER");
    println!("----------------------------");

    for device in pci_list {
        let vendor = device.get_vendor_id();
        let device_id = device.get_device_id();
        println!(
            "{:02x}   {:02x}   {:01x}    [{:04x}:{:04x}]",
            device.bus(),
            device.device(),
            device.function(),
            vendor,
            device_id
        );
    }
}
