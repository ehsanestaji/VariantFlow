# VariantFlow v2.3 BGZF Pipeline Benchmark

This report measures native BGZF decompression and predicate evaluation as separable pipeline stages. Rows are accepted only when VariantFlow modes match byte-for-byte and core records match `bcftools filter`.

## Required Modes

- `forced-single`
- `bgzf-only`
- `predicate-only`
- `combined-pipeline`
- `bcftools filter`

## Measured Rows

Source generated report: `tests/output/benchmark-results/v23-bgzf-pipeline-full-linux.md` from Docker/Linux on 2026-05-08. Exact per-run commands are retained in that generated report.

| dataset | mode | runtime mean/stddev | peak RSS KB | correctness result | claim decision |
| --- | --- | ---: | ---: | --- | --- |
| stress-format-100000 | forced-single | 0.210622s / 0.001141s | 11632 | VariantFlow modes match byte-for-byte; forced-single matches `bcftools filter` core records | fastest VariantFlow mode; 1.98x faster than `bcftools` on this stress row |
| stress-format-100000 | bgzf-only | 0.646988s / 0.007758s | 19024 | matched | no speed claim; scheduler overhead dominates |
| stress-format-100000 | predicate-only | 0.632489s / 0.004263s | 16848 | matched | no speed claim; scheduler overhead dominates |
| stress-format-100000 | combined-pipeline | 0.628891s / 0.003349s | 17572 | matched | no speed claim; scheduler overhead dominates |
| stress-format-100000 | `bcftools filter` | 0.417766s / 0.003965s | 11632 | baseline core-record comparator | competitor baseline |
| stress-format-1000000 | forced-single | 2.058554s / 0.016270s | 11632 | VariantFlow modes match byte-for-byte; forced-single matches `bcftools filter` core records | fastest VariantFlow mode; 1.96x faster than `bcftools` on this stress row |
| stress-format-1000000 | bgzf-only | 6.612326s / 0.083572s | 21068 | matched | no speed claim; scheduler overhead dominates |
| stress-format-1000000 | predicate-only | 6.666288s / 0.052783s | 17620 | matched | no speed claim; scheduler overhead dominates |
| stress-format-1000000 | combined-pipeline | 6.363786s / 0.115813s | 22480 | matched | no speed claim; scheduler overhead dominates |
| stress-format-1000000 | `bcftools filter` | 4.026999s / 0.068896s | 11672 | baseline core-record comparator | competitor baseline |

## Policy Update

The v2.3 Linux run showed that forced-single was the winning mode for the stress FORMAT aggregate workload, while BGZF-only, predicate-only, and combined scheduling were slower and used more RSS. VariantFlow therefore keeps FORMAT aggregate predicate evaluation conservative by default; explicit `VCF_FAST_NATIVE_BGZF_THREADS` and `VCF_FAST_NATIVE_FILTER_THREADS` remain available for opt-in experiments and future public-data tuning.

## Claim Policy

Runtime claims for v2.3 require measured rows from this harness with `correctness result` showing byte-for-byte agreement across VariantFlow modes and core-record agreement against `bcftools filter`.
