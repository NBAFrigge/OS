use crate::vgadriver::writer::WRITER;

pub fn cmd_clear(_args: &str) {
    WRITER.lock().clear();
}
