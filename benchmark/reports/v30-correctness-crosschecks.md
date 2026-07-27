# v3.0 Independent Correctness Cross-Checks (scikit-allel and pixy)

This report records the independent correctness cross-checks that underwrite the
manuscript's Supplementary Table S1, Supplementary Table S4, and the correctness
sentence in the abstract. The cross-checks were executed by the scripts in
`benchmark/` and their machine-readable outputs are committed alongside this
report. Previously these results existed only as JSON artifacts under
`tests/output/` and were not written up as a tracked benchmark report.

## Provenance

| Cross-check | Script | Output artifact |
| --- | --- | --- |
| per-individual missingness vs scikit-allel | `benchmark/scikit_allel_crosscheck.py` | `tests/output/correctness-comparison/scikit-allel-crosscheck.json` |
| site-frequency spectrum vs scikit-allel | `benchmark/sfs_crosscheck.py` | `tests/output/sfs-validation/sfs-crosscheck.json` |
| missing-data-aware pi and dxy vs pixy | `benchmark/pixy_crosscheck.py` | `tests/output/pixy-validation/pixy-crosscheck.json` |
| pixy validation data generator | `benchmark/make_pixy_validation_data.py` | `tests/output/pixy-validation/allsites.vcf` |

Comparison tools: scikit-allel 1.3.13, pixy 2.2.1. VariantFlow 1.5.0.

## 1. Per-individual missingness vs scikit-allel — PASS

Dataset: IGSR 1000 Genomes chromosome 22 high-coverage, **1,066,557 variants,
3,202 samples**. scikit-allel computes per-individual missingness independently
from the genotype matrix (`gt.is_missing().sum(axis=0)`).

| metric | value |
| --- | --- |
| exact match on missing counts (`n_miss`) | **true** |
| max absolute difference, counts | **0** |
| max absolute difference, `f_miss` | **0.0** |
| mean absolute difference, `f_miss` | **0.0** |
| correlation | **1.0** |
| verdict | **PASS** |

**Scope note.** This script cross-checks **per-individual missingness only**. It
does not compute allele frequency. Any claim of a scikit-allel allele-frequency
cross-check is not supported by this evidence.

## 2. Site-frequency spectrum vs scikit-allel — PASS (exact)

Dataset: `tests/output/pixy-validation/allsites.vcf`, 5,000 sites.

| spectrum | bins (scikit-allel / VariantFlow) | total sites | max abs diff | exact |
| --- | --- | ---: | ---: | --- |
| unfolded | 50 / 50 | 5,000 / 5,000 | **0** | **true** |
| folded | 41 / 41 | 5,000 / 5,000 | **0** | **true** |

Verdict **PASS**. Both spectra are bin-for-bin identical.

**Binary availability.** The `sfs` subcommand is present in the VariantFlow 1.5.0
release binary (`variantflow --version` → 1.5.0; `sfs` listed in `--help`). An
earlier note in the supplementary user guide stating that `sfs` was absent from
the prebuilt binary refers to a pre-1.5.0 build and is superseded.

## 3. Missing-data-aware pi and dxy vs pixy — PASS

Dataset: all-sites cohort, 5,000 records, 40 samples, 2 populations, 15% of
genotypes set to missing (`make_pixy_validation_data.py`). Agreement tolerance
1e-8.

| statistic | windows (pixy / VariantFlow / compared) | max abs diff in window average | count mismatches |
| --- | --- | ---: | ---: |
| pi | 10 / 10 / 10 | **4.93e-09** | **0** |
| dxy | 5 / 5 / 5 | **4.46e-09** | **0** |

Verdict **PASS**. The integer pairwise counts (`count_diffs`, `count_comparisons`)
that pixy's estimator is defined on are **identical in every window**; the derived
window averages agree to within 5e-09, i.e. floating-point round-off.

## Summary

| Cross-check | Tool | Criterion | Result |
| --- | --- | --- | --- |
| per-individual missingness | scikit-allel 1.3.13 | exact | PASS, max diff 0 |
| SFS folded and unfolded | scikit-allel 1.3.13 | exact | PASS, max diff 0 |
| pi and dxy, 15% missing | pixy 2.2.1 | counts exact; averages < 1e-8 | PASS, counts identical |

No cross-check compares allele frequency against scikit-allel. Allele-frequency
correctness rests on the VCFtools parity check recorded in
`v29-m5max-popgen-reproduction.md` (925,730 records identical).
