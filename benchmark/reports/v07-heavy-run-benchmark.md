# v0.7 Heavy-Run And HTSlib Optimization Benchmark

## Status

`public-heavy` 10k, 100k, and 1M benchmarks completed locally with host `bcftools 1.23.1` and `hyperfine 1.20.0`.

Command:

```bash
VCF_FAST_BENCH_MODE=public-heavy \
VCF_FAST_BENCH_SIZES="10000" \
VCF_FAST_BENCH_RUNS=3 \
VCF_FAST_BENCH_WARMUP=1 \
VCF_FAST_HEAVY_MAX_PLAIN_BYTES=200000000 \
VCF_FAST_BENCH_REPORT="tests/output/benchmark-results/v07-public-heavy-10k-benchmark.md" \
make bench-smoke
```

This report is still not a broad v0.7 performance claim. It proves the measured IGSR chr22 public-heavy tiers listed below, with correctness matched against `bcftools` for supported filter core records and normalized TSV rows.

## Measured 10k Public-Heavy Results

- Dataset: 1000 Genomes high-coverage chr22, region `chr22:1-20000000`
- Staging cap: `200000000` bytes
- Generated BGZF subset size: `3382499` bytes
- Correctness: VCF-Fast matched `bcftools` for supported filter core records and normalized TSV rows.

| case | competitor | correctness result | vcf-fast mean | competitor mean | speedup | caveat |
|---|---|---|---:|---:|---:|---|
| Heavy QUAL gzip input | `bcftools filter` | matched filtered core records | `0.089076s` | `0.285341s` | `3.20x` faster | 10k bounded subset only |
| Heavy Convert TSV gzip input | `bcftools query -u` | matched normalized TSV rows | `0.107405s` | `0.065505s` | `0.61x` | `bcftools query` faster; TSV path remains an optimization target |

## Optimization Follow-Up

The TSV bottleneck was narrowed with the same 10k public-heavy input:

- Plain uncompressed TSV conversion: VCF-Fast `0.0247s` vs `bcftools query` `0.0506s`, `2.05x` faster. This showed the selective TSV parser was not the remaining bottleneck.
- Gzip/BGZF TSV conversion before the final fix: VCF-Fast improved from `0.107405s` to `0.0858s` after avoiding full sample-tail line materialization, but still trailed `bcftools query` at `0.0636s`.
- Final measured optimization: use streaming field reads that stop materializing records after INFO, skip unused FORMAT/sample tails with delimiter scans, and build flate2 against `zlib-ng`.

Command:

```bash
VCF_FAST_BENCH_MODE=public-heavy \
VCF_FAST_BENCH_SIZES="10000" \
VCF_FAST_BENCH_RUNS=3 \
VCF_FAST_BENCH_WARMUP=1 \
VCF_FAST_HEAVY_MAX_PLAIN_BYTES=200000000 \
VCF_FAST_BENCH_REPORT="tests/output/benchmark-results/v07-public-heavy-10k-after-zlib-ng-streaming-tsv.md" \
make bench-smoke
```

| case | competitor | correctness result | vcf-fast mean | competitor mean | speedup | bottleneck finding |
|---|---|---|---:|---:|---:|---|
| Heavy QUAL gzip input, optimized | `bcftools filter` | matched filtered core records | `0.0541s` | `0.2830s` | `5.23x` faster | gzip decompression backend was a major cost |
| Heavy Convert TSV gzip input, optimized | `bcftools query -u` | matched normalized TSV rows | `0.0535s` | `0.0646s` | `1.21x` faster | wide-line materialization plus gzip backend were the gap |

An earlier 10k attempt with `VCF_FAST_HEAVY_MAX_PLAIN_BYTES=20000000` deferred correctly because the plain staging file would have been `139093224` bytes. That confirms the artifact cap prevents accidental giant intermediates.

## Completed 100k/1M Public-Heavy Results

The heavy builder now streams public records directly into BGZF output and indexes the result, avoiding the old full plain staging artifact.

Command:

```bash
VCF_FAST_BENCH_SIZES="100000 1000000" \
VCF_FAST_BENCH_RUNS=3 \
VCF_FAST_BENCH_WARMUP=1 \
VCF_FAST_BENCH_REPORT="tests/output/benchmark-results/v07-public-heavy-balanced.md" \
make bench-heavy
```

| case | record count | dataset size bytes | competitor | correctness result | vcf-fast mean | competitor mean | speedup | variants/sec | caveat |
|---|---:|---:|---|---|---:|---:|---:|---:|---|
| Heavy QUAL gzip input | 100,000 | 34,792,224 | `bcftools filter` | matched filtered core records | `0.492310s` | `2.781363s` | `5.65x` faster | `203124 / 35954` | bounded chr22 region |
| Heavy Convert TSV gzip input | 100,000 | 34,792,224 | `bcftools query -u` | matched normalized TSV rows | `0.514039s` | `0.566454s` | `1.10x` faster | `194538 / 176537` | bounded chr22 region |
| Heavy QUAL gzip input | 1,000,000 | 75,203,311 | `bcftools filter` | matched filtered core records | `1.000930s` | `5.233588s` | `5.23x` faster | `999071 / 191074` | bounded chr22 region |
| Heavy Convert TSV gzip input | 1,000,000 | 75,203,311 | `bcftools query -u` | matched normalized TSV rows | `0.961222s` | `1.038689s` | `1.08x` faster | `1040342 / 962752` | bounded chr22 region |

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
| public-heavy | large public evidence | 100k/1M completed without giant plain IGSR intermediates |
