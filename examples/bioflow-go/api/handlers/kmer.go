package handlers

import (
	"encoding/json"
	"net/http"

	"github.com/aria-lang/bioflow-go/pkg/bioflow"
)

// KMerRequest represents a k-mer count request.
type KMerRequest struct {
	Sequence string `json:"sequence"`
	K        int    `json:"k"`
}

// KMerCountResponse represents the response for k-mer counting.
type KMerCountResponse struct {
	K           int               `json:"k"`
	UniqueCount int               `json:"unique_count"`
	TotalCount  int               `json:"total_count"`
	Counts      map[string]int    `json:"counts"`
}

// KMerCountHandler handles k-mer counting requests.
func KMerCountHandler(w http.ResponseWriter, r *http.Request) {
	var req KMerRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	if req.K <= 0 {
		http.Error(w, `{"error": "k must be positive"}`, http.StatusBadRequest)
		return
	}

	seq, err := bioflow.NewSequence(req.Sequence)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	counter, err := bioflow.CountKMers(seq, req.K)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(KMerCountResponse{
		K:           req.K,
		UniqueCount: counter.UniqueCount(),
		TotalCount:  counter.Total,
		Counts:      counter.Counts,
	})
}

// MostFrequentRequest represents a most frequent k-mers request.
type MostFrequentRequest struct {
	Sequence string `json:"sequence"`
	K        int    `json:"k"`
	N        int    `json:"n"`
}

// MostFrequentResponse represents the response for most frequent k-mers.
type MostFrequentResponse struct {
	KMers []KMerItem `json:"kmers"`
}

// KMerItem represents a k-mer and its count.
type KMerItem struct {
	KMer  string `json:"kmer"`
	Count int    `json:"count"`
}

// MostFrequentKMersHandler handles most frequent k-mers requests.
func MostFrequentKMersHandler(w http.ResponseWriter, r *http.Request) {
	var req MostFrequentRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	if req.K <= 0 || req.N <= 0 {
		http.Error(w, `{"error": "k and n must be positive"}`, http.StatusBadRequest)
		return
	}

	seq, err := bioflow.NewSequence(req.Sequence)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	kmers, err := bioflow.MostFrequentKMers(seq, req.K, req.N)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	items := make([]KMerItem, len(kmers))
	for i, kc := range kmers {
		items[i] = KMerItem{KMer: kc.KMer, Count: kc.Count}
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(MostFrequentResponse{KMers: items})
}

// KMerDistanceRequest represents a k-mer distance request.
type KMerDistanceRequest struct {
	Sequence1 string `json:"sequence1"`
	Sequence2 string `json:"sequence2"`
	K         int    `json:"k"`
}

// KMerDistanceResponse represents the response for k-mer distance.
type KMerDistanceResponse struct {
	Distance   float64 `json:"distance"`
	Similarity float64 `json:"similarity"`
}

// KMerDistanceHandler handles k-mer distance requests.
func KMerDistanceHandler(w http.ResponseWriter, r *http.Request) {
	var req KMerDistanceRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	if req.K <= 0 {
		http.Error(w, `{"error": "k must be positive"}`, http.StatusBadRequest)
		return
	}

	seq1, err := bioflow.NewSequence(req.Sequence1)
	if err != nil {
		http.Error(w, `{"error": "sequence1: `+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	seq2, err := bioflow.NewSequence(req.Sequence2)
	if err != nil {
		http.Error(w, `{"error": "sequence2: `+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	distance, err := bioflow.KMerDistance(seq1, seq2, req.K)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(KMerDistanceResponse{
		Distance:   distance,
		Similarity: 1.0 - distance,
	})
}

// SharedKMersRequest represents a shared k-mers request.
type SharedKMersRequest struct {
	Sequence1 string `json:"sequence1"`
	Sequence2 string `json:"sequence2"`
	K         int    `json:"k"`
}

// SharedKMersResponse represents the response for shared k-mers.
type SharedKMersResponse struct {
	SharedKMers []string `json:"shared_kmers"`
	Count       int      `json:"count"`
}

// SharedKMersHandler handles shared k-mers requests.
func SharedKMersHandler(w http.ResponseWriter, r *http.Request) {
	var req SharedKMersRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	if req.K <= 0 {
		http.Error(w, `{"error": "k must be positive"}`, http.StatusBadRequest)
		return
	}

	seq1, err := bioflow.NewSequence(req.Sequence1)
	if err != nil {
		http.Error(w, `{"error": "sequence1: `+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	seq2, err := bioflow.NewSequence(req.Sequence2)
	if err != nil {
		http.Error(w, `{"error": "sequence2: `+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	shared, err := bioflow.SharedKMers(seq1, seq2, req.K)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(SharedKMersResponse{
		SharedKMers: shared,
		Count:       len(shared),
	})
}
