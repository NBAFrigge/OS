use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    kdebug,
    net::ipv4::http::{client, constants::METHOD, request::Request, url_parser::parse_url},
};

pub fn cmd_curl(args: &str) {
    let tokens = tokenize(args);
    if tokens.is_empty() {
        println!("Usage: curl [-X METHOD] [-H \"header: value\"] [-d body] <url>");
        return;
    }

    let mut method = METHOD::GET;
    let mut headers: Vec<(String, String)> = Vec::new();
    let mut body: Option<String> = None;
    let mut url: Option<String> = None;

    let mut i = 0;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "-X" => {
                i += 1;
                if i >= tokens.len() {
                    println!("curl: -X requires a method argument");
                    return;
                }
                method = match tokens[i].as_str() {
                    "GET" => METHOD::GET,
                    "POST" => METHOD::POST,
                    "PUT" => METHOD::PUT,
                    "DELETE" => METHOD::DELETE,
                    "HEAD" => METHOD::HEAD,
                    "OPTIONS" => METHOD::OPTIONS,
                    "PATCH" => METHOD::PATCH,
                    m => {
                        println!("curl: unknown method {}", m);
                        return;
                    }
                };
            }
            "-H" => {
                i += 1;
                if i >= tokens.len() {
                    println!("curl: -H requires a header argument");
                    return;
                }
                match tokens[i].split_once(": ") {
                    Some((k, v)) => headers.push((k.into(), v.into())),
                    None => {
                        println!("curl: invalid header format, expected \"Key: Value\"");
                        return;
                    }
                }
            }
            "-d" => {
                i += 1;
                if i >= tokens.len() {
                    println!("curl: -d requires a body argument");
                    return;
                }
                body = Some(tokens[i].clone());
            }
            arg if arg.starts_with('-') => {
                println!("curl: unknown option {}", arg);
                return;
            }
            arg => {
                if url.is_some() {
                    println!("curl: too many URLs specified");
                    return;
                }
                url = Some(arg.into());
            }
        }
        i += 1;
    }

    let url = match url {
        Some(u) => u,
        None => {
            println!("curl: no URL specified");
            return;
        }
    };

    if !url.starts_with("http://") {
        println!("curl: only http:// is supported");
        return;
    }

    let body_bytes: Vec<u8> = body.map(|b| b.into_bytes()).unwrap_or_default();

    let parsed_url = match parse_url(url.as_str()) {
        Some(p) => p,
        None => {
            println!("invalid url");
            return;
        }
    };
    let host = parsed_url.host.to_string();
    let port = parsed_url.port;

    let mut client = match client::connection::new(host.as_str(), port) {
        Ok(c) => c,
        Err(e) => { println!("{}", e.to_string()); return; }
    };
    if let Err(e) = client.connect() {
        println!("{}", e.to_string());
        return;
    }

    let req = Request::new(method, url, headers, &body_bytes);
    let resp = match client.send(req) {
        Ok(p) => p,
        Err(e) => {
            println!("{}", e.to_string());
            return;
        }
    };

    kdebug!("status_code: {}", resp.status_code);

    println!("{}", resp.status_code);
}

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in input.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ' ' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}
