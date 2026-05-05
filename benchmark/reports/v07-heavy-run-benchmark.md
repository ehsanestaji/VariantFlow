# v0.7 Heavy-Run And HTSlib Optimization Benchmark

## Status

Tiny `public-heavy` smoke was attempted with `VCF_FAST_BENCH_MODE=public-heavy VCF_FAST_BENCH_SIZES="100" VCF_FAST_BENCH_RUNS=1 VCF_FAST_BENCH_WARMUP=0 make bench-smoke`. Cached public data exists in this environment, but host `bcftools` is unavailable on `PATH`; the command failed clearly with `bcftools is required for VCF_FAST_BENCH_MODE=public-heavy`. No v0.7 performance claim is made.

## Required Report Fields

Each generated report must include report-level dataset source, dataset shape, and competitor version metadata. Each measured row must include correctness result, runtime mean, runtime stddev, speedup, variants/sec, peak RSS, exact VCF-Fast command, exact competitor command, bottleneck, caveat, and next action.

## Path Classes

| path class | current intent | caveat |
|---|---|---|
| native-filter | keep as the winning core | only claim wins from measured correctness-matched rows |
| native-tsv | measure selected-column export | columnar export is later |
| native-stats | compare overlapping stats before richer parity | stats parity is intentionally scoped |
| htslib-region-filter | compatibility region filter | not byte-preserving |
| htslib-region-tsv | indexed TSV compatibility | known v0.6 lag path |
| htslib-region-stats | indexed stats compatibility | only overlapping stats parity claimed |
| bcf-filter | BCF compatibility | v0.6 correctness matched but speed lagged |
| bcf-tsv | BCF TSV compatibility | preserve normalized bcftools query rows |
| bgzf-output | indexable BGZF output | measure compression/write overhead |
| public-heavy | large public evidence | avoid giant plain IGSR intermediates |
