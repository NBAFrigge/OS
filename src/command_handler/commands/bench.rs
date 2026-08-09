use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    kdebug,
    net::ipv4::http::{self, client, constants::HttpError, request, url_parser::parse_url},
    task::task::sleep,
    timer::tsc,
};

pub fn cmd_bench(args: &str) {
    let tokens: Vec<&str> = args.split_whitespace().collect();

    if tokens.len() != 2 {
        println!("Usage: bench <url> <num_requests>");
        return;
    }

    let raw_url = tokens[0];
    let url = if raw_url.starts_with("http://") {
        raw_url.to_string()
    } else if raw_url.contains("://") {
        println!("bench: only http:// is supported");
        return;
    } else {
        alloc::format!("http://{}", raw_url)
    };

    let num_requests: usize = match tokens[1].parse() {
        Ok(n) if n >= 1 => n,
        _ => {
            println!("bench: num_requests must be a number >= 1");
            return;
        }
    };

    let parsed_url = match parse_url(url.as_str()) {
        Some(p) => p,
        None => {
            println!("invalid url");
            return;
        }
    };
    let host = parsed_url.host.to_string();
    let port = parsed_url.port;

    let request = request::Request::new(
        http::constants::METHOD::GET,
        url,
        Vec::<(String, String)>::new(),
        &[],
    );

    println!("starting keep-alive bench");

    let mut ka_client = match client::connection::new(host.as_str(), port, true) {
        Ok(c) => c,
        Err(e) => {
            println!("bench: {}", e.to_string());
            return;
        }
    };
    if let Err(e) = ka_client.connect() {
        println!(
            "bench: connect to {}:{} failed: {}",
            host,
            port,
            e.to_string()
        );
        return;
    }
    let resolved_ip = ka_client.remote_ip;

    let mut ka_err_timeout = 0usize;
    let mut ka_err_connection = 0usize;
    let mut ka_err_parse = 0usize;
    let mut ka_err_dns = 0usize;
    let mut ka_err_socket = 0usize;
    let mut ka_response_time_vec = Vec::<u64>::new();
    let mut ka_ttfb_vec = Vec::<u64>::new();
    let mut ka_read_vec = Vec::<u64>::new();

    while !ka_client.is_open() {
        sleep(1);
    }

    kdebug!("connection established on port {}", ka_client.local_port);

    for _ in 0..num_requests {
        let start = tsc::now_us();
        let res = ka_client.send(request.clone());
        let delta = tsc::now_us() - start;
        match res {
            Ok(_) => {
                ka_response_time_vec.push(delta);
                ka_ttfb_vec.push(client::LAST_TTFB_US.load(core::sync::atomic::Ordering::Relaxed));
                ka_read_vec.push(client::LAST_READ_US.load(core::sync::atomic::Ordering::Relaxed));
            }
            Err(e) => match e {
                HttpError::Timeout => ka_err_timeout += 1,
                HttpError::ConnectionFailed => ka_err_connection += 1,
                HttpError::ParseError => ka_err_parse += 1,
                HttpError::DnsError => ka_err_dns += 1,
                HttpError::SocketNotFound => ka_err_socket += 1,
            },
        }
    }

    ka_client.close();
    sleep(200);
    drop(ka_client);
    ka_response_time_vec.sort();
    print_results(
        "keep-alive",
        &ka_response_time_vec,
        ka_err_timeout,
        ka_err_connection,
        ka_err_parse,
        ka_err_dns,
        ka_err_socket,
    );
    ka_ttfb_vec.sort();
    ka_read_vec.sort();
    print_split(&ka_ttfb_vec, &ka_read_vec);

    kdebug!("keep-alive done");

    println!("starting per-request bench");

    let mut pr_err_timeout = 0usize;
    let mut pr_err_connection = 0usize;
    let mut pr_err_parse = 0usize;
    let mut pr_err_dns = 0usize;
    let mut pr_err_socket = 0usize;
    let mut pr_response_time_vec = Vec::<u64>::new();

    for _ in 0..num_requests {
        let start = tsc::now_us();
        let mut pr_client =
            client::connection::new_with_ip(host.as_str(), resolved_ip, port, false);
        match pr_client.connect() {
            Ok(_) => {
                let res = pr_client.send(request.clone());
                let delta = tsc::now_us() - start;
                pr_client.close();
                sleep(50);
                match res {
                    Ok(_) => pr_response_time_vec.push(delta),
                    Err(e) => match e {
                        HttpError::Timeout => pr_err_timeout += 1,
                        HttpError::ConnectionFailed => pr_err_connection += 1,
                        HttpError::ParseError => pr_err_parse += 1,
                        HttpError::DnsError => pr_err_dns += 1,
                        HttpError::SocketNotFound => pr_err_socket += 1,
                    },
                }
            }
            Err(_) => pr_err_connection += 1,
        }
    }

    pr_response_time_vec.sort();
    print_results(
        "per-request",
        &pr_response_time_vec,
        pr_err_timeout,
        pr_err_connection,
        pr_err_parse,
        pr_err_dns,
        pr_err_socket,
    );
}

fn print_results(
    label: &str,
    times: &[u64],
    err_timeout: usize,
    err_connection: usize,
    err_parse: usize,
    err_dns: usize,
    err_socket: usize,
) {
    println!("=== {} ===", label);
    if times.is_empty() {
        println!("no successful requests");
    } else {
        let sum: u64 = times.iter().sum();
        if sum == 0 {
            return;
        }
        let avg = sum / times.len() as u64;
        let p95 = times[times.len() * 95 / 100];
        let p99 = times[times.len() * 99 / 100];
        let req_s = times.len() as u64 * 1_000_000 / sum;
        println!("min:    {} us", times.first().unwrap());
        println!("max:    {} us", times.last().unwrap());
        println!("avg:    {} us", avg);
        println!("p95:    {} us", p95);
        println!("p99:    {} us", p99);
        println!("req/s:  {}", req_s);
    }
    println!(
        "errors: timeout={} conn={} parse={} dns={} socket={}",
        err_timeout, err_connection, err_parse, err_dns, err_socket
    );
}

fn print_split(ttfb: &[u64], read: &[u64]) {
    if ttfb.is_empty() {
        return;
    }
    let ttfb_avg = ttfb.iter().sum::<u64>() / ttfb.len() as u64;
    let read_avg = read.iter().sum::<u64>() / read.len() as u64;
    println!(
        "split:  ttfb avg={} min={} p99={} us | read avg={} min={} us",
        ttfb_avg,
        ttfb.first().unwrap(),
        ttfb[ttfb.len() * 99 / 100],
        read_avg,
        read.first().unwrap()
    );
}
