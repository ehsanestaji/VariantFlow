# VariantFlow v1.7 LD Rolling-Window Optimization

Status: targeted post-optimization evidence for `variantflow ld` on the same
1000 Genomes / IGSR true-public biallelic tiers used by
`benchmark/reports/v17-true-public-population-evidence.md`.

Change measured: bounded LD now uses a rolling `CHROM:POS` window with compact
per-sample genotype dosages instead of retaining every full site summary. This
preserves VCFtools-compatible duplicate-position output ordering by flushing
same-position groups in nested-loop order.

Correctness gate: after regenerating each VariantFlow LD output,
`python3 benchmark/check_vcftools_parity.py` passed for the full tier output
directory, including frequency, missingness, HWE, heterozygosity, pi,
Tajima's D, LD, and Weir-Cockerham Fst. The LD comparison is normalized against
VCFtools `--geno-r2 --ld-window-bp 500`.

Measurement method: one resource-helper run per tool and tier using
`python3 benchmark/command_resource_metrics.py` on the local macOS workstation.
These rows are apples-to-apples for the LD optimization, but they are not a
replacement for a full repeated hyperfine rerun of all population-genetics
cases.

| tier | actual record count | sample count | VariantFlow wall | VCFtools wall | speedup | VariantFlow peak RSS KB | VCFtools peak RSS KB | VariantFlow CPU seconds | VCFtools CPU seconds | VariantFlow CPU-hour estimate | VCFtools CPU-hour estimate | exact VariantFlow command | exact VCFtools command | correctness result | caveats |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|---|---|
| public cohort 10000 | 10000 | 3202 | 1.193690s | 6.267450s | 5.25x | 8688 | 5104 | 1.018765 | 6.217761 | 0.000282990 | 0.001727156 | `./target/release/variantflow ld tests/output/v17-true-population-evidence/public-cohort.biallelic.10000.vcf.gz --max-distance 500 -o tests/output/v17-true-population-evidence/public\ cohort\ 10000/variantflow.geno.ld` | `vcftools --gzvcf tests/output/v17-true-population-evidence/public-cohort.biallelic.10000.vcf.gz --geno-r2 --ld-window-bp 500 --out tests/output/ld-optimization-evidence/vcftools-ld-10000` | passed: full tier parity via `benchmark/check_vcftools_parity.py` | VCFtools still has lower RSS, but VariantFlow no longer has GB-scale LD memory use |
| public cohort 50000 | 50000 | 3202 | 5.385195s | 36.967226s | 6.86x | 8880 | 5344 | 5.789156 | 36.731400 | 0.001608099 | 0.010203167 | `./target/release/variantflow ld tests/output/v17-true-population-evidence/public-cohort.biallelic.50000.vcf.gz --max-distance 500 -o tests/output/v17-true-population-evidence/public\ cohort\ 50000/variantflow.geno.ld` | `vcftools --gzvcf tests/output/v17-true-population-evidence/public-cohort.biallelic.50000.vcf.gz --geno-r2 --ld-window-bp 500 --out tests/output/ld-optimization-evidence/vcftools-ld-50000` | passed: full tier parity via `benchmark/check_vcftools_parity.py` | VCFtools still has lower RSS, but VariantFlow no longer has GB-scale LD memory use |
| public cohort 100000 | 100000 | 3202 | 9.688155s | 65.152775s | 6.72x | 8880 | 5696 | 10.536130 | 64.930897 | 0.002926703 | 0.018036360 | `./target/release/variantflow ld tests/output/v17-true-population-evidence/public-cohort.biallelic.100000.vcf.gz --max-distance 500 -o tests/output/v17-true-population-evidence/public\ cohort\ 100000/variantflow.geno.ld` | `vcftools --gzvcf tests/output/v17-true-population-evidence/public-cohort.biallelic.100000.vcf.gz --geno-r2 --ld-window-bp 500 --out tests/output/ld-optimization-evidence/vcftools-ld-100000` | passed: full tier parity via `benchmark/check_vcftools_parity.py` | VCFtools still has lower RSS, but VariantFlow no longer has GB-scale LD memory use |

Claim decision: bounded genotype-dosage LD changed from a correctness-matched
optimization gap into a measured VariantFlow win on these true-public IGSR
tiers. The claim remains scoped to supported diploid biallelic rows with
`--max-distance 500`; broader LD modes still need validation.
