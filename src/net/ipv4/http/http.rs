use alloc::vec::Vec;

use crate::net::ipv4::http::constants::{self, HTTP_VERSION, METHOD};

pub struct request {}

impl request {
    pub fn build(
        &self,
        method: METHOD,
        url: &'static str,
        headers: Vec<(&'static str, &'static str)>,
    ) -> Vec<u8> {
        let mut request_bytes: Vec<u8> = Vec::new();
        request_bytes.extend_from_slice(method.to_string().as_bytes());
        request_bytes.extend_from_slice(" ".as_bytes());
        request_bytes.extend_from_slice(HTTP_VERSION.as_bytes());
        for (key, value) in headers {}

        request_bytes
    }
}

