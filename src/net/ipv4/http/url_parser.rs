use crate::kinfo;
use alloc::vec::Vec;

pub struct UrlParsed<'a> {
    pub scheme: &'a str,
    pub host: &'a str,
    pub port: u16,
    pub path: &'a str,
}

pub fn parse_url(url: &str) -> Option<UrlParsed> {
    let splitted_scheme: Vec<&str> = url.split("://").collect();
    if splitted_scheme.len() != 2 {
        return None;
    }
    let scheme = splitted_scheme[0];
    let mut index = 0;
    while index < splitted_scheme[1].len()
        && splitted_scheme[1].as_bytes()[index] != b':'
        && splitted_scheme[1].as_bytes()[index] != b'/'
    {
        index += 1;
    }

    if index == splitted_scheme[1].len() {
        return Some(UrlParsed {
            scheme,
            host: splitted_scheme[1],
            port: 80,
            path: "/",
        });
    }

    let splitted_host = splitted_scheme[1].split_at(index);
    let host = splitted_host.0;
    let mut port = 80;
    let mut path = "/";
    if splitted_host.1.as_bytes()[0] == b':' {
        let splitted_port: Vec<&str> = splitted_host.1.split("/").collect();
        port = splitted_port[0][1..].parse::<u16>().unwrap();
        if splitted_port.len() > 1 {
            index = 0;
            while index < splitted_scheme[1].len()
                && splitted_host.1.as_bytes()[index] != b'/'
            {
                index += 1;
            }
            path = splitted_host.1.split_at(index).1;
        }
    }
    if splitted_host.1.as_bytes()[0] == b'/' {
        path = splitted_host.1
    }

    Some(UrlParsed {
        scheme,
        host,
        port,
        path,
    })
}

pub fn run_tests() {
    let mut passed = 0;
    let mut failed = 0;

    check!(
        "full url - is some",
        parse_url("http://192.168.1.100:8080/benchmark").is_some(),
        passed,
        failed
    );
    if let Some(r) = parse_url("http://192.168.1.100:8080/benchmark") {
        check!("full url - scheme", r.scheme == "http", passed, failed);
        check!("full url - host", r.host == "192.168.1.100", passed, failed);
        check!("full url - port", r.port == 8080, passed, failed);
        check!("full url - path", r.path == "/benchmark", passed, failed);
    }

    check!(
        "no port - is some",
        parse_url("http://192.168.1.100/benchmark").is_some(),
        passed,
        failed
    );
    if let Some(r) = parse_url("http://192.168.1.100/benchmark") {
        check!(
            "no port - port defaults to 80",
            r.port == 80,
            passed,
            failed
        );
        check!("no port - path", r.path == "/benchmark", passed, failed);
    }

    check!(
        "no path - is some",
        parse_url("http://192.168.1.100:8080").is_some(),
        passed,
        failed
    );
    if let Some(r) = parse_url("http://192.168.1.100:8080") {
        check!(
            "no path - path defaults to /",
            r.path == "/",
            passed,
            failed
        );
    }

    check!(
        "no port no path - is some",
        parse_url("http://192.168.1.100").is_some(),
        passed,
        failed
    );
    if let Some(r) = parse_url("http://192.168.1.100") {
        check!("no port no path - port", r.port == 80, passed, failed);
        check!("no port no path - path", r.path == "/", passed, failed);
    }

    check!(
        "missing scheme - returns None",
        parse_url("192.168.1.100/benchmark").is_none(),
        passed,
        failed
    );
    check!(
        "empty string - returns None",
        parse_url("").is_none(),
        passed,
        failed
    );

    kinfo!("url_parser tests: {} passed, {} failed", passed, failed);
}
