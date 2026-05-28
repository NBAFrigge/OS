use alloc::{format, vec::Vec};

use crate::{
    kinfo,
    net::ipv4::http::{
        constants::{EMPTYLINE, HTTP_VERSION, METHOD},
        url_parser::parse_url,
    },
};

pub struct request {}

impl request {
    pub fn build(
        &self,
        method: METHOD,
        url: &'static str,
        headers: Vec<(&'static str, &'static str)>,
        body: &[u8],
    ) -> Option<Vec<u8>> {
        let parsed_url = parse_url(url)?;
        let mut request_bytes: Vec<u8> = Vec::new();

        request_bytes.extend_from_slice(method.to_string().as_bytes());
        request_bytes.extend_from_slice(b" ");
        request_bytes.extend_from_slice(parsed_url.path.as_bytes());
        request_bytes.extend_from_slice(b" ");
        request_bytes.extend_from_slice(HTTP_VERSION.as_bytes());

        request_bytes.extend_from_slice(b"Host: ");
        request_bytes.extend_from_slice(parsed_url.host.as_bytes());
        if parsed_url.port != 80 {
            request_bytes.extend_from_slice(b":");
            request_bytes.extend_from_slice(format!("{}", parsed_url.port).as_bytes());
        }
        request_bytes.extend_from_slice(EMPTYLINE.as_bytes());

        for (key, value) in &headers {
            if *key == "Host" {
                continue;
            }
            request_bytes.extend_from_slice(key.as_bytes());
            request_bytes.extend_from_slice(b": ");
            request_bytes.extend_from_slice(value.as_bytes());
            request_bytes.extend_from_slice(EMPTYLINE.as_bytes());
        }

        if !body.is_empty() {
            request_bytes.extend_from_slice(b"Content-Length: ");
            request_bytes.extend_from_slice(format!("{}", body.len()).as_bytes());
            request_bytes.extend_from_slice(EMPTYLINE.as_bytes());
        }

        request_bytes.extend_from_slice(EMPTYLINE.as_bytes());

        if !body.is_empty() {
            request_bytes.extend_from_slice(body);
        }

        Some(request_bytes)
    }
}

pub fn run_tests() {
    let mut passed = 0;
    let mut failed = 0;
    let r = request {};

    let result = r.build(
        METHOD::GET,
        "http://192.168.1.100:8080/benchmark",
        alloc::vec![("Connection", "close")],
        b"",
    );
    let expected =
        b"GET /benchmark HTTP/1.1\r\nHost: 192.168.1.100:8080\r\nConnection: close\r\n\r\n";
    check!(
        "GET with path and headers",
        result.as_deref() == Some(expected.as_ref()),
        passed,
        failed
    );

    let result = r.build(METHOD::GET, "http://192.168.1.100/", alloc::vec![], b"");
    let expected = b"GET / HTTP/1.1\r\nHost: 192.168.1.100\r\n\r\n";
    check!(
        "GET root path",
        result.as_deref() == Some(expected.as_ref()),
        passed,
        failed
    );

    let result = r.build(METHOD::GET, "http://192.168.1.100/test", alloc::vec![], b"");
    let expected = b"GET /test HTTP/1.1\r\nHost: 192.168.1.100\r\n\r\n";
    check!(
        "GET no body",
        result.as_deref() == Some(expected.as_ref()),
        passed,
        failed
    );

    let result = r.build(METHOD::GET, "not-a-url", alloc::vec![], b"");
    check!("invalid url returns None", result.is_none(), passed, failed);

    let body = b"hello=world";
    let result = r.build(
        METHOD::POST,
        "http://192.168.1.100/api",
        alloc::vec![("Content-Type", "application/x-www-form-urlencoded")],
        body,
    );
    let expected = b"POST /api HTTP/1.1\r\nHost: 192.168.1.100\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 11\r\n\r\nhello=world";
    check!(
        "POST with body",
        result.as_deref() == Some(expected.as_ref()),
        passed,
        failed
    );

    let result = r.build(METHOD::POST, "http://192.168.1.100/api", alloc::vec![], b"");
    let expected = b"POST /api HTTP/1.1\r\nHost: 192.168.1.100\r\n\r\n";
    check!(
        "POST empty body",
        result.as_deref() == Some(expected.as_ref()),
        passed,
        failed
    );

    kinfo!("http::build tests: {} passed, {} failed", passed, failed);
}
