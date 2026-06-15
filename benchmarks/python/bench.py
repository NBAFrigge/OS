#!/usr/bin/env python3
import sys
import time
import http.client
from urllib.parse import urlparse

def percentile(sorted_data, p):
    idx = len(sorted_data) * p // 100
    return sorted_data[idx]

def print_results(label, times_ms, errors):
    print(f"=== {label} ===")
    if not times_ms:
        print("no successful requests")
    else:
        total = sum(times_ms)
        avg = total // len(times_ms)
        p95 = percentile(times_ms, 95)
        p99 = percentile(times_ms, 99)
        req_s = len(times_ms) * 1_000_000 // total if total > 0 else 0
        print(f"min:    {times_ms[0]} µs")
        print(f"max:    {times_ms[-1]} µs")
        print(f"avg:    {avg} µs")
        print(f"p95:    {p95} µs")
        print(f"p99:    {p99} µs")
        print(f"req/s:  {req_s}")
    print(f"errors: timeout={errors['timeout']} conn={errors['conn']} parse={errors['parse']} dns={errors['dns']} socket={errors['socket']}")

def bench_keep_alive(host, port, path, num_requests):
    times = []
    errors = dict(timeout=0, conn=0, parse=0, dns=0, socket=0)
    conn = http.client.HTTPConnection(host, port, timeout=10)
    try:
        conn.connect()
    except Exception:
        errors['conn'] += 1
        return times, errors

    for _ in range(num_requests):
        start = time.monotonic()
        try:
            conn.request("GET", path, headers={"Connection": "keep-alive"})
            resp = conn.getresponse()
            resp.read()
            elapsed = int((time.monotonic() - start) * 1_000_000)
            times.append(elapsed)
        except TimeoutError:
            errors['timeout'] += 1
        except ConnectionError:
            errors['conn'] += 1
            try:
                conn = http.client.HTTPConnection(host, port, timeout=10)
                conn.connect()
            except Exception:
                pass
        except Exception:
            errors['parse'] += 1

    conn.close()
    times.sort()
    return times, errors

def bench_per_request(host, port, path, num_requests):
    times = []
    errors = dict(timeout=0, conn=0, parse=0, dns=0, socket=0)

    for _ in range(num_requests):
        start = time.monotonic()
        try:
            conn = http.client.HTTPConnection(host, port, timeout=10)
            conn.connect()
            conn.request("GET", path, headers={"Connection": "close"})
            resp = conn.getresponse()
            resp.read()
            elapsed = int((time.monotonic() - start) * 1_000_000)
            conn.close()
            times.append(elapsed)
        except TimeoutError:
            errors['timeout'] += 1
        except ConnectionError:
            errors['conn'] += 1
        except Exception:
            errors['parse'] += 1

    times.sort()
    return times, errors

def main():
    if len(sys.argv) != 3:
        print("Usage: bench.py <url> <num_requests>")
        sys.exit(1)

    raw_url = sys.argv[1]
    if not raw_url.startswith("http://"):
        raw_url = "http://" + raw_url

    parsed = urlparse(raw_url)
    host = parsed.hostname
    port = parsed.port or 80
    path = parsed.path or "/"

    num_requests = int(sys.argv[2])
    if num_requests < 1:
        print("bench: num_requests must be >= 1")
        sys.exit(1)

    print("starting keep-alive bench")
    ka_times, ka_errors = bench_keep_alive(host, port, path, num_requests)
    print_results("keep-alive", ka_times, ka_errors)

    print("starting per-request bench")
    pr_times, pr_errors = bench_per_request(host, port, path, num_requests)
    print_results("per-request", pr_times, pr_errors)

if __name__ == "__main__":
    main()
