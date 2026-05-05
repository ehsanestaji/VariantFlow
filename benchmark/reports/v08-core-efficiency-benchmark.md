# v0.8 Core Efficiency Benchmark

## Status

Pending measured repeated runs after the byte-core surgery. This report must only be filled from generated local reports under `tests/output/benchmark-results`.

## Scope

v0.8 refactored the native core around borrowed byte-slice parsing:

- `RecordView` for core VCF columns.
- `InfoView` for cached INFO lookup and comma-separated numeric predicates.
- Byte-backed expression evaluation through `EvalContext`.
- Native filter and stats paths migrated away from line-level `String` parsing.

## Evidence Commands

```bash
make verify
cargo test --features htslib-static
cargo clippy --features htslib-static --all-targets -- -D warnings

VCF_FAST_BENCH_MODE=stress \
VCF_FAST_BENCH_SIZES="1000000" \
VCF_FAST_BENCH_RUNS=3 \
VCF_FAST_BENCH_WARMUP=1 \
VCF_FAST_BENCH_REPORT="tests/output/benchmark-results/v08-stress-1m-after-byte-core.md" \
make bench-smoke

VCF_FAST_BENCH_SIZES="1000000" \
VCF_FAST_BENCH_RUNS=3 \
VCF_FAST_BENCH_WARMUP=1 \
VCF_FAST_BENCH_REPORT="tests/output/benchmark-results/v08-public-heavy-1m-after-byte-core.md" \
make bench-heavy
```

## Required Report Fields

Each measured row copied into this report must include dataset source, dataset shape, record count, dataset size bytes, correctness result, runtime mean, runtime stddev, speedup, variants/sec, peak RSS, exact VCF-Fast command, exact competitor command, caveat, and claim decision.

## Measured Results

No measured v0.8 rows are copied yet.

| case | dataset source | record count | dataset size bytes | correctness result | runtime mean | runtime stddev | speedup | variants/sec | peak RSS | exact VCF-Fast command | exact competitor command | caveat | claim decision |
|---|---|---:|---:|---|---:|---:|---:|---|---|---|---|---|---|

## Claim Decision Rules

- If correctness matches and VCF-Fast is faster, mark claim decision as `measured win`.
- If correctness matches and VCF-Fast is slower, mark claim decision as `correctness match; optimization needed`.
- If correctness fails, mark claim decision as `no performance claim; correctness target`.
- If a tier fails for environmental reasons, mark claim decision as `deferred with failure reason`.
