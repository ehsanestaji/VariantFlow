# VariantFlow

[![CI](https://github.com/ehsanestaji/VariantFlow/actions/workflows/ci.yml/badge.svg)](https://github.com/ehsanestaji/VariantFlow/actions/workflows/ci.yml)
[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.21198171.svg)](https://doi.org/10.5281/zenodo.21198171)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Docs](https://img.shields.io/badge/docs-ehsanestaji.com-2A7F73.svg)](https://ehsanestaji.com/software/variantflow)

**VariantFlow** is a fast, selective-execution engine for post-calling VCF/BCF
operations — filtering, VCFtools-style population summaries, and analytical
export. Its core idea is *selective execution*: a query is compiled to the
minimal set of fields it touches, and only those fields are decoded. Unused
INFO annotations, FORMAT vectors, and sample values are never parsed, and
original records are preserved where possible.

VariantFlow **complements** bcftools, HTSlib, GATK, VCFtools, and scikit-allel;
it does not replace them. Every performance claim is backed by a
correctness-matched benchmark.

- **Documentation & guides:** https://ehsanestaji.com/software/variantflow
- **Archived release (Zenodo):** [10.5281/zenodo.21198171](https://doi.org/10.5281/zenodo.21198171)

---

## Contents

- [Installation](#installation)
- [Quick start](#quick-start)
- [Command reference](#command-reference)
- [Population-genetics statistics](#population-genetics-statistics)
- [Choosing the right tool](#choosing-the-right-tool)
- [Benchmarks & correctness](#benchmarks--correctness)
- [Documentation](#documentation)
- [Citation](#citation)
- [Contributing](#contributing)
- [License](#license)

## Installation

VariantFlow is written in Rust. Build from source with a recent stable
toolchain ([rustup](https://rustup.rs)):

```bash
git clone https://github.com/ehsanestaji/VariantFlow.git
cd VariantFlow
cargo build --release
# binary at ./target/release/variantflow
```

Optional HTSlib-backed paths (BCF input, indexed region reads, BGZF output):

```bash
cargo build --release --features htslib-static
```

Companion tools used in typical workflows: `bcftools`, `htslib`/`tabix`
(available via [Bioconda](https://bioconda.github.io)).

## Quick start

```bash
# Field-selective filtering (only QUAL and FORMAT/DP are decoded)
variantflow filter input.vcf.gz --where "QUAL > 30 && ANY(FORMAT/DP > 20)" -o out.vcf

# Population-genetics summaries
variantflow missingness input.vcf.gz -o out
variantflow pi input.vcf.gz --window-size 100000 -o out.windowed.pi
variantflow fst input.vcf.gz --pop pop1.txt --pop pop2.txt -o out.fst

# Missing-data-aware diversity (pixy-equivalent; needs an all-sites VCF)
variantflow pixy allsites.vcf.gz --populations pops.txt --window-size 10000 \
  --out-pi pi.tsv --out-dxy dxy.tsv

# Export once, query many with DuckDB
variantflow convert input.vcf.gz --to parquet -o variants.parquet
```

## Command reference

**Filtering, indexing & I/O:** `filter` (expression-based filtering) ·
`index` (`.vfi` chunk-skip index) · `convert` (TSV / Parquet export) ·
`diff` (compare two files) · `stats` (site/allele summaries)

**Population genetics:** `freq` · `missingness` · `het` · `hardy` · `pi` ·
`pixy` (missing-data-aware π / d\_XY) · `tajima-d` · `ld` · `fst` · `sfs`
(site-frequency spectrum)

See the full [command and statistics reference](https://ehsanestaji.com/software/variantflow)
or [`docs/statistics.md`](docs/statistics.md).

## Population-genetics statistics

VariantFlow implements the statistics that are computable in a single
**streaming pass** over sites or windows, and validates the overlapping ones
against VCFtools and scikit-allel:

- **Diversity & divergence:** `pi`, `pixy` (unbiased π/d\_XY under missing
  data — validated exact against pixy 2.2.1), `tajima-d`, `sfs`
  (folded/unfolded — validated exact against scikit-allel).
- **Differentiation & linkage:** `fst` (Weir–Cockerham, Hudson), `ld` (r²).
- **Frequencies & quality:** `freq`, `missingness`, `het`, `hardy`.

Matrix-wide and haplotype-based statistics (selection scans, PCA,
D/f-statistics) require the whole genotype matrix in memory and are **deferred
to [scikit-allel](https://scikit-allel.readthedocs.io) by design** — see
[`docs/statistics.md`](docs/statistics.md).

## Choosing the right tool

| Tool | Best for |
|------|----------|
| **VariantFlow** | fast field-selective filtering and streamable site/window statistics on large cohorts, on commodity CPUs |
| **bcftools** | general filtering, normalization, format conversion (the compatibility reference) |
| **VCFtools** | classic population-genetics file operations |
| **DuckDB** | ad hoc analytical SQL over exported (Parquet) tables |
| **scikit-allel** | matrix/haplotype statistics (selection scans, PCA, D/f-statistics) |

Full discussion: [`docs/tool-comparison.md`](docs/tool-comparison.md).

## Benchmarks & correctness

VariantFlow reports a speedup only where a benchmark row records a correctness
comparison. On the 1000 Genomes 3,202-sample high-coverage dataset, measured
results include missingness 3.67–4.78× over VCFtools (constant ~9 MB memory),
FORMAT filtering 4.74–17.78× over bcftools, indexed high-skip predicates
123–273×, and LD 9.88×. Population-genetics outputs are verified byte-identical
against VCFtools and independently cross-checked with scikit-allel. Tracked
benchmark reports live under [`benchmark/reports/`](benchmark/reports) and
[`docs/public-benchmark-table.md`](docs/public-benchmark-table.md).

## Documentation

- **Guides, command & statistics reference, tutorials:**
  https://ehsanestaji.com/software/variantflow
- **End-to-end tutorial** (chr22 1000 Genomes): [`docs/user-guide.md`](docs/user-guide.md)
- **Statistics reference:** [`docs/statistics.md`](docs/statistics.md)
- **Tool comparison:** [`docs/tool-comparison.md`](docs/tool-comparison.md)

## Citation

If you use VariantFlow, please cite the archived release:

> Estaji, E. and Mao, J.-F. *VariantFlow: a selective-execution engine for
> efficient population genomic computation on large variant datasets.*
> Zenodo. https://doi.org/10.5281/zenodo.21198171

Machine-readable metadata is in [`CITATION.cff`](CITATION.cff).

## Contributing

Issues and pull requests are welcome. Please run `cargo fmt`,
`cargo clippy --all-targets -- -D warnings`, and `cargo test` before opening a
PR.

## License

Dual-licensed under either of

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
