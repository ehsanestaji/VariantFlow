# VCF-Fast

VCF-Fast is an experimental high-performance engine for genomic variant data. The v0.1 slice focuses on a streaming, selective VCF filter that preserves original VCF lines instead of reconstructing records.

## Current CLI

```bash
vcf-fast filter tests/data/example.vcf --where "QUAL > 30" -o tests/output/filtered.vcf
vcf-fast filter tests/data/example.vcf --where "QUAL >= 30 && DP > 10" -o tests/output/dp.vcf
vcf-fast filter tests/data/example.vcf.gz --where "AF > 0.01 && FILTER == \"PASS\"" -o tests/output/af.vcf.gz
```

`stats` and `diff` are present as explicit v0.1 placeholders:

```bash
vcf-fast stats tests/data/example.vcf
vcf-fast diff a.vcf b.vcf -o diff.tsv
```

## v0.1 Filter Support

- Inputs: `.vcf`, `.vcf.gz`
- Outputs: `.vcf`, `.vcf.gz`
- Fields: `QUAL`, `DP`, `AF`, `CHROM`, `POS`, `FILTER`
- Operators: `>`, `>=`, `<`, `<=`, `==`, `!=`
- Boolean operator: `&&`
- INFO aliases: `DP` maps to `INFO/DP`; `AF` maps to `INFO/AF`

Missing numeric values such as `.` or absent INFO fields make that predicate false. Comma-separated numeric INFO values pass when any value satisfies the predicate.

## Limitations

This release is a line-preserving streaming filter, not the future columnar execution engine. Gzip output is valid gzip-compressed VCF text, but v0.1 does not promise BGZF or tabix-indexable output. FORMAT/sample-specific filtering, `||`, parentheses, BCF, Arrow, and Parquet are deferred.

## Development

```bash
make fmt
make clippy
make test
make build
```

Run the smoke command:

```bash
cargo run -- filter tests/data/example.vcf --where "QUAL > 30" -o tests/output/filtered.vcf
```

## Docker

```bash
docker build -t vcf-fast .
docker run --rm -v "$PWD:/work" vcf-fast cargo test
```
