# v1.0 Columnar Workflow Evidence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add reproducible evidence for the “export once, query many times” Parquet workflow.

**Architecture:** Keep the CLI unchanged. Add a shell benchmark harness that stages data, exports native Parquet, runs DuckDB repeated queries, compares counts against repeated `bcftools` scans, and writes an evidence report. Add a small Python DuckDB helper to keep SQL/query behavior testable and shell quoting manageable.

**Tech Stack:** Rust CLI, Bash benchmark scripts, Python 3, optional DuckDB Python module, `bcftools`, `hyperfine`.

---

### Task 1: Harness Contract Tests

**Files:**
- Modify: `tests/benchmark_harness_tests.rs`
- Modify: `Makefile`
- Create: `benchmark/run_v10_columnar_workflow_benchmarks.sh`
- Create: `benchmark/query_parquet_duckdb.py`
- Create: `benchmark/reports/v10-columnar-workflow-benchmark.md`

- [x] Add a test that requires the new script to mention DuckDB, repeated queries, `bcftools filter`, `--to parquet`, public-heavy mode, and honest caveats.
- [x] Run `cargo test --test benchmark_harness_tests v10_columnar -- --nocapture` and confirm it fails because files are missing.
- [x] Implement the script/helper/report/Makefile target.
- [x] Re-run the focused benchmark harness test and confirm it passes.

### Task 2: DuckDB Helper

**Files:**
- Create: `benchmark/query_parquet_duckdb.py`

- [x] Implement `count_qual_gt_30` and repeated execution.
- [x] Print one integer count to stdout for correctness checks.
- [x] Exit with a clear message if Python DuckDB is unavailable.
- [x] Include `python3 -m py_compile benchmark/*.py` in existing verification.

### Task 3: Evidence Report And Docs

**Files:**
- Create: `benchmark/reports/v10-columnar-workflow-benchmark.md`
- Modify: `README.md`
- Modify: `docs/contribution-map.md`

- [x] Add a report table with dataset source, record count, export command, DuckDB command, bcftools command, correctness result, runtime, speedup, variants/sec, RSS, caveat, and claim decision.
- [x] Update README and contribution map only with measured smoke/stress evidence and cautious wording.
- [x] Keep public-data and DuckDB/Polars broader claims as pending until larger runs exist.

### Task 4: Verification And Commit

**Files:**
- All changed files.

- [x] Run `bash -n benchmark/run_v10_columnar_workflow_benchmarks.sh`.
- [x] Run `cargo test --test benchmark_harness_tests v10_columnar -- --nocapture`.
- [x] Run a small benchmark smoke command.
- [x] Run `make verify`.
- [x] Run `cargo test --features htslib-static`.
- [x] Commit as `bench: add columnar workflow evidence harness`.
