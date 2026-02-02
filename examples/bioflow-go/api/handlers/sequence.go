// Package handlers provides HTTP handlers for the BioFlow API.
package handlers

import (
	"encoding/json"
	"net/http"

	"github.com/aria-lang/bioflow-go/pkg/bioflow"
)

// SequenceRequest represents a request with a sequence.
type SequenceRequest struct {
	Sequence string `json:"sequence"`
}

// GCContentResponse represents the response for GC content.
type GCContentResponse struct {
	GCContent float64 `json:"gc_content"`
	Percent   float64 `json:"percent"`
}

// GCContentHandler handles GC content calculation requests.
func GCContentHandler(w http.ResponseWriter, r *http.Request) {
	var req SequenceRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	seq, err := bioflow.NewSequence(req.Sequence)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	gc := seq.GCContent()
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(GCContentResponse{
		GCContent: gc,
		Percent:   gc * 100,
	})
}

// ATContentResponse represents the response for AT content.
type ATContentResponse struct {
	ATContent float64 `json:"at_content"`
	Percent   float64 `json:"percent"`
}

// ATContentHandler handles AT content calculation requests.
func ATContentHandler(w http.ResponseWriter, r *http.Request) {
	var req SequenceRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	seq, err := bioflow.NewSequence(req.Sequence)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	at, err := seq.ATContent()
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(ATContentResponse{
		ATContent: at,
		Percent:   at * 100,
	})
}

// ComplementResponse represents the response for complement.
type ComplementResponse struct {
	Complement string `json:"complement"`
}

// ComplementHandler handles complement requests.
func ComplementHandler(w http.ResponseWriter, r *http.Request) {
	var req SequenceRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	seq, err := bioflow.NewSequence(req.Sequence)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	comp, err := seq.Complement()
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(ComplementResponse{
		Complement: comp.Bases,
	})
}

// ReverseComplementResponse represents the response for reverse complement.
type ReverseComplementResponse struct {
	ReverseComplement string `json:"reverse_complement"`
}

// ReverseComplementHandler handles reverse complement requests.
func ReverseComplementHandler(w http.ResponseWriter, r *http.Request) {
	var req SequenceRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	seq, err := bioflow.NewSequence(req.Sequence)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	rc, err := seq.ReverseComplement()
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(ReverseComplementResponse{
		ReverseComplement: rc.Bases,
	})
}

// TranscribeResponse represents the response for transcription.
type TranscribeResponse struct {
	RNA string `json:"rna"`
}

// TranscribeHandler handles transcription requests.
func TranscribeHandler(w http.ResponseWriter, r *http.Request) {
	var req SequenceRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	seq, err := bioflow.NewSequence(req.Sequence)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	rna, err := seq.Transcribe()
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(TranscribeResponse{
		RNA: rna.Bases,
	})
}

// SequenceInfoResponse represents sequence information.
type SequenceInfoResponse struct {
	Length       int     `json:"length"`
	GCContent    float64 `json:"gc_content"`
	ATContent    float64 `json:"at_content"`
	ACounts      int     `json:"a_count"`
	CCount       int     `json:"c_count"`
	GCount       int     `json:"g_count"`
	TCount       int     `json:"t_count"`
	NCount       int     `json:"n_count"`
	HasAmbiguous bool    `json:"has_ambiguous"`
}

// SequenceInfoHandler handles sequence info requests.
func SequenceInfoHandler(w http.ResponseWriter, r *http.Request) {
	var req SequenceRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	seq, err := bioflow.NewSequence(req.Sequence)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	stats := bioflow.SequenceStats(seq)

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(SequenceInfoResponse{
		Length:       stats.Length,
		GCContent:    stats.GCContent,
		ATContent:    stats.ATContent,
		ACounts:      stats.ACount,
		CCount:       stats.CCount,
		GCount:       stats.GCount,
		TCount:       stats.TCount,
		NCount:       stats.NCount,
		HasAmbiguous: stats.HasAmbiguous,
	})
}

// ValidateResponse represents validation result.
type ValidateResponse struct {
	Valid   bool   `json:"valid"`
	Message string `json:"message,omitempty"`
}

// ValidateHandler handles sequence validation requests.
func ValidateHandler(w http.ResponseWriter, r *http.Request) {
	var req SequenceRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	_, err := bioflow.NewSequence(req.Sequence)

	w.Header().Set("Content-Type", "application/json")
	if err != nil {
		json.NewEncoder(w).Encode(ValidateResponse{
			Valid:   false,
			Message: err.Error(),
		})
	} else {
		json.NewEncoder(w).Encode(ValidateResponse{
			Valid: true,
		})
	}
}

// SequenceStatsHandler handles sequence statistics requests.
func SequenceStatsHandler(w http.ResponseWriter, r *http.Request) {
	var req SequenceRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	seq, err := bioflow.NewSequence(req.Sequence)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	stats := bioflow.SequenceStats(seq)

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}

// SequenceSetRequest represents a request with multiple sequences.
type SequenceSetRequest struct {
	Sequences []string `json:"sequences"`
}

// SequenceSetStatsHandler handles sequence set statistics requests.
func SequenceSetStatsHandler(w http.ResponseWriter, r *http.Request) {
	var req SequenceSetRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	sequences := make([]*bioflow.Sequence, 0, len(req.Sequences))
	for _, s := range req.Sequences {
		seq, err := bioflow.NewSequence(s)
		if err != nil {
			http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
			return
		}
		sequences = append(sequences, seq)
	}

	stats, err := bioflow.SequenceSetStats(sequences)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}
