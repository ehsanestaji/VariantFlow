# VariantFlow Release And Install Guide

VariantFlow, formerly VCF-Fast, carries the release-hardening surface forward while making native BGZF input acceleration the default for BGZF `.vcf.gz` reads. The project is still evidence-gated: the README and claim matrix should only be updated from tracked benchmark reports.

## Install From Source

Requirements:

- Rust stable toolchain with `cargo`
- `make`
- `bcftools`, `tabix`, and `hyperfine` for benchmark reproduction
- `clang`, `cmake`, `libclang`, `pkg-config`, and zlib development headers for `htslib-static`

Build the default native engine:

```bash
cargo build --release
./target/release/variantflow --version
```

Install into Cargo's binary directory:

```bash
cargo install --path .
variantflow --version
vcf-fast --version
```

`vcf-fast` is kept as a compatibility alias for at least one release after the VariantFlow rename.

## Docker

The Docker image includes Rust, `bcftools`, `tabix`, `hyperfine`, Python, and htslib build prerequisites used by the benchmark harness.

```bash
docker build -t variantflow .
docker run --rm -v "$PWD:/work" variantflow make verify
docker run --rm -v "$PWD:/work" variantflow cargo test --features htslib-static
```

## Compatibility Build

The default build stays dependency-light and Rust-native. Build with htslib only when you need `.bcf` input, indexed regions, or tabix-indexable BGZF output:

```bash
cargo build --release --features htslib-static
./target/release/variantflow filter input.bcf --where "QUAL > 30" -o output.vcf
./target/release/variantflow filter input.vcf.gz --region chr22:1-20000000 --where "QUAL > 30" -o output.vcf
./target/release/variantflow filter input.vcf --where "QUAL > 30" --compression bgzf -o output.vcf.gz
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
docker build -t variantflow .
docker run --rm -v "$PWD:/work" variantflow bash -lc 'make verify && make bench-v12'
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

- `variantflow --version` and `vcf-fast --version` print the intended version.
- `README.md` quickstart works from a fresh checkout.
- `docs/public-benchmark-table.md` was regenerated from tracked reports.
- `docs/contribution-map.md` does not contain unmeasured speed claims.
- Large benchmark artifacts remain under ignored `tests/output/...` paths.

## Distribution And Naming TODO

The Bioconda release and professional rename migration are tracked in the top-level `TODO.md`, `docs/rename-plan.md`, and `docs/bioconda-packaging.md`. Do not publish a Bioconda recipe until the final package/binary name is checked for collisions, the project license is chosen, and a tagged source release exists with deterministic `cargo install --locked --no-track --root $PREFIX --path .` build behavior.
