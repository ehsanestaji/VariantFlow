# VCF-Fast

VCF-Fast is a selective execution engine for post-calling variant operations. It treats VCF/BCF as exchange formats, avoids parsing unused fields, preserves original records where possible, and tracks correctness/performance against trusted tools such as bcftools.

The current boost strategy is **Evidence First**: prove correctness and speed on reproducible synthetic and public datasets before claiming broader superiority or moving deeper into parallel, vectorized, or columnar internals.

## Why It Can Be Faster

- Selective parsing avoids unused INFO/FORMAT work.
- Original-record preservation avoids reconstruction cost for passing VCF records.
- Typed predicates avoid ad hoc string evaluation.
- The benchmark harness checks outputs against `bcftools`.
- Future gains should come from larger public evidence, stress datasets, parallel/vectorized execution, and Arrow/Parquet export.

## Language And Engine Direction

VCF-Fast stays Rust-first. Rust gives the project C-like performance, strict memory control, and safer concurrency without making memory safety a permanent tax on development speed. C/htslib interop is reserved for targeted compatibility work such as BGZF output, BCF input, tabix-indexed reads, and cases where htslib is clearly the fastest correct path.

## Current Evidence

| scenario | competitor | correctness check | current result | caveat |
|---|---|---|---|---|
| Synthetic 1M filters | `bcftools filter` | matched filtered core records | `1.62x` to `1.82x` faster | three-run container benchmark |
| Synthetic 1M TSV conversion | `bcftools query` | matched normalized TSV rows | `1.57x` faster | selected columns only |
| GIAB HG002 10k public QUAL filters | `bcftools filter` | matched filtered core records | `2.08x` to `2.11x` faster | first public smoke only |
| GIAB HG002 10k public TSV conversion | `bcftools query` | matched normalized TSV rows | `1.12x` faster | GIAB lacks `INFO/AF`; baseline uses `bcftools query -u` |
| IGSR chr22 100k public-region QUAL filters | `bcftools filter` | matched filtered core records | `5.35x` to `8.33x` faster | region subset, not whole cohort |
| IGSR chr22 100k public-region TSV conversion | `bcftools query` | matched normalized TSV rows | `1.11x` faster | selected columns only |
| Stress 1M filters with unused INFO/FORMAT/sample payload | `bcftools filter` | matched filtered core records | `1.96x` to `2.45x` faster on plain VCF | synthetic stress shape |
| Stress 1M TSV conversion | `bcftools query` | matched normalized TSV rows | `1.20x` faster | selected columns only |
| Stress 1M stats | `bcftools stats` | matched overlapping record count | `1.53x` faster | richer stats equivalence pending |

Detailed evidence lives in:

- `benchmark/reports/synthetic-filter-benchmark.md`
- `benchmark/reports/public-dataset-benchmark.md`
- `benchmark/reports/stress-speed-benchmark.md`
- `docs/contribution-map.md`

Public evidence is still early. Stress evidence now supports the selective parsing claim on synthetic records with many unused fields; larger whole-cohort public runs and memory/throughput trend reporting are next.

## Milestones

1. `v0.1 Evidence Baseline`: streaming filter, stats/diff, TSV conversion, synthetic and GIAB benchmark reports.
2. `v0.2 Public Benchmark Expansion`: IGSR chr22 public-region, repeated hyperfine runs, 1M-record synthetic cases, memory/throughput reporting.
3. `v0.3 Stress And Speed`: synthetic stress VCFs with many unused INFO/FORMAT/sample fields, parser hot-path improvements, and stress benchmark reporting.
4. `v0.4 FORMAT-Aware Filtering`: support `FORMAT/GT`, `FORMAT/DP`, `FORMAT/GQ`, selected sample predicates, bcftools comparison.
5. `v0.5 Columnar Bridge`: Parquet/Arrow export for repeated analytical workloads and DuckDB-style workflows.

## Quickstart

```bash
cargo build
cargo test
make verify

vcf-fast filter input.vcf.gz --where "QUAL > 30" -o output.vcf.gz
vcf-fast stats input.vcf.gz
vcf-fast diff a.vcf.gz b.vcf.gz -o diff.tsv
vcf-fast convert input.vcf.gz --to tsv -o variants.tsv

cargo run -- filter tests/data/example.vcf --where "QUAL > 30" -o tests/output/filtered.vcf

docker build -t vcf-fast .
docker run --rm -v "$PWD:/work" vcf-fast cargo test
docker run --rm -v "$PWD:/work" -e VCF_FAST_BENCH_SIZES="10000 100000" vcf-fast make bench-smoke
```

## Current CLI

```bash
vcf-fast filter tests/data/example.vcf --where "QUAL > 30" -o tests/output/filtered.vcf
vcf-fast filter tests/data/example.vcf --where "QUAL >= 30 && DP > 10" -o tests/output/dp.vcf
vcf-fast filter tests/data/example.vcf --where "(QUAL > 55 || INFO/DP > 45) && FILTER == \"PASS\"" -o tests/output/grouped.vcf
vcf-fast filter tests/data/example.vcf.gz --where "AF > 0.01 && FILTER == \"PASS\"" -o tests/output/af.vcf.gz
vcf-fast stats tests/data/example.vcf
vcf-fast diff tests/data/diff_a.vcf tests/data/diff_b.vcf -o tests/output/diff.tsv
vcf-fast convert tests/data/example.vcf --to tsv -o tests/output/variants.tsv
```

## v0.1 Filter Support

- Inputs: `.vcf`, `.vcf.gz`
- Outputs: `.vcf`, `.vcf.gz`
- Fields: `QUAL`, `DP`, `AF`, `INFO/DP`, `INFO/AF`, `CHROM`, `POS`, `FILTER`
- Operators: `>`, `>=`, `<`, `<=`, `==`, `!=`
- Boolean operators: `&&`, `||`
- Grouping: parentheses
- INFO aliases: `DP` maps to `INFO/DP`; `AF` maps to `INFO/AF`

Missing numeric values such as `.` or absent INFO fields make that predicate false. Comma-separated numeric INFO values pass when any value satisfies the predicate.

## Limitations

This release is a line-preserving streaming filter, not the future columnar execution engine. Gzip output is valid gzip-compressed VCF text, but v0.1 does not promise BGZF or tabix-indexable output. FORMAT/sample-specific filtering, BCF, Arrow, and Parquet are deferred.

## Stats Output

`stats` writes JSON to stdout with site-level and allele-level metrics:

- record count in `variants`
- allele-level `snps` and `indels`
- `variants_per_chromosome`
- `qual` and `af` count/min/max/mean summaries
- `missing_filter_values`
- `transition_transversion_ratio`

## Diff Output

`diff` compares variant keys as `CHROM + POS + REF + ALT`, writes a TSV to `-o`, and prints summary counts to stderr:

```text
status	chrom	pos	ref	alt
only_in_a	1	100	A	G
shared	1	200	C	T
only_in_b	2	400	G	A
```

## Convert Output

`convert --to tsv` writes analysis-friendly TSV with stable columns:

```text
CHROM POS ID REF ALT QUAL FILTER INFO/DP INFO/AF
```

Missing values are written as `.`. Multi-allelic `INFO/AF` values are preserved as the original comma-separated value.

## Development

```bash
make fmt
make clippy
make test
make build
make verify
```

Run the smoke command:

```bash
cargo run -- filter tests/data/example.vcf --where "QUAL > 30" -o tests/output/filtered.vcf
```

Run the synthetic benchmark and correctness smoke harness:

```bash
make bench-smoke
```

Set `VCF_FAST_BENCH_SIZES` to control synthetic dataset sizes. If `hyperfine` and `bcftools` are installed, the harness times VCF-Fast against the comparable bcftools filter/query command and compares filtered core records or normalized TSV rows. The TSV baseline uses `bcftools query -u` so public VCFs without optional `INFO/DP` or `INFO/AF` header definitions still produce `.` values for those columns.

The current tracked benchmark report is in `benchmark/reports/synthetic-filter-benchmark.md`.

Run the synthetic stress benchmark with unused INFO/FORMAT/sample payloads:

```bash
make bench-stress
VCF_FAST_BENCH_MODE=stress VCF_FAST_BENCH_SIZES="100000 1000000" make bench-smoke
```

Download public benchmark data into the ignored local cache:

```bash
benchmark/download_public_data.sh giab-hg002
benchmark/download_public_data.sh igsr-chr22
```

Run public-data benchmark modes after downloading:

```bash
VCF_FAST_BENCH_MODE=public-small VCF_FAST_BENCH_SIZES="10000" make bench-smoke
VCF_FAST_BENCH_MODE=public-region VCF_FAST_BENCH_SIZES="10000" make bench-smoke
```

## Docker

```bash
docker build -t vcf-fast .
docker run --rm -v "$PWD:/work" vcf-fast cargo test
```
