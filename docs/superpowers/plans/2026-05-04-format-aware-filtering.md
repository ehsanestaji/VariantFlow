# FORMAT-Aware Filtering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add v0.4 single-sample FORMAT-aware filtering for `FORMAT/GT`, `FORMAT/DP`, and `FORMAT/GQ`, with correctness and benchmark evidence against `bcftools`.

**Architecture:** Extend the current Rust-native streaming filter. Resolve the selected sample once from `#CHROM`, parse only required FORMAT keys from the selected sample column, and continue writing original passing records unchanged. Keep FORMAT support narrow and explicit.

**Tech Stack:** Rust 2024, `clap`, `anyhow`, existing expression parser, existing benchmark shell harness, `bcftools`, `hyperfine`, Docker.

---

## File Structure

- Modify `src/cli.rs`: add optional `--sample` to `filter`.
- Modify `src/expr/mod.rs`: add FORMAT fields, required FORMAT flags, and FORMAT predicate evaluation.
- Modify `src/vcf.rs`: add borrowed header/sample/FORMAT helpers.
- Modify `src/engine/filter.rs`: resolve selected sample and build FORMAT values per record only when required.
- Modify `benchmark/generate_stress_vcf.sh` and `benchmark/run_benchmarks.sh`: add FORMAT benchmark cases.
- Add `tests/data/format_example.vcf`: focused two-sample FORMAT fixture.
- Modify `tests/expr_tests.rs`, `tests/filter_cli_tests.rs`, and `tests/benchmark_harness_tests.rs`: cover parser, CLI behavior, and benchmark contract.
- Update `README.md`, `docs/contribution-map.md`, and benchmark reports only after measured FORMAT results exist.

## Task 1: FORMAT Expression Model

**Files:**
- Modify: `src/expr/mod.rs`
- Test: `tests/expr_tests.rs`

- [ ] **Step 1: Write failing expression tests**

Add this helper and tests to `tests/expr_tests.rs`:

```rust
use vcf_fast::expr::{EvalRecord, FormatValues, parse_expression};

fn record_with_format<'a>(format: FormatValues<'a>) -> EvalRecord<'a> {
    EvalRecord {
        chrom: "1",
        pos: 100,
        qual: Some(50.0),
        filter: "PASS",
        info: "DP=10;AF=0.1",
        format,
    }
}

#[test]
fn evaluates_format_numeric_predicates() {
    let expr = parse_expression("FORMAT/DP > 20 && FORMAT/GQ >= 30").unwrap();

    assert!(expr.evaluate(&record_with_format(FormatValues {
        gt: Some("0/1"),
        dp: Some("25"),
        gq: Some("40"),
    })));
    assert!(!expr.evaluate(&record_with_format(FormatValues {
        gt: Some("0/1"),
        dp: Some("10"),
        gq: Some("40"),
    })));
}

#[test]
fn evaluates_format_gt_as_exact_string() {
    let expr = parse_expression("FORMAT/GT == \"0/1\"").unwrap();

    assert!(expr.evaluate(&record_with_format(FormatValues {
        gt: Some("0/1"),
        dp: None,
        gq: None,
    })));
    assert!(!expr.evaluate(&record_with_format(FormatValues {
        gt: Some("0|1"),
        dp: None,
        gq: None,
    })));
}

#[test]
fn format_missing_or_invalid_numeric_values_are_false() {
    let expr = parse_expression("FORMAT/DP > 20").unwrap();

    assert!(!expr.evaluate(&record_with_format(FormatValues {
        gt: None,
        dp: None,
        gq: None,
    })));
    assert!(!expr.evaluate(&record_with_format(FormatValues {
        gt: None,
        dp: Some("."),
        gq: None,
    })));
    assert!(!expr.evaluate(&record_with_format(FormatValues {
        gt: None,
        dp: Some("not-a-number"),
        gq: None,
    })));
}

#[test]
fn rejects_bare_gq_and_requires_explicit_format_prefix() {
    let error = parse_expression("GQ > 20").unwrap_err().to_string();

    assert!(error.contains("unsupported field 'GQ'"));
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
cargo test --test expr_tests
```

Expected: compile failure because `FormatValues` and `EvalRecord::format` do not exist, or parse failure for `FORMAT/...`.

- [ ] **Step 3: Implement FORMAT expression types**

In `src/expr/mod.rs`:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RequiredFormatFields {
    pub gt: bool,
    pub dp: bool,
    pub gq: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RequiredFields {
    pub chrom: bool,
    pub pos: bool,
    pub qual: bool,
    pub filter: bool,
    pub info: bool,
    pub format: RequiredFormatFields,
}

impl RequiredFields {
    pub(crate) fn requires_format(&self) -> bool {
        self.format.gt || self.format.dp || self.format.gq
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FormatValues<'a> {
    pub gt: Option<&'a str>,
    pub dp: Option<&'a str>,
    pub gq: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct EvalRecord<'a> {
    pub chrom: &'a str,
    pub pos: u64,
    pub qual: Option<f64>,
    pub filter: &'a str,
    pub info: &'a str,
    pub format: FormatValues<'a>,
}
```

Extend `Field`:

```rust
enum Field {
    Chrom,
    Pos,
    Qual,
    Filter,
    Dp,
    Af,
    FormatGt,
    FormatDp,
    FormatGq,
}
```

Extend field parsing:

```rust
"FORMAT/GT" => Ok(Field::FormatGt),
"FORMAT/DP" => Ok(Field::FormatDp),
"FORMAT/GQ" => Ok(Field::FormatGq),
```

Extend evaluation:

```rust
(Field::FormatGt, Literal::String(expected)) => record
    .format
    .gt
    .is_some_and(|actual| compare_strings(actual, expected, self.op)),
(Field::FormatDp, Literal::Number(expected)) => record
    .format
    .dp
    .and_then(parse_format_number)
    .is_some_and(|actual| compare_numbers(actual, *expected, self.op)),
(Field::FormatGq, Literal::Number(expected)) => record
    .format
    .gq
    .and_then(parse_format_number)
    .is_some_and(|actual| compare_numbers(actual, *expected, self.op)),
```

Add helper:

```rust
fn parse_format_number(value: &str) -> Option<f64> {
    if value == "." || value.is_empty() {
        None
    } else {
        value.parse::<f64>().ok()
    }
}
```

Extend required-field collection:

```rust
Field::FormatGt => required.format.gt = true,
Field::FormatDp => required.format.dp = true,
Field::FormatGq => required.format.gq = true,
```

Update existing `EvalRecord` construction in tests and `src/engine/filter.rs` with `format: FormatValues::default()`.

- [ ] **Step 4: Run tests to verify GREEN**

Run:

```bash
cargo test --test expr_tests
```

Expected: all expression tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/expr/mod.rs tests/expr_tests.rs src/engine/filter.rs
git commit -m "feat: add format fields to expression engine"
```

## Task 2: Borrowed FORMAT And Sample Helpers

**Files:**
- Modify: `src/vcf.rs`

- [ ] **Step 1: Write failing unit tests**

Add tests in `src/vcf.rs`:

```rust
#[test]
fn resolves_sample_column_from_chrom_header() {
    let header = "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tHG002\tNA12878\n";

    assert_eq!(resolve_sample_column(header, "HG002").unwrap(), 9);
    assert_eq!(resolve_sample_column(header, "NA12878").unwrap(), 10);
}

#[test]
fn unknown_sample_reports_clear_error() {
    let header = "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tHG002\n";
    let error = resolve_sample_column(header, "MISSING").unwrap_err().to_string();

    assert!(error.contains("sample 'MISSING' not found in VCF header"));
}

#[test]
fn reads_absolute_record_columns_by_index() {
    let line = "1\t100\t.\tA\tG\t50\tPASS\t.\tGT:DP:GQ\t0/1:25:40\t0/0:5:10\n";

    assert_eq!(column_value(line, 8), Some("GT:DP:GQ"));
    assert_eq!(column_value(line, 9), Some("0/1:25:40"));
    assert_eq!(column_value(line, 10), Some("0/0:5:10"));
    assert_eq!(column_value(line, 11), None);
}

#[test]
fn extracts_selected_sample_format_values() {
    let values = selected_format_values(
        "GT:DP:GQ",
        "0/1:25:40",
        crate::expr::RequiredFormatFields {
            gt: true,
            dp: true,
            gq: true,
        },
    );

    assert_eq!(values.gt, Some("0/1"));
    assert_eq!(values.dp, Some("25"));
    assert_eq!(values.gq, Some("40"));
}

#[test]
fn missing_format_values_return_none() {
    let values = selected_format_values(
        "GT:DP:GQ",
        "0/1:.",
        crate::expr::RequiredFormatFields {
            gt: true,
            dp: true,
            gq: true,
        },
    );

    assert_eq!(values.gt, Some("0/1"));
    assert_eq!(values.dp, Some("."));
    assert_eq!(values.gq, None);
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
cargo test --lib vcf::tests::
```

Expected: compile failure because helper functions do not exist.

- [ ] **Step 3: Implement borrowed helpers**

In `src/vcf.rs`, import the expression types:

```rust
use crate::expr::{FormatValues, RequiredFormatFields};
```

Add helpers:

```rust
pub fn resolve_sample_column(chrom_header: &str, sample: &str) -> Result<usize> {
    let trimmed = chrom_header.trim_end_matches(['\r', '\n']);
    let mut column = 0usize;
    let mut start = 0usize;
    let bytes = trimmed.as_bytes();

    while start <= bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|byte| *byte == b'\t')
            .map_or(bytes.len(), |offset| start + offset);

        if column >= 9 && &trimmed[start..end] == sample {
            return Ok(column);
        }

        if end == bytes.len() {
            break;
        }
        column += 1;
        start = end + 1;
    }

    anyhow::bail!("sample '{sample}' not found in VCF header")
}

pub fn column_value(line: &str, target_column: usize) -> Option<&str> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    let bytes = trimmed.as_bytes();
    let mut column = 0usize;
    let mut start = 0usize;

    while start <= bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|byte| *byte == b'\t')
            .map_or(bytes.len(), |offset| start + offset);

        if column == target_column {
            return Some(&trimmed[start..end]);
        }

        if end == bytes.len() {
            break;
        }
        column += 1;
        start = end + 1;
    }

    None
}

pub fn selected_format_values<'a>(
    format: &'a str,
    sample: &'a str,
    required: RequiredFormatFields,
) -> FormatValues<'a> {
    if sample == "." {
        return FormatValues::default();
    }

    FormatValues {
        gt: required.gt.then(|| format_value(format, sample, "GT")).flatten(),
        dp: required.dp.then(|| format_value(format, sample, "DP")).flatten(),
        gq: required.gq.then(|| format_value(format, sample, "GQ")).flatten(),
    }
}

fn format_value<'a>(format: &'a str, sample: &'a str, key: &str) -> Option<&'a str> {
    let mut key_index = None;
    for_each_colon_value(format, |index, value| {
        if value == key && key_index.is_none() {
            key_index = Some(index);
        }
    });

    let target = key_index?;
    let mut found = None;
    for_each_colon_value(sample, |index, value| {
        if index == target && found.is_none() {
            found = Some(value);
        }
    });
    found
}

fn for_each_colon_value<'a>(value: &'a str, mut observe: impl FnMut(usize, &'a str)) {
    let bytes = value.as_bytes();
    let mut index = 0usize;
    let mut start = 0usize;

    while start <= bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|byte| *byte == b':')
            .map_or(bytes.len(), |offset| start + offset);

        observe(index, &value[start..end]);

        if end == bytes.len() {
            break;
        }
        index += 1;
        start = end + 1;
    }
}
```

- [ ] **Step 4: Run tests to verify GREEN**

Run:

```bash
cargo test --lib vcf::tests::
```

Expected: all VCF helper tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/vcf.rs src/expr/mod.rs
git commit -m "feat: add borrowed sample format helpers"
```

## Task 3: Filter CLI And Engine Integration

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/engine/filter.rs`
- Create: `tests/data/format_example.vcf`
- Test: `tests/filter_cli_tests.rs`

- [ ] **Step 1: Add FORMAT fixture**

Create `tests/data/format_example.vcf`:

```text
##fileformat=VCFv4.3
##contig=<ID=1>
##contig=<ID=2>
##FORMAT=<ID=GT,Number=1,Type=String,Description="Genotype">
##FORMAT=<ID=DP,Number=1,Type=Integer,Description="Sample depth">
##FORMAT=<ID=GQ,Number=1,Type=Integer,Description="Genotype quality">
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO	FORMAT	HG002	NA12878
1	100	fmtLow	A	G	50	PASS	DP=10;AF=0.1	GT:DP:GQ	0/1:10:50	0/1:35:60
1	200	fmtPass	C	T	50	PASS	DP=20;AF=0.2	GT:DP:GQ	0/1:25:40	0/0:5:10
1	300	fmtOtherSample	G	A	50	PASS	DP=30;AF=0.3	GT:DP:GQ	0/0:5:10	0/1:30:40
2	400	fmtMissing	T	C	50	PASS	DP=40;AF=0.4	GT:DP:GQ	./.:.:.	0/1:30:40
2	500	fmtShort	A	C	50	PASS	DP=50;AF=0.5	GT:DP:GQ	0/1:30	0/1:30:40
```

- [ ] **Step 2: Write failing CLI tests**

Add to `tests/filter_cli_tests.rs`:

```rust
#[test]
fn format_filter_uses_selected_sample_and_preserves_records() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("format.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/format_example.vcf").to_str().unwrap(),
            "--where",
            "FORMAT/DP > 20 && FORMAT/GQ >= 30",
            "--sample",
            "HG002",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = fs::read_to_string(output).unwrap();
    assert!(text.contains("#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tHG002\tNA12878\n"));
    assert!(text.contains("1\t200\tfmtPass\tC\tT\t50\tPASS\tDP=20;AF=0.2\tGT:DP:GQ\t0/1:25:40\t0/0:5:10\n"));
    assert!(!text.contains("fmtLow"));
    assert!(!text.contains("fmtOtherSample"));
    assert!(!text.contains("fmtMissing"));
    assert!(!text.contains("fmtShort"));
}

#[test]
fn format_filter_result_changes_with_sample() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("format-na12878.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/format_example.vcf").to_str().unwrap(),
            "--where",
            "FORMAT/DP > 20 && FORMAT/GQ >= 30",
            "--sample",
            "NA12878",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = fs::read_to_string(output).unwrap();
    assert!(text.contains("fmtLow"));
    assert!(text.contains("fmtOtherSample"));
    assert!(text.contains("fmtMissing"));
    assert!(text.contains("fmtShort"));
    assert!(!text.contains("fmtPass"));
}

#[test]
fn format_gt_filter_uses_exact_string_comparison() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("format-gt.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/format_example.vcf").to_str().unwrap(),
            "--where",
            "FORMAT/GT == \"0/1\"",
            "--sample",
            "HG002",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = fs::read_to_string(output).unwrap();
    assert!(text.contains("fmtLow"));
    assert!(text.contains("fmtPass"));
    assert!(text.contains("fmtShort"));
    assert!(!text.contains("fmtOtherSample"));
    assert!(!text.contains("fmtMissing"));
}

#[test]
fn format_filter_requires_sample() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("missing-sample.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/format_example.vcf").to_str().unwrap(),
            "--where",
            "FORMAT/DP > 20",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("FORMAT predicates require --sample <name>"));
}

#[test]
fn format_filter_rejects_unknown_sample() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("unknown-sample.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/format_example.vcf").to_str().unwrap(),
            "--where",
            "FORMAT/DP > 20",
            "--sample",
            "MISSING",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("sample 'MISSING' not found in VCF header"));
}

#[test]
fn sample_argument_is_allowed_for_site_only_filters() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("site-with-sample.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/example.vcf").to_str().unwrap(),
            "--where",
            "QUAL > 30",
            "--sample",
            "HG002",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();
}
```

- [ ] **Step 3: Run tests to verify RED**

Run:

```bash
cargo test --test filter_cli_tests
```

Expected: clap rejects `--sample`, or FORMAT expressions are unsupported.

- [ ] **Step 4: Implement CLI option**

In `src/cli.rs`, add the sample field:

```rust
Filter {
    input: PathBuf,
    #[arg(long = "where")]
    where_expr: String,
    #[arg(long)]
    sample: Option<String>,
    #[arg(short, long)]
    output: PathBuf,
},
```

Update the match:

```rust
Command::Filter {
    input,
    where_expr,
    sample,
    output,
} => filter::run(&input, &where_expr, sample.as_deref(), &output),
```

- [ ] **Step 5: Implement filter integration**

In `src/engine/filter.rs`, change `run`:

```rust
pub fn run(input: &Path, where_expr: &str, sample: Option<&str>, output: &Path) -> Result<()> {
    let expr = parse_expression(where_expr)?;
    let required = expr.required_fields();

    if required.requires_format() && sample.is_none() {
        anyhow::bail!("FORMAT predicates require --sample <name>");
    }

    let mut reader = open_reader(input)?;
    let mut writer = open_writer(output)?;
    let mut line = String::new();
    let mut sample_column = None;

    while reader.read_line(&mut line)? != 0 {
        if line.starts_with("#CHROM") {
            if required.requires_format() {
                sample_column = Some(resolve_sample_column(&line, sample.unwrap())?);
            }
            writer.write_all(line.as_bytes())?;
            line.clear();
            continue;
        }

        if line.starts_with('#') {
            writer.write_all(line.as_bytes())?;
            line.clear();
            continue;
        }

        if required.requires_format() && sample_column.is_none() {
            anyhow::bail!("FORMAT predicates require #CHROM header with sample columns");
        }

        let record = parse_eval_record_line(&line, required, sample_column)?;
        if expr.evaluate(&record) {
            writer.write_all(line.as_bytes())?;
        }
        line.clear();
    }

    writer.flush()?;
    Ok(())
}
```

Update imports:

```rust
use crate::expr::{EvalRecord, FormatValues, RequiredFields, parse_expression};
use crate::vcf::{
    SiteRecord, column_value, parse_record_fields, resolve_sample_column, selected_format_values,
};
```

Update record parsing:

```rust
fn parse_eval_record_line(
    line: &str,
    required: RequiredFields,
    sample_column: Option<usize>,
) -> Result<EvalRecord<'_>> {
    let fields = parse_record_fields(line)?;
    let chrom = if required.chrom { fields.chrom } else { "" };
    let pos = if required.pos { fields.pos_u64()? } else { 0 };
    let qual = if required.qual {
        fields.qual_float()?
    } else {
        None
    };
    let filter = if required.filter { fields.filter } else { "" };
    let info = if required.info { fields.info } else { "" };
    let format = if required.requires_format() {
        let format_column = column_value(line, 8).unwrap_or("");
        let sample_value = sample_column.and_then(|column| column_value(line, column)).unwrap_or(".");
        selected_format_values(format_column, sample_value, required.format)
    } else {
        FormatValues::default()
    };

    Ok(EvalRecord {
        chrom,
        pos,
        qual,
        filter,
        info,
        format,
    })
}
```

Update the existing `From<&SiteRecord>` impl with `format: FormatValues::default()`.

- [ ] **Step 6: Run tests to verify GREEN**

Run:

```bash
cargo test --test filter_cli_tests
```

Expected: all filter CLI tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/cli.rs src/engine/filter.rs tests/filter_cli_tests.rs tests/data/format_example.vcf
git commit -m "feat: add selected-sample format filtering"
```

## Task 4: Benchmark FORMAT Cases

**Files:**
- Modify: `benchmark/generate_stress_vcf.sh`
- Modify: `benchmark/run_benchmarks.sh`
- Modify: `tests/benchmark_harness_tests.rs`

- [ ] **Step 1: Write failing benchmark contract tests**

Add assertions to `tests/benchmark_harness_tests.rs`:

```rust
assert!(script.contains("FORMAT/DP > 20"));
assert!(script.contains("FORMAT/GQ >= 30"));
assert!(script.contains("FORMAT/GT == \\\"0/1\\\""));
assert!(script.contains("FMT/DP[0]>20"));
assert!(script.contains("FMT/GQ[0]>=30"));
assert!(script.contains("FMT/GT[0]=\\\"0/1\\\""));
assert!(script.contains("--sample SAMPLE_001"));
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
cargo test --test benchmark_harness_tests
```

Expected: assertions fail because FORMAT benchmark cases are absent.

- [ ] **Step 3: Add FORMAT stress cases**

In `benchmark/run_benchmarks.sh`, add FORMAT cases only for `MODE=stress`:

```bash
"FORMAT DP plain|plain|FORMAT/DP > 20|FMT/DP[0]>20|SAMPLE_001"
"FORMAT GQ plain|plain|FORMAT/GQ >= 30|FMT/GQ[0]>=30|SAMPLE_001"
"FORMAT GT plain|plain|FORMAT/GT == \"0/1\"|FMT/GT[0]=\"0/1\"|SAMPLE_001"
```

Extend case parsing to include `sample_name`:

```bash
IFS='|' read -r case_name input_kind fast_expr bcftools_expr sample_name <<<"$case_spec"
```

For VCF-Fast filter commands, pass `--sample "$sample_name"` only when `sample_name` is non-empty:

```bash
fast_filter_command=(./target/release/vcf-fast filter "$dataset" --where "$fast_expr" -o "$fast_out")
if [[ -n "${sample_name:-}" ]]; then
  fast_filter_command=(./target/release/vcf-fast filter "$dataset" --where "$fast_expr" --sample "$sample_name" -o "$fast_out")
fi
"${fast_filter_command[@]}"
```

For hyperfine strings, add `--sample SAMPLE_001` to the VCF-Fast FORMAT cases. For bcftools, use sample index `0` because stress generator sample `SAMPLE_001` is the first sample.

- [ ] **Step 4: Run benchmark contract test**

Run:

```bash
cargo test --test benchmark_harness_tests
```

Expected: benchmark harness tests pass.

- [ ] **Step 5: Run stress smoke in Docker**

Run:

```bash
docker build -t vcf-fast .
docker run --rm -v "$PWD:/work" \
  -e VCF_FAST_BENCH_MODE=stress \
  -e VCF_FAST_BENCH_SIZES="1000" \
  -e VCF_FAST_BENCH_RUNS=1 \
  -e VCF_FAST_BENCH_WARMUP=0 \
  vcf-fast make bench-smoke
find tests/output/benchmark-results -maxdepth 1 -name 'equivalence-*.diff' -type f -size +0 -print
```

Expected: benchmark completes and `find` prints no non-empty diff files.

- [ ] **Step 6: Commit**

```bash
git add benchmark/run_benchmarks.sh benchmark/generate_stress_vcf.sh tests/benchmark_harness_tests.rs
git commit -m "bench: add selected-sample format benchmark cases"
```

## Task 5: Evidence, Docs, And Verification

**Files:**
- Modify: `README.md`
- Modify: `docs/contribution-map.md`
- Create or modify: `benchmark/reports/format-filter-benchmark.md`

- [ ] **Step 1: Run measured FORMAT benchmark**

Run:

```bash
docker run --rm -v "$PWD:/work" \
  -e VCF_FAST_BENCH_MODE=stress \
  -e VCF_FAST_BENCH_SIZES="100000 1000000" \
  -e VCF_FAST_BENCH_RUNS=3 \
  -e VCF_FAST_BENCH_WARMUP=1 \
  vcf-fast make bench-smoke
find tests/output/benchmark-results -maxdepth 1 -name 'equivalence-*.diff' -type f -size +0 -print
```

Expected: benchmark completes and `find` prints no non-empty diff files. If any diff exists, stop and fix correctness before docs.

- [ ] **Step 2: Write measured report**

Create `benchmark/reports/format-filter-benchmark.md` from `tests/output/benchmark-results/benchmark-report.md`. Include:

- generated date
- stress shape
- `bcftools` version
- `hyperfine` version
- exact command templates
- FORMAT cases and site-level cases
- correctness result
- runtime, speedup, variants/sec, RSS
- caveat that this is single-sample selected FORMAT filtering

- [ ] **Step 3: Update README and contribution map with measured claims**

Update `README.md` Current Evidence with one row copied from the measured FORMAT report. The row must contain the exact minimum and maximum FORMAT filter speedups from `benchmark/reports/format-filter-benchmark.md`, not rounded guesses. Do not write the row until the report exists and the numbers can be copied exactly.

Update `docs/contribution-map.md`:

- add selected-sample FORMAT filtering under current implemented contributions
- add evidence path `benchmark/reports/format-filter-benchmark.md`
- keep caveat: no multi-sample `ANY`/`ALL`, no arbitrary FORMAT keys, no BCF/BGZF/tabix support

- [ ] **Step 4: Run full verification**

Run:

```bash
make verify
docker build -t vcf-fast .
docker run --rm -v "$PWD:/work" vcf-fast make verify
git diff --check
```

Expected: all commands pass.

- [ ] **Step 5: Commit**

```bash
git add README.md docs/contribution-map.md benchmark/reports/format-filter-benchmark.md
git commit -m "docs: publish format filtering benchmark evidence"
```

## Task 6: PR And CI

**Files:**
- No direct code files.

- [ ] **Step 1: Push branch**

Run:

```bash
git status --short --branch
git push -u origin format-aware-filtering-v04
```

Expected: clean working tree before push.

- [ ] **Step 2: Open PR**

Run:

```bash
gh pr create --title "Add selected-sample FORMAT-aware filtering" --body "$(cat <<'EOF'
## Summary
- Add selected-sample FORMAT-aware filtering for FORMAT/GT, FORMAT/DP, and FORMAT/GQ.
- Preserve line-oriented streaming and parse only required FORMAT keys from the selected sample.
- Add bcftools-checked FORMAT stress benchmark evidence.

## Test Plan
- [ ] make verify
- [ ] docker build -t vcf-fast .
- [ ] docker run --rm -v "$PWD:/work" vcf-fast make verify
- [ ] docker stress FORMAT benchmark at 100k and 1M records
EOF
)"
```

- [ ] **Step 3: Watch PR checks**

Run:

```bash
gh pr checks --watch
```

Expected: all checks pass.

- [ ] **Step 4: Merge after green CI**

Run from the main worktree if using a git worktree:

```bash
gh pr merge --squash --delete-branch
main_run_id="$(gh run list --branch main --limit 1 --json databaseId --jq '.[0].databaseId')"
gh run watch "$main_run_id" --exit-status
```

Expected: PR merges, `main` CI passes, and local main can be fast-forwarded with `git pull --ff-only`.

## Self-Review Notes

- Spec coverage: covers public CLI, expression semantics, missing values, single-sample behavior, tests, benchmarks, docs, and CI.
- Placeholder scan: no placeholder tasks; measured values are intentionally deferred until the benchmark task because docs must use real results, and the docs task forbids writing evidence rows before exact report values exist.
- Type consistency: `FormatValues`, `RequiredFormatFields`, and `RequiredFields::requires_format()` are introduced before use by filter and VCF helpers.
- Scope: intentionally excludes multi-sample predicates, arbitrary FORMAT keys, htslib/BCF/BGZF/tabix, and genotype normalization.
