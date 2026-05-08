# VariantFlow v2.2 Scheduler Benchmark

This report is the evidence gate for the native auto scheduler. It compares
forced single-thread, default auto, BGZF-only, predicate-only, combined
BGZF+predicate scheduling, and `bcftools filter` on FORMAT-heavy BGZF
workloads.

## Status

Not yet measured in the tracked report. Run:

```bash
VCF_FAST_V22_STRESS_TIERS="100000 1000000" \
VCF_FAST_V22_PUBLIC_TIERS="1000 10000 50000" \
VCF_FAST_V22_RUNS=3 \
VCF_FAST_V22_WARMUP=1 \
make bench-v22-scheduler
```

## Measured Rows

| dataset | source | size bytes | record count | exact single-thread command | exact default-auto command | exact BGZF-only command | exact predicate-only command | exact combined command | exact competitor command | correctness result | runtime mean/stddev single/default/bgzf-only/predicate-only/combined/bcftools | speedup combined vs single/default/bcftools | variants/sec combined | peak RSS KB single/default/bgzf-only/predicate-only/combined/bcftools | CPU seconds single/default/bgzf-only/predicate-only/combined/bcftools | caveat | claim decision |
| --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | ---: | --- | --- | --- | --- |
| pending | v2.2 scheduler harness | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | not yet measured | n/a | n/a | n/a | n/a | n/a | run the benchmark before updating claims | no claim |

## Required Report Fields

runtime mean, runtime stddev, speedup, variants/sec, peak RSS KB, CPU seconds,
exact commands, correctness result, caveat, and claim decision.
