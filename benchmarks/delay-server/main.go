package main

import (
	"net/http"
	"time"
)

func main() {
	http.HandleFunc("/benchmark", func(w http.ResponseWriter, r *http.Request) {
		time.Sleep(1 * time.Millisecond)
		w.Header().Set("Content-Type", "text/plain")
		w.WriteHeader(200)
		w.Write([]byte("ok"))
	})
	http.ListenAndServe(":80", nil)
}
