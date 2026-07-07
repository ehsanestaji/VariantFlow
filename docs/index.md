# VariantFlow

**VariantFlow** is a fast command-line tool for **post-calling VCF/BCF operations** —
the analysis steps that happen *after* variants have been called. It is a
**selective-execution engine**: it parses and computes only the fields your query
actually needs, so filtering, population-genetics summaries, and columnar export
stay fast even on large, multi-thousand-sample cohorts on commodity CPUs.

It reproduces the numbers you would get from established tools (VCFtools, pixy,
scikit-allel) from a single self-contained Rust binary, invoked as
`variantflow <command>`.

## What it does

- **Field-selective filtering** with an expression language (`filter`), region
  slicing, and sample subsetting.
- **Streaming population-genetics statistics** — allele frequencies, missingness,
  heterozygosity, Hardy–Weinberg, nucleotide diversity (π), missing-data-aware
  π and d~XY~ (pixy), Tajima's *D*, the site frequency spectrum, F~ST~, and
  linkage disequilibrium.
- **Columnar export** to Parquet (`convert`) for an *export-once* hand-off to
  analytical engines such as DuckDB.
- **Utility operations** — indexing, VCF/BCF conversion, and set-difference
  (`diff`) between two callsets.

## Who this is for

- **Population and conservation / ecological geneticists** computing diversity,
  differentiation, and neutrality statistics on large VCFs.
- **Plant and animal breeding** programs running per-site and windowed summaries
  across many individuals.
- **Large-cohort human-genomics QC** — allele-frequency, missingness, and
  Hardy–Weinberg screening at biobank scale.
- **Pipeline developers** who need a single dependency-light binary that produces
  VCFtools-compatible output and a fast Parquet export for downstream SQL.

## Quick start

```bash
# Build the release binary (see the Installation page for details)
cargo build --release

# Keep only high-quality biallelic SNPs, writing a bgzipped VCF
./target/release/variantflow filter input.vcf.gz \
    --where 'QUAL >= 30 && TYPE == "snp"' \
    --output clean.vcf.gz

# Windowed nucleotide diversity (10 kb windows)
./target/release/variantflow pi clean.vcf.gz \
    --window-size 10000 \
    --output pi.tsv

# Export once to Parquet for ad hoc SQL in DuckDB
./target/release/variantflow convert clean.vcf.gz \
    --to parquet \
    --output clean.parquet
```

New to the tool? Start with the [User Guide](user-guide.md), a complete
chromosome-22 walkthrough on real 1000 Genomes data. For the full catalogue of
statistics and their formulas, see the [Statistics reference](statistics.md). To
decide whether VariantFlow or an established tool fits your task, see
[Choosing the right tool](tool-comparison.md).

## Citing and links

- **Repository:** <https://github.com/ehsanestaji/VariantFlow>
- **Archived release (Zenodo):** [doi:10.5281/zenodo.21198171](https://doi.org/10.5281/zenodo.21198171)

Please cite the archived Zenodo record and the accompanying application note when
you use VariantFlow in published work.
