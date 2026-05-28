use core::u16;

use crate::{kdebug, kinfo, net::ipv4::http::constants::SP};
use alloc::vec::{self, Vec};

pub struct response_parsed<'a> {
    pub status_code: u16,
    pub headers: Vec<(&'a str, &'a str)>,
    pub body: &'a [u8],
}

pub fn parse_response(raw: &[u8]) -> Option<response_parsed> {
    let mut parts = raw.splitn(3, |&b| b == b' ');
    let _version = parts.next()?;
    let status = parts.next()?;
    let remaining = parts.next()?;
    let status_code = core::str::from_utf8(status).ok()?.parse::<u16>().ok()?;

    let mut resp = response_parsed {
        status_code,
        headers: Vec::new(),
        body: &[],
    };

    let headers_body_str = core::str::from_utf8(remaining).ok()?;
    let splitted_headers_body_str = headers_body_str.split("\r\n");
    let mut headers_finished = false;
    for s in splitted_headers_body_str {
        if s == "" {
            headers_finished = true;
            continue;
        }

        if !s.contains(':') && !headers_finished {
            continue;
        }

        if s.contains(":") && !headers_finished {
            let splitted = s.split_once(": ")?;
            resp.headers.push(splitted);
            continue;
        }

        resp.body = s.as_bytes();
    }

    Some(resp)
}

pub fn run_tests() {
    let mut passed = 0;
    let mut failed = 0;

    let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\r\nhello";
    let result = parse_response(raw);
    check!("200 OK - is some", result.is_some(), passed, failed);
    if let Some(r) = parse_response(raw) {
        check!("200 OK - status code", r.status_code == 200, passed, failed);
        check!(
            "200 OK - Content-Type header",
            r.headers
                .iter()
                .any(|&(k, v)| k == "Content-Type" && v == "text/plain"),
            passed,
            failed
        );
        check!(
            "200 OK - Content-Length header",
            r.headers
                .iter()
                .any(|&(k, v)| k == "Content-Length" && v == "5"),
            passed,
            failed
        );
        check!("200 OK - body", r.body == b"hello", passed, failed);
    }

    let raw = b"HTTP/1.1 200 OK\r\n\r\n";
    let result = parse_response(raw);
    check!("200 no body - is some", result.is_some(), passed, failed);
    if let Some(r) = parse_response(raw) {
        check!(
            "200 no body - status code",
            r.status_code == 200,
            passed,
            failed
        );
        check!(
            "200 no body - body is empty",
            r.body.is_empty(),
            passed,
            failed
        );
    }

    let raw = b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
    let result = parse_response(raw);
    check!("404 - is some", result.is_some(), passed, failed);
    if let Some(r) = parse_response(raw) {
        check!("404 - status code", r.status_code == 404, passed, failed);
    }

    let raw = b"HTTP/1.1 500 Internal Server Error\r\n\r\n";
    let result = parse_response(raw);
    check!("500 - is some", result.is_some(), passed, failed);
    if let Some(r) = parse_response(raw) {
        check!("500 - status code", r.status_code == 500, passed, failed);
    }

    let raw = b"HTTP/1.1 301 Moved Permanently\r\nLocation: http://example.com/\r\n\r\n";
    if let Some(r) = parse_response(raw) {
        check!(
            "301 - Location header",
            r.headers
                .iter()
                .any(|&(k, v)| k == "Location" && v == "http://example.com/"),
            passed,
            failed
        );
    } else {
        check!("301 - is some", false, passed, failed);
    }

    let raw = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 13\r\nConnection: close\r\n\r\n{\"ok\":\"true\"}";
    if let Some(r) = parse_response(raw) {
        check!(
            "multiple headers - count",
            r.headers.len() == 3,
            passed,
            failed
        );
        check!(
            "multiple headers - body",
            r.body == b"{\"ok\":\"true\"}",
            passed,
            failed
        );
    } else {
        check!("multiple headers - is some", false, passed, failed);
    }

    let raw = b"GARBAGE\r\n\r\n";
    check!(
        "malformed status line - returns None",
        parse_response(raw).is_none(),
        passed,
        failed
    );

    check!(
        "empty input - returns None",
        parse_response(b"").is_none(),
        passed,
        failed
    );

    let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc";
    if let Some(r) = parse_response(raw) {
        check!(
            "body length matches Content-Length",
            r.body.len() == 3,
            passed,
            failed
        );
    } else {
        check!("body length - is some", false, passed, failed);
    }

    kinfo!(
        "response parser tests: {} passed, {} failed",
        passed,
        failed
    );
}
