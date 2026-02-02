// Command bioflow provides a CLI for genomic sequence analysis.
//
// Usage:
//
//	bioflow [command] [options]
//
// Commands:
//
//	info        Show sequence information
//	gc          Calculate GC content
//	kmer        Count k-mers
//	align       Align two sequences
//	stats       Calculate sequence statistics
//	filter      Filter reads by quality
//	version     Show version information
package main

import (
	"flag"
	"fmt"
	"os"
	"strings"

	"github.com/aria-lang/bioflow-go/pkg/bioflow"
)

func main() {
	if len(os.Args) < 2 {
		printUsage()
		os.Exit(1)
	}

	command := os.Args[1]

	switch command {
	case "info":
		infoCmd(os.Args[2:])
	case "gc":
		gcCmd(os.Args[2:])
	case "kmer":
		kmerCmd(os.Args[2:])
	case "align":
		alignCmd(os.Args[2:])
	case "stats":
		statsCmd(os.Args[2:])
	case "filter":
		filterCmd(os.Args[2:])
	case "version":
		fmt.Println(bioflow.Info())
	case "help", "-h", "--help":
		printUsage()
	default:
		fmt.Fprintf(os.Stderr, "Unknown command: %s\n", command)
		printUsage()
		os.Exit(1)
	}
}

func printUsage() {
	fmt.Println(`BioFlow - Genomic Sequence Analysis Tool

Usage:
  bioflow <command> [options]

Commands:
  info      Show sequence information
  gc        Calculate GC content
  kmer      Count k-mers
  align     Align two sequences
  stats     Calculate sequence statistics
  filter    Filter reads by quality
  version   Show version information
  help      Show this help message

Use "bioflow <command> -h" for more information about a command.`)
}

func infoCmd(args []string) {
	fs := flag.NewFlagSet("info", flag.ExitOnError)
	file := fs.String("file", "", "FASTA file to analyze")
	seq := fs.String("seq", "", "Sequence string to analyze")
	fs.Parse(args)

	if *file == "" && *seq == "" {
		fmt.Fprintln(os.Stderr, "Error: Either -file or -seq is required")
		fs.Usage()
		os.Exit(1)
	}

	var sequences []*bioflow.Sequence
	var err error

	if *file != "" {
		sequences, err = bioflow.ReadFASTA(*file)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error reading file: %v\n", err)
			os.Exit(1)
		}
	} else {
		s, err := bioflow.NewSequence(*seq)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error creating sequence: %v\n", err)
			os.Exit(1)
		}
		sequences = []*bioflow.Sequence{s}
	}

	for i, s := range sequences {
		stats := bioflow.SequenceStats(s)
		fmt.Printf("Sequence %d:\n", i+1)
		if s.ID != "" {
			fmt.Printf("  ID: %s\n", s.ID)
		}
		fmt.Printf("  Length: %d bp\n", stats.Length)
		fmt.Printf("  GC Content: %.2f%%\n", stats.GCContent*100)
		fmt.Printf("  AT Content: %.2f%%\n", stats.ATContent*100)
		fmt.Printf("  Base Counts: A=%d, C=%d, G=%d, T=%d, N=%d\n",
			stats.ACount, stats.CCount, stats.GCount, stats.TCount, stats.NCount)
		fmt.Println()
	}
}

func gcCmd(args []string) {
	fs := flag.NewFlagSet("gc", flag.ExitOnError)
	file := fs.String("file", "", "FASTA file to analyze")
	seq := fs.String("seq", "", "Sequence string to analyze")
	fs.Parse(args)

	if *file == "" && *seq == "" {
		fmt.Fprintln(os.Stderr, "Error: Either -file or -seq is required")
		fs.Usage()
		os.Exit(1)
	}

	var sequences []*bioflow.Sequence
	var err error

	if *file != "" {
		sequences, err = bioflow.ReadFASTA(*file)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error reading file: %v\n", err)
			os.Exit(1)
		}
	} else {
		s, err := bioflow.NewSequence(*seq)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error creating sequence: %v\n", err)
			os.Exit(1)
		}
		sequences = []*bioflow.Sequence{s}
	}

	for _, s := range sequences {
		id := s.ID
		if id == "" {
			id = "sequence"
		}
		fmt.Printf("%s: %.4f (%.2f%%)\n", id, s.GCContent(), s.GCContent()*100)
	}
}

func kmerCmd(args []string) {
	fs := flag.NewFlagSet("kmer", flag.ExitOnError)
	file := fs.String("file", "", "FASTA file to analyze")
	seq := fs.String("seq", "", "Sequence string to analyze")
	k := fs.Int("k", 21, "K-mer size")
	top := fs.Int("top", 10, "Number of top k-mers to show")
	fs.Parse(args)

	if *file == "" && *seq == "" {
		fmt.Fprintln(os.Stderr, "Error: Either -file or -seq is required")
		fs.Usage()
		os.Exit(1)
	}

	var s *bioflow.Sequence
	var err error

	if *file != "" {
		sequences, err := bioflow.ReadFASTA(*file)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error reading file: %v\n", err)
			os.Exit(1)
		}
		if len(sequences) == 0 {
			fmt.Fprintln(os.Stderr, "No sequences found in file")
			os.Exit(1)
		}
		s = sequences[0]
	} else {
		s, err = bioflow.NewSequence(*seq)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error creating sequence: %v\n", err)
			os.Exit(1)
		}
	}

	counter, err := bioflow.CountKMers(s, *k)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error counting k-mers: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("K-mer Analysis (k=%d)\n", *k)
	fmt.Printf("Unique k-mers: %d\n", counter.UniqueCount())
	fmt.Printf("Total k-mers: %d\n", counter.Total)
	fmt.Println()

	topKMers, err := counter.MostFrequent(*top)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error getting top k-mers: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Top %d k-mers:\n", len(topKMers))
	for i, kc := range topKMers {
		fmt.Printf("%2d. %s: %d\n", i+1, kc.KMer, kc.Count)
	}
}

func alignCmd(args []string) {
	fs := flag.NewFlagSet("align", flag.ExitOnError)
	seq1 := fs.String("seq1", "", "First sequence")
	seq2 := fs.String("seq2", "", "Second sequence")
	global := fs.Bool("global", false, "Use global alignment (Needleman-Wunsch)")
	fs.Parse(args)

	if *seq1 == "" || *seq2 == "" {
		fmt.Fprintln(os.Stderr, "Error: Both -seq1 and -seq2 are required")
		fs.Usage()
		os.Exit(1)
	}

	s1, err := bioflow.NewSequence(*seq1)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating sequence 1: %v\n", err)
		os.Exit(1)
	}

	s2, err := bioflow.NewSequence(*seq2)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating sequence 2: %v\n", err)
		os.Exit(1)
	}

	var alignment *bioflow.Alignment
	if *global {
		alignment, err = bioflow.AlignGlobal(s1, s2)
	} else {
		alignment, err = bioflow.Align(s1, s2)
	}

	if err != nil {
		fmt.Fprintf(os.Stderr, "Error aligning sequences: %v\n", err)
		os.Exit(1)
	}

	fmt.Println(alignment.Format())
}

func statsCmd(args []string) {
	fs := flag.NewFlagSet("stats", flag.ExitOnError)
	file := fs.String("file", "", "FASTA file to analyze")
	fs.Parse(args)

	if *file == "" {
		fmt.Fprintln(os.Stderr, "Error: -file is required")
		fs.Usage()
		os.Exit(1)
	}

	sequences, err := bioflow.ReadFASTA(*file)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error reading file: %v\n", err)
		os.Exit(1)
	}

	if len(sequences) == 0 {
		fmt.Fprintln(os.Stderr, "No sequences found in file")
		os.Exit(1)
	}

	stats, err := bioflow.SequenceSetStats(sequences)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error calculating statistics: %v\n", err)
		os.Exit(1)
	}

	fmt.Println("Sequence Set Statistics")
	fmt.Println(strings.Repeat("-", 40))
	fmt.Printf("Number of sequences: %d\n", stats.Count)
	fmt.Printf("Total bases: %d\n", stats.TotalBases)
	fmt.Printf("Length range: %d - %d bp\n", stats.MinLength, stats.MaxLength)
	fmt.Printf("Mean length: %.1f bp\n", stats.MeanLength)
	fmt.Printf("Median length: %d bp\n", stats.MedianLength)
	fmt.Printf("N50: %d bp\n", stats.N50)
	fmt.Printf("Mean GC content: %.2f%%\n", stats.MeanGCContent*100)
	fmt.Printf("Total ambiguous bases: %d\n", stats.TotalAmbiguous)
}

func filterCmd(args []string) {
	fs := flag.NewFlagSet("filter", flag.ExitOnError)
	file := fs.String("file", "", "FASTQ file to filter")
	minQuality := fs.Int("min-quality", 20, "Minimum average quality")
	minLength := fs.Int("min-length", 50, "Minimum sequence length")
	strict := fs.Bool("strict", false, "Use strict filtering")
	fs.Parse(args)

	if *file == "" {
		fmt.Fprintln(os.Stderr, "Error: -file is required")
		fs.Usage()
		os.Exit(1)
	}

	reads, err := bioflow.ReadFASTQ(*file)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error reading file: %v\n", err)
		os.Exit(1)
	}

	var filter *bioflow.Filter
	if *strict {
		filter = bioflow.StrictFilter()
	} else {
		filter = bioflow.DefaultFilter()
		filter.MinQuality = *minQuality
		filter.MinLength = *minLength
	}

	pipeline := bioflow.NewPipeline(filter)
	result, err := pipeline.ProcessReads(reads)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error filtering reads: %v\n", err)
		os.Exit(1)
	}

	fmt.Println("Filter Results")
	fmt.Println(strings.Repeat("-", 40))
	fmt.Printf("Total reads: %d\n", result.TotalProcessed)
	fmt.Printf("Passed: %d (%.1f%%)\n", result.PassedCount, result.PassRate()*100)
	fmt.Printf("Failed: %d (%.1f%%)\n", result.FailedCount, (1-result.PassRate())*100)
}
