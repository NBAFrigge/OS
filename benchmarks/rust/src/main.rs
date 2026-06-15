use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

struct ParsedUrl {
    host: String,
    port: u16,
    path: String,
}

fn parse_url(url: &str) -> Option<ParsedUrl> {
    let url = url.strip_prefix("http://")?;
    let (host_port, path) = match url.find('/') {
        Some(i) => (&url[..i], &url[i..]),
        None => (url, "/"),
    };
    let (host, port) = match host_port.find(':') {
        Some(i) => (&host_port[..i], host_port[i + 1..].parse().ok()?),
        None => (host_port, 80u16),
    };
    Some(ParsedUrl {
        host: host.to_string(),
        port,
        path: path.to_string(),
    })
}

#[derive(Default)]
struct Errors {
    timeout: usize,
    conn: usize,
    parse: usize,
    dns: usize,
    socket: usize,
}

fn percentile(sorted: &[u128], p: usize) -> u128 {
    sorted[sorted.len() * p / 100]
}

fn print_results(label: &str, times: &[u128], errs: &Errors) {
    println!("=== {} ===", label);
    if times.is_empty() {
        println!("no successful requests");
    } else {
        let sum: u128 = times.iter().sum();
        let avg = sum / times.len() as u128;
        let p95 = percentile(times, 95);
        let p99 = percentile(times, 99);
        let req_s = if sum > 0 { times.len() as u128 * 1_000_000 / sum } else { 0 };
        println!("min:    {} µs", times[0]);
        println!("max:    {} µs", times[times.len() - 1]);
        println!("avg:    {} µs", avg);
        println!("p95:    {} µs", p95);
        println!("p99:    {} µs", p99);
        println!("req/s:  {}", req_s);
    }
    println!(
        "errors: timeout={} conn={} parse={} dns={} socket={}",
        errs.timeout, errs.conn, errs.parse, errs.dns, errs.socket
    );
}

fn http_get(stream: &mut TcpStream, host: &str, path: &str) -> Result<(), ()> {
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: keep-alive\r\n\r\n",
        path, host
    );
    stream.write_all(request.as_bytes()).map_err(|_| ())?;

    // Read headers byte by byte until \r\n\r\n
    let mut header_buf = Vec::with_capacity(512);
    let mut byte = [0u8; 1];
    loop {
        stream.read_exact(&mut byte).map_err(|_| ())?;
        header_buf.push(byte[0]);
        if header_buf.ends_with(b"\r\n\r\n") {
            break;
        }
        if header_buf.len() > 8192 {
            return Err(());
        }
    }

    let headers = std::str::from_utf8(&header_buf).map_err(|_| ())?;
    if !headers.starts_with("HTTP/") {
        return Err(());
    }

    // Parse Content-Length and read body
    let content_length = headers
        .lines()
        .find(|l| l.to_lowercase().starts_with("content-length:"))
        .and_then(|l| l.splitn(2, ':').nth(1))
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(0);

    if content_length > 0 {
        let mut body = vec![0u8; content_length];
        stream.read_exact(&mut body).map_err(|_| ())?;
    }

    Ok(())
}

fn bench_keep_alive(parsed: &ParsedUrl, num_requests: usize) -> (Vec<u128>, Errors) {
    let mut times = Vec::new();
    let mut errs = Errors::default();
    let addr = format!("{}:{}", parsed.host, parsed.port);

    let mut stream = match TcpStream::connect(&addr) {
        Ok(s) => s,
        Err(_) => {
            errs.conn += 1;
            return (times, errs);
        }
    };
    stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(10))).ok();

    for _ in 0..num_requests {
        let start = Instant::now();
        match http_get(&mut stream, &parsed.host, &parsed.path) {
            Ok(()) => {
                let elapsed = start.elapsed().as_micros();
                times.push(elapsed);
            }
            Err(()) => {
                errs.conn += 1;
                match TcpStream::connect(&addr) {
                    Ok(s) => stream = s,
                    Err(_) => { errs.conn += 1; break; }
                }
            }
        }
    }

    times.sort_unstable();
    (times, errs)
}

fn bench_per_request(parsed: &ParsedUrl, num_requests: usize) -> (Vec<u128>, Errors) {
    let mut times = Vec::new();
    let mut errs = Errors::default();
    let addr = format!("{}:{}", parsed.host, parsed.port);

    for _ in 0..num_requests {
        let start = Instant::now();
        let mut stream = match TcpStream::connect(&addr) {
            Ok(s) => s,
            Err(_) => {
                errs.conn += 1;
                continue;
            }
        };
        stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(10))).ok();

        match http_get(&mut stream, &parsed.host, &parsed.path) {
            Ok(()) => {
                let elapsed = start.elapsed().as_micros();
                times.push(elapsed);
            }
            Err(()) => errs.parse += 1,
        }
    }

    times.sort_unstable();
    (times, errs)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: bench <url> <num_requests>");
        std::process::exit(1);
    }

    let raw_url = &args[1];
    let url = if raw_url.starts_with("http://") {
        raw_url.clone()
    } else {
        format!("http://{}", raw_url)
    };

    let num_requests: usize = match args[2].parse() {
        Ok(n) if n >= 1 => n,
        _ => {
            eprintln!("bench: num_requests must be a number >= 1");
            std::process::exit(1);
        }
    };

    let parsed = match parse_url(&url) {
        Some(p) => p,
        None => {
            eprintln!("invalid url");
            std::process::exit(1);
        }
    };

    println!("starting keep-alive bench");
    let (ka_times, ka_errs) = bench_keep_alive(&parsed, num_requests);
    print_results("keep-alive", &ka_times, &ka_errs);

    println!("starting per-request bench");
    let (pr_times, pr_errs) = bench_per_request(&parsed, num_requests);
    print_results("per-request", &pr_times, &pr_errs);
}
