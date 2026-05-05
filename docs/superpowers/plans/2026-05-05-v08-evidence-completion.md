# VCF-Fast v0.8 Evidence Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the post-byte-core v0.8 evidence pass with repeated 1M stress and public-heavy measurements, then update tracked reports and claims only from correctness-matched measured rows.

**Architecture:** Do not add user-facing CLI features. Reuse the existing benchmark harness, generated local reports under `tests/output/benchmark-results`, and tracked Markdown reports under `benchmark/reports`. Treat README and `docs/contribution-map.md` as evidence consumers, not sources of truth.

**Tech Stack:** Rust 2024, existing Bash benchmark harness, `bcftools`, `tabix`, `hyperfine`, Python report helpers, Markdown reports, existing Make targets.

---

## File Structure

- Create `benchmark/reports/v08-core-efficiency-benchmark.md`: tracked report summarizing v0.8 post-surgery evidence.
- Modify `README.md`: align milestones with v0.8 core surgery/evidence completion, v0.9 expression parity, and v1.0 parallel/columnar; update current evidence only from measured rows.
- Modify `docs/contribution-map.md`: add v0.8 byte-core contribution and update the claim matrix only from measured rows.
- Modify `tests/benchmark_harness_tests.rs`: assert the v0.8 report exists and contains required report fields.
- Use generated local reports only as inputs:
  - `tests/output/benchmark-results/v08-stress-1m-after-byte-core.md`
  - `tests/output/benchmark-results/v08-public-heavy-1m-after-byte-core.md`

## Task 1: Add v0.8 Report Scaffold And Harness Guard

**Files:**
- Create: `benchmark/reports/v08-core-efficiency-benchmark.md`
- Modify: `tests/benchmark_harness_tests.rs`

- [ ] **Step 1: Write the failing report-field test**

Add this test to `tests/benchmark_harness_tests.rs`:

```rust
#[test]
fn v08_core_efficiency_report_tracks_required_fields() {
    let root = repo_root();
    let report = fs::read_to_string(root.join("benchmark/reports/v08-core-efficiency-benchmark.md"))
        .expect("v0.8 report should exist");

    for required in [
        "v0.8 Core Efficiency Benchmark",
        "byte-core surgery",
        "correctness result",
        "runtime mean",
        "runtime stddev",
        "speedup",
        "variants/sec",
        "peak RSS",
        "exact VCF-Fast command",
        "exact competitor command",
        "dataset source",
        "caveat",
        "claim decision",
    ] {
        assert!(report.contains(required), "missing report field: {required}");
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test --test benchmark_harness_tests v08_core_efficiency_report_tracks_required_fields
```

Expected: FAIL with `v0.8 report should exist`.

- [ ] **Step 3: Create the tracked report scaffold**

Create `benchmark/reports/v08-core-efficiency-benchmark.md` with this content:

```markdown
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
```

- [ ] **Step 4: Run the report-field test**

Run:

```bash
cargo test --test benchmark_harness_tests v08_core_efficiency_report_tracks_required_fields
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add benchmark/reports/v08-core-efficiency-benchmark.md tests/benchmark_harness_tests.rs
git commit -m "docs: scaffold v08 core efficiency report"
```

## Task 2: Run Verification Before Evidence

**Files:**
- No tracked source edits.

- [ ] **Step 1: Run default verification**

Run:

```bash
make verify
```

Expected: PASS. If `benchmark/__pycache__` appears, remove it with `rm -rf benchmark/__pycache__`; it is ignored and must not be committed.

- [ ] **Step 2: Run htslib feature verification**

Run:

```bash
cargo test --features htslib-static
cargo clippy --features htslib-static --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 3: Commit only if tracked verification files changed**

Run:

```bash
git status --short
```

Expected: no tracked changes from verification. If only ignored `tests/output/...` files changed, do not commit.

## Task 3: Run v0.8 Stress Evidence

**Files:**
- Generated input: `tests/output/benchmark-results/v08-stress-1m-after-byte-core.md`
- Tracked destination: `benchmark/reports/v08-core-efficiency-benchmark.md`

- [ ] **Step 1: Run repeated 1M stress benchmark**

Run:

```bash
VCF_FAST_BENCH_MODE=stress \
VCF_FAST_BENCH_SIZES="1000000" \
VCF_FAST_BENCH_RUNS=3 \
VCF_FAST_BENCH_WARMUP=1 \
VCF_FAST_BENCH_REPORT="tests/output/benchmark-results/v08-stress-1m-after-byte-core.md" \
make bench-smoke
```

Expected: command completes and writes `tests/output/benchmark-results/v08-stress-1m-after-byte-core.md`.

- [ ] **Step 2: Check stress correctness rows**

Run:

```bash
rg -n "not matched|mismatch|failed|deferred" tests/output/benchmark-results/v08-stress-1m-after-byte-core.md
```

Expected: no output for supported measured rows. If output appears, open the matching row and set the tracked report claim decision for that row to `no performance claim; correctness target`.

- [ ] **Step 3: Copy stress rows into the tracked v0.8 report**

Open `tests/output/benchmark-results/v08-stress-1m-after-byte-core.md`. Copy the measured table rows for these stress cases into `benchmark/reports/v08-core-efficiency-benchmark.md`:

```text
QUAL plain filter
DP plain filter
AF plain filter
FORMAT/DP selected-sample filter
FORMAT/GQ selected-sample filter
FORMAT/GT selected-sample filter
QUAL gzip-input filter
convert --to tsv
stats
```

For each copied row, convert the generated columns into the tracked table shape:

```text
case = generated case name
dataset source = stress synthetic data
record count = generated record count
dataset size bytes = generated dataset size bytes
correctness result = generated correctness result
runtime mean = generated vcf-fast mean vs competitor mean
runtime stddev = generated vcf-fast stddev vs competitor stddev
speedup = generated speedup
variants/sec = generated variants/sec
peak RSS = generated peak RSS
exact VCF-Fast command = generated exact VCF-Fast command
exact competitor command = generated exact competitor command
caveat = generated caveat, or "synthetic stress shape"
claim decision = apply the Claim Decision Rules section
```

- [ ] **Step 4: Commit stress evidence**

```bash
git add benchmark/reports/v08-core-efficiency-benchmark.md
git commit -m "docs: record v08 stress evidence"
```

## Task 4: Run v0.8 Public-Heavy Evidence

**Files:**
- Generated input: `tests/output/benchmark-results/v08-public-heavy-1m-after-byte-core.md`
- Tracked destination: `benchmark/reports/v08-core-efficiency-benchmark.md`

- [ ] **Step 1: Ensure public IGSR data is cached**

Run:

```bash
benchmark/download_public_data.sh igsr-chr22
```

Expected: `tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz` and its `.tbi` index exist.

- [ ] **Step 2: Run repeated 1M public-heavy benchmark**

Run:

```bash
VCF_FAST_BENCH_SIZES="1000000" \
VCF_FAST_BENCH_RUNS=3 \
VCF_FAST_BENCH_WARMUP=1 \
VCF_FAST_BENCH_REPORT="tests/output/benchmark-results/v08-public-heavy-1m-after-byte-core.md" \
make bench-heavy
```

Expected: command completes and writes `tests/output/benchmark-results/v08-public-heavy-1m-after-byte-core.md`.

- [ ] **Step 3: Check public-heavy correctness rows**

Run:

```bash
rg -n "not matched|mismatch|failed|deferred" tests/output/benchmark-results/v08-public-heavy-1m-after-byte-core.md
```

Expected: no output for supported measured rows. If output appears, open the matching row and set the tracked report claim decision for that row to `no performance claim; correctness target` or `deferred with failure reason`.

- [ ] **Step 4: Copy public-heavy rows into the tracked v0.8 report**

Open `tests/output/benchmark-results/v08-public-heavy-1m-after-byte-core.md`. Copy the measured table rows for:

```text
Heavy QUAL gzip input
Heavy Convert TSV gzip input
```

For each copied row, use:

```text
dataset source = 1000 Genomes high-coverage chr22 public-heavy bounded region
caveat = bounded chr22 region
claim decision = apply the Claim Decision Rules section
```

- [ ] **Step 5: Commit public-heavy evidence**

```bash
git add benchmark/reports/v08-core-efficiency-benchmark.md
git commit -m "docs: record v08 public-heavy evidence"
```

## Task 5: Update README Evidence And Milestones

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update milestone names**

In `README.md`, replace the current milestone list items 8 and 9 with:

```markdown
8. `v0.8 Core Efficiency And Evidence`: byte-slice native record views, cached INFO scanning, byte-backed expression evaluation, native filter/stats hot-path migration, and repeated post-surgery evidence.
9. `v0.9 Expression Parity`: arbitrary selected `INFO/*` and `FORMAT/*`, selected sample predicates, sample `ANY`/`ALL`, and documented compatibility with common `bcftools filter` semantics.
10. `v1.0 Parallel And Columnar`: native parallel BGZF execution, Parquet export, release-grade claim matrix, installer docs, and reproducible benchmark reports.
```

- [ ] **Step 2: Update Current Evidence from the v0.8 report**

Read `benchmark/reports/v08-core-efficiency-benchmark.md`. In README `Current Evidence`, update only rows whose v0.8 report claim decision is `measured win` or `correctness match; optimization needed`.

Add two rows if the corresponding report rows exist. Write the `current result` cell as exact measured prose copied from the v0.8 report: use the report speedup values for faster rows, and write `correctness matched; VCF-Fast slower on this row` for slower rows.

```markdown
| Stress 1M after v0.8 byte-core surgery | `bcftools filter` / `bcftools query` / `bcftools stats` | matched supported rows or core records | exact measured stress speedups and slower-path caveats from `benchmark/reports/v08-core-efficiency-benchmark.md` | synthetic stress shape; repeated local run |
| IGSR chr22 public-heavy after v0.8 byte-core surgery | `bcftools filter` / `bcftools query` | matched filtered core records / normalized TSV rows | exact measured public-heavy speedups and slower-path caveats from `benchmark/reports/v08-core-efficiency-benchmark.md` | bounded chr22 region; repeated local run |
```

Before committing, compare each number in the README row against the v0.8 report table. Do not invent numbers.

- [ ] **Step 3: Add the v0.8 report to the evidence file list**

Add this bullet to README detailed evidence list:

```markdown
- `benchmark/reports/v08-core-efficiency-benchmark.md`
```

- [ ] **Step 4: Commit README update**

```bash
git add README.md
git commit -m "docs: update readme with v08 evidence path"
```

## Task 6: Update Contribution Map Claim Matrix

**Files:**
- Modify: `docs/contribution-map.md`

- [ ] **Step 1: Add v0.8 byte-core evidence to Current Implemented Contributions**

In `docs/contribution-map.md`, under `Selective Parsing`, add this evidence bullet:

```markdown
- v0.8 moved native filter and stats onto shared byte-slice `RecordView`/`InfoView` parsing and byte-backed expression evaluation, reducing line-level string parsing and repeated INFO scans.
```

- [ ] **Step 2: Add v0.8 measured claim text**

Under `Claims Supported So Far`, add one bullet. Compose the measured-result clause from the v0.8 report table and include every slower row caveat in the same sentence.

```markdown
- v0.8 byte-core evidence: On the repeated post-surgery benchmark, VCF-Fast matched supported correctness checks. The measured stress and IGSR public-heavy results are recorded in `benchmark/reports/v08-core-efficiency-benchmark.md`; this claim text must quote only those measured speedups and slower-path caveats.
```

Before committing, verify the sentence contains no numbers that are absent from `benchmark/reports/v08-core-efficiency-benchmark.md`.

- [ ] **Step 3: Update claim matrix evidence paths**

In the `Claim Matrix`, add `benchmark/reports/v08-core-efficiency-benchmark.md` to evidence paths for:

```text
Streaming selective filter on supported predicates
TSV export for selected columns
Stats simple counts
```

- [ ] **Step 4: Commit contribution-map update**

```bash
git add docs/contribution-map.md
git commit -m "docs: update claim matrix with v08 evidence"
```

## Task 7: Final Verification And Cleanup

**Files:**
- No new files unless verification reveals a tracked docs/test issue.

- [ ] **Step 1: Run full verification**

Run:

```bash
make verify
cargo test --features htslib-static
cargo clippy --features htslib-static --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 2: Check tracked state**

Run:

```bash
rm -rf benchmark/__pycache__
git status --short --branch
```

Expected: branch is clean except ignored benchmark artifacts under `tests/output/...`.

- [ ] **Step 3: Commit any final tracked fixes**

If verification required tracked fixes, commit them:

```bash
git status --short
git add README.md docs/contribution-map.md benchmark/reports/v08-core-efficiency-benchmark.md tests/benchmark_harness_tests.rs
git commit -m "test: finalize v08 evidence verification"
```

If no tracked files changed, do not create an empty commit.

## Acceptance Criteria

- `make verify` passes.
- `cargo test --features htslib-static` passes.
- `cargo clippy --features htslib-static --all-targets -- -D warnings` passes.
- `benchmark/reports/v08-core-efficiency-benchmark.md` contains measured stress and public-heavy rows or explicit deferrals with failure reasons.
- README and contribution map claims match the v0.8 report exactly.
- No broad "best VCF tool" claim is added.
- Worktree is clean on completion.
