package handlers

import (
	"encoding/json"
	"net/http"

	"github.com/aria-lang/bioflow-go/pkg/bioflow"
)

// QualityRequest represents a quality scores request.
type QualityRequest struct {
	Scores   []int  `json:"scores,omitempty"`
	Encoded  string `json:"encoded,omitempty"`
	Encoding string `json:"encoding,omitempty"` // "phred33" or "phred64"
}

// QualityResponse represents the response for quality parsing.
type QualityResponse struct {
	Scores []int `json:"scores"`
	Length int   `json:"length"`
}

// ParseQualityHandler handles quality parsing requests.
func ParseQualityHandler(w http.ResponseWriter, r *http.Request) {
	var req QualityRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	var quality *bioflow.QualityScores
	var err error

	if req.Encoded != "" {
		encoding := req.Encoding
		if encoding == "" {
			encoding = "phred33"
		}

		if encoding == "phred33" {
			quality, err = bioflow.ParseQualityPhred33(req.Encoded)
		} else if encoding == "phred64" {
			quality, err = bioflow.ParseQualityPhred64(req.Encoded)
		} else {
			http.Error(w, `{"error": "unknown encoding, use 'phred33' or 'phred64'"}`, http.StatusBadRequest)
			return
		}
	} else if len(req.Scores) > 0 {
		quality, err = bioflow.NewQualityScores(req.Scores)
	} else {
		http.Error(w, `{"error": "either 'scores' or 'encoded' is required"}`, http.StatusBadRequest)
		return
	}

	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(QualityResponse{
		Scores: quality.Values,
		Length: quality.Len(),
	})
}

// QualityStatsRequest represents a quality stats request.
type QualityStatsRequest struct {
	Scores []int `json:"scores"`
}

// QualityStatsResponse represents the response for quality stats.
type QualityStatsResponse struct {
	Count            int     `json:"count"`
	Min              int     `json:"min"`
	Max              int     `json:"max"`
	Mean             float64 `json:"mean"`
	Median           int     `json:"median"`
	HighQualityRatio float64 `json:"high_quality_ratio"`
	Category         string  `json:"category"`
}

// QualityStatsHandler handles quality statistics requests.
func QualityStatsHandler(w http.ResponseWriter, r *http.Request) {
	var req QualityStatsRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	if len(req.Scores) == 0 {
		http.Error(w, `{"error": "scores array is required"}`, http.StatusBadRequest)
		return
	}

	quality, err := bioflow.NewQualityScores(req.Scores)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	stats := quality.Statistics()

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(QualityStatsResponse{
		Count:            stats.Count,
		Min:              stats.MinScore,
		Max:              stats.MaxScore,
		Mean:             stats.Mean,
		Median:           stats.Median,
		HighQualityRatio: stats.HighQualityRatio,
		Category:         stats.Category.String(),
	})
}

// FilterReadRequest represents a filter read request.
type FilterReadRequest struct {
	Sequence   string `json:"sequence"`
	Scores     []int  `json:"scores"`
	MinQuality int    `json:"min_quality,omitempty"`
	MinLength  int    `json:"min_length,omitempty"`
	Strict     bool   `json:"strict,omitempty"`
}

// FilterReadResponse represents the response for read filtering.
type FilterReadResponse struct {
	Passed           bool    `json:"passed"`
	Reason           string  `json:"reason,omitempty"`
	TrimmedSequence  string  `json:"trimmed_sequence,omitempty"`
	TrimmedScores    []int   `json:"trimmed_scores,omitempty"`
	TrimStart        int     `json:"trim_start"`
	TrimEnd          int     `json:"trim_end"`
	OriginalLength   int     `json:"original_length"`
	TrimmedLength    int     `json:"trimmed_length"`
	MeanQuality      float64 `json:"mean_quality"`
}

// FilterReadHandler handles read filtering requests.
func FilterReadHandler(w http.ResponseWriter, r *http.Request) {
	var req FilterReadRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, `{"error": "invalid request body"}`, http.StatusBadRequest)
		return
	}

	seq, err := bioflow.NewSequence(req.Sequence)
	if err != nil {
		http.Error(w, `{"error": "sequence: `+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	quality, err := bioflow.NewQualityScores(req.Scores)
	if err != nil {
		http.Error(w, `{"error": "scores: `+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	if seq.Len() != quality.Len() {
		http.Error(w, `{"error": "sequence and scores must have same length"}`, http.StatusBadRequest)
		return
	}

	var filter *bioflow.Filter
	if req.Strict {
		filter = bioflow.StrictFilter()
	} else {
		filter = bioflow.DefaultFilter()
		if req.MinQuality > 0 {
			filter.MinQuality = req.MinQuality
		}
		if req.MinLength > 0 {
			filter.MinLength = req.MinLength
		}
	}

	result, err := filter.TrimAndFilter(seq, quality)
	if err != nil {
		http.Error(w, `{"error": "`+err.Error()+`"}`, http.StatusBadRequest)
		return
	}

	response := FilterReadResponse{
		Passed:         result.Passed,
		Reason:         result.Reason,
		TrimStart:      result.TrimStart,
		TrimEnd:        result.TrimEnd,
		OriginalLength: seq.Len(),
		MeanQuality:    result.MeanQuality,
	}

	if result.TrimmedSeq != nil {
		response.TrimmedSequence = result.TrimmedSeq.Bases
		response.TrimmedScores = result.TrimmedQual.Values
		response.TrimmedLength = result.TrimmedSeq.Len()
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)
}
