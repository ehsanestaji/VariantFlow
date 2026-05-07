# VariantFlow v1.7 True Public Population Evidence

Status: scaffold. Run `make bench-vcftools-true-popgen` with
`VCF_FAST_V17_TRUE_POP_INPUT` and `VCF_FAST_V17_TRUE_POP_METADATA` pointing to a
cached 1000 Genomes / IGSR cohort and official metadata.

This report requires official population metadata, actual record count, sample
count, runtime mean/stddev where available, peak RSS KB, CPU seconds, CPU-hour
estimate, exact commands, VCFtools version, correctness result, and caveats.
The harness uses no header-fallback population files.

| tier | case | actual record count | sample count | population metadata source | runtime mean | speedup | VariantFlow peak RSS KB | VCFtools peak RSS KB | VariantFlow CPU seconds | VCFtools CPU seconds | VariantFlow CPU-hour estimate | VCFtools CPU-hour estimate | correctness result | caveats |
|---|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| public cohort 10000 | all cases | pending | pending | official population metadata required | pending | pending | pending | pending | pending | pending | pending | pending | pending | not measured |
| public cohort 50000 | all cases | pending | pending | official population metadata required | pending | pending | pending | pending | pending | pending | pending | pending | pending | not measured |
| public cohort 100000 | all cases | pending | pending | official population metadata required | pending | pending | pending | pending | pending | pending | pending | pending | pending | not measured |

Claim decision: no broad VCFtools replacement claim. Only correctness-matched
measured rows may support scoped performance claims.
