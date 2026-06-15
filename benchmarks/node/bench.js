#!/usr/bin/env node
'use strict';

const http = require('http');
const { URL } = require('url');

function percentile(sorted, p) {
    const idx = Math.floor(sorted.length * p / 100);
    return sorted[idx];
}

function printResults(label, times, errors) {
    console.log(`=== ${label} ===`);
    if (times.length === 0) {
        console.log('no successful requests');
    } else {
        const sum = times.reduce((a, b) => a + b, 0n);
        const avg = sum / BigInt(times.length);
        const p95 = percentile(times, 95);
        const p99 = percentile(times, 99);
        console.log(`min:    ${times[0]} µs`);
        console.log(`max:    ${times[times.length - 1]} µs`);
        console.log(`avg:    ${avg} µs`);
        console.log(`p95:    ${p95} µs`);
        console.log(`p99:    ${p99} µs`);
        if (sum === 0n) {
            console.log(`req/s:  0`);
        } else {
            console.log(`req/s:  ${BigInt(times.length) * 1_000_000n / sum}`);
        }
    }
    console.log(`errors: timeout=${errors.timeout} conn=${errors.conn} parse=${errors.parse} dns=${errors.dns} socket=${errors.socket}`);
}

function httpGet(options, agent) {
    return new Promise((resolve, reject) => {
        const req = http.request({ ...options, agent }, (res) => {
            res.resume();
            res.on('end', () => resolve());
            res.on('error', reject);
        });
        req.on('error', reject);
        req.on('timeout', () => { req.destroy(); reject(new Error('timeout')); });
        req.end();
    });
}

async function benchKeepAlive(host, port, path, numRequests) {
    const times = [];
    const errors = { timeout: 0, conn: 0, parse: 0, dns: 0, socket: 0 };

    const agent = new http.Agent({
        keepAlive: true,
        maxSockets: 1,
        timeout: 10000,
    });

    const options = { host, port, path, method: 'GET', timeout: 10000 };

    // warmup connect
    try {
        await httpGet(options, agent);
    } catch {
        errors.conn++;
        agent.destroy();
        return { times, errors };
    }

    for (let i = 0; i < numRequests; i++) {
        const start = process.hrtime.bigint();
        try {
            await httpGet(options, agent);
            const elapsed = (process.hrtime.bigint() - start) / 1000n;
            times.push(elapsed);
        } catch (e) {
            if (e.message === 'timeout') errors.timeout++;
            else errors.conn++;
        }
    }

    agent.destroy();
    times.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
    return { times, errors };
}

async function benchPerRequest(host, port, path, numRequests) {
    const times = [];
    const errors = { timeout: 0, conn: 0, parse: 0, dns: 0, socket: 0 };
    const options = { host, port, path, method: 'GET', timeout: 10000 };

    for (let i = 0; i < numRequests; i++) {
        const agent = new http.Agent({ keepAlive: false });
        const start = process.hrtime.bigint();
        try {
            await httpGet(options, agent);
            const elapsed = (process.hrtime.bigint() - start) / 1000n;
            times.push(elapsed);
        } catch (e) {
            if (e.message === 'timeout') errors.timeout++;
            else errors.conn++;
        } finally {
            agent.destroy();
        }
    }

    times.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
    return { times, errors };
}

async function main() {
    const args = process.argv.slice(2);
    if (args.length !== 2) {
        console.error('Usage: bench.js <url> <num_requests>');
        process.exit(1);
    }

    let rawUrl = args[0];
    if (!rawUrl.startsWith('http://')) rawUrl = 'http://' + rawUrl;

    const parsed = new URL(rawUrl);
    const host = parsed.hostname;
    const port = parseInt(parsed.port) || 80;
    const path = parsed.pathname || '/';

    const numRequests = parseInt(args[1]);
    if (isNaN(numRequests) || numRequests < 1) {
        console.error('bench: num_requests must be a number >= 1');
        process.exit(1);
    }

    console.log('starting keep-alive bench');
    const ka = await benchKeepAlive(host, port, path, numRequests);
    printResults('keep-alive', ka.times, ka.errors);

    console.log('starting per-request bench');
    const pr = await benchPerRequest(host, port, path, numRequests);
    printResults('per-request', pr.times, pr.errors);
}

main().catch(e => { console.error(e); process.exit(1); });
