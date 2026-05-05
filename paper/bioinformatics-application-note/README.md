# VariantFlow Bioinformatics Application Note LaTeX Project

This directory contains a separate LaTeX manuscript project for a
Bioinformatics-style Application Note. It is intentionally separate from the
JOSS Markdown draft in `paper/paper.md`.

## Build

```bash
make
```

The generated PDF is written to `build/variantflow-bioinformatics-note.pdf`.

## Layout

- `main.tex`: manuscript entry point and title/abstract metadata.
- `sections/`: manuscript body sections.
- `tables/benchmark_summary.tex`: evidence table sourced from tracked reports.
- `references.bib`: bibliography for VCF, bcftools/HTSlib, Bioconda, Arrow,
  Parquet, DuckDB, VCFtools, and GATK.

Before submission, replace the contact email placeholder, confirm funding and
conflict statements, tag/archive the software release, and update availability
URLs with the final release DOI.

