# VariantFlow v2.5 Packed Genotype Benchmark

This scaffold tracks the packed diploid biallelic genotype core for VCFtools-style workflows. The immediate performance target is LD RSS, then shared packed storage across frequency, missingness, HWE, heterozygosity, site pi, window pi, Tajima's D, and Weir-Cockerham Fst.

## Correctness Gate

All rows must pass VCFtools parity before any claim update. The supported replacement scope remains diploid biallelic genotype data.

## Measured Rows

| workflow | dataset | tier | record count | sample count | runtime mean/stddev | speedup | samples/sec | peak RSS KB | CPU seconds | CPU-hour estimate | exact VariantFlow command | exact VCFtools command | correctness result | caveat |
| --- | --- | ---: | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| LD RSS | pending | pending | pending | pending | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | packed-genotype claim pending larger public rows |
| frequency | pending | pending | pending | pending | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | VCFtools parity required |
| missingness | pending | pending | pending | pending | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | VCFtools parity required |
| HWE | pending | pending | pending | pending | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | VCFtools parity required |
| Fst | pending | pending | pending | pending | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | VCFtools parity required |

## Caveat

This report is not a broad VCFtools replacement claim. It becomes evidence only after `make vcftools-parity` and true public population rows pass.
