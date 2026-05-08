# VariantFlow Blazing-Fast v2.3-v3.0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the next evidence-gated VariantFlow speed stack: true native BGZF pipeline, smarter `.vfi` planning, bit-packed genotype kernels, columnar pushdown, profiler-guided parser surgery, and a final public evidence pass.

**Architecture:** Keep the existing Rust-native selective filter as the fast default. Add isolated internal modules for BGZF pipeline scheduling, index planning, compact genotype storage, columnar metadata, and parser helpers so each accelerator can fall back safely when its preconditions are not met.

**Tech Stack:** Rust, Cargo, existing `flate2`/BGZF helpers, existing `memchr`, Parquet/Arrow crates already in the project, shell/Python benchmark harnesses, `bcftools`, VCFtools, DuckDB, Docker/Linux resource measurement.

---

## File Structure

- Modify `src/io.rs`: native compressed-input detection, BGZF reader entrypoint, environment defaults.
- Modify `src/engine/filter.rs`: scheduler selection, batch predicate execution, ordered output integration.
- Modify `src/engine/index/bgzf.rs`: BGZF block source, virtual offset accounting, reusable block metadata.
- Modify `src/engine/index/metadata.rs`: richer `.vfi` chunk statistics.
- Modify `src/engine/index/planner.rs`: expression-to-index planning, skip/fallback reasons.
- Modify `src/engine/index/schema.rs`: serialized schema version and new fields.
- Modify `src/engine/index.rs`: public index build/read glue and report summaries.
- Modify `src/expr/mod.rs`: expose requirement discovery for index planning and specialized evaluator plans.
- Modify `src/vcf.rs`: record/INFO/FORMAT byte helper integration for pipeline and packed genotype consumers.
- Modify `src/engine/popgen.rs`: packed genotype core integration, LD memory optimization, shared summary kernels.
- Modify `src/engine/convert.rs`: Parquet row-group and metadata improvements.
- Create `src/engine/pipeline.rs`: bounded ordered BGZF/predicate pipeline internals.
- Create `src/engine/genotype.rs`: compact diploid biallelic genotype/dosage representation.
- Create `src/engine/parser.rs`: profiler-gated parser helper API with scalar fallback.
- Modify `src/engine/mod.rs`: module exports for `pipeline`, `genotype`, and `parser`.
- Modify `tests/filter_cli_tests.rs`: native mode output equivalence tests.
- Modify `tests/index_cli_tests.rs`: `.vfi` planner correctness and fallback tests.
- Modify `tests/popgen_cli_tests.rs`: packed genotype and LD parity tests.
- Modify `tests/convert_cli_tests.rs`: Parquet metadata and query-equivalence tests.
- Modify `tests/benchmark_harness_tests.rs`: new benchmark target/report-field assertions.
- Modify `benchmark/run_v22_scheduler_benchmarks.sh`: keep as baseline and add references to v2.3.
- Create `benchmark/run_v23_bgzf_pipeline_benchmarks.sh`: pipeline evidence harness.
- Create `benchmark/run_v24_index_planner_benchmarks.sh`: high-skip/low-skip `.vfi` benchmark harness.
- Create `benchmark/run_v25_packed_genotype_benchmarks.sh`: LD/popgen RSS and runtime harness.
- Create `benchmark/run_v26_columnar_pushdown_benchmarks.sh`: repeated query and break-even harness.
- Create `benchmark/run_v27_parser_profile.sh`: profiler collection script.
- Create `benchmark/run_v28_big_evidence_pass.sh`: public evidence campaign wrapper.
- Create reports under `benchmark/reports/`: `v23-bgzf-pipeline-benchmark.md`, `v24-index-planner-benchmark.md`, `v25-packed-genotype-benchmark.md`, `v26-columnar-pushdown-benchmark.md`, `v27-parser-profile.md`, `v28-big-evidence-pass.md`.
- Modify `Makefile`: add `bench-v23-pipeline`, `bench-v24-index`, `bench-v25-genotype`, `bench-v26-columnar`, `bench-v27-profile`, `bench-v28-evidence`.
- Modify `README.md`, `docs/claim-matrix.md`, and `docs/public-benchmark-table.md` only after measured correctness-matched evidence exists.

## Global Guardrails

- [ ] Work on one milestone branch at a time using branch names such as `codex/v23-bgzf-pipeline`.
- [ ] Commit each coherent task slice before dispatching the next worker.
- [ ] Do not update README performance wording from smoke rows.
- [ ] Do not change public CLI syntax unless the plan explicitly names a new command.
- [ ] Keep benchmark artifacts under ignored `tests/output/...`.
- [ ] Run full verification before merging each milestone branch.

## Task 1: v2.3 Pipeline Failing Tests

**Files:**
- Modify: `tests/filter_cli_tests.rs`
- Modify: `tests/benchmark_harness_tests.rs`
- Modify: `src/engine/mod.rs`

- [ ] **Step 1: Add native pipeline mode equivalence test**

Add a test to `tests/filter_cli_tests.rs` near the existing parallel/native equivalence tests:

```rust
#[test]
fn bgzf_pipeline_output_matches_default_native_byte_for_byte() {
    let input = fixture_path("stress_small.vcf");
    let gz_input = output_path("pipeline-equivalence-input.vcf.gz");
    gzip_fixture(&input, &gz_input);

    let default_out = output_path("pipeline-default.vcf");
    let pipeline_out = output_path("pipeline-forced.vcf");

    run_variantflow([
        "filter",
        gz_input.to_str().unwrap(),
        "--where",
        "ANY(FORMAT/AD > 80)",
        "-o",
        default_out.to_str().unwrap(),
    ])
    .success();

    run_variantflow_with_env(
        [
            "filter",
            gz_input.to_str().unwrap(),
            "--where",
            "ANY(FORMAT/AD > 80)",
            "-o",
            pipeline_out.to_str().unwrap(),
        ],
        [
            ("VCF_FAST_NATIVE_BGZF_THREADS", "4"),
            ("VCF_FAST_NATIVE_FILTER_THREADS", "4"),
            ("VCF_FAST_NATIVE_FILTER_BATCH_RECORDS", "128"),
            ("VCF_FAST_NATIVE_FILTER_QUEUE_BATCHES", "2"),
        ],
    )
    .success();

    let default_bytes = std::fs::read(default_out).unwrap();
    let pipeline_bytes = std::fs::read(pipeline_out).unwrap();
    assert_eq!(default_bytes, pipeline_bytes);
}
```

- [ ] **Step 2: Add benchmark harness assertions**

Add a test to `tests/benchmark_harness_tests.rs`:

```rust
#[test]
fn v23_pipeline_benchmark_harness_declares_required_modes() {
    let makefile = std::fs::read_to_string("Makefile").unwrap();
    assert!(makefile.contains("bench-v23-pipeline"));

    let script = std::fs::read_to_string("benchmark/run_v23_bgzf_pipeline_benchmarks.sh")
        .unwrap_or_default();
    assert!(script.contains("forced-single"));
    assert!(script.contains("bgzf-only"));
    assert!(script.contains("predicate-only"));
    assert!(script.contains("combined-pipeline"));
    assert!(script.contains("bcftools filter"));
    assert!(script.contains("peak RSS KB"));
    assert!(script.contains("correctness result"));
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run:

```bash
cargo test --test filter_cli_tests bgzf_pipeline_output_matches_default_native_byte_for_byte
cargo test --test benchmark_harness_tests v23_pipeline_benchmark_harness_declares_required_modes
```

Expected: the filter test fails until helpers exist or the pipeline env is wired; the harness test fails because the v2.3 script/Make target do not exist.

- [ ] **Step 4: Commit failing tests**

```bash
git add tests/filter_cli_tests.rs tests/benchmark_harness_tests.rs
git commit -m "test: define v23 bgzf pipeline expectations"
```

## Task 2: v2.3 Pipeline Module Skeleton And Scalar Fallback

**Files:**
- Create: `src/engine/pipeline.rs`
- Modify: `src/engine/mod.rs`
- Modify: `src/engine/filter.rs`

- [ ] **Step 1: Add pipeline data types**

Create `src/engine/pipeline.rs`:

```rust
use std::collections::BTreeMap;
use std::io::{self, Write};

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub bgzf_threads: usize,
    pub filter_threads: usize,
    pub batch_records: usize,
    pub queue_batches: usize,
}

impl PipelineConfig {
    pub fn enabled(&self) -> bool {
        self.bgzf_threads > 1 || self.filter_threads > 1
    }

    pub fn bounded_capacity(&self) -> usize {
        self.batch_records.saturating_mul(self.queue_batches.max(1))
    }
}

#[derive(Debug, Clone)]
pub struct RecordBatch {
    pub sequence: u64,
    pub bytes: Vec<u8>,
    pub record_count: usize,
}

#[derive(Debug, Clone)]
pub struct AcceptedBatch {
    pub sequence: u64,
    pub bytes: Vec<u8>,
}

pub fn write_ordered_batches<W, I>(mut writer: W, batches: I) -> io::Result<()>
where
    W: Write,
    I: IntoIterator<Item = AcceptedBatch>,
{
    let mut next = 0_u64;
    let mut pending: BTreeMap<u64, Vec<u8>> = BTreeMap::new();
    for batch in batches {
        pending.insert(batch.sequence, batch.bytes);
        while let Some(bytes) = pending.remove(&next) {
            writer.write_all(&bytes)?;
            next += 1;
        }
    }
    if pending.is_empty() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "pipeline ended with non-contiguous output batches",
        ))
    }
}
```

- [ ] **Step 2: Export the module**

Add this line to `src/engine/mod.rs`:

```rust
pub mod pipeline;
```

- [ ] **Step 3: Wire config parsing without behavior change**

In `src/engine/filter.rs`, add a small helper near existing environment helpers:

```rust
fn native_pipeline_config_from_env() -> crate::engine::pipeline::PipelineConfig {
    crate::engine::pipeline::PipelineConfig {
        bgzf_threads: native_bgzf_threads_from_env(),
        filter_threads: native_filter_threads_from_env(),
        batch_records: native_filter_batch_records_from_env(),
        queue_batches: native_filter_queue_batches_from_env(),
    }
}
```

Use the helper only for logging or branch selection that still calls the existing implementation. This keeps the tests failing only where the benchmark harness is absent and avoids changing behavior prematurely.

- [ ] **Step 4: Add unit tests for ordered writer**

Add to `src/engine/pipeline.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_writer_flushes_batches_in_sequence_order() {
        let mut out = Vec::new();
        write_ordered_batches(
            &mut out,
            [
                AcceptedBatch { sequence: 1, bytes: b"b\n".to_vec() },
                AcceptedBatch { sequence: 0, bytes: b"a\n".to_vec() },
                AcceptedBatch { sequence: 2, bytes: b"c\n".to_vec() },
            ],
        )
        .unwrap();
        assert_eq!(out, b"a\nb\nc\n");
    }
}
```

- [ ] **Step 5: Run targeted tests**

```bash
cargo test pipeline::tests::ordered_writer_flushes_batches_in_sequence_order
cargo test --test filter_cli_tests bgzf_pipeline_output_matches_default_native_byte_for_byte
```

Expected: ordered writer passes; pipeline equivalence still uses current behavior and should pass if existing env paths already preserve output.

- [ ] **Step 6: Commit skeleton**

```bash
git add src/engine/pipeline.rs src/engine/mod.rs src/engine/filter.rs
git commit -m "feat: add native pipeline coordination primitives"
```

## Task 3: v2.3 BGZF Block Source And Record Batch Splitter

**Files:**
- Modify: `src/engine/index/bgzf.rs`
- Modify: `src/engine/pipeline.rs`
- Test: `tests/filter_cli_tests.rs`

- [ ] **Step 1: Add block and split data structures**

In `src/engine/pipeline.rs`, add:

```rust
#[derive(Debug, Clone)]
pub struct DecodedBlock {
    pub block_sequence: u64,
    pub virtual_offset: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct LineCarry {
    pending: Vec<u8>,
    next_batch_sequence: u64,
}

impl LineCarry {
    pub fn push_block(&mut self, block: DecodedBlock, batch_records: usize) -> Vec<RecordBatch> {
        self.pending.extend_from_slice(&block.bytes);
        self.drain_complete_batches(batch_records)
    }

    pub fn finish(mut self) -> Vec<RecordBatch> {
        if self.pending.is_empty() {
            return Vec::new();
        }
        let sequence = self.next_batch_sequence;
        self.next_batch_sequence += 1;
        vec![RecordBatch {
            sequence,
            record_count: self.pending.iter().filter(|&&b| b == b'\n').count(),
            bytes: std::mem::take(&mut self.pending),
        }]
    }

    fn drain_complete_batches(&mut self, batch_records: usize) -> Vec<RecordBatch> {
        let target = batch_records.max(1);
        let mut batches = Vec::new();
        let mut start = 0_usize;
        let mut records = 0_usize;
        for (idx, byte) in self.pending.iter().enumerate() {
            if *byte == b'\n' {
                records += 1;
                if records >= target {
                    let end = idx + 1;
                    let bytes = self.pending[start..end].to_vec();
                    batches.push(RecordBatch {
                        sequence: self.next_batch_sequence,
                        bytes,
                        record_count: records,
                    });
                    self.next_batch_sequence += 1;
                    start = end;
                    records = 0;
                }
            }
        }
        if start > 0 {
            self.pending.drain(..start);
        }
        batches
    }
}
```

- [ ] **Step 2: Add line carry tests**

In `src/engine/pipeline.rs`, extend the test module:

```rust
#[test]
fn line_carry_preserves_lines_across_blocks() {
    let mut carry = LineCarry::default();
    let first = carry.push_block(
        DecodedBlock { block_sequence: 0, virtual_offset: 0, bytes: b"a\nb".to_vec() },
        2,
    );
    assert!(first.is_empty());
    let second = carry.push_block(
        DecodedBlock { block_sequence: 1, virtual_offset: 1, bytes: b"\nc\n".to_vec() },
        2,
    );
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].bytes, b"a\nb\n");
    let final_batches = carry.finish();
    assert_eq!(final_batches[0].bytes, b"c\n");
}
```

- [ ] **Step 3: Add BGZF decoded block iterator**

In `src/engine/index/bgzf.rs`, expose a function that yields decoded blocks from an existing BGZF input using the project’s current BGZF parsing primitives:

```rust
pub fn decoded_bgzf_blocks(path: &std::path::Path) -> std::io::Result<Vec<crate::engine::pipeline::DecodedBlock>> {
    let blocks = read_bgzf_blocks(path)?;
    let mut decoded = Vec::with_capacity(blocks.len());
    for (idx, block) in blocks.into_iter().enumerate() {
        decoded.push(crate::engine::pipeline::DecodedBlock {
            block_sequence: idx as u64,
            virtual_offset: block.virtual_offset,
            bytes: block.uncompressed_data,
        });
    }
    Ok(decoded)
}
```

If the existing BGZF helper uses different field names, adapt only inside this function and keep the exported `DecodedBlock` shape unchanged.

- [ ] **Step 4: Run targeted tests**

```bash
cargo test pipeline::tests::line_carry_preserves_lines_across_blocks
cargo test --test filter_cli_tests bgzf_pipeline_output_matches_default_native_byte_for_byte
```

- [ ] **Step 5: Commit block splitting**

```bash
git add src/engine/index/bgzf.rs src/engine/pipeline.rs tests/filter_cli_tests.rs
git commit -m "feat: split decoded bgzf blocks into ordered record batches"
```

## Task 4: v2.3 Bounded Predicate Pipeline Integration

**Files:**
- Modify: `src/engine/pipeline.rs`
- Modify: `src/engine/filter.rs`
- Test: `tests/filter_cli_tests.rs`

- [ ] **Step 1: Add predicate batch runner API**

In `src/engine/pipeline.rs`, add:

```rust
pub fn evaluate_batches_ordered<F>(
    batches: Vec<RecordBatch>,
    filter_threads: usize,
    mut evaluate_batch: F,
) -> io::Result<Vec<AcceptedBatch>>
where
    F: FnMut(&RecordBatch) -> io::Result<Vec<u8>>,
{
    if filter_threads <= 1 || batches.len() <= 1 {
        let mut accepted = Vec::with_capacity(batches.len());
        for batch in &batches {
            accepted.push(AcceptedBatch {
                sequence: batch.sequence,
                bytes: evaluate_batch(batch)?,
            });
        }
        return Ok(accepted);
    }

    let mut accepted = Vec::with_capacity(batches.len());
    for batch in &batches {
        accepted.push(AcceptedBatch {
            sequence: batch.sequence,
            bytes: evaluate_batch(batch)?,
        });
    }
    accepted.sort_by_key(|batch| batch.sequence);
    Ok(accepted)
}
```

This first implementation is order-safe and intentionally conservative. A later step can replace the loop with `std::thread::scope` or the existing worker pool once tests pin behavior.

- [ ] **Step 2: Extract existing per-line filter evaluation**

In `src/engine/filter.rs`, extract the existing native record evaluation loop into a helper with this shape:

```rust
fn filter_record_batch_bytes(
    batch_bytes: &[u8],
    expression: &crate::expr::Expression,
) -> std::io::Result<Vec<u8>> {
    let mut accepted = Vec::with_capacity(batch_bytes.len());
    for line in batch_bytes.split_inclusive(|byte| *byte == b'\n') {
        if line.starts_with(b"#") {
            accepted.extend_from_slice(line);
            continue;
        }
        let record = crate::vcf::RecordView::parse(line)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))?;
        if expression.evaluate_record_view(&record) {
            accepted.extend_from_slice(line);
        }
    }
    Ok(accepted)
}
```

Use the project’s actual expression and record-view method names if they differ. Keep behavior identical to the current native path.

- [ ] **Step 3: Add pipeline branch for BGZF input**

In `src/engine/filter.rs`, when native input is BGZF and `native_pipeline_config_from_env().enabled()` is true:

```rust
let config = native_pipeline_config_from_env();
if is_native_bgzf_input(input_path) && config.enabled() {
    return filter_native_bgzf_pipeline(input_path, output_path, expression, &config);
}
```

Implement `filter_native_bgzf_pipeline` in the same file:

```rust
fn filter_native_bgzf_pipeline(
    input_path: &std::path::Path,
    output_path: &std::path::Path,
    expression: &crate::expr::Expression,
    config: &crate::engine::pipeline::PipelineConfig,
) -> std::io::Result<()> {
    let blocks = crate::engine::index::bgzf::decoded_bgzf_blocks(input_path)?;
    let mut carry = crate::engine::pipeline::LineCarry::default();
    let mut batches = Vec::new();
    for block in blocks {
        batches.extend(carry.push_block(block, config.batch_records));
    }
    batches.extend(carry.finish());

    let accepted = crate::engine::pipeline::evaluate_batches_ordered(
        batches,
        config.filter_threads,
        |batch| filter_record_batch_bytes(&batch.bytes, expression),
    )?;

    let mut writer = crate::io::create_output_writer(output_path)?;
    crate::engine::pipeline::write_ordered_batches(&mut writer, accepted)
}
```

Keep output compression behavior aligned with existing native output writer.

- [ ] **Step 4: Run equivalence tests**

```bash
cargo test --test filter_cli_tests bgzf_pipeline_output_matches_default_native_byte_for_byte
cargo test --test filter_cli_tests
```

Expected: all native filter tests pass; any failure means the pipeline is not preserving header/record bytes.

- [ ] **Step 5: Commit integration**

```bash
git add src/engine/filter.rs src/engine/pipeline.rs
git commit -m "feat: route native bgzf filters through ordered pipeline"
```

## Task 5: v2.3 True Parallel Workers And Benchmark Harness

**Files:**
- Modify: `src/engine/pipeline.rs`
- Create: `benchmark/run_v23_bgzf_pipeline_benchmarks.sh`
- Modify: `Makefile`
- Create: `benchmark/reports/v23-bgzf-pipeline-benchmark.md`
- Modify: `tests/benchmark_harness_tests.rs`

- [ ] **Step 1: Replace conservative evaluator with scoped workers**

In `src/engine/pipeline.rs`, replace the parallel branch of `evaluate_batches_ordered` with bounded chunked workers:

```rust
let worker_count = filter_threads.max(1).min(batches.len());
let chunk_size = batches.len().div_ceil(worker_count);
let mut results: Vec<AcceptedBatch> = Vec::with_capacity(batches.len());

std::thread::scope(|scope| {
    let mut handles = Vec::new();
    for chunk in batches.chunks(chunk_size) {
        let local = chunk.to_vec();
        let mut local_eval = &mut evaluate_batch;
        handles.push(scope.spawn(move || -> io::Result<Vec<AcceptedBatch>> {
            let mut out = Vec::with_capacity(local.len());
            for batch in &local {
                out.push(AcceptedBatch {
                    sequence: batch.sequence,
                    bytes: local_eval(batch)?,
                });
            }
            Ok(out)
        }));
    }
    for handle in handles {
        results.extend(handle.join().expect("pipeline worker panicked")?);
    }
    Ok::<(), io::Error>(())
})?;

results.sort_by_key(|batch| batch.sequence);
Ok(results)
```

If Rust borrow rules reject sharing `evaluate_batch`, change the API to accept `Arc<dyn Fn(&RecordBatch) -> io::Result<Vec<u8>> + Send + Sync>` and update call sites accordingly.

- [ ] **Step 2: Add v2.3 benchmark script**

Create `benchmark/run_v23_bgzf_pipeline_benchmarks.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results/v23-bgzf-pipeline}"
REPORT="${VCF_FAST_BENCH_REPORT:-benchmark/reports/v23-bgzf-pipeline-benchmark.md}"
TIERS="${VCF_FAST_V23_TIERS:-100000 1000000}"
RUNS="${VCF_FAST_BENCH_RUNS:-3}"
WARMUP="${VCF_FAST_BENCH_WARMUP:-1}"
EXPR='ANY(FORMAT/AD > 80)'
BCFTOOLS_EXPR='N_PASS(FMT/AD[*:*]>80)>0'

mkdir -p "$OUT_DIR" "$(dirname "$REPORT")"
cargo build --release --bin variantflow

{
  echo "# VariantFlow v2.3 Native BGZF Pipeline Benchmark"
  echo
  echo "| dataset | records | mode | runtime mean/stddev | variants/sec | peak RSS KB | CPU seconds | correctness result | caveat |"
  echo "| --- | ---: | --- | --- | ---: | ---: | ---: | --- | --- |"
} > "$REPORT"

for records in $TIERS; do
  VCF_FAST_BENCH_MODE=stress VCF_FAST_BENCH_SIZES="$records" benchmark/run_benchmarks.sh >/dev/null
  input="tests/output/benchmark-results/data/stress-${records}.vcf.gz"
  default_out="$OUT_DIR/default-${records}.vcf"
  ./target/release/variantflow filter "$input" --where "$EXPR" -o "$default_out"

  for mode in forced-single bgzf-only predicate-only combined-pipeline bcftools; do
    case "$mode" in
      forced-single)
        cmd="./target/release/variantflow filter '$input' --where '$EXPR' -o /dev/null"
        env_prefix="VCF_FAST_NATIVE_BGZF_THREADS=1 VCF_FAST_NATIVE_FILTER_THREADS=1"
        ;;
      bgzf-only)
        cmd="./target/release/variantflow filter '$input' --where '$EXPR' -o /dev/null"
        env_prefix="VCF_FAST_NATIVE_BGZF_THREADS=6 VCF_FAST_NATIVE_FILTER_THREADS=1"
        ;;
      predicate-only)
        cmd="./target/release/variantflow filter '$input' --where '$EXPR' -o /dev/null"
        env_prefix="VCF_FAST_NATIVE_BGZF_THREADS=1 VCF_FAST_NATIVE_FILTER_THREADS=4"
        ;;
      combined-pipeline)
        cmd="./target/release/variantflow filter '$input' --where '$EXPR' -o /dev/null"
        env_prefix="VCF_FAST_NATIVE_BGZF_THREADS=6 VCF_FAST_NATIVE_FILTER_THREADS=4 VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=2048 VCF_FAST_NATIVE_FILTER_QUEUE_BATCHES=2"
        ;;
      bcftools)
        cmd="bcftools filter -i '$BCFTOOLS_EXPR' '$input' -o /dev/null"
        env_prefix=""
        ;;
    esac
    : "run $mode for $records records and append one row after hyperfine/resource/correctness collection"
  done
done
```

- [ ] **Step 3: Add Make target**

In `Makefile`:

```make
bench-v23-pipeline:
	bash benchmark/run_v23_bgzf_pipeline_benchmarks.sh
```

- [ ] **Step 4: Run harness syntax and tests**

```bash
chmod +x benchmark/run_v23_bgzf_pipeline_benchmarks.sh
bash -n benchmark/run_v23_bgzf_pipeline_benchmarks.sh
cargo test --test benchmark_harness_tests v23_pipeline_benchmark_harness_declares_required_modes
```

- [ ] **Step 5: Commit benchmark harness**

```bash
git add src/engine/pipeline.rs benchmark/run_v23_bgzf_pipeline_benchmarks.sh benchmark/reports/v23-bgzf-pipeline-benchmark.md Makefile tests/benchmark_harness_tests.rs
git commit -m "bench: add v23 native bgzf pipeline harness"
```

## Task 6: v2.3 Verification And Evidence Gate

**Files:**
- Modify after measured runs only: `benchmark/reports/v23-bgzf-pipeline-benchmark.md`
- Modify after measured wins only: `docs/claim-matrix.md`, `docs/public-benchmark-table.md`, `README.md`

- [ ] **Step 1: Run full correctness verification**

```bash
make verify
cargo test --features htslib-static
cargo clippy --features htslib-static --all-targets -- -D warnings
```

- [ ] **Step 2: Run smoke benchmark**

```bash
VCF_FAST_V23_TIERS="100" VCF_FAST_BENCH_RUNS=1 VCF_FAST_BENCH_WARMUP=0 make bench-v23-pipeline
```

Expected: report is generated, syntax is valid, and correctness fields are present.

- [ ] **Step 3: Run full benchmark on Linux/Docker**

```bash
VCF_FAST_V23_TIERS="100000 1000000" VCF_FAST_BENCH_RUNS=3 VCF_FAST_BENCH_WARMUP=1 make bench-v23-pipeline
```

- [ ] **Step 4: Update claims only from matched rows**

If correctness matches and measured results improve, update `docs/claim-matrix.md` and `docs/public-benchmark-table.md` with exact rows. Add README wording only if the result is public or stress evidence with repeated runs.

- [ ] **Step 5: Commit evidence**

```bash
git add benchmark/reports/v23-bgzf-pipeline-benchmark.md docs/claim-matrix.md docs/public-benchmark-table.md README.md
git commit -m "docs: record v23 bgzf pipeline evidence"
```

## Task 7: v2.4 `.vfi` Metadata Expansion Tests

**Files:**
- Modify: `tests/index_cli_tests.rs`
- Modify: `src/engine/index/metadata.rs`
- Modify: `src/engine/index/schema.rs`

- [ ] **Step 1: Add metadata fixture test**

Add to `tests/index_cli_tests.rs`:

```rust
#[test]
fn vfi_metadata_records_filter_info_and_position_ranges() {
    let input = fixture_path("example.vcf");
    let index = output_path("example.vcf.vfi");
    run_variantflow(["index", input.to_str().unwrap(), "-o", index.to_str().unwrap()]).success();

    let json = std::fs::read_to_string(index).unwrap();
    assert!(json.contains("\"chrom\""));
    assert!(json.contains("\"pos_min\""));
    assert!(json.contains("\"pos_max\""));
    assert!(json.contains("\"qual_min\""));
    assert!(json.contains("\"filter_tokens\""));
    assert!(json.contains("\"info_numeric\""));
}
```

- [ ] **Step 2: Run failure**

```bash
cargo test --test index_cli_tests vfi_metadata_records_filter_info_and_position_ranges
```

Expected: fails until metadata fields are serialized.

- [ ] **Step 3: Commit failing test**

```bash
git add tests/index_cli_tests.rs
git commit -m "test: require richer vfi chunk metadata"
```

## Task 8: v2.4 Metadata And Planner Implementation

**Files:**
- Modify: `src/engine/index/metadata.rs`
- Modify: `src/engine/index/schema.rs`
- Modify: `src/engine/index/planner.rs`
- Modify: `src/expr/mod.rs`
- Modify: `src/engine/index.rs`

- [ ] **Step 1: Add chunk metadata fields**

In `src/engine/index/metadata.rs`, extend the chunk summary type:

```rust
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct IndexedInfoNumeric {
    pub key: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub observed_values: u64,
    pub missing_values: u64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ChunkMetadata {
    pub chrom: String,
    pub pos_min: Option<u64>,
    pub pos_max: Option<u64>,
    pub qual_min: Option<f64>,
    pub qual_max: Option<f64>,
    pub qual_missing: u64,
    pub filter_tokens: std::collections::BTreeSet<String>,
    pub info_numeric: std::collections::BTreeMap<String, IndexedInfoNumeric>,
    pub format_keys: std::collections::BTreeSet<String>,
}
```

If the project already has `ChunkMetadata`, add these fields with serde defaults to preserve old index readability.

- [ ] **Step 2: Discover expression index requirements**

In `src/expr/mod.rs`, add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum IndexRequirement {
    Chrom,
    Pos,
    Qual,
    Filter,
    InfoNumeric(String),
    FormatKey(String),
}

pub fn index_requirements(expr: &Expression) -> std::collections::BTreeSet<IndexRequirement> {
    let mut requirements = std::collections::BTreeSet::new();
    collect_index_requirements(expr, &mut requirements);
    requirements
}
```

Implement `collect_index_requirements` by walking the existing AST variants and adding requirements for `CHROM`, `POS`, `QUAL`, `FILTER`, `INFO/<KEY>`, and `FORMAT/<KEY>`.

- [ ] **Step 3: Add planner result**

In `src/engine/index/planner.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkDecision {
    Scan { reason: String },
    Skip { reason: String },
    Fallback { reason: String },
}

#[derive(Debug, Clone, Default)]
pub struct PlanSummary {
    pub chunks_total: usize,
    pub chunks_scanned: usize,
    pub chunks_skipped: usize,
    pub fallback_reasons: Vec<String>,
}
```

Add planner functions that skip only when metadata proves impossibility, for example `QUAL > threshold` skips when `qual_max <= threshold`, and `INFO/DP > threshold` skips when indexed `DP.max <= threshold`.

- [ ] **Step 4: Run index tests**

```bash
cargo test --test index_cli_tests
```

- [ ] **Step 5: Commit `.vfi` planner**

```bash
git add src/engine/index/metadata.rs src/engine/index/schema.rs src/engine/index/planner.rs src/expr/mod.rs src/engine/index.rs tests/index_cli_tests.rs
git commit -m "feat: expand vfi metadata and safe planner decisions"
```

## Task 9: v2.4 Indexed Filter Pushdown And Benchmarks

**Files:**
- Modify: `src/engine/filter.rs`
- Create: `benchmark/run_v24_index_planner_benchmarks.sh`
- Create: `benchmark/reports/v24-index-planner-benchmark.md`
- Modify: `Makefile`
- Modify: `tests/benchmark_harness_tests.rs`

- [ ] **Step 1: Use planner in native filter**

In `src/engine/filter.rs`, before streaming all chunks, add guarded index lookup:

```rust
if let Some(index_path) = discover_vfi_for_input(input_path) {
    let plan = crate::engine::index::planner::plan_filter(input_path, &index_path, expression)?;
    if plan.can_use_index() {
        return filter_with_vfi_plan(input_path, output_path, expression, &plan);
    }
}
```

`filter_with_vfi_plan` must scan only chunks marked `Scan` and must preserve byte-for-byte output by reusing original record bytes. If any chunk has `Fallback`, call the existing native streaming function.

- [ ] **Step 2: Add tests for safe fallback**

In `tests/index_cli_tests.rs`:

```rust
#[test]
fn vfi_filter_falls_back_for_unprovable_format_predicate() {
    let input = fixture_path("format_example.vcf");
    let index = output_path("format_example.vcf.vfi");
    run_variantflow(["index", input.to_str().unwrap(), "-o", index.to_str().unwrap()]).success();

    let out = output_path("format-index-fallback.vcf");
    run_variantflow([
        "filter",
        input.to_str().unwrap(),
        "--where",
        "ANY(FORMAT/AD > 80)",
        "-o",
        out.to_str().unwrap(),
    ])
    .success();
    assert!(std::fs::metadata(out).unwrap().len() > 0);
}
```

- [ ] **Step 3: Add v2.4 benchmark script**

Create `benchmark/run_v24_index_planner_benchmarks.sh` with modes:

```bash
#!/usr/bin/env bash
set -euo pipefail
REPORT="${VCF_FAST_BENCH_REPORT:-benchmark/reports/v24-index-planner-benchmark.md}"
OUT_DIR="${VCF_FAST_BENCH_OUT_DIR:-tests/output/benchmark-results/v24-index-planner}"
mkdir -p "$OUT_DIR" "$(dirname "$REPORT")"
echo "# VariantFlow v2.4 Index Planner Benchmark" > "$REPORT"
echo "| dataset | predicate | chunks scanned | chunks skipped | skip percentage | runtime mean/stddev | correctness result | caveat |" >> "$REPORT"
echo "| --- | --- | ---: | ---: | ---: | --- | --- | --- |" >> "$REPORT"
echo "" >> "$REPORT"
```

- [ ] **Step 4: Add Make target and tests**

In `Makefile`:

```make
bench-v24-index:
	bash benchmark/run_v24_index_planner_benchmarks.sh
```

Run:

```bash
bash -n benchmark/run_v24_index_planner_benchmarks.sh
cargo test --test index_cli_tests
cargo test --test benchmark_harness_tests
```

- [ ] **Step 5: Commit indexed pushdown**

```bash
git add src/engine/filter.rs tests/index_cli_tests.rs benchmark/run_v24_index_planner_benchmarks.sh benchmark/reports/v24-index-planner-benchmark.md Makefile tests/benchmark_harness_tests.rs
git commit -m "feat: use vfi planner for safe indexed filtering"
```

## Task 10: v2.5 Packed Genotype Core Tests And Module

**Files:**
- Create: `src/engine/genotype.rs`
- Modify: `src/engine/mod.rs`
- Modify: `tests/popgen_cli_tests.rs`

- [ ] **Step 1: Add packed genotype module**

Create `src/engine/genotype.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackedDosage {
    HomRef,
    Het,
    HomAlt,
    Missing,
}

impl PackedDosage {
    pub fn from_gt(gt: &[u8]) -> Self {
        match gt {
            b"0/0" | b"0|0" => Self::HomRef,
            b"0/1" | b"1/0" | b"0|1" | b"1|0" => Self::Het,
            b"1/1" | b"1|1" => Self::HomAlt,
            _ => Self::Missing,
        }
    }

    pub fn dosage(self) -> Option<u8> {
        match self {
            Self::HomRef => Some(0),
            Self::Het => Some(1),
            Self::HomAlt => Some(2),
            Self::Missing => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PackedGenotypes {
    len: usize,
    words: Vec<u64>,
}

impl PackedGenotypes {
    pub fn from_dosages(values: &[PackedDosage]) -> Self {
        let mut words = vec![0_u64; values.len().div_ceil(32)];
        for (idx, value) in values.iter().enumerate() {
            let code = match value {
                PackedDosage::HomRef => 0_u64,
                PackedDosage::Het => 1_u64,
                PackedDosage::HomAlt => 2_u64,
                PackedDosage::Missing => 3_u64,
            };
            words[idx / 32] |= code << ((idx % 32) * 2);
        }
        Self { len: values.len(), words }
    }

    pub fn get(&self, idx: usize) -> Option<PackedDosage> {
        if idx >= self.len {
            return None;
        }
        let code = (self.words[idx / 32] >> ((idx % 32) * 2)) & 0b11;
        Some(match code {
            0 => PackedDosage::HomRef,
            1 => PackedDosage::Het,
            2 => PackedDosage::HomAlt,
            _ => PackedDosage::Missing,
        })
    }

    pub fn len(&self) -> usize {
        self.len
    }
}
```

- [ ] **Step 2: Export module**

In `src/engine/mod.rs`:

```rust
pub mod genotype;
```

- [ ] **Step 3: Add unit tests**

Append to `src/engine/genotype.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_genotypes_round_trip_diploid_dosages() {
        let values = [
            PackedDosage::HomRef,
            PackedDosage::Het,
            PackedDosage::HomAlt,
            PackedDosage::Missing,
        ];
        let packed = PackedGenotypes::from_dosages(&values);
        assert_eq!(packed.len(), 4);
        for (idx, expected) in values.iter().enumerate() {
            assert_eq!(packed.get(idx), Some(*expected));
        }
        assert_eq!(packed.get(4), None);
    }
}
```

- [ ] **Step 4: Run targeted tests**

```bash
cargo test genotype::tests::packed_genotypes_round_trip_diploid_dosages
```

- [ ] **Step 5: Commit packed core**

```bash
git add src/engine/genotype.rs src/engine/mod.rs
git commit -m "feat: add packed genotype core"
```

## Task 11: v2.5 LD Migration And VCFtools Parity

**Files:**
- Modify: `src/engine/popgen.rs`
- Modify: `tests/popgen_cli_tests.rs`
- Create: `benchmark/run_v25_packed_genotype_benchmarks.sh`
- Create: `benchmark/reports/v25-packed-genotype-benchmark.md`
- Modify: `Makefile`

- [ ] **Step 1: Replace LD window storage**

In `src/engine/popgen.rs`, replace LD per-site dosage vectors with:

```rust
struct LdWindowSite {
    chrom: String,
    pos: u64,
    genotypes: crate::engine::genotype::PackedGenotypes,
    called_count: u32,
    alt_sum: u32,
}
```

Add a converter from existing parsed sample GT bytes:

```rust
fn packed_site_from_sample_gts(sample_gts: &[&[u8]]) -> crate::engine::genotype::PackedGenotypes {
    let dosages: Vec<_> = sample_gts
        .iter()
        .map(|gt| crate::engine::genotype::PackedDosage::from_gt(gt))
        .collect();
    crate::engine::genotype::PackedGenotypes::from_dosages(&dosages)
}
```

- [ ] **Step 2: Update LD r2 calculation to read packed values**

Use `PackedGenotypes::get(idx).and_then(PackedDosage::dosage)` inside the existing VCFtools-compatible genotype-dosage `r2` calculation. Keep missing samples excluded exactly as before.

- [ ] **Step 3: Run VCFtools parity**

```bash
make vcftools-parity
cargo test --test popgen_cli_tests
```

- [ ] **Step 4: Add v2.5 benchmark script**

Create `benchmark/run_v25_packed_genotype_benchmarks.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
REPORT="${VCF_FAST_BENCH_REPORT:-benchmark/reports/v25-packed-genotype-benchmark.md}"
mkdir -p "$(dirname "$REPORT")"
echo "# VariantFlow v2.5 Packed Genotype Benchmark" > "$REPORT"
echo "| workflow | records | samples | runtime mean/stddev | VariantFlow peak RSS KB | VCFtools peak RSS KB | correctness result | caveat |" >> "$REPORT"
echo "| --- | ---: | ---: | --- | ---: | ---: | --- | --- |" >> "$REPORT"
echo "" >> "$REPORT"
```

Add to `Makefile`:

```make
bench-v25-genotype:
	bash benchmark/run_v25_packed_genotype_benchmarks.sh
```

- [ ] **Step 5: Commit LD migration**

```bash
git add src/engine/popgen.rs tests/popgen_cli_tests.rs benchmark/run_v25_packed_genotype_benchmarks.sh benchmark/reports/v25-packed-genotype-benchmark.md Makefile
git commit -m "perf: use packed genotypes for ld windows"
```

## Task 12: v2.6 Columnar Pushdown Benchmark Expansion

**Files:**
- Modify: `src/engine/convert.rs`
- Modify: `tests/convert_cli_tests.rs`
- Create: `benchmark/run_v26_columnar_pushdown_benchmarks.sh`
- Create: `benchmark/reports/v26-columnar-pushdown-benchmark.md`
- Modify: `benchmark/query_parquet_duckdb.py`
- Modify: `Makefile`

- [ ] **Step 1: Add Parquet row-group env control**

In `src/engine/convert.rs`, add:

```rust
fn parquet_row_group_size_from_env() -> usize {
    std::env::var("VCF_FAST_PARQUET_ROW_GROUP_RECORDS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(65_536)
}
```

Use the returned value where the Parquet writer batches rows.

- [ ] **Step 2: Add DuckDB query names**

In `benchmark/query_parquet_duckdb.py`, ensure these query labels exist:

```python
QUERIES = {
    "qual_gt_30": "SELECT count(*) FROM variants WHERE QUAL > 30",
    "info_dp_gt_40": "SELECT count(*) FROM variants WHERE INFO_DP > 40",
    "filter_pass": "SELECT count(*) FROM variants WHERE FILTER = 'PASS'",
    "chrom_filter_counts": "SELECT CHROM, FILTER, count(*) FROM variants GROUP BY CHROM, FILTER ORDER BY CHROM, FILTER",
    "qual_summary_by_filter": "SELECT FILTER, count(*), min(QUAL), max(QUAL), avg(QUAL) FROM variants GROUP BY FILTER ORDER BY FILTER",
}
```

Match the actual column names emitted by VariantFlow Parquet.

- [ ] **Step 3: Add conversion test**

In `tests/convert_cli_tests.rs`:

```rust
#[test]
fn parquet_export_accepts_row_group_env() {
    let input = fixture_path("example.vcf");
    let output = output_path("row-group.parquet");
    run_variantflow_with_env(
        ["convert", input.to_str().unwrap(), "--to", "parquet", "-o", output.to_str().unwrap()],
        [("VCF_FAST_PARQUET_ROW_GROUP_RECORDS", "2")],
    )
    .success();
    assert!(std::fs::metadata(output).unwrap().len() > 0);
}
```

- [ ] **Step 4: Add v2.6 benchmark script**

Create `benchmark/run_v26_columnar_pushdown_benchmarks.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
REPORT="${VCF_FAST_BENCH_REPORT:-benchmark/reports/v26-columnar-pushdown-benchmark.md}"
mkdir -p "$(dirname "$REPORT")"
echo "# VariantFlow v2.6 Columnar Pushdown Benchmark" > "$REPORT"
echo "| dataset | row group records | query | export time | query-only time | amortized time | break-even query count | correctness result | caveat |" >> "$REPORT"
echo "| --- | ---: | --- | ---: | ---: | ---: | ---: | --- | --- |" >> "$REPORT"
echo "" >> "$REPORT"
```

Add to `Makefile`:

```make
bench-v26-columnar:
	bash benchmark/run_v26_columnar_pushdown_benchmarks.sh
```

- [ ] **Step 5: Run checks and commit**

```bash
python3 -m py_compile benchmark/query_parquet_duckdb.py
bash -n benchmark/run_v26_columnar_pushdown_benchmarks.sh
cargo test --test convert_cli_tests parquet_export_accepts_row_group_env
git add src/engine/convert.rs tests/convert_cli_tests.rs benchmark/query_parquet_duckdb.py benchmark/run_v26_columnar_pushdown_benchmarks.sh benchmark/reports/v26-columnar-pushdown-benchmark.md Makefile
git commit -m "bench: expand columnar pushdown workflows"
```

## Task 13: v2.7 Profiling And Parser Helper Gate

**Files:**
- Create: `src/engine/parser.rs`
- Modify: `src/engine/mod.rs`
- Create: `benchmark/run_v27_parser_profile.sh`
- Create: `benchmark/reports/v27-parser-profile.md`
- Modify: `Makefile`

- [ ] **Step 1: Add parser helper module with scalar baseline**

Create `src/engine/parser.rs`:

```rust
pub fn find_byte(haystack: &[u8], needle: u8) -> Option<usize> {
    memchr::memchr(needle, haystack)
}

pub fn parse_u64_ascii(bytes: &[u8]) -> Option<u64> {
    if bytes.is_empty() {
        return None;
    }
    let mut value = 0_u64;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add((byte - b'0') as u64)?;
    }
    Some(value)
}
```

Export from `src/engine/mod.rs`:

```rust
pub mod parser;
```

- [ ] **Step 2: Add unit tests**

Append to `src/engine/parser.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_u64_ascii_rejects_empty_and_non_digits() {
        assert_eq!(parse_u64_ascii(b"12345"), Some(12345));
        assert_eq!(parse_u64_ascii(b""), None);
        assert_eq!(parse_u64_ascii(b"12a"), None);
    }
}
```

- [ ] **Step 3: Add profiling script**

Create `benchmark/run_v27_parser_profile.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
REPORT="${VCF_FAST_PROFILE_REPORT:-benchmark/reports/v27-parser-profile.md}"
mkdir -p "$(dirname "$REPORT")"
echo "# VariantFlow v2.7 Parser Profile" > "$REPORT"
echo >> "$REPORT"
echo "- profiler command: cargo instruments/perf/samply depending on platform availability" >> "$REPORT"
echo "- decision rule: parser helper changes proceed only when profile shows parsing hot loops after v2.3-v2.6" >> "$REPORT"
echo "- candidate loops: tabs, INFO delimiters, FORMAT delimiters, genotype separators, integers, floats" >> "$REPORT"
```

Add to `Makefile`:

```make
bench-v27-profile:
	bash benchmark/run_v27_parser_profile.sh
```

- [ ] **Step 4: Run tests and commit**

```bash
cargo test parser::tests::parse_u64_ascii_rejects_empty_and_non_digits
bash -n benchmark/run_v27_parser_profile.sh
git add src/engine/parser.rs src/engine/mod.rs benchmark/run_v27_parser_profile.sh benchmark/reports/v27-parser-profile.md Makefile
git commit -m "chore: add parser helper and profiling gate"
```

## Task 14: v2.8 Big Evidence Harness

**Files:**
- Create: `benchmark/run_v28_big_evidence_pass.sh`
- Create: `benchmark/reports/v28-big-evidence-pass.md`
- Modify: `Makefile`
- Modify: `tests/benchmark_harness_tests.rs`

- [ ] **Step 1: Add v2.8 evidence script**

Create `benchmark/run_v28_big_evidence_pass.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
REPORT="${VCF_FAST_BENCH_REPORT:-benchmark/reports/v28-big-evidence-pass.md}"
mkdir -p "$(dirname "$REPORT")"
echo "# VariantFlow v2.8 Big Evidence Pass" > "$REPORT"
echo "| evidence family | datasets | workflows | required correctness | status |" >> "$REPORT"
echo "| --- | --- | --- | --- | --- |" >> "$REPORT"
echo "| IGSR BGZF filtering | public chr22 tiers | default, pipeline, vfi high-skip, bcftools | byte-for-byte native and bcftools core records | run-required |" >> "$REPORT"
echo "| CHM13 FORMAT-rich | bounded human tiers | aggregate, selected-sample, mixed predicates | bcftools core records | run-required |" >> "$REPORT"
echo "| VCFtools popgen | true public population cohort | freq, missingness, HWE, het, pi, TajimaD, LD, Fst | normalized VCFtools parity | run-required |" >> "$REPORT"
echo "| Columnar workflows | public Parquet export | DuckDB repeated queries | normalized VCF/bcftools baselines | run-required |" >> "$REPORT"
```

- [ ] **Step 2: Add Make target**

In `Makefile`:

```make
bench-v28-evidence:
	bash benchmark/run_v28_big_evidence_pass.sh
```

- [ ] **Step 3: Add harness test**

In `tests/benchmark_harness_tests.rs`:

```rust
#[test]
fn v28_big_evidence_harness_lists_required_families() {
    let script = std::fs::read_to_string("benchmark/run_v28_big_evidence_pass.sh").unwrap();
    assert!(script.contains("IGSR BGZF filtering"));
    assert!(script.contains("CHM13 FORMAT-rich"));
    assert!(script.contains("VCFtools popgen"));
    assert!(script.contains("Columnar workflows"));
    assert!(script.contains("bcftools core records"));
    assert!(script.contains("normalized VCFtools parity"));
}
```

- [ ] **Step 4: Run checks and commit**

```bash
bash -n benchmark/run_v28_big_evidence_pass.sh
cargo test --test benchmark_harness_tests v28_big_evidence_harness_lists_required_families
git add benchmark/run_v28_big_evidence_pass.sh benchmark/reports/v28-big-evidence-pass.md Makefile tests/benchmark_harness_tests.rs
git commit -m "bench: add v28 big evidence pass harness"
```

## Task 15: v3.0 Public Candidate Docs Gate

**Files:**
- Modify: `README.md`
- Modify: `docs/claim-matrix.md`
- Modify: `docs/public-benchmark-table.md`
- Modify: `paper/bioinformatics/main.tex` if present
- Modify: `docs/release.md`

- [ ] **Step 1: Regenerate benchmark table**

```bash
make benchmark-table
```

- [ ] **Step 2: Update claim matrix from measured rows**

For each correctness-matched row in `benchmark/reports/v23-*` through `benchmark/reports/v28-*`, add a claim matrix row with:

```markdown
| workflow | status | evidence path | competitor | current result | caveat |
| --- | --- | --- | --- | --- | --- |
| Native BGZF pipeline filtering | beats/matches/needs optimization | `benchmark/reports/v23-bgzf-pipeline-benchmark.md` | `bcftools filter` | measured row only | exact caveat from report |
```

- [ ] **Step 3: Keep release paused unless gates pass**

In `docs/release.md`, keep release status as evidence-gated:

```markdown
VariantFlow release remains paused until the v2.8 evidence pass is reviewed and the claim matrix contains no unsupported broad replacement claims.
```

- [ ] **Step 4: Run docs checks**

```bash
rg "best VCF tool|broadly superior|drop-in replacement" README.md docs paper || true
make paper-check
make verify
```

Expected: no broad unsupported claim remains; paper check and verify pass.

- [ ] **Step 5: Commit docs**

```bash
git add README.md docs/claim-matrix.md docs/public-benchmark-table.md docs/release.md paper benchmark/reports
git commit -m "docs: refresh public candidate evidence gates"
```

## Final Milestone Verification

- [ ] Run:

```bash
make verify
cargo test --features htslib-static
cargo clippy --features htslib-static --all-targets -- -D warnings
make vcftools-parity
```

- [ ] Run smoke evidence commands:

```bash
VCF_FAST_V23_TIERS="100" VCF_FAST_BENCH_RUNS=1 VCF_FAST_BENCH_WARMUP=0 make bench-v23-pipeline
VCF_FAST_BENCH_RUNS=1 VCF_FAST_BENCH_WARMUP=0 make bench-v24-index
VCF_FAST_BENCH_RUNS=1 VCF_FAST_BENCH_WARMUP=0 make bench-v25-genotype
VCF_FAST_BENCH_RUNS=1 VCF_FAST_BENCH_WARMUP=0 make bench-v26-columnar
make bench-v27-profile
make bench-v28-evidence
```

- [ ] Run full evidence only on a machine prepared for public-data benchmark time and disk:

```bash
benchmark/download_public_data.sh all
VCF_FAST_BENCH_RUNS=3 VCF_FAST_BENCH_WARMUP=1 make bench-v23-pipeline
VCF_FAST_BENCH_RUNS=3 VCF_FAST_BENCH_WARMUP=1 make bench-v24-index
VCF_FAST_BENCH_RUNS=3 VCF_FAST_BENCH_WARMUP=1 make bench-v25-genotype
VCF_FAST_BENCH_RUNS=3 VCF_FAST_BENCH_WARMUP=1 make bench-v26-columnar
VCF_FAST_BENCH_RUNS=3 VCF_FAST_BENCH_WARMUP=1 make bench-v28-evidence
```

- [ ] Before merge, use `superpowers:requesting-code-review`.
- [ ] After review and fixes, use `superpowers:finishing-a-development-branch`.
