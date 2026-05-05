---
title: "VariantFlow: a selective Rust execution engine for evidence-tracked VCF filtering and analytical export"
tags:
  - bioinformatics
  - genomics
  - VCF
  - Rust
  - benchmarking
authors:
  - name: "Ehsan"
    affiliation: 1
affiliations:
  - name: "Umeå Plant Science Center"
    index: 1
date: 2026-05-06
bibliography: paper.bib
---

# Summary

VariantFlow is a command-line engine for post-calling variant workflows over VCF,
BGZF-compressed VCF, and selected BCF-compatible paths. It focuses on a common
pattern in variant analysis: users often want to filter, summarize, compare, or
export a small subset of fields from large variant files. Instead of eagerly
reconstructing full records for every operation, VariantFlow uses a Rust-native
streaming core that parses only fields required by the current predicate or
projection, preserves original VCF records where possible, and records
correctness and performance evidence against trusted ecosystem tools such as
`bcftools` [@bcftools; @htslib].

# Statement of need

VCF remains the dominant exchange format for small variant calls, but production
analysis often repeatedly scans large VCF, BGZF, or BCF files for selective
tasks: quality filters, INFO/FORMAT predicates, cohort summaries, variant set
comparisons, and tabular exports for downstream analysis [@vcf_spec]. Mature
tools such as `bcftools`, HTSlib, GATK, and VCFtools provide broad compatibility
and deep functionality [@bcftools; @htslib; @gatk; @vcftools]. VariantFlow is
designed to complement that ecosystem by optimizing a narrower but frequent
class of workflows: selective streaming operations where not all INFO, FORMAT,
and sample fields are needed.

The project is evidence-gated. Each public performance statement is tied to a
tracked benchmark report that includes a correctness comparison, exact commands,
competitor version, runtime, throughput, and caveats. This is important because
variant tooling is sensitive to missing-value semantics, compression behavior,
region access, and output equivalence. VariantFlow therefore avoids broad
"best tool" claims and instead reports where it beats, matches, or complements
existing tools for measured workflows.

# State of the field

`bcftools` and HTSlib are the primary compatibility baseline for VCF/BCF
streaming, BGZF, tabix/CSI indexing, filtering, querying, and statistics
[@bcftools; @htslib]. VCFtools remains an older but influential toolkit for
variant filtering and summary workflows [@vcftools]. GATK provides heavier Java
pipelines for variant selection and filtration in clinical and research
contexts [@gatk]. Columnar systems such as Apache Arrow, Parquet, and DuckDB
offer an analytical path for repeated queries after data have been exported
from exchange formats [@apache_arrow; @parquet; @duckdb]. VariantFlow sits
between these worlds: it keeps a native line-preserving streaming path for
one-pass variant operations and adds selected Parquet export for repeated
analysis workflows.

# Software design

VariantFlow is implemented primarily in Rust. The native engine represents VCF
records with borrowed byte-slice views, performs typed expression evaluation
over required fields, and writes passing records back in input order without
reconstructing record text when the native path is used. This design reduces
per-record allocation and avoids parsing unused INFO, FORMAT, and sample fields.
Native BGZF input workers accelerate compressed reads for BGZF `.vcf.gz` input,
while ordered batch evaluation can be enabled for CPU-heavy FORMAT aggregate
predicates.

Compatibility paths remain explicit and feature-gated. With HTSlib support,
VariantFlow can handle BCF input, indexed region reads, and BGZF output where
ecosystem compatibility matters more than byte-for-byte record preservation.
The `convert` command exports stable TSV columns and a first Parquet projection
with typed POS, nullable QUAL, nullable INFO/DP, and lossless string INFO/AF.
The `diff` command compares canonical variant keys, and `stats` reports
site-level and allele-level summaries for supported metrics.

# Research impact statement

VariantFlow's current evidence supports selective workflow claims rather than a
universal replacement claim. In the v1.4 public parallel scale report,
auto-threaded native BGZF input on bounded IGSR chr22 QUAL filters matched
`bcftools` core records and measured `13.44x to 13.47x` faster than
`bcftools filter`, while also measuring `2.26x to 2.39x` faster than forced
single-thread native input
(`benchmark/reports/v14-public-parallel-scale-benchmark.md`). The same report
shows CPU-heavy deterministic stress filters for `ANY(FORMAT/AD > 80)` matched
`bcftools` core records and measured `1.77x to 2.01x` faster than default
native execution and `4.33x to 5.27x` faster than `bcftools filter`.

For repeated analytical workflows, the v1.2 public parallel workflow report
shows VariantFlow Parquet export plus DuckDB repeated queries matched normalized
`bcftools` baselines for QUAL, INFO/DP, FILTER, and grouped CHROM/FILTER
queries, with amortized speedups of `3.18x to 25.67x`
(`benchmark/reports/v12-public-parallel-workflow-benchmark.md`). These results
suggest a practical split: streaming filters remain efficient for one-pass
selection, while Parquet becomes useful when the same variant subset is queried
many times. Remaining caveats are tracked openly: BCF/region TSV compatibility
paths still trail in some cases, public FORMAT-heavy evidence is pending, RSS
measurements are incomplete on some macOS benchmark rows, and this is explicitly
not a claim that VariantFlow replaces bcftools or GATK outside supported
workflows.

# AI usage disclosure

Codex and related OpenAI systems were used during development and manuscript
preparation for planning, code review assistance, refactoring support, test
scaffolding, benchmark-report organization, and prose drafting. The human
project owner is responsible for final design decisions, source-code review,
benchmark validation, authorship, licensing, and all claims in this manuscript.

# Acknowledgements

The project builds on the open genomics software ecosystem, especially the VCF
specification, HTSlib, `bcftools`, Bioconda, Apache Arrow, Parquet, and DuckDB.
Funding, institutional acknowledgements, and contributor acknowledgements should
be finalized before submission.

# References
