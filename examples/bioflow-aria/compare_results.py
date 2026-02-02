#!/usr/bin/env python3
"""
BioFlow Cross-Language Benchmark Comparison
Parses benchmark results and generates markdown comparison table
"""

import argparse
import re
import sys
from dataclasses import dataclass
from typing import Dict, List, Optional
from pathlib import Path


@dataclass
class BenchmarkMetric:
    """Single benchmark measurement"""
    operation: str
    input_size: Optional[int]
    time_ms: float
    iterations: int
    language: str


def parse_aria_results(filepath: Path) -> List[BenchmarkMetric]:
    """Parse Aria benchmark output"""
    metrics = []

    if not filepath.exists():
        return metrics

    with open(filepath) as f:
        content = f.read()

    # Look for benchmark result lines
    # Format: | Benchmark Name | 1.23ms | 0.99ms | 1.45ms | 1000 |
    pattern = r'\| (.+?) \| ([\d.]+)ms \| ([\d.]+)ms \| ([\d.]+)ms \| (\d+) \|'

    for match in re.finditer(pattern, content):
        name = match.group(1).strip()
        avg_time = float(match.group(2))
        iterations = int(match.group(5))

        # Extract size from name if present
        size_match = re.search(r'\((\d+)(?:bp)?\)', name)
        size = int(size_match.group(1)) if size_match else None

        # Extract operation name
        operation = re.sub(r'\s*\([^)]+\)', '', name).strip()

        metrics.append(BenchmarkMetric(
            operation=operation,
            input_size=size,
            time_ms=avg_time,
            iterations=iterations,
            language='Aria'
        ))

    return metrics


def parse_python_results(filepath: Path) -> List[BenchmarkMetric]:
    """Parse Python benchmark output"""
    metrics = []

    if not filepath.exists():
        return metrics

    with open(filepath) as f:
        content = f.read()

    # GC Content benchmarks
    # Format: 1,000 bp x 1000 iterations: 14.59ms (0.0146ms/call)
    gc_pattern = r'(\d+(?:,\d+)*) bp x (\d+) iterations: ([\d.]+)ms \(([\d.]+)ms/call\)'
    for match in re.finditer(gc_pattern, content):
        size = int(match.group(1).replace(',', ''))
        iterations = int(match.group(2))
        time_ms = float(match.group(3))

        metrics.append(BenchmarkMetric(
            operation='GC Content',
            input_size=size,
            time_ms=time_ms,
            iterations=iterations,
            language='Python'
        ))

    # K-mer benchmarks
    # Format: 5,000 bp, k=11: 2.71ms
    kmer_pattern = r'(\d+(?:,\d+)*) bp, k=(\d+): ([\d.]+)ms'
    for match in re.finditer(kmer_pattern, content):
        size = int(match.group(1).replace(',', ''))
        k = int(match.group(2))
        time_ms = float(match.group(3))

        metrics.append(BenchmarkMetric(
            operation=f'K-mer k={k}',
            input_size=size,
            time_ms=time_ms,
            iterations=1,
            language='Python'
        ))

    # Smith-Waterman benchmarks
    # Format: 100 × 100 bp:
    #   Smith-Waterman (full): 2.13ms
    sw_pattern = r'(\d+) × (\d+) bp:\s+Smith-Waterman \(full\): ([\d.]+)ms'
    for match in re.finditer(sw_pattern, content, re.MULTILINE):
        size = int(match.group(1))
        time_ms = float(match.group(3))

        metrics.append(BenchmarkMetric(
            operation='Smith-Waterman',
            input_size=size,
            time_ms=time_ms,
            iterations=1,
            language='Python'
        ))

    return metrics


def parse_go_results(filepath: Path) -> List[BenchmarkMetric]:
    """Parse Go benchmark output"""
    metrics = []

    if not filepath.exists():
        return metrics

    with open(filepath) as f:
        content = f.read()

    # Go benchmark format: BenchmarkGCContent-8  100000000  10.41 ns/op  0 B/op  0 allocs/op
    pattern = r'Benchmark(\w+)-\d+\s+(\d+)\s+([\d.]+)\s+(ns|µs|ms)/op'

    for match in re.finditer(pattern, content):
        operation = match.group(1)
        iterations = int(match.group(2))
        time_value = float(match.group(3))
        time_unit = match.group(4)

        # Convert to milliseconds
        if time_unit == 'ns':
            time_ms = time_value / 1_000_000
        elif time_unit == 'µs':
            time_ms = time_value / 1_000
        else:  # ms
            time_ms = time_value

        # Clean up operation name
        operation = re.sub(r'([A-Z])', r' \1', operation).strip()

        metrics.append(BenchmarkMetric(
            operation=operation,
            input_size=None,
            time_ms=time_ms,
            iterations=iterations,
            language='Go'
        ))

    return metrics


def parse_rust_results(filepath: Path) -> List[BenchmarkMetric]:
    """Parse Rust Criterion benchmark output"""
    metrics = []

    if not filepath.exists():
        return metrics

    with open(filepath) as f:
        content = f.read()

    # Criterion format: gc_content/1000        time:   [12.345 µs 12.456 µs 12.567 µs]
    pattern = r'(\w+(?:/\w+)*?)/(\d+)\s+time:\s+\[([\d.]+)\s+(ns|µs|ms)\s+([\d.]+)\s+(ns|µs|ms)\s+([\d.]+)\s+(ns|µs|ms)\]'

    for match in re.finditer(pattern, content):
        operation = match.group(1).replace('_', ' ').title()
        size = int(match.group(2))
        avg_value = float(match.group(5))  # Middle value
        time_unit = match.group(6)

        # Convert to milliseconds
        if time_unit == 'ns':
            time_ms = avg_value / 1_000_000
        elif time_unit == 'µs':
            time_ms = avg_value / 1_000
        else:  # ms
            time_ms = avg_value

        metrics.append(BenchmarkMetric(
            operation=operation,
            input_size=size,
            time_ms=time_ms,
            iterations=1,
            language='Rust'
        ))

    return metrics


def group_metrics(metrics: List[BenchmarkMetric]) -> Dict[str, Dict[str, BenchmarkMetric]]:
    """Group metrics by operation and language"""
    grouped = {}

    for metric in metrics:
        key = f"{metric.operation}"
        if metric.input_size:
            key += f" ({metric.input_size}bp)"

        if key not in grouped:
            grouped[key] = {}

        grouped[key][metric.language] = metric

    return grouped


def format_time(time_ms: float) -> str:
    """Format time in appropriate unit"""
    if time_ms < 0.001:
        return f"{time_ms * 1_000_000:.2f}ns"
    elif time_ms < 1.0:
        return f"{time_ms * 1_000:.2f}µs"
    elif time_ms < 1000:
        return f"{time_ms:.2f}ms"
    else:
        return f"{time_ms / 1000:.2f}s"


def calculate_speedup(baseline_ms: float, target_ms: float) -> str:
    """Calculate speedup ratio"""
    if target_ms == 0 or baseline_ms == 0:
        return "N/A"

    speedup = baseline_ms / target_ms
    return f"{speedup:.1f}x"


def generate_comparison_report(
    aria_file: Path,
    python_file: Path,
    go_file: Path,
    rust_file: Path,
    output_file: Path
):
    """Generate markdown comparison report"""

    # Parse all results
    print("Parsing benchmark results...")
    aria_metrics = parse_aria_results(aria_file)
    python_metrics = parse_python_results(python_file)
    go_metrics = parse_go_results(go_file)
    rust_metrics = parse_rust_results(rust_file)

    all_metrics = aria_metrics + python_metrics + go_metrics + rust_metrics
    grouped = group_metrics(all_metrics)

    print(f"Found {len(all_metrics)} total measurements across {len(grouped)} operations")

    # Generate report
    with open(output_file, 'w') as f:
        f.write("# BioFlow Cross-Language Benchmark Comparison\n\n")
        f.write(f"**Generated:** {Path(output_file).stat().st_mtime}\n\n")
        f.write("---\n\n")

        f.write("## Executive Summary\n\n")

        # Count available implementations
        langs_tested = set()
        for metrics_dict in grouped.values():
            langs_tested.update(metrics_dict.keys())

        f.write(f"Languages tested: {', '.join(sorted(langs_tested))}\n\n")

        f.write("---\n\n")
        f.write("## Detailed Results\n\n")

        # GC Content comparison
        f.write("### GC Content Calculation\n\n")
        f.write("| Input Size | Aria | Go | Rust | Python | Aria vs Python |\n")
        f.write("|------------|------|-----|------|--------|----------------|\n")

        for key, metrics_dict in sorted(grouped.items()):
            if 'GC Content' in key:
                aria_time = metrics_dict.get('Aria')
                go_time = metrics_dict.get('Go')
                rust_time = metrics_dict.get('Rust')
                python_time = metrics_dict.get('Python')

                size = aria_time.input_size if aria_time else (python_time.input_size if python_time else "?")

                aria_str = format_time(aria_time.time_ms) if aria_time else "N/A"
                go_str = format_time(go_time.time_ms) if go_time else "N/A"
                rust_str = format_time(rust_time.time_ms) if rust_time else "N/A"
                python_str = format_time(python_time.time_ms) if python_time else "N/A"

                speedup = "N/A"
                if aria_time and python_time:
                    speedup = calculate_speedup(python_time.time_ms, aria_time.time_ms)

                f.write(f"| {size} bp | {aria_str} | {go_str} | {rust_str} | {python_str} | {speedup} |\n")

        f.write("\n")

        # K-mer comparison
        f.write("### K-mer Counting\n\n")
        f.write("| Operation | Aria | Go | Rust | Python | Aria vs Python |\n")
        f.write("|-----------|------|-----|------|--------|----------------|\n")

        for key, metrics_dict in sorted(grouped.items()):
            if 'K-mer' in key or 'Kmer' in key:
                aria_time = metrics_dict.get('Aria')
                go_time = metrics_dict.get('Go')
                rust_time = metrics_dict.get('Rust')
                python_time = metrics_dict.get('Python')

                aria_str = format_time(aria_time.time_ms) if aria_time else "N/A"
                go_str = format_time(go_time.time_ms) if go_time else "N/A"
                rust_str = format_time(rust_time.time_ms) if rust_time else "N/A"
                python_str = format_time(python_time.time_ms) if python_time else "N/A"

                speedup = "N/A"
                if aria_time and python_time:
                    speedup = calculate_speedup(python_time.time_ms, aria_time.time_ms)

                f.write(f"| {key} | {aria_str} | {go_str} | {rust_str} | {python_str} | {speedup} |\n")

        f.write("\n")

        # Alignment comparison
        f.write("### Sequence Alignment\n\n")
        f.write("| Algorithm | Aria | Go | Rust | Python | Aria vs Python |\n")
        f.write("|-----------|------|-----|------|--------|----------------|\n")

        for key, metrics_dict in sorted(grouped.items()):
            if 'Smith' in key or 'Needleman' in key or 'Alignment' in key:
                aria_time = metrics_dict.get('Aria')
                go_time = metrics_dict.get('Go')
                rust_time = metrics_dict.get('Rust')
                python_time = metrics_dict.get('Python')

                aria_str = format_time(aria_time.time_ms) if aria_time else "N/A"
                go_str = format_time(go_time.time_ms) if go_time else "N/A"
                rust_str = format_time(rust_time.time_ms) if rust_time else "N/A"
                python_str = format_time(python_time.time_ms) if python_time else "N/A"

                speedup = "N/A"
                if aria_time and python_time:
                    speedup = calculate_speedup(python_time.time_ms, aria_time.time_ms)

                f.write(f"| {key} | {aria_str} | {go_str} | {rust_str} | {python_str} | {speedup} |\n")

        f.write("\n")

        # Summary statistics
        f.write("---\n\n")
        f.write("## Performance Summary\n\n")

        # Calculate average speedups
        aria_vs_python_speedups = []
        for metrics_dict in grouped.values():
            aria_time = metrics_dict.get('Aria')
            python_time = metrics_dict.get('Python')

            if aria_time and python_time and python_time.time_ms > 0:
                speedup = python_time.time_ms / aria_time.time_ms
                aria_vs_python_speedups.append(speedup)

        if aria_vs_python_speedups:
            avg_speedup = sum(aria_vs_python_speedups) / len(aria_vs_python_speedups)
            f.write(f"**Average Aria vs Python speedup:** {avg_speedup:.1f}x\n\n")
        else:
            f.write("**Aria benchmarks not yet available** (compiler in development)\n\n")

        f.write("### Language Characteristics\n\n")
        f.write("| Language | Type | GC | Strengths |\n")
        f.write("|----------|------|-----|----------|\n")
        f.write("| **Aria** | Compiled | No | Contracts, Safety, Performance |\n")
        f.write("| **Rust** | Compiled | No | Maximum Performance, Zero-cost Abstractions |\n")
        f.write("| **Go** | Compiled | Yes | Fast Compilation, Simplicity |\n")
        f.write("| **Python** | Interpreted | Yes | Rapid Development, Rich Ecosystem |\n")
        f.write("\n")

    print(f"\n✓ Comparison report generated: {output_file}")


def main():
    parser = argparse.ArgumentParser(description='Compare BioFlow benchmark results')
    parser.add_argument('--aria', type=Path, required=True, help='Aria results file')
    parser.add_argument('--python', type=Path, required=True, help='Python results file')
    parser.add_argument('--go', type=Path, required=True, help='Go results file')
    parser.add_argument('--rust', type=Path, required=True, help='Rust results file')
    parser.add_argument('--output', type=Path, required=True, help='Output markdown file')

    args = parser.parse_args()

    generate_comparison_report(
        args.aria,
        args.python,
        args.go,
        args.rust,
        args.output
    )

    return 0


if __name__ == '__main__':
    sys.exit(main())
