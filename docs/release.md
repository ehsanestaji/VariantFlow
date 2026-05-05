# VCF-Fast Release And Install Guide

VCF-Fast v1.3 is the first release-hardening milestone. The project is still evidence-gated: the README and claim matrix should only be updated from tracked benchmark reports.

## Install From Source

Requirements:

- Rust stable toolchain with `cargo`
- `make`
- `bcftools`, `tabix`, and `hyperfine` for benchmark reproduction
- `clang`, `cmake`, `libclang`, `pkg-config`, and zlib development headers for `htslib-static`

Build the default native engine:

```bash
cargo build --release
./target/release/vcf-fast --version
```

Install into Cargo's binary directory:

```bash
cargo install --path .
vcf-fast --version
```

## Docker

The Docker image includes Rust, `bcftools`, `tabix`, `hyperfine`, Python, and htslib build prerequisites used by the benchmark harness.

```bash
docker build -t vcf-fast .
docker run --rm -v "$PWD:/work" vcf-fast make verify
docker run --rm -v "$PWD:/work" vcf-fast cargo test --features htslib-static
```

## Compatibility Build

The default build stays dependency-light and Rust-native. Build with htslib only when you need `.bcf` input, indexed regions, or tabix-indexable BGZF output:

```bash
cargo build --release --features htslib-static
./target/release/vcf-fast filter input.bcf --where "QUAL > 30" -o output.vcf
./target/release/vcf-fast filter input.vcf.gz --region chr22:1-20000000 --where "QUAL > 30" -o output.vcf
./target/release/vcf-fast filter input.vcf --where "QUAL > 30" --compression bgzf -o output.vcf.gz
```

## Benchmark Prerequisites

Local benchmark reproduction expects:

- `bcftools`
- `tabix` and `bgzip`
- `hyperfine`
- `python3`
- optional DuckDB Python package for columnar workflow checks

The easiest fully provisioned path is Docker:

```bash
docker build -t vcf-fast .
docker run --rm -v "$PWD:/work" vcf-fast bash -lc 'make verify && make bench-v12'
```

## Public Data

Public data is downloaded into ignored paths under `tests/output/public-data` and should not be committed.

```bash
benchmark/download_public_data.sh giab-hg002
benchmark/download_public_data.sh igsr-chr22
benchmark/download_public_data.sh all
```

Run public-heavy evidence after the IGSR cache is available:

```bash
VCF_FAST_V12_PUBLIC_TIERS="100000 1000000" \
VCF_FAST_V12_STRESS_TIERS="100000 1000000" \
VCF_FAST_BENCH_RUNS=3 \
VCF_FAST_BENCH_WARMUP=1 \
make bench-v12
```

## Release Checklist

Before tagging a release:

```bash
make verify
cargo test --features htslib-static
cargo clippy --features htslib-static --all-targets -- -D warnings
make benchmark-table
git diff --check
```

Then confirm:

- `vcf-fast --version` prints the intended version.
- `README.md` quickstart works from a fresh checkout.
- `docs/public-benchmark-table.md` was regenerated from tracked reports.
- `docs/contribution-map.md` does not contain unmeasured speed claims.
- Large benchmark artifacts remain under ignored `tests/output/...` paths.
