pub enum METHOD {
    GET,
    POST,
    HEAD,
    OPTIONS,
    DELETE,
    PUT,
    PATCH,
}

impl METHOD {
    pub fn to_string(self) -> &'static str {
        match self {
            METHOD::GET => "GET",
            METHOD::POST => "POST",
            METHOD::HEAD => "HEAD",
            METHOD::OPTIONS => "OPTIONS",
            METHOD::DELETE => "DELETE",
            METHOD::PUT => "PUT",
            METHOD::PATCH => "PATCH",
        }
    }
}

pub const SP: u8 = 0x20;
pub const HTTP_VERSION: &'static str = "HTTP/1.1\r\n";
pub const EMPTYLINE: &'static str = "\r\n";
