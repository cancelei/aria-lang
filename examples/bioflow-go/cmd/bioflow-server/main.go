// Command bioflow-server provides a REST API for BioFlow operations.
//
// Usage:
//
//	bioflow-server [options]
//
// Options:
//
//	-port     Port to listen on (default: 8080)
//	-host     Host to bind to (default: localhost)
package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/aria-lang/bioflow-go/api/handlers"
	"github.com/aria-lang/bioflow-go/api/middleware"
	"github.com/go-chi/chi/v5"
	chimiddleware "github.com/go-chi/chi/v5/middleware"
)

func main() {
	port := flag.Int("port", 8080, "Port to listen on")
	host := flag.String("host", "localhost", "Host to bind to")
	flag.Parse()

	r := chi.NewRouter()

	// Global middleware
	r.Use(chimiddleware.RequestID)
	r.Use(chimiddleware.RealIP)
	r.Use(middleware.Logger)
	r.Use(chimiddleware.Recoverer)
	r.Use(chimiddleware.Timeout(60 * time.Second))

	// Health check
	r.Get("/health", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		w.Write([]byte("OK"))
	})

	// API routes
	r.Route("/api", func(r chi.Router) {
		// Sequence endpoints
		r.Route("/sequence", func(r chi.Router) {
			r.Post("/gc-content", handlers.GCContentHandler)
			r.Post("/at-content", handlers.ATContentHandler)
			r.Post("/complement", handlers.ComplementHandler)
			r.Post("/reverse-complement", handlers.ReverseComplementHandler)
			r.Post("/transcribe", handlers.TranscribeHandler)
			r.Post("/info", handlers.SequenceInfoHandler)
			r.Post("/validate", handlers.ValidateHandler)
		})

		// K-mer endpoints
		r.Route("/kmer", func(r chi.Router) {
			r.Post("/count", handlers.KMerCountHandler)
			r.Post("/most-frequent", handlers.MostFrequentKMersHandler)
			r.Post("/distance", handlers.KMerDistanceHandler)
			r.Post("/shared", handlers.SharedKMersHandler)
		})

		// Alignment endpoints
		r.Route("/alignment", func(r chi.Router) {
			r.Post("/local", handlers.LocalAlignHandler)
			r.Post("/global", handlers.GlobalAlignHandler)
			r.Post("/score", handlers.AlignmentScoreHandler)
		})

		// Quality endpoints
		r.Route("/quality", func(r chi.Router) {
			r.Post("/parse", handlers.ParseQualityHandler)
			r.Post("/stats", handlers.QualityStatsHandler)
			r.Post("/filter", handlers.FilterReadHandler)
		})

		// Statistics endpoints
		r.Route("/stats", func(r chi.Router) {
			r.Post("/sequence", handlers.SequenceStatsHandler)
			r.Post("/set", handlers.SequenceSetStatsHandler)
		})
	})

	// Serve static files
	fileServer := http.FileServer(http.Dir("./web/static"))
	r.Handle("/static/*", http.StripPrefix("/static/", fileServer))

	// Home page
	r.Get("/", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html")
		w.Write([]byte(`<!DOCTYPE html>
<html>
<head>
    <title>BioFlow API</title>
    <style>
        body { font-family: system-ui, sans-serif; max-width: 800px; margin: 2rem auto; padding: 0 1rem; }
        h1 { color: #2563eb; }
        pre { background: #f3f4f6; padding: 1rem; border-radius: 0.5rem; overflow-x: auto; }
        .endpoint { margin: 1rem 0; padding: 1rem; border: 1px solid #e5e7eb; border-radius: 0.5rem; }
        .method { display: inline-block; padding: 0.25rem 0.5rem; background: #10b981; color: white; border-radius: 0.25rem; font-size: 0.875rem; }
    </style>
</head>
<body>
    <h1>BioFlow API</h1>
    <p>A REST API for genomic sequence analysis.</p>

    <h2>Endpoints</h2>

    <div class="endpoint">
        <span class="method">POST</span> <code>/api/sequence/gc-content</code>
        <p>Calculate GC content of a sequence.</p>
        <pre>{"sequence": "ATGCATGC"}</pre>
    </div>

    <div class="endpoint">
        <span class="method">POST</span> <code>/api/sequence/complement</code>
        <p>Get the complement of a DNA sequence.</p>
        <pre>{"sequence": "ATGC"}</pre>
    </div>

    <div class="endpoint">
        <span class="method">POST</span> <code>/api/kmer/count</code>
        <p>Count k-mers in a sequence.</p>
        <pre>{"sequence": "ATGATGATG", "k": 3}</pre>
    </div>

    <div class="endpoint">
        <span class="method">POST</span> <code>/api/alignment/local</code>
        <p>Perform local alignment (Smith-Waterman).</p>
        <pre>{"sequence1": "ATGCATGC", "sequence2": "ATGCGGGG"}</pre>
    </div>

    <div class="endpoint">
        <span class="method">POST</span> <code>/api/quality/stats</code>
        <p>Calculate quality score statistics.</p>
        <pre>{"scores": [30, 30, 35, 35, 40]}</pre>
    </div>

    <p>For more information, see the <a href="https://github.com/aria-lang/bioflow-go">documentation</a>.</p>
</body>
</html>`))
	})

	addr := fmt.Sprintf("%s:%d", *host, *port)
	server := &http.Server{
		Addr:         addr,
		Handler:      r,
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 15 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	// Graceful shutdown
	done := make(chan bool, 1)
	quit := make(chan os.Signal, 1)

	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		<-quit
		log.Println("Server is shutting down...")

		ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
		defer cancel()

		server.SetKeepAlivesEnabled(false)
		if err := server.Shutdown(ctx); err != nil {
			log.Fatalf("Could not gracefully shutdown: %v\n", err)
		}
		close(done)
	}()

	log.Printf("BioFlow API server starting on http://%s\n", addr)
	if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		log.Fatalf("Could not listen on %s: %v\n", addr, err)
	}

	<-done
	log.Println("Server stopped")
}
