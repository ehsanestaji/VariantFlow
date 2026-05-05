# VCF-Fast v0.7 Heavy-Run And Htslib Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add v0.7 heavy public benchmark execution that avoids giant plain IGSR intermediates, while reducing low-risk htslib compatibility-path overhead and preserving the native selective filter as the core winning path.

**Architecture:** Keep the existing native/htslib backend split. Add htslib tuning in `src/compat.rs`, apply it in `src/htslib_backend.rs`, convert htslib loops to reusable records where practical, and add `public-heavy` benchmark orchestration with bounded materialization. Reports and docs must separate measured wins from matched-but-slower paths and deferrals.

**Tech Stack:** Rust 2024, `rust-htslib` optional features, Bash benchmark harness, Python report summarization, Docker with `bcftools`/`tabix`/`hyperfine`, existing Rust integration tests.

---

## File Structure

- Modify `src/compat.rs`: add `VCF_FAST_HTSLIB_THREADS` parsing and validation helpers.
- Modify `src/htslib_backend.rs`: apply reader/writer threading, introduce reusable htslib record loops, and reduce unnecessary full-record reconstruction in stats where safe.
- Modify `tests/compatibility_unit_tests.rs`: unit-test thread config parsing.
- Modify `tests/compatibility_cli_tests.rs`: integration-test valid and invalid htslib thread env behavior.
- Modify `benchmark/run_benchmarks.sh`: add `public-heavy`, artifact caps, compressed/indexed subset creation, and extra report columns.
- Modify `Makefile`: add `bench-heavy`.
- Modify `tests/benchmark_harness_tests.rs`: static and smoke tests for `public-heavy`, artifact caps, and required v0.7 report fields.
- Create `benchmark/reports/v07-heavy-run-benchmark.md`: pending report scaffold, filled only with measured rows after local runs.
- Modify `README.md` and `docs/contribution-map.md`: update roadmap and claim matrix only with measured v0.7 evidence or pending caveats.

## Task 1: Add Htslib Thread Config Parsing

**Files:**
- Modify: `src/compat.rs`
- Modify: `tests/compatibility_unit_tests.rs`

- [ ] **Step 1: Write failing unit tests for thread config parsing**

Add these tests to `tests/compatibility_unit_tests.rs`:

```rust
use vcf_fast::compat::parse_htslib_threads;

#[test]
fn htslib_thread_config_accepts_unset_and_positive_integer() {
    assert_eq!(parse_htslib_threads(None).unwrap(), None);
    assert_eq!(parse_htslib_threads(Some("1")).unwrap(), Some(1));
    assert_eq!(parse_htslib_threads(Some("4")).unwrap(), Some(4));
}

#[test]
fn htslib_thread_config_rejects_zero_negative_and_non_integer() {
    let zero = parse_htslib_threads(Some("0")).unwrap_err().to_string();
    assert!(zero.contains("VCF_FAST_HTSLIB_THREADS must be a positive integer"));

    let negative = parse_htslib_threads(Some("-2")).unwrap_err().to_string();
    assert!(negative.contains("VCF_FAST_HTSLIB_THREADS must be a positive integer"));

    let text = parse_htslib_threads(Some("fast")).unwrap_err().to_string();
    assert!(text.contains("VCF_FAST_HTSLIB_THREADS must be a positive integer"));
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test --test compatibility_unit_tests htslib_thread_config
```

Expected: FAIL because `parse_htslib_threads` is not defined.

- [ ] **Step 3: Implement thread config parsing**

Add to `src/compat.rs`:

```rust
use std::env;
```

Add below `select_backend`:

```rust
pub const HTSLIB_THREADS_ENV: &str = "VCF_FAST_HTSLIB_THREADS";

pub fn htslib_threads_from_env() -> Result<Option<usize>> {
    match env::var(HTSLIB_THREADS_ENV) {
        Ok(raw) => parse_htslib_threads(Some(raw.as_str())),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => {
            bail!("{HTSLIB_THREADS_ENV} must be valid UTF-8")
        }
    }
}

pub fn parse_htslib_threads(raw: Option<&str>) -> Result<Option<usize>> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    let value = raw
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("{HTSLIB_THREADS_ENV} must be a positive integer"))?;
    if value == 0 {
        bail!("{HTSLIB_THREADS_ENV} must be a positive integer");
    }

    Ok(Some(value))
}
```

- [ ] **Step 4: Run unit tests**

Run:

```bash
cargo test --test compatibility_unit_tests htslib_thread_config
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/compat.rs tests/compatibility_unit_tests.rs
git commit -m "feat: parse htslib thread config"
```

## Task 2: Apply Htslib Threads And Reusable Record Loops

**Files:**
- Modify: `src/htslib_backend.rs`
- Modify: `tests/compatibility_cli_tests.rs`

- [ ] **Step 1: Write failing integration tests for env behavior**

Add these tests to `tests/compatibility_cli_tests.rs` inside the existing htslib-enabled test module or under `#[cfg(feature = "htslib")]`:

```rust
#[test]
fn htslib_commands_accept_valid_thread_env() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("threaded.vcf");

    assert_cmd::Command::cargo_bin("vcf-fast")
        .unwrap()
        .env("VCF_FAST_HTSLIB_THREADS", "2")
        .args([
            "filter",
            "tests/data/compat_example.bcf",
            "--where",
            "QUAL > 30",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn htslib_commands_reject_invalid_thread_env() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("invalid-thread.vcf");

    assert_cmd::Command::cargo_bin("vcf-fast")
        .unwrap()
        .env("VCF_FAST_HTSLIB_THREADS", "0")
        .args([
            "filter",
            "tests/data/compat_example.bcf",
            "--where",
            "QUAL > 30",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "VCF_FAST_HTSLIB_THREADS must be a positive integer",
        ));
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test --features htslib-static --test compatibility_cli_tests htslib_commands_
```

Expected: the invalid-env test FAILS because commands do not read `VCF_FAST_HTSLIB_THREADS` yet.

- [ ] **Step 3: Add htslib thread helpers and reusable loop**

In `src/htslib_backend.rs`, update imports:

```rust
use crate::compat::{CompressionMode, Region, htslib_threads_from_env};
```

Add helpers near `vcf_writer`:

```rust
fn apply_reader_threads<R: Read>(reader: &mut R) -> Result<()> {
    if let Some(threads) = htslib_threads_from_env()? {
        reader.set_threads(threads)?;
    }
    Ok(())
}

fn apply_writer_threads(writer: &mut Writer) -> Result<()> {
    if let Some(threads) = htslib_threads_from_env()? {
        writer.set_threads(threads)?;
    }
    Ok(())
}

fn for_each_record<R: Read>(
    reader: &mut R,
    mut observe: impl FnMut(&Record) -> Result<()>,
) -> Result<()> {
    let mut record = reader.empty_record();
    while let Some(result) = reader.read(&mut record) {
        result?;
        observe(&record)?;
    }
    Ok(())
}
```

- [ ] **Step 4: Apply helpers in filter**

Replace `for result in reader.records()` blocks in `filter` with reusable loops.

For the region branch:

```rust
let mut reader = indexed_reader(input, region)?;
apply_reader_threads(&mut reader)?;
let header = Header::from_template(reader.header());
let sample_id = sample_id(reader.header(), sample, required)?;
let mut writer = vcf_writer(output, &header, compression)?;
apply_writer_threads(&mut writer)?;
for_each_record(&mut reader, |record| {
    if evaluate_record(record, required, sample_id, &expr)? {
        writer.write(record)?;
    }
    Ok(())
})?;
```

For the non-region branch:

```rust
let mut reader = Reader::from_path(input)
    .with_context(|| format!("failed to open input {}", input.display()))?;
apply_reader_threads(&mut reader)?;
let header = Header::from_template(reader.header());
let sample_id = sample_id(reader.header(), sample, required)?;
let mut writer = vcf_writer(output, &header, compression)?;
apply_writer_threads(&mut writer)?;
for_each_record(&mut reader, |record| {
    if evaluate_record(record, required, sample_id, &expr)? {
        writer.write(record)?;
    }
    Ok(())
})?;
```

- [ ] **Step 5: Apply helpers in convert and stats**

For `convert_to_tsv`, use:

```rust
let mut reader = Reader::from_path(input)
    .with_context(|| format!("failed to open input {}", input.display()))?;
apply_reader_threads(&mut reader)?;
for_each_record(&mut reader, |record| write_tsv_record(record, &mut writer))?;
```

Use the same pattern for the indexed branch with `indexed_reader`.

For `stats`, use:

```rust
let mut reader = Reader::from_path(input)
    .with_context(|| format!("failed to open input {}", input.display()))?;
apply_reader_threads(&mut reader)?;
for_each_record(&mut reader, |record| {
    let owned = owned_record_fields(record)?;
    summary.observe(&owned.as_fields(), &mut titv)
})?;
```

Use the same pattern for the indexed branch.

- [ ] **Step 6: Run compatibility tests**

Run:

```bash
cargo test --features htslib-static --test compatibility_cli_tests
cargo test --features htslib-static --test compatibility_unit_tests
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/htslib_backend.rs tests/compatibility_cli_tests.rs
git commit -m "perf: reuse htslib records and apply thread config"
```

## Task 3: Reduce Htslib Stats Reconstruction

**Files:**
- Modify: `src/htslib_backend.rs`
- Modify: `tests/compatibility_cli_tests.rs`

- [ ] **Step 1: Add stats regression test for numeric INFO parity**

Add this test under `#[cfg(feature = "htslib")]` in `tests/compatibility_cli_tests.rs`:

```rust
#[test]
fn bcf_stats_observes_qual_and_af_without_vcf_text_reconstruction() {
    let output = assert_cmd::Command::cargo_bin("vcf-fast")
        .unwrap()
        .args(["stats", "tests/data/compat_example.bcf"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["variants"], 5);
    assert_eq!(json["qual"]["count"], 5);
    assert!(json["af"]["count"].as_u64().unwrap() > 0);
}
```

- [ ] **Step 2: Run test before implementation**

Run:

```bash
cargo test --features htslib-static --test compatibility_cli_tests bcf_stats_observes_qual
```

Expected: PASS before implementation. This is a guardrail test for refactoring.

- [ ] **Step 3: Split htslib fields for stats vs TSV**

In `src/htslib_backend.rs`, add a smaller stats-only struct:

```rust
struct HtslibStatsFields {
    chrom: String,
    pos: String,
    reference: String,
    alternate: String,
    qual: String,
    filter: String,
    info: String,
}

impl HtslibStatsFields {
    fn as_fields(&self) -> RecordFields<'_> {
        RecordFields {
            chrom: &self.chrom,
            pos: &self.pos,
            id: ".",
            reference: &self.reference,
            alternate: &self.alternate,
            qual: &self.qual,
            filter: &self.filter,
            info: &self.info,
        }
    }
}
```

Add:

```rust
fn stats_record_fields(record: &Record) -> Result<HtslibStatsFields> {
    Ok(HtslibStatsFields {
        chrom: chrom(record)?,
        pos: (record.pos() + 1).to_string(),
        reference: allele_string(record, 0)?,
        alternate: alternate_string(record)?,
        qual: qual(record)
            .map(|value| format_float(value as f64))
            .unwrap_or_else(|| ".".to_string()),
        filter: filter_string(record)?,
        info: info_string(record)?,
    })
}
```

- [ ] **Step 4: Use stats-only fields in `stats`**

Replace in both stats branches:

```rust
let owned = owned_record_fields(record)?;
summary.observe(&owned.as_fields(), &mut titv)
```

with:

```rust
let fields = stats_record_fields(record)?;
summary.observe(&fields.as_fields(), &mut titv)
```

This avoids `record_vcf_column(record, 7)` in stats.

- [ ] **Step 5: Run compatibility and stats tests**

Run:

```bash
cargo test --features htslib-static --test compatibility_cli_tests bcf_stats_observes_qual
cargo test --features htslib-static --test stats_diff_cli_tests
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/htslib_backend.rs tests/compatibility_cli_tests.rs
git commit -m "perf: avoid htslib stats VCF text reconstruction"
```

## Task 4: Add Public-Heavy Benchmark Mode And Artifact Caps

**Files:**
- Modify: `benchmark/run_benchmarks.sh`
- Modify: `Makefile`
- Modify: `tests/benchmark_harness_tests.rs`

- [ ] **Step 1: Write static tests for public-heavy mode**

Add to `tests/benchmark_harness_tests.rs`:

```rust
#[test]
fn v07_public_heavy_mode_and_artifact_caps_are_declared() {
    let harness = std::fs::read_to_string("benchmark/run_benchmarks.sh").unwrap();
    assert!(harness.contains("public-heavy"));
    assert!(harness.contains("VCF_FAST_HEAVY_MAX_PLAIN_BYTES"));
    assert!(harness.contains("build_public_heavy_dataset"));
    assert!(harness.contains("deferred: plain artifact cap exceeded"));

    let makefile = std::fs::read_to_string("Makefile").unwrap();
    assert!(makefile.contains("bench-heavy"));
    assert!(makefile.contains("VCF_FAST_BENCH_MODE=public-heavy"));
}
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cargo test --test benchmark_harness_tests v07_public_heavy_mode
```

Expected: FAIL because `public-heavy` does not exist yet.

- [ ] **Step 3: Add env defaults**

Near the top of `benchmark/run_benchmarks.sh`, add:

```bash
HEAVY_MAX_PLAIN_BYTES="${VCF_FAST_HEAVY_MAX_PLAIN_BYTES:-1073741824}"
HEAVY_REGION="${VCF_FAST_HEAVY_REGION:-$PUBLIC_REGION}"
```

- [ ] **Step 4: Add artifact cap helper**

Add after `markdown_cell`:

```bash
file_size_bytes() {
  wc -c <"$1" | tr -d ' '
}

assert_plain_artifact_under_cap() {
  local path="$1"
  local max_bytes="$2"
  local actual
  actual="$(file_size_bytes "$path")"
  if (( actual > max_bytes )); then
    echo "deferred: plain artifact cap exceeded for $path (${actual} > ${max_bytes})" >&2
    rm -f "$path"
    return 77
  fi
}
```

- [ ] **Step 5: Add heavy dataset builder**

Add after `build_public_region_dataset`:

```bash
build_public_heavy_dataset() {
  local source="$1"
  local output="$2"
  local records="$3"
  local region="$4"

  require_tool bcftools
  require_tool bgzip
  require_tool tabix

  local temp_plain="${output%.gz}.plain.tmp.vcf"
  {
    bcftools view -h "$source"
    bcftools view -H -r "$region" "$source" | awk -v limit="$records" 'NR <= limit'
  } >"$temp_plain"

  if ! awk 'BEGIN { found = 0 } !/^#/ { found = 1 } END { exit found ? 0 : 1 }' "$temp_plain"; then
    echo "region $region produced no records from $source; set VCF_FAST_HEAVY_REGION to a matching indexed region" >&2
    rm -f "$temp_plain"
    exit 2
  fi

  if ! assert_plain_artifact_under_cap "$temp_plain" "$HEAVY_MAX_PLAIN_BYTES"; then
    return 77
  fi

  bgzip -c "$temp_plain" >"$output"
  tabix -f -p vcf "$output"
  rm -f "$temp_plain"
}
```

- [ ] **Step 6: Add mode to `configure_inputs`**

Add a case:

```bash
public-heavy)
  local igsr="tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz"
  if [[ ! -s "$igsr" ]]; then
    echo "missing $igsr; run benchmark/download_public_data.sh igsr-chr22 first" >&2
    exit 2
  fi
  echo "1000 Genomes high-coverage chr22 heavy compressed/indexed tiers ${HEAVY_REGION}|$igsr"
  ;;
```

Update the unsupported mode error to include `public-heavy`.

- [ ] **Step 7: Build compressed heavy datasets in the main loop**

In the dataset creation branch, add before `public-region`:

```bash
elif [[ "$MODE" == "public-heavy" ]]; then
  gzip_dataset="$DATA_DIR/public-heavy-${records}.vcf.gz"
  if ! build_public_heavy_dataset "$PUBLIC_SOURCE" "$gzip_dataset" "$records" "$HEAVY_REGION"; then
    note="deferred: plain artifact cap exceeded"
    printf '| %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |\n' \
      "public-heavy setup" "$records" "n/a" "VCF" "BGZF" "n/a" "n/a" "$note" "n/a" "n/a" "n/a" "n/a" "n/a" "n/a" "n/a" "$note" >>"$REPORT"
    continue
  fi
  plain_dataset="$gzip_dataset"
```

Then guard the generic `gzip -c "$plain_dataset" >"$gzip_dataset"` line so it does not recompress the heavy `.vcf.gz`:

```bash
if [[ "$MODE" != "public-heavy" ]]; then
  gzip -c "$plain_dataset" >"$gzip_dataset"
fi
```

Set `base_dataset_size_bytes` for heavy after dataset creation:

```bash
if [[ "$MODE" == "public-heavy" ]]; then
  base_dataset_size_bytes="$(file_size_bytes "$gzip_dataset")"
else
  base_dataset_size_bytes="$(file_size_bytes "$plain_dataset")"
fi
```

- [ ] **Step 8: Limit public-heavy cases to compressed evidence**

Add after public mode cases:

```bash
if [[ "$MODE" == "public-heavy" ]]; then
  CASES=(
    "Heavy QUAL gzip input|gzip|QUAL > 30|QUAL>30"
    "Heavy Convert TSV gzip input|convert-tsv-gzip|convert-tsv|query-tsv"
  )
fi
```

Add `convert-tsv-gzip` handling where `gzip` is handled:

```bash
if [[ "$input_kind" == "gzip" || "$input_kind" == "convert-tsv-gzip" ]]; then
  dataset="$gzip_dataset"
  input_label="gzip"
  input_compression="BGZF"
fi
```

- [ ] **Step 9: Add Make target**

Add to `Makefile`:

```make
bench-heavy:
	VCF_FAST_BENCH_MODE=public-heavy ./benchmark/run_benchmarks.sh
```

If `.PHONY` exists, include `bench-heavy`.

- [ ] **Step 10: Run static tests and shell syntax checks**

Run:

```bash
cargo test --test benchmark_harness_tests v07_public_heavy_mode
bash -n benchmark/run_benchmarks.sh
make -n bench-heavy
```

Expected: PASS.

- [ ] **Step 11: Commit**

```bash
git add benchmark/run_benchmarks.sh Makefile tests/benchmark_harness_tests.rs
git commit -m "feat: add public heavy benchmark mode"
```

## Task 5: Add v0.7 Report Fields And Pending Report

**Files:**
- Modify: `benchmark/run_benchmarks.sh`
- Create: `benchmark/reports/v07-heavy-run-benchmark.md`
- Modify: `tests/benchmark_harness_tests.rs`

- [ ] **Step 1: Write report-field test**

Add to `tests/benchmark_harness_tests.rs`:

```rust
#[test]
fn v07_report_tracks_bottleneck_caveat_and_next_action() {
    let harness = std::fs::read_to_string("benchmark/run_benchmarks.sh").unwrap();
    assert!(harness.contains("bottleneck"));
    assert!(harness.contains("next action"));
    assert!(harness.contains("native-filter"));
    assert!(harness.contains("htslib-region-tsv"));

    let report =
        std::fs::read_to_string("benchmark/reports/v07-heavy-run-benchmark.md").unwrap();
    for required in [
        "correctness result",
        "runtime mean",
        "speedup",
        "variants/sec",
        "peak RSS",
        "bottleneck",
        "next action",
        "caveat",
    ] {
        assert!(report.contains(required), "missing {required}");
    }
}
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cargo test --test benchmark_harness_tests v07_report_tracks
```

Expected: FAIL because the report file and fields do not exist yet.

- [ ] **Step 3: Add report columns**

In `benchmark/run_benchmarks.sh`, change the table header from:

```bash
echo "| case | record count | dataset size bytes | input format | input compression | exact VCF-Fast command | exact competitor command | correctness result | vcf-fast mean | vcf-fast stddev | bcftools mean | bcftools stddev | speedup | variants/sec | peak RSS | caveats |"
```

to:

```bash
echo "| case | path class | record count | dataset size bytes | input format | input compression | exact VCF-Fast command | exact competitor command | correctness result | vcf-fast mean | vcf-fast stddev | bcftools mean | bcftools stddev | speedup | variants/sec | peak RSS | bottleneck | caveat | next action |"
```

Change the separator row to 19 columns:

```bash
echo "|---|---|---:|---:|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|---|---|---|"
```

- [ ] **Step 4: Add path classification helper**

Add after `slugify`:

```bash
path_class_for_case() {
  local input_kind="$1"
  local fast_expr="$2"

  case "$input_kind:$fast_expr" in
    region:convert-tsv|region-convert-tsv:convert-tsv) echo "htslib-region-tsv" ;;
    region-stats-json:stats-json|bcf-region-stats-json:stats-json) echo "htslib-region-stats" ;;
    region:*) echo "htslib-region-filter" ;;
    bcf:*) echo "bcf-filter" ;;
    bcf-convert-tsv:convert-tsv) echo "bcf-tsv" ;;
    bgzf-output:*) echo "bgzf-output" ;;
    *:convert-tsv) echo "native-tsv" ;;
    *:stats-json) echo "native-stats" ;;
    *) echo "native-filter" ;;
  esac
}

next_action_for_path() {
  local path_class="$1"
  case "$path_class" in
    native-filter) echo "keep as winning core and expand evidence" ;;
    native-tsv) echo "measure before adding columnar export" ;;
    htslib-region-filter) echo "compare thread counts and record reuse" ;;
    htslib-region-tsv) echo "reduce full-record reconstruction" ;;
    htslib-region-stats) echo "prefer typed stats fields" ;;
    bcf-filter) echo "measure htslib threading and write overhead" ;;
    bcf-tsv) echo "avoid raw VCF reconstruction when possible" ;;
    bgzf-output) echo "measure writer threading and compression cost" ;;
    *) echo "inspect benchmark artifact" ;;
  esac
}
```

- [ ] **Step 5: Emit new columns**

Before final `printf`, set:

```bash
path_class="$(path_class_for_case "$input_kind" "$fast_expr")"
bottleneck="${note:-measured path; inspect speedup/RSS}"
next_action="$(next_action_for_path "$path_class")"
```

Update `printf` format and arguments:

```bash
printf '| %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |\n' \
  "$case_name" "$path_class" "$records" "$dataset_size_bytes" "$input_format" "$input_compression" "$fast_command_cell" "$competitor_command_cell" "$equivalence" "$fast_mean" "$fast_stddev" "$bcftools_mean" "$bcftools_stddev" "$speedup" "$variants_per_second_cell" "$peak_rss_cell" "$bottleneck" "${note:-}" "$next_action" >>"$REPORT"
```

Update any early deferral `printf` from Task 4 to the same 19-column format.

- [ ] **Step 6: Create pending report scaffold**

Create `benchmark/reports/v07-heavy-run-benchmark.md`:

```markdown
# v0.7 Heavy-Run And Htslib Optimization Benchmark

## Status

Pending implementation and measured runs.

## Required Report Fields

Each measured row must include correctness result, runtime mean, runtime stddev, speedup, variants/sec, peak RSS, dataset source, dataset shape, exact VCF-Fast command, exact competitor command, competitor version, bottleneck, caveat, and next action.

## Path Classes

| path class | current intent | caveat |
|---|---|---|
| native-filter | keep as the winning core | only claim wins from measured correctness-matched rows |
| native-tsv | measure selected-column export | columnar export is later |
| htslib-region-filter | compatibility region filter | not byte-preserving |
| htslib-region-tsv | indexed TSV compatibility | known v0.6 lag path |
| htslib-region-stats | indexed stats compatibility | only overlapping stats parity claimed |
| bcf-filter | BCF compatibility | v0.6 correctness matched but speed lagged |
| bcf-tsv | BCF TSV compatibility | preserve normalized bcftools query rows |
| bgzf-output | indexable BGZF output | measure compression/write overhead |
| public-heavy | large public evidence | avoid giant plain IGSR intermediates |
```

- [ ] **Step 7: Run tests**

Run:

```bash
cargo test --test benchmark_harness_tests v07_report_tracks
bash -n benchmark/run_benchmarks.sh
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add benchmark/run_benchmarks.sh benchmark/reports/v07-heavy-run-benchmark.md tests/benchmark_harness_tests.rs
git commit -m "docs: scaffold v0.7 heavy benchmark report"
```

## Task 6: Smoke-Test Public Heavy Without Large Plain Artifacts

**Files:**
- Modify: `tests/benchmark_harness_tests.rs`
- Modify: `benchmark/run_benchmarks.sh`

- [ ] **Step 1: Add static no-plain-heavy-artifact assertion**

Add to `tests/benchmark_harness_tests.rs`:

```rust
#[test]
fn public_heavy_mode_does_not_reuse_plain_public_whole_builder() {
    let harness = std::fs::read_to_string("benchmark/run_benchmarks.sh").unwrap();
    let heavy_start = harness.find("build_public_heavy_dataset").unwrap();
    let whole_builder = harness.find("build_public_small_dataset").unwrap();
    assert!(heavy_start > whole_builder);
    assert!(harness.contains("bgzip -c \"$temp_plain\" >\"$output\""));
    assert!(harness.contains("rm -f \"$temp_plain\""));
}
```

- [ ] **Step 2: Run static smoke**

Run:

```bash
cargo test --test benchmark_harness_tests public_heavy_mode_does_not_reuse
```

Expected: PASS after Task 4 implementation.

- [ ] **Step 3: Run tiny public-heavy smoke if public data exists**

Run:

```bash
if test -s tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz; then
  VCF_FAST_BENCH_MODE=public-heavy \
  VCF_FAST_BENCH_SIZES="100" \
  VCF_FAST_BENCH_RUNS=1 \
  VCF_FAST_BENCH_WARMUP=0 \
  VCF_FAST_HEAVY_MAX_PLAIN_BYTES=20000000 \
  ./benchmark/run_benchmarks.sh
fi
```

Expected when public data exists: command exits 0 and writes `tests/output/benchmark-results/benchmark-report.md`.

- [ ] **Step 4: Check no large plain heavy files remain**

Run:

```bash
find tests/output/benchmark-results/data -name 'public-heavy-*.plain.tmp.vcf' -print
```

Expected: no output.

- [ ] **Step 5: Commit any harness fixes**

If Step 3 or Step 4 required changes:

```bash
git add benchmark/run_benchmarks.sh tests/benchmark_harness_tests.rs
git commit -m "test: smoke public heavy benchmark mode"
```

If no changes were needed, do not create an empty commit.

## Task 7: Run Verification And Update Docs Conservatively

**Files:**
- Modify: `README.md`
- Modify: `docs/contribution-map.md`
- Modify: `benchmark/reports/v07-heavy-run-benchmark.md`

- [ ] **Step 1: Run full verification**

Run:

```bash
make verify
cargo test --features htslib-static
```

Expected: PASS.

- [ ] **Step 2: Run focused benchmark smoke**

Run:

```bash
VCF_FAST_BENCH_MODE=public-heavy \
VCF_FAST_BENCH_SIZES="100" \
VCF_FAST_BENCH_RUNS=1 \
VCF_FAST_BENCH_WARMUP=0 \
make bench-smoke
```

Expected if public data is present: PASS. If public data is absent: command exits with a clear missing-data message; do not claim measured heavy-run evidence.

- [ ] **Step 3: Update report status from measured smoke only**

If Step 2 passed, add this to `benchmark/reports/v07-heavy-run-benchmark.md` under `## Status`:

```markdown
Tiny `public-heavy` smoke completed locally. This proves the harness path and artifact cleanup, not a performance claim.
```

If Step 2 did not run because public data was absent, add:

```markdown
Tiny `public-heavy` smoke was not run because cached IGSR public data was unavailable. No v0.7 performance claim is made.
```

- [ ] **Step 4: Update README roadmap without performance claims**

In `README.md`, add or update the v0.7 milestone line:

```markdown
7. `v0.7 Heavy-Run And Htslib Optimization`: avoid giant public-data intermediates, tune htslib compatibility paths, and report path-specific bottlenecks before broader claims.
```

Do not add speedup numbers unless full measured benchmark rows exist.

- [ ] **Step 5: Update contribution map next targets**

In `docs/contribution-map.md`, update the first next contribution target to:

```markdown
1. Complete v0.7 heavy-run benchmarking without giant plain IGSR intermediates and optimize htslib-backed TSV/stats/BCF/BGZF paths where measurements show low-risk wins.
```

- [ ] **Step 6: Run doc/static tests**

Run:

```bash
cargo test --test benchmark_harness_tests
bash -n benchmark/run_benchmarks.sh
```

Expected: PASS.

- [ ] **Step 7: Commit docs**

```bash
git add README.md docs/contribution-map.md benchmark/reports/v07-heavy-run-benchmark.md
git commit -m "docs: update v0.7 heavy-run roadmap"
```

## Task 8: Final Verification And Clean Worktable

**Files:**
- Verify all changed files.

- [ ] **Step 1: Run required verification**

Run:

```bash
make verify
cargo test --features htslib-static
cargo test --test benchmark_harness_tests
```

Expected: PASS.

- [ ] **Step 2: Clean generated local artifacts**

Run:

```bash
rm -rf benchmark/__pycache__
find tests/output/benchmark-results -type f ! -name '*benchmark.md' -delete
find tests/output/benchmark-results -type d -empty -delete
```

Expected: ignored generated files are removed; tracked reports remain.

- [ ] **Step 3: Inspect git status**

Run:

```bash
git status --short --branch
```

Expected: clean worktree on the implementation branch, ahead by the planned commits.

- [ ] **Step 4: Summarize measured state**

Prepare final notes with:

```text
- verification commands that passed
- whether public-heavy smoke ran or was skipped
- whether any speed claim was added
- remaining bottlenecks for full public-heavy runs
```

Do not say "best VCF tool" as a present-tense claim. Say this milestone moves VCF-Fast toward that goal by strengthening evidence and compatibility-path performance.
