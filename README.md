# VCF-Fast

VCF-Fast is an experimental high-performance engine for genomic variant data. It treats VCF/BCF as exchange formats and focuses on selective parsing, typed execution, original-record preservation, and competitor-checked benchmarks.

## Quickstart

```bash
cargo build
cargo test
make verify

cargo run -- filter tests/data/example.vcf --where "QUAL > 30" -o tests/output/filtered.vcf
cargo run -- stats tests/data/example.vcf
cargo run -- diff tests/data/diff_a.vcf tests/data/diff_b.vcf -o tests/output/diff.tsv
cargo run -- convert tests/data/example.vcf --to tsv -o tests/output/variants.tsv

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

Set `VCF_FAST_BENCH_SIZES` to control synthetic dataset sizes. If `hyperfine` and `bcftools` are installed, the harness times VCF-Fast against the comparable bcftools filter command and compares filtered core records.

The current tracked benchmark report is in `benchmark/reports/synthetic-filter-benchmark.md`.

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
