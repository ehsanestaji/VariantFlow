# VariantFlow v2.5 Packed Genotype Benchmark

This report tracks the packed diploid biallelic genotype core for VCFtools-style workflows. The immediate performance target is LD RSS, then shared packed storage across frequency, missingness, HWE, heterozygosity, site pi, window pi, Tajima's D, and Weir-Cockerham Fst.

## Correctness Gate

All rows must pass VCFtools parity before any claim update. The supported replacement scope remains diploid biallelic genotype data.

## Measured Rows

Source generated report: `tests/output/benchmark-results/v25-packed-genotype/true-popgen-report.md` from Docker/Linux on 2026-05-08. Exact commands are retained there.

| workflow | dataset | tier | record count | sample count | runtime mean/stddev | speedup | samples/sec | peak RSS KB | CPU seconds | CPU-hour estimate | exact VariantFlow command | exact VCFtools command | correctness result | caveat |
| --- | --- | ---: | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| frequency | 1000 Genomes / IGSR public biallelic human cohort | 100k | 100000 | 3202 | VariantFlow 2.448149s; VCFtools 9.942010s | 4.06x | 40847 records/s | VariantFlow 11632; VCFtools 11616 | VariantFlow 3.190032; VCFtools 9.835647 | VariantFlow 0.000886120; VCFtools 0.002732124 | see generated report | see generated report | passed VCFtools parity and tier output normalization | supported diploid biallelic rows only |
| missingness | 1000 Genomes / IGSR public biallelic human cohort | 100k | 100000 | 3202 | VariantFlow 2.537515s; VCFtools 20.388467s | 8.03x | 39409 records/s | VariantFlow 11620; VCFtools 11632 | VariantFlow 3.088448; VCFtools 18.993660 | VariantFlow 0.000857902; VCFtools 0.005276017 | see generated report | see generated report | passed VCFtools parity and tier output normalization | VCFtools site and individual missingness are two commands |
| HWE | 1000 Genomes / IGSR public biallelic human cohort | 100k | 100000 | 3202 | VariantFlow 8.523551s; VCFtools 12.573955s | 1.48x | 11732 records/s | VariantFlow 11632; VCFtools 11608 | VariantFlow 9.373226; VCFtools 10.885069 | VariantFlow 0.002603674; VCFtools 0.003023630 | see generated report | see generated report | passed VCFtools parity and tier output normalization | exact p-value column is outside current output |
| heterozygosity | 1000 Genomes / IGSR public biallelic human cohort | 100k | 100000 | 3202 | VariantFlow 8.846830s; VCFtools 10.816501s | 1.22x | 11303 records/s | VariantFlow 11628; VCFtools 11672 | VariantFlow 9.794643; VCFtools 10.823740 | VariantFlow 0.002720734; VCFtools 0.003006594 | see generated report | see generated report | passed VCFtools parity and tier output normalization | supported diploid biallelic rows only |
| site pi | 1000 Genomes / IGSR public biallelic human cohort | 100k | 100000 | 3202 | VariantFlow 8.191548s; VCFtools 12.117107s | 1.48x | 12208 records/s | VariantFlow 11608; VCFtools 11632 | VariantFlow 8.980127; VCFtools 10.603812 | VariantFlow 0.002494480; VCFtools 0.002945503 | see generated report | see generated report | passed VCFtools parity and tier output normalization | supported diploid biallelic rows only |
| window pi | 1000 Genomes / IGSR public biallelic human cohort | 100k | 100000 | 3202 | VariantFlow 8.370996s; VCFtools 10.293167s | 1.23x | 11946 records/s | VariantFlow 11632; VCFtools 11632 | VariantFlow 9.332986; VCFtools 10.232768 | VariantFlow 0.002592496; VCFtools 0.002842436 | see generated report | see generated report | passed VCFtools parity and tier output normalization | window size 200 |
| Tajima's D | 1000 Genomes / IGSR public biallelic human cohort | 100k | 100000 | 3202 | VariantFlow 8.727080s; VCFtools 10.454500s | 1.20x | 11459 records/s | VariantFlow 11632; VCFtools 11632 | VariantFlow 9.555840; VCFtools 10.034056 | VariantFlow 0.002654400; VCFtools 0.002787238 | see generated report | see generated report | passed VCFtools parity and tier output normalization | window size 200 |
| LD RSS | 1000 Genomes / IGSR public biallelic human cohort | 100k | 100000 | 3202 | VariantFlow 13.050669s; VCFtools 145.378068s | 11.14x | 7662 records/s | VariantFlow 11620; VCFtools 11616 | VariantFlow 13.576299; VCFtools 95.867102 | VariantFlow 0.003771194; VCFtools 0.026629751 | see generated report | see generated report | passed VCFtools parity and tier output normalization | bounded `--max-distance 500`; memory now comparable on this row |
| Weir-Cockerham Fst | 1000 Genomes / IGSR public biallelic human cohort | 100k | 100000 | 3202 | VariantFlow 8.689425s; VCFtools 11.566337s | 1.33x | 11508 records/s | VariantFlow 11620; VCFtools 11628 | VariantFlow 9.310935; VCFtools 10.036271 | VariantFlow 0.002586371; VCFtools 0.002787853 | see generated report | see generated report | passed VCFtools parity and tier output normalization | two population files; AFR/EUR official superpopulations |

## Caveat

This report is not a broad VCFtools replacement claim. It supports scoped claims for the measured 100k 1000 Genomes / IGSR diploid biallelic human cohort only. Larger 1M repeated rows remain the next evidence step.

## v3.0 1M Evidence Attempt

On 2026-05-08, the Docker/Linux 1M tier was started with `VCF_FAST_V25_TIERS="1000000"`, `VCF_FAST_V25_RUNS=3`, and `VCF_FAST_V25_WARMUP=1`. The run staged `public-cohort.biallelic.1000000.vcf.gz` and completed the frequency hyperfine pair, but it was interrupted before the tier-level VCFtools parity gate could run. At interruption, `public-cohort-1000000-frequency.hyperfine.json` contained VariantFlow `24.123s` and VCFtools `91.810s`, and `public-cohort-1000000-missingness.hyperfine.json` contained only the first VariantFlow mean (`22.220s`) while VCFtools `--missing-indv` was still running. Because the full tier did not finish and normalized tier outputs were not accepted by `benchmark/check_vcftools_parity.py`, no 1M row is promoted to the claim matrix.
