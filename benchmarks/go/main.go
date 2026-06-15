package main

import (
	"fmt"
	"net"
	"net/http"
	"os"
	"sort"
	"strconv"
	"strings"
	"time"
)

func percentile(sorted []int64, p int) int64 {
	idx := len(sorted) * p / 100
	return sorted[idx]
}

type errors struct {
	timeout int
	conn    int
	parse   int
	dns     int
	socket  int
}

func printResults(label string, times []int64, errs errors) {
	fmt.Printf("=== %s ===\n", label)
	if len(times) == 0 {
		fmt.Println("no successful requests")
	} else {
		var sum int64
		for _, t := range times {
			sum += t
		}
		avg := sum / int64(len(times))
		p95 := percentile(times, 95)
		p99 := percentile(times, 99)
		fmt.Printf("min:    %d µs\n", times[0])
		fmt.Printf("max:    %d µs\n", times[len(times)-1])
		fmt.Printf("avg:    %d µs\n", avg)
		fmt.Printf("p95:    %d µs\n", p95)
		fmt.Printf("p99:    %d µs\n", p99)
		if sum == 0 {
			fmt.Printf("req/s:  0\n")
		} else {
			fmt.Printf("req/s:  %d\n", int64(len(times))*1_000_000/sum)
		}
	}
	fmt.Printf("errors: timeout=%d conn=%d parse=%d dns=%d socket=%d\n",
		errs.timeout, errs.conn, errs.parse, errs.dns, errs.socket)
}

func benchKeepAlive(url string, numRequests int) ([]int64, errors) {
	var times []int64
	var errs errors

	transport := &http.Transport{
		MaxIdleConnsPerHost: 1,
		DisableKeepAlives:   false,
		DialContext: (&net.Dialer{
			Timeout: 10 * time.Second,
		}).DialContext,
	}
	client := &http.Client{
		Transport: transport,
		Timeout:   10 * time.Second,
	}

	// warmup connect
	resp, err := client.Get(url)
	if err != nil {
		errs.conn++
		return times, errs
	}
	resp.Body.Close()

	for i := 0; i < numRequests; i++ {
		start := time.Now()
		resp, err := client.Get(url)
		elapsed := time.Since(start).Microseconds()
		if err != nil {
			if strings.Contains(err.Error(), "timeout") {
				errs.timeout++
			} else {
				errs.conn++
			}
			continue
		}
		resp.Body.Close()
		times = append(times, elapsed)
	}

	transport.CloseIdleConnections()
	sort.Slice(times, func(i, j int) bool { return times[i] < times[j] })
	return times, errs
}

func benchPerRequest(url string, numRequests int) ([]int64, errors) {
	var times []int64
	var errs errors

	for i := 0; i < numRequests; i++ {
		transport := &http.Transport{
			DisableKeepAlives: true,
			DialContext: (&net.Dialer{
				Timeout: 10 * time.Second,
			}).DialContext,
		}
		client := &http.Client{
			Transport: transport,
			Timeout:   10 * time.Second,
		}

		start := time.Now()
		resp, err := client.Get(url)
		elapsed := time.Since(start).Microseconds()
		if err != nil {
			if strings.Contains(err.Error(), "timeout") {
				errs.timeout++
			} else {
				errs.conn++
			}
			transport.CloseIdleConnections()
			continue
		}
		resp.Body.Close()
		transport.CloseIdleConnections()
		times = append(times, elapsed)
	}

	sort.Slice(times, func(i, j int) bool { return times[i] < times[j] })
	return times, errs
}

func main() {
	if len(os.Args) != 3 {
		fmt.Println("Usage: bench <url> <num_requests>")
		os.Exit(1)
	}

	rawURL := os.Args[1]
	if !strings.HasPrefix(rawURL, "http://") {
		rawURL = "http://" + rawURL
	}

	numRequests, err := strconv.Atoi(os.Args[2])
	if err != nil || numRequests < 1 {
		fmt.Println("bench: num_requests must be a number >= 1")
		os.Exit(1)
	}

	fmt.Println("starting keep-alive bench")
	kaTimes, kaErrs := benchKeepAlive(rawURL, numRequests)
	printResults("keep-alive", kaTimes, kaErrs)

	fmt.Println("starting per-request bench")
	prTimes, prErrs := benchPerRequest(rawURL, numRequests)
	printResults("per-request", prTimes, prErrs)
}
