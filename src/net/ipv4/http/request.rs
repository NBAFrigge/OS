use alloc::{format, string::String, vec::Vec};

use crate::{
    kinfo,
    net::ipv4::http::{
        constants::{EMPTYLINE, HTTP_VERSION, METHOD},
        url_parser::parse_url,
    },
};

pub struct Request {
    method: METHOD,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    pub timeout: usize,
}

impl Request {
    pub fn new(method: METHOD, url: String, headers: Vec<(String, String)>, body: &[u8]) -> Self {
        Self {
            method,
            url,
            headers,
            body: body.to_vec(),
            timeout: 200,
        }
    }

    pub fn build(&self) -> Option<Vec<u8>> {
        let parsed_url = parse_url(self.url.as_str())?;
        let mut request_bytes: Vec<u8> = Vec::new();

        request_bytes.extend_from_slice(self.method.to_string().as_bytes());
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

        for (key, value) in &self.headers {
            if *key == "Host" {
                continue;
            }
            request_bytes.extend_from_slice(key.as_bytes());
            request_bytes.extend_from_slice(b": ");
            request_bytes.extend_from_slice(value.as_bytes());
            request_bytes.extend_from_slice(EMPTYLINE.as_bytes());
        }

        if !self.body.is_empty() {
            request_bytes.extend_from_slice(b"Content-Length: ");
            request_bytes.extend_from_slice(format!("{}", self.body.len()).as_bytes());
            request_bytes.extend_from_slice(EMPTYLINE.as_bytes());
        }

        request_bytes.extend_from_slice(EMPTYLINE.as_bytes());

        if !self.body.is_empty() {
            request_bytes.extend_from_slice(&self.body);
        }

        Some(request_bytes)
    }
}

pub fn run_tests() {
    let mut passed = 0;
    let mut failed = 0;

    let result = Request::new(
        METHOD::GET,
        "http://192.168.1.100:8080/benchmark".into(),
        alloc::vec![("Connection".into(), "close".into())],
        b"",
    )
    .build();
    let expected =
        b"GET /benchmark HTTP/1.1\r\nHost: 192.168.1.100:8080\r\nConnection: close\r\n\r\n";
    check!(
        "GET with path and headers",
        result.as_deref() == Some(expected.as_ref()),
        passed,
        failed
    );

    let result = Request::new(
        METHOD::GET,
        "http://192.168.1.100/".into(),
        alloc::vec![],
        b"",
    )
    .build();
    let expected = b"GET / HTTP/1.1\r\nHost: 192.168.1.100\r\n\r\n";
    check!(
        "GET root path",
        result.as_deref() == Some(expected.as_ref()),
        passed,
        failed
    );

    let result = Request::new(
        METHOD::GET,
        "http://192.168.1.100/test".into(),
        alloc::vec![],
        b"",
    )
    .build();
    let expected = b"GET /test HTTP/1.1\r\nHost: 192.168.1.100\r\n\r\n";
    check!(
        "GET no body",
        result.as_deref() == Some(expected.as_ref()),
        passed,
        failed
    );

    let result = Request::new(METHOD::GET, "not-a-url".into(), alloc::vec![], b"").build();
    check!("invalid url returns None", result.is_none(), passed, failed);

    let result = Request::new(
        METHOD::POST,
        "http://192.168.1.100/api".into(),
        alloc::vec![(
            "Content-Type".into(),
            "application/x-www-form-urlencoded".into()
        )],
        b"hello=world",
    )
    .build();
    let expected = b"POST /api HTTP/1.1\r\nHost: 192.168.1.100\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 11\r\n\r\nhello=world";
    check!(
        "POST with body",
        result.as_deref() == Some(expected.as_ref()),
        passed,
        failed
    );

    let result = Request::new(
        METHOD::POST,
        "http://192.168.1.100/api".into(),
        alloc::vec![],
        b"",
    )
    .build();
    let expected = b"POST /api HTTP/1.1\r\nHost: 192.168.1.100\r\n\r\n";
    check!(
        "POST empty body",
        result.as_deref() == Some(expected.as_ref()),
        passed,
        failed
    );

    kinfo!("http::build tests: {} passed, {} failed", passed, failed);
}
