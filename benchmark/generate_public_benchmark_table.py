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
        workflow="Query-aware indexed filter",
        dataset="Synthetic stress BGZF skip-heavy QUAL predicate",
        correctness="default and indexed outputs matched byte-for-byte; indexed core records matched bcftools",
        measured_result="1M skip-heavy row was 6.06x faster than default native and 55.80x faster than bcftools",
        source="benchmark/reports/v21-indexed-filter-benchmark.md",
        required_tokens=("6.06x vs default", "55.80x vs bcftools", "100.0%", "1000000"),
        caveat="synthetic skip-heavy workload; not a broad public-data claim",
    ),
    Row(
        workflow="Public query-aware indexed filter",
        dataset="Bounded IGSR chr22 BGZF AF predicate",
        correctness="default and indexed outputs matched byte-for-byte; indexed core records matched bcftools",
        measured_result="guarded `.vfi` planner fell back to default native on low-skip rows and measured 10.01x to 13.31x faster than bcftools",
        source="benchmark/reports/v21-public-indexed-filter-benchmark.md",
        required_tokens=(
            "AF > 0.99",
            "1000000",
            "default and indexed byte-for-byte match",
            "fell back: VFI skip estimate",
            "10.01x vs bcftools",
            "13.31x vs bcftools",
            "VFI planner fell back to default native",
            "indexed 8464 KB",
        ),
        caveat="public AF predicate skipped only 0.0% to 50.0% of chunks; raw index acceleration still needs high-skip predicates or deeper BGZF scheduling",
    ),
    Row(
        workflow="Parallel FORMAT aggregate filter",
        dataset="Stress 100k/1M ANY(FORMAT/AD > 80)",
        correctness="parallel output matched default byte-for-byte and bcftools core records",
        measured_result="1.77x to 2.01x faster than default native; 4.33x to 5.27x faster than bcftools filter",
        source="benchmark/reports/v14-public-parallel-scale-benchmark.md",
        required_tokens=("1.77x", "2.01x", "4.33x", "5.27x"),
        caveat="synthetic CPU-heavy stress shape; public 453-sample FORMAT cohort evidence is tracked separately",
    ),
    Row(
        workflow="Public FORMAT aggregate filter",
        dataset="ENA Ovis aries 453-sample cohort through full chromosome N_PASS(FORMAT/AD[1] > 10)",
        correctness="matched bcftools core records",
        measured_result="1.76x to 3.50x faster than bcftools filter",
        source="benchmark/reports/v17-public-format-baselines.md",
        required_tokens=("1.76x", "3.50x", "samples=453", "full requested / 1097167 actual", "matched core records"),
        caveat="Docker/Linux timing; 1M/full tiers use heavy-output mode with core-record correctness and /dev/null timed output",
    ),
    Row(
        workflow="Public FORMAT expression breadth",
        dataset="ENA Ovis aries 453-sample cohort 1M/full chromosome FORMAT DP/GQ/AD expressions",
        correctness="matched bcftools core records",
        measured_result="3.22x to 8.77x faster than bcftools filter",
        source="benchmark/reports/v18-public-format-expression-breadth.md",
        required_tokens=(
            "3.22x",
            "8.77x",
            "ANY(FORMAT/DP > 20)",
            "ALL(FORMAT/GQ >= 30)",
            "selected-sample FORMAT/DP > 20",
            "full requested / 1097167 actual",
            "matched core records",
        ),
        caveat="Docker/Linux repeated timing; hyperfine reported outliers on some rows; heavy-output mode avoids retained full VCF artifacts",
    ),
    Row(
        workflow="Second public FORMAT-rich cohort",
        dataset="ENA Dutch Genebank Cattle 29-sample full Y-chromosome VCF",
        correctness="matched bcftools core records",
        measured_result="1.46x to 26.66x faster than bcftools filter across DP/GQ/AD/selected-sample/mixed FORMAT expressions",
        source="benchmark/reports/v19-second-public-format-cohort.md",
        required_tokens=(
            "ERZ18456468",
            "5488549 actual",
            "1.46x",
            "26.66x",
            "ANY(FORMAT/DP > 20)",
            "ALL(FORMAT/GQ >= 30)",
            "N_PASS(FORMAT/AD[1] > 10) >= 2",
            "selected-sample FORMAT/DP > 20",
            "QUAL > 30 && ANY(FORMAT/DP > 20)",
            "matched core records",
        ),
        caveat="Second cohort is non-sheep but not human/plant; Mayo human 629-sample VCF-Miner downloads returned 403 during automated validation",
    ),
    Row(
        workflow="Human FORMAT-rich cohort",
        dataset="DDBJ CHM13 chr22 3715-sample human VCF, bounded 1k/10k/50k tiers",
        correctness="matched bcftools core records",
        measured_result="4.74x to 17.78x faster than bcftools filter across DP/GQ/AD/selected-sample/mixed FORMAT expressions",
        source="benchmark/reports/v20-human-format-cohort.md",
        required_tokens=(
            "DDBJ CHM13 public-human-genomes",
            "3715-sample",
            "1000 requested / 1000 actual",
            "10000 requested / 10000 actual",
            "50000 requested / 50000 actual",
            "4.74x",
            "17.78x",
            "ANY(FORMAT/DP > 20)",
            "ALL(FORMAT/GQ >= 30)",
            "N_PASS(FORMAT/AD[1] > 10) >= 10",
            "selected-sample FORMAT/DP > 20",
            "QUAL > 30 && ANY(FORMAT/DP > 20)",
            "matched core records",
        ),
        caveat="bounded streaming tiers only; full 27 GB remote VCF is not cached by default",
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
    Row(
        workflow="VCFtools-style population summaries",
        dataset="Staged bounded DDBJ CHM13 3715-sample human biallelic cohort",
        correctness="normalized VCFtools parity passed for supported diploid biallelic rows",
        measured_result="1.50x to 8.26x faster than VCFtools across frequency, missingness, HWE, heterozygosity, site/window pi, Tajima's D, LD, and Weir-Cockerham Fst",
        source="benchmark/reports/vcftools-popgen-parity-benchmark.md",
        required_tokens=(
            "public cohort",
            "3715",
            "682",
            "3.23x",
            "8.26x",
            "1.50x",
            "Weir-Cockerham Fst",
            "passed: make vcftools-parity",
            "This report does not support a broad VCFtools replacement claim",
        ),
        caveat="three measured runs; requested 1k/10k/50k tiers all resolved to 682 actual staged records from this cached source; HWE exact p-value is outside current output; auto-derived Fst population files are benchmark-only",
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
