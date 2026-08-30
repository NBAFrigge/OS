use crate::vgadriver::writer::WRITER;

pub fn cmd_clear(_args: &str) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.clear();
        writer.redraw_shell_line();
    });
}
