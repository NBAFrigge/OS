use alloc::string::String;

pub fn parser(s: &str) -> Option<(&str, &str)> {
    s.trim().split_once(' ')
}
