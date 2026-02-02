package handlers

import (
	"encoding/json"
	"net/http"

	"github.com/aria-lang/bioflow-go/pkg/bioflow"
)

// AlignmentRequest represents an alignment request.
type AlignmentRequest struct {
	Sequence1 string `json:"sequence1"`
	Sequence2 string `json:"sequence2"`
}

// AlignmentResponse represents the response for alignment.
type AlignmentResponse struct {
	AlignedSeq1 string  `json:"aligned_seq1"`
	AlignedSeq2 string  `json:"aligned_seq2"`
	Score       int     `json:"score"`
	Identity    float64 `json:"identity"`
	CIGAR       string  `json:"cigar"`
	Matches     int     `json:"matches"`
	Mismatches  int     `json:"mismatches"`
	Gaps        int     `json:"gaps"`
}

// LocalAlignHandler handles local alignment requests.
func LocalAlignHandler(w http.ResponseWriter, r *http.Request) {
	var req AlignmentRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
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

	alignment, err := bioflow.Align(seq1, seq2)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(AlignmentResponse{
		AlignedSeq1: alignment.AlignedSeq1,
		AlignedSeq2: alignment.AlignedSeq2,
		Score:       alignment.Score,
		Identity:    alignment.Identity,
		CIGAR:       alignment.ToCIGAR(),
		Matches:     alignment.MatchCount(),
		Mismatches:  alignment.MismatchCount(),
		Gaps:        alignment.TotalGaps(),
	})
}

// GlobalAlignHandler handles global alignment requests.
func GlobalAlignHandler(w http.ResponseWriter, r *http.Request) {
	var req AlignmentRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
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

	alignment, err := bioflow.AlignGlobal(seq1, seq2)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(AlignmentResponse{
		AlignedSeq1: alignment.AlignedSeq1,
		AlignedSeq2: alignment.AlignedSeq2,
		Score:       alignment.Score,
		Identity:    alignment.Identity,
		CIGAR:       alignment.ToCIGAR(),
		Matches:     alignment.MatchCount(),
		Mismatches:  alignment.MismatchCount(),
		Gaps:        alignment.TotalGaps(),
	})
}

// ScoreResponse represents the response for alignment score.
type ScoreResponse struct {
	Score int `json:"score"`
}

// AlignmentScoreHandler handles alignment score requests.
func AlignmentScoreHandler(w http.ResponseWriter, r *http.Request) {
	var req AlignmentRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
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

	alignment, err := bioflow.Align(seq1, seq2)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(ScoreResponse{Score: alignment.Score})
}
