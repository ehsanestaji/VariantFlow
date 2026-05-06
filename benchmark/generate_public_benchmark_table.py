#!/usr/bin/env python3
"""Generate the polished public benchmark summary from tracked reports."""

from __future__ import annotations

import argparse
import difflib
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs" / "public-benchmark-table.md"


@dataclass(frozen=True)
class Row:
    workflow: str
    dataset: str
    correctness: str
    measured_result: str
    source: str
    required_tokens: tuple[str, ...]
    caveat: str


ROWS = [
    Row(
        workflow="Native selective filter",
        dataset="Synthetic 1M VCF",
        correctness="matched filtered core records",
        measured_result="1.62x to 1.82x faster than bcftools filter",
        source="benchmark/reports/synthetic-filter-benchmark.md",
        required_tokens=("1.62x", "1.82x", "bcftools filter"),
        caveat="deterministic synthetic workload",
    ),
    Row(
        workflow="Public QUAL filter",
        dataset="GIAB HG002 public-whole tiers",
        correctness="matched filtered core records",
        measured_result="1.80x to 2.38x faster on plain tiers; 1.89x faster at 1M gzip",
        source="benchmark/reports/public-whole-cohort-benchmark.md",
        required_tokens=("1.80x", "2.38x", "1.89x"),
        caveat="100k gzip was mixed at 0.94x",
    ),
    Row(
        workflow="Public QUAL filter",
        dataset="IGSR chr22 public-whole 10k/100k",
        correctness="matched filtered core records",
        measured_result="4.85x to 5.71x faster than bcftools filter",
        source="benchmark/reports/public-whole-cohort-benchmark.md",
        required_tokens=("4.85x", "5.71x"),
        caveat="1M whole-file tier deferred after a large plain intermediate",
    ),
    Row(
        workflow="Default auto BGZF input",
        dataset="IGSR chr22 public-heavy BGZF 100k and bounded 191526-record tier",
        correctness="native modes matched byte-for-byte; core records matched bcftools",
        measured_result="2.26x to 2.39x faster than forced single-thread native; 13.44x to 13.47x faster than bcftools filter",
        source="benchmark/reports/v14-public-parallel-scale-benchmark.md",
        required_tokens=("2.26x", "2.39x", "13.44x", "13.47x"),
        caveat="requested 1M public tier reached 191526 available records",
    ),
    Row(
        workflow="Parallel FORMAT aggregate filter",
        dataset="Stress 100k/1M ANY(FORMAT/AD > 80)",
        correctness="parallel output matched default byte-for-byte and bcftools core records",
        measured_result="1.77x to 2.01x faster than default native; 4.33x to 5.27x faster than bcftools filter",
        source="benchmark/reports/v14-public-parallel-scale-benchmark.md",
        required_tokens=("1.77x", "2.01x", "4.33x", "5.27x"),
        caveat="synthetic CPU-heavy stress shape; public FORMAT WGS trio evidence is positive but only three samples",
    ),
    Row(
        workflow="Public FORMAT aggregate filter",
        dataset="Zenodo WGS trio 10k/50k/100k/250k N_PASS(FORMAT/AD[1] > 10)",
        correctness="matched bcftools core records",
        measured_result="3.01x to 5.89x faster than bcftools filter",
        source="benchmark/reports/v17-public-format-baselines.md",
        required_tokens=("3.01x", "5.89x", "250000 requested / 250000 actual", "matched core records"),
        caveat="repeated macOS timing; FORMAT-rich WGS trio has three samples, larger cohort rows pending",
    ),
    Row(
        workflow="Native TSV conversion",
        dataset="Stress 1M after byte-core surgery",
        correctness="matched normalized TSV rows",
        measured_result="2.54x faster than bcftools query",
        source="benchmark/reports/v08-core-efficiency-benchmark.md",
        required_tokens=("2.54x", "bcftools query"),
        caveat="selected columns only",
    ),
    Row(
        workflow="Stats simple counts",
        dataset="Stress 1M after byte-core surgery",
        correctness="matched supported stats records",
        measured_result="2.50x faster than bcftools stats",
        source="benchmark/reports/v08-core-efficiency-benchmark.md",
        required_tokens=("2.50x", "bcftools stats"),
        caveat="rich bcftools stats parity is not claimed",
    ),
    Row(
        workflow="Parquet export",
        dataset="Deterministic stress projection",
        correctness="Arrow readback and row-count checks passed",
        measured_result="1.93x to 1.94x faster than bcftools query projection",
        source="benchmark/reports/v10-parquet-export-benchmark.md",
        required_tokens=("1.93x", "1.94x"),
        caveat="native TSV is still faster; selected-column export only",
    ),
    Row(
        workflow="DuckDB repeated-query workflow",
        dataset="Bounded IGSR chr22 public-heavy BGZF",
        correctness="DuckDB predicate/grouped results matched normalized bcftools baselines",
        measured_result="3.18x to 25.67x faster amortized; 29.27x to 497.74x query-only",
        source="benchmark/reports/v12-public-parallel-workflow-benchmark.md",
        required_tokens=("3.18x", "25.67x", "29.27x", "497.74x"),
        caveat="native selected-column Parquet only; Polars/PyArrow pending",
    ),
    Row(
        workflow="Compatibility interop",
        dataset="BCF, indexed region, and BGZF output fixtures",
        correctness="BCF/region/BGZF correctness and indexability matched",
        measured_result="near parity or faster on several v0.7 compatibility rows; BCF TSV remains slower",
        source="benchmark/reports/compatibility-benchmark.md",
        required_tokens=("BCF input", "BGZF output", "tabix -p vcf"),
        caveat="htslib-backed TSV remains a tracked optimization gap",
    ),
]


def read_source(path: str) -> str:
    source_path = ROOT / path
    if not source_path.exists():
        raise FileNotFoundError(f"missing source report: {path}")
    return source_path.read_text(encoding="utf-8")


def validate_sources() -> None:
    for row in ROWS:
        text = read_source(row.source)
        missing = [token for token in row.required_tokens if token not in text]
        if missing:
            formatted = ", ".join(repr(token) for token in missing)
            raise SystemExit(f"{row.source} is missing required evidence token(s): {formatted}")


def render() -> str:
    validate_sources()
    lines = [
        "# VCF-Fast Public Benchmark Table",
        "",
        "This table is generated by `benchmark/generate_public_benchmark_table.py` from tracked benchmark reports. It is a release-facing summary, not a substitute for the source reports.",
        "",
        "| Workflow | Dataset | Correctness | Measured Result | Source Report | Caveat |",
        "|---|---|---|---|---|---|",
    ]
    for row in ROWS:
        lines.append(
            f"| {row.workflow} | {row.dataset} | {row.correctness} | {row.measured_result} | `{row.source}` | {row.caveat} |"
        )
    lines.extend(
        [
            "",
            "Claim rule: VCF-Fast only says it beats, matches, or complements another tool when the linked report contains a correctness check and measured runtime/RSS/throughput fields for that workflow.",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail if the checked-in table is stale")
    args = parser.parse_args()

    generated = render()
    if args.check:
        current = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else ""
        if current != generated:
            diff = "\n".join(
                difflib.unified_diff(
                    current.splitlines(),
                    generated.splitlines(),
                    fromfile=str(OUTPUT),
                    tofile="generated",
                    lineterm="",
                )
            )
            raise SystemExit(f"{OUTPUT} is stale; run make benchmark-table\n{diff}")
        return 0

    OUTPUT.write_text(generated, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
