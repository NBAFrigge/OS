use crate::vgadriver::writer::WRITER;

pub fn cmd_clear(_args: &str) {
    let mut writer = WRITER.lock();
    writer.clear();
    writer.redraw_shell_line();
}

