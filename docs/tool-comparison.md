# Choosing the right tool

VariantFlow does not aim to replace the established VCF ecosystem — it accelerates
a well-defined subset of it and hands off cleanly to the rest. This page places
VariantFlow next to the tools it is most often compared with, so you can pick the
right one for the task in front of you.

## At a glance

| Tool | Primary role | Data model | Best for | Relationship to VariantFlow |
|------|--------------|-----------|----------|-----------------------------|
| **VariantFlow** | Fast field-selective filtering + streamable site/window statistics | Streaming VCF/BCF, selective parsing; Parquet export | Interactive turnaround on large VCFs on commodity CPUs; VCFtools-compatible stats; export-once hand-off | — |
| **bcftools filter** | Reference for general filtering, normalization, format conversion | Streaming VCF/BCF | The full generality of VCF surgery: norm, annotate, merge, view | VariantFlow accelerates a *subset* via selective parsing; bcftools remains the compatibility reference |
| **VCFtools** | Classic population-genetics file operations | Streaming VCF | Established, widely cited pop-gen file stats | VariantFlow reproduces supported stats faster and treats VCFtools as the correctness baseline |
| **DuckDB** (on Parquet) | Ad hoc analytical SQL | Columnar / OLAP | Interactive SQL, joins, aggregations over exported variant tables | VariantFlow's *export-once* Parquet output feeds DuckDB — complementary, not competing |
| **scikit-allel** | Matrix / haplotype pop-gen library (Python) | Full genotype/haplotype arrays in memory | Selection scans, PCA, D/f-statistics, haplotype stats | VariantFlow defers matrix-wide statistics to it by design |

## How they relate

**bcftools filter** is the reference implementation for general-purpose VCF/BCF
work — filtering, normalization (`norm`), annotation, merging, and format
conversion — over a streaming data model. VariantFlow accelerates a *subset* of
these operations through selective parsing (reading only the fields a query
touches), but bcftools stays the compatibility reference for anything outside that
subset.

**VCFtools** is the classic toolkit for population-genetics file operations on
streaming VCF. VariantFlow reproduces its supported statistics — allele
frequencies, missingness, heterozygosity, Hardy–Weinberg, π, Tajima's *D*, F~ST~,
LD — faster and in the same output layout, and treats VCFtools as the **correctness
baseline** those outputs are validated against.

**DuckDB** is an in-process analytical (OLAP) SQL engine that reads Parquet
directly. It is columnar, so it excels at ad hoc queries, joins, and aggregations
over tabular variant data. VariantFlow's `convert --to parquet` produces exactly
that input: **export once**, then run as many DuckDB queries as you like. The two
are complementary — VariantFlow does the streaming extraction, DuckDB does the
interactive SQL.

**scikit-allel** is a Python library built around **in-memory genotype and
haplotype matrices**. That model is what enables the statistics VariantFlow does
*not* implement — selection scans (EHH, iHS, XP-EHH, nSL), PCA/PCoA, D/f-statistics
(ABBA-BABA), and haplotype diversity — all of which need the whole matrix at once.
VariantFlow **defers these to scikit-allel by design**; see
[Out of scope](statistics.md#out-of-scope).

## Practical rule

> Reach for **VariantFlow** when you want **interactive turnaround on large VCFs on
> commodity CPUs** — fast filtering, streamable site/window statistics, and a
> one-time Parquet export. Reach for the **established tools** when you need their
> **full generality**: bcftools for arbitrary VCF surgery, VCFtools as the
> reference baseline, DuckDB for ad hoc SQL over the exported table, and
> scikit-allel for matrix- and haplotype-wide statistics.
