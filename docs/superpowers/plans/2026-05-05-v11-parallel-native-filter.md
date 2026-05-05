# v1.1 Parallel Native Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add opt-in native parallel filter predicate evaluation with ordered, line-preserving output.

**Architecture:** Keep native reader/writer APIs unchanged. In `src/engine/filter.rs`, choose the existing streaming loop for single-thread mode and a bounded batch path for `VCF_FAST_NATIVE_FILTER_THREADS>1`. The batch path evaluates records with Rayon and writes accepted original bytes in input order.

**Tech Stack:** Rust, Rayon, existing byte-slice `RecordView`/`InfoView`, Bash/hyperfine benchmark scripts.

---

### Task 1: Env And CLI Behavior Tests

**Files:**
- Modify: `tests/filter_cli_tests.rs`
- Modify: `src/engine/filter.rs`

- [x] Add CLI tests proving parallel native output matches default output byte-for-byte.
- [x] Add CLI tests proving invalid thread/batch env values fail clearly.
- [x] Implement env parsing and fallback to default streaming when unset or `1`.

### Task 2: Ordered Parallel Batch Evaluator

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/engine/filter.rs`

- [x] Add Rayon.
- [x] Read record lines into bounded `Vec<Vec<u8>>` batches.
- [x] Evaluate each batch in parallel using the already parsed expression, required fields, and sample-column mapping.
- [x] Write passing original lines back in original order.

### Task 3: Benchmark Harness And Evidence Report

**Files:**
- Create: `benchmark/run_v11_parallel_filter_benchmarks.sh`
- Create: `benchmark/reports/v11-parallel-native-filter-benchmark.md`
- Modify: `tests/benchmark_harness_tests.rs`
- Modify: `Makefile`

- [x] Add a test enforcing the v1.1 harness/report fields.
- [x] Benchmark default native, parallel native, and `bcftools filter` on deterministic stress data.
- [x] Record correctness result, runtime, speedup, variants/sec, RSS, caveats, and claim decision.

### Task 4: Docs And Verification

**Files:**
- Modify: `README.md`
- Modify: `docs/contribution-map.md`

- [x] Update docs only with measured claims.
- [x] Run `make verify`.
- [x] Run `cargo test --features htslib-static`.
- [x] Run `cargo clippy --features htslib-static --all-targets -- -D warnings`.
- [x] Commit as `feat: add parallel native filter execution`.
