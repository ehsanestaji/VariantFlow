# v0.7 Heavy-Run And HTSlib Optimization Benchmark

## Status

Bounded `public-heavy` 10k benchmark completed locally after installing host `bcftools 1.23.1` and `hyperfine 1.20.0`.

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

This is a bounded 10k evidence run, not a broad v0.7 performance claim. Larger 100k/1M heavy tiers still need to be run and published.

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
