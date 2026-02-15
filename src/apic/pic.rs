use x86_64::instructions::port::Port;

pub unsafe fn disable_8259_pic() {
    let mut master_data: Port<u8> = Port::new(0x21);
    let mut slave_data: Port<u8> = Port::new(0xA1);

    master_data.write(0xFF);
    slave_data.write(0xFF);
}
