# VCF-Fast v0.9 Expression Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand VCF-Fast native filtering from fixed built-in fields to practical expression parity for arbitrary `INFO/*`, selected-sample `FORMAT/*`, and `ANY`/`ALL` sample aggregate predicates while preserving the fast byte-slice streaming core.

**Architecture:** Keep the AST and streaming engine, but replace fixed INFO/FORMAT enums with borrowed-key expression requirements. Native `.vcf` and `.vcf.gz` filtering evaluates arbitrary keys directly against `RecordView`, `InfoView`, and FORMAT/sample byte slices; htslib-backed compatibility paths keep existing behavior and fail clearly for native-only aggregate predicates. The milestone is correctness-first: supported semantics must be explicit, tested, and compared against `bcftools` where practical before performance claims are updated.

**Tech Stack:** Rust 2021, existing `clap` CLI, existing `flate2`/`memchr` native streaming path, optional `rust-htslib` compatibility feature, shell benchmark harness, `bcftools` as correctness baseline.

---

## Scope And Semantics

v0.9 adds native expression support for these forms:

```bash
vcf-fast filter input.vcf.gz --where 'INFO/MQ >= 50' -o high_mq.vcf.gz
vcf-fast filter input.vcf.gz --where 'INFO/CSQ != "missense_variant"' -o no_missense.vcf.gz
vcf-fast filter input.vcf.gz --sample HG002 --where 'FORMAT/AD != "." && FORMAT/DP > 10' -o sample_dp.vcf.gz
vcf-fast filter input.vcf.gz --where 'ANY(FORMAT/DP > 20)' -o any_sample_dp.vcf.gz
vcf-fast filter input.vcf.gz --where 'ALL(FORMAT/GQ >= 30)' -o all_sample_gq.vcf.gz
```

Supported semantics:

- `DP` and `AF` remain aliases for `INFO/DP` and `INFO/AF`.
- `INFO/<KEY>` supports numeric comparisons against numeric literals and byte-exact string comparisons against quoted literals.
- Numeric `INFO/<KEY>` comparisons pass if any comma-separated numeric value satisfies the predicate.
- String `INFO/<KEY>` comparisons compare the full raw INFO value bytes; comma splitting is only for numeric values.
- Missing INFO keys, empty INFO values, flag-only INFO entries, and `.` values make the predicate false for numeric and string comparisons.
- `FORMAT/<KEY>` without `ANY` or `ALL` uses the existing selected-sample model and requires `--sample`.
- `ANY(FORMAT/<KEY> op literal)` scans all sample columns and passes if at least one sample has a present value satisfying the comparison.
- `ALL(FORMAT/<KEY> op literal)` scans all sample columns and passes only if every sample has a present value satisfying the comparison.
- `ANY` and `ALL` require a `#CHROM` header with at least one sample column.
- htslib paths support existing v0.8 expressions. Native-only sample aggregate expressions exit non-zero on htslib paths with a clear message: `ANY/ALL FORMAT predicates are not implemented for htslib-backed input in v0.9`.

## File Structure

- Modify `src/expr/mod.rs`: replace fixed `Field::Dp`, `Field::Af`, `Field::FormatDp`, `Field::FormatGq`, and `Field::FormatGt` with key-bearing `Field::Info(Vec<u8>)` and `Field::Format(Vec<u8>)`; add sample aggregate AST nodes; update tokenizer/parser/evaluator and required-field discovery.
- Modify `src/vcf.rs`: expose arbitrary FORMAT value lookup, add sample-column iteration helpers on `RecordView`, and keep `InfoView::value`/`number_any` as the shared borrowed INFO scanner.
- Modify `src/engine/filter.rs`: store selected FORMAT/sample bytes and sample-column ranges in `ByteEvalRecord`; implement arbitrary FORMAT lookup and `ANY`/`ALL` aggregate evaluation.
- Modify `src/htslib_backend.rs`: detect unsupported aggregate requirements before running htslib-backed filter and return the v0.9 feature message.
- Modify `tests/expr_tests.rs`: add parser/evaluator unit tests for arbitrary INFO, arbitrary selected FORMAT, and aggregate predicate parsing.
- Modify `tests/filter_cli_tests.rs`: add native integration tests for arbitrary INFO, selected FORMAT, `ANY`, `ALL`, missing values, and byte-for-byte output preservation.
- Create `tests/data/expression_parity.vcf`: compact fixture with arbitrary INFO and FORMAT keys across multiple samples.
- Modify `tests/benchmark_harness_tests.rs`: assert the v0.9 report exists and contains command/correctness/caveat fields.
- Create `benchmark/reports/v09-expression-parity-benchmark.md`: evidence scaffold for native expression parity comparisons against `bcftools`.
- Modify `README.md`: document v0.9 expression support only after tests pass.
- Modify `docs/contribution-map.md`: add v0.9 row with implemented evidence and caveats.

---

### Task 1: Add Expression Tests For Arbitrary INFO Fields

**Files:**
- Modify: `tests/expr_tests.rs`
- Modify: `src/expr/mod.rs`

- [ ] **Step 1: Write failing parser/evaluator tests for arbitrary INFO**

Append these tests to `tests/expr_tests.rs`:

```rust
#[test]
fn parses_and_evaluates_arbitrary_info_numeric_field() {
    let expr = parse_expression("INFO/MQ >= 50").unwrap();
    let record = EvalRecord::new(b"chr1", Some(101), Some(60.0), b"PASS")
        .with_info(b"MQ=60;CSQ=synonymous_variant");

    assert!(expr.evaluate_record(&record));
}

#[test]
fn arbitrary_info_numeric_uses_any_comma_value_semantics() {
    let expr = parse_expression("INFO/FS < 10").unwrap();
    let record = EvalRecord::new(b"chr1", Some(101), Some(60.0), b"PASS")
        .with_info(b"FS=12.5,8.2,30.0");

    assert!(expr.evaluate_record(&record));
}

#[test]
fn arbitrary_info_string_compares_raw_value() {
    let expr = parse_expression("INFO/CSQ == \"synonymous_variant\"").unwrap();
    let record = EvalRecord::new(b"chr1", Some(101), Some(60.0), b"PASS")
        .with_info(b"MQ=60;CSQ=synonymous_variant");

    assert!(expr.evaluate_record(&record));
}

#[test]
fn arbitrary_info_missing_empty_flag_and_dot_are_false() {
    let missing = parse_expression("INFO/MQ >= 50").unwrap();
    let empty = parse_expression("INFO/EMPTY == \"value\"").unwrap();
    let flag = parse_expression("INFO/SOMATIC == \"true\"").unwrap();
    let dot = parse_expression("INFO/AF > 0.01").unwrap();
    let record = EvalRecord::new(b"chr1", Some(101), Some(60.0), b"PASS")
        .with_info(b"EMPTY=;SOMATIC;AF=.");

    assert!(!missing.evaluate_record(&record));
    assert!(!empty.evaluate_record(&record));
    assert!(!flag.evaluate_record(&record));
    assert!(!dot.evaluate_record(&record));
}
```

- [ ] **Step 2: Run the failing tests**

Run:

```bash
cargo test --test expr_tests arbitrary_info -- --nocapture
```

Expected: FAIL because `INFO/MQ`, `INFO/FS`, and `INFO/CSQ` currently parse as unsupported fields.

- [ ] **Step 3: Replace fixed INFO field variants with key-bearing fields**

In `src/expr/mod.rs`, change the `Field` enum to this shape:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
enum Field {
    Chrom,
    Pos,
    Qual,
    Filter,
    Info(Vec<u8>),
    Format(Vec<u8>),
}
```

Add this helper near the parser functions:

```rust
fn parse_field_name(name: &str) -> Result<Field> {
    match name {
        "CHROM" => Ok(Field::Chrom),
        "POS" => Ok(Field::Pos),
        "QUAL" => Ok(Field::Qual),
        "FILTER" => Ok(Field::Filter),
        "DP" => Ok(Field::Info(b"DP".to_vec())),
        "AF" => Ok(Field::Info(b"AF".to_vec())),
        _ if name.starts_with("INFO/") && name.len() > "INFO/".len() => {
            Ok(Field::Info(name.as_bytes()["INFO/".len()..].to_vec()))
        }
        _ if name.starts_with("FORMAT/") && name.len() > "FORMAT/".len() => {
            Ok(Field::Format(name.as_bytes()["FORMAT/".len()..].to_vec()))
        }
        _ => Err(anyhow!("unsupported field `{name}`")),
    }
}
```

Update the existing parser branch that maps identifiers to fields so it calls `parse_field_name(name)`.

- [ ] **Step 4: Update required-field discovery for arbitrary INFO**

Replace the fixed INFO flag in `RequiredFields` with key storage:

```rust
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct RequiredFields {
    pub chrom: bool,
    pub pos: bool,
    pub qual: bool,
    pub filter: bool,
    pub info_keys: Vec<Vec<u8>>,
    pub format_keys: Vec<Vec<u8>>,
    pub format_aggregates: bool,
}

impl RequiredFields {
    pub(crate) fn requires_info(&self) -> bool {
        !self.info_keys.is_empty()
    }

    pub(crate) fn requires_format(&self) -> bool {
        !self.format_keys.is_empty() || self.format_aggregates
    }

    fn add_info_key(&mut self, key: &[u8]) {
        if !self.info_keys.iter().any(|existing| existing == key) {
            self.info_keys.push(key.to_vec());
        }
    }

    fn add_format_key(&mut self, key: &[u8]) {
        if !self.format_keys.iter().any(|existing| existing == key) {
            self.format_keys.push(key.to_vec());
        }
    }
}
```

Update `mark_required_field`:

```rust
fn mark_required_field(required: &mut RequiredFields, field: &Field) {
    match field {
        Field::Chrom => required.chrom = true,
        Field::Pos => required.pos = true,
        Field::Qual => required.qual = true,
        Field::Filter => required.filter = true,
        Field::Info(key) => required.add_info_key(key),
        Field::Format(key) => required.add_format_key(key),
    }
}
```

- [ ] **Step 5: Update INFO evaluation**

Update comparison evaluation so numeric and string INFO fields use the key from the AST:

```rust
Field::Info(key) => match &comparison.value {
    Value::Number(threshold) => {
        let mut predicate = |value| compare_numbers(value, comparison.op, *threshold);
        record.info_number_any(key, &mut predicate)
    }
    Value::String(expected) => record
        .info_value(key)
        .filter(|value| is_present_value(value))
        .is_some_and(|actual| compare_bytes(actual, comparison.op, expected.as_bytes())),
}
```

Add the trait method:

```rust
fn info_value(&self, key: &[u8]) -> Option<&[u8]>;
```

Implement it for the existing test `EvalRecord` by delegating to the borrowed INFO scanner:

```rust
fn info_value(&self, key: &[u8]) -> Option<&[u8]> {
    InfoView::new(self.info.unwrap_or(b".")).value(key)
}
```

Add this present-value helper in `src/expr/mod.rs`:

```rust
fn is_present_value(value: &[u8]) -> bool {
    !value.is_empty() && value != b"."
}
```

- [ ] **Step 6: Run tests for Task 1**

Run:

```bash
cargo test --test expr_tests arbitrary_info
cargo test --test expr_tests tokenize_and_parse_comparison_expression
```

Expected: PASS.

- [ ] **Step 7: Commit Task 1**

Run:

```bash
git add src/expr/mod.rs tests/expr_tests.rs
git commit -m "feat: support arbitrary INFO expressions"
```

---

### Task 2: Add Selected-Sample Arbitrary FORMAT Expressions

**Files:**
- Create: `tests/data/expression_parity.vcf`
- Modify: `tests/expr_tests.rs`
- Modify: `tests/filter_cli_tests.rs`
- Modify: `src/expr/mod.rs`
- Modify: `src/vcf.rs`
- Modify: `src/engine/filter.rs`

- [ ] **Step 1: Add failing selected FORMAT evaluator tests**

Append to `tests/expr_tests.rs`:

```rust
#[test]
fn arbitrary_format_numeric_selected_sample_predicate_passes() {
    let expr = parse_expression("FORMAT/AD > 8").unwrap();
    let record = EvalRecord::new(b"chr1", Some(101), Some(60.0), b"PASS")
        .with_format_value(b"AD", b"4,11");

    assert!(expr.evaluate_record(&record));
}

#[test]
fn arbitrary_format_string_selected_sample_predicate_passes() {
    let expr = parse_expression("FORMAT/FT == \"PASS\"").unwrap();
    let record = EvalRecord::new(b"chr1", Some(101), Some(60.0), b"PASS")
        .with_format_value(b"FT", b"PASS");

    assert!(expr.evaluate_record(&record));
}

#[test]
fn arbitrary_format_missing_and_dot_are_false() {
    let missing = parse_expression("FORMAT/AD > 8").unwrap();
    let dot = parse_expression("FORMAT/FT == \"PASS\"").unwrap();
    let record = EvalRecord::new(b"chr1", Some(101), Some(60.0), b"PASS")
        .with_format_value(b"FT", b".");

    assert!(!missing.evaluate_record(&record));
    assert!(!dot.evaluate_record(&record));
}
```

- [ ] **Step 2: Run the failing FORMAT tests**

Run:

```bash
cargo test --test expr_tests arbitrary_format -- --nocapture
```

Expected: FAIL because the test helper and evaluator do not yet support arbitrary FORMAT keys.

- [ ] **Step 3: Extend the test EvalRecord with arbitrary FORMAT values**

In `src/expr/mod.rs`, replace the fixed `FormatValues` test/storage helper with this key-value storage:

```rust
#[derive(Debug, Clone, Default)]
pub(crate) struct FormatValues {
    entries: Vec<(Vec<u8>, Vec<u8>)>,
}

impl FormatValues {
    pub(crate) fn with(mut self, key: &[u8], value: &[u8]) -> Self {
        self.entries.push((key.to_vec(), value.to_vec()));
        self
    }

    fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.entries
            .iter()
            .find(|(existing, _)| existing.as_slice() == key)
            .map(|(_, value)| value.as_slice())
    }
}
```

Add this builder to `EvalRecord`:

```rust
pub(crate) fn with_format_value(mut self, key: &[u8], value: &[u8]) -> Self {
    let values = self.format.unwrap_or_default().with(key, value);
    self.format = Some(values);
    self
}
```

Keep the existing `with_format_gt`, `with_format_dp`, and `with_format_gq` builders by delegating to `with_format_value`.

- [ ] **Step 4: Update FORMAT evaluation**

Add this trait method to `EvalContext`:

```rust
fn format_value(&self, key: &[u8]) -> Option<&[u8]>;
```

Implement selected-sample FORMAT comparison:

```rust
Field::Format(key) => match &comparison.value {
    Value::Number(threshold) => record
        .format_value(key)
        .filter(|value| is_present_value(value))
        .is_some_and(|actual| {
            number_list_any(actual, |value| compare_numbers(value, comparison.op, *threshold))
        }),
    Value::String(expected) => record
        .format_value(key)
        .filter(|value| is_present_value(value))
        .is_some_and(|actual| compare_bytes(actual, comparison.op, expected.as_bytes())),
}
```

Add this helper if the existing numeric list helper is private to INFO evaluation:

```rust
fn number_list_any<F>(value: &[u8], mut predicate: F) -> bool
where
    F: FnMut(f64) -> bool,
{
    for part in value.split(|byte| *byte == b',') {
        if part.is_empty() || part == b"." {
            continue;
        }
        if std::str::from_utf8(part)
            .ok()
            .and_then(|text| text.parse::<f64>().ok())
            .is_some_and(&mut predicate)
        {
            return true;
        }
    }
    false
}
```

- [ ] **Step 5: Expose arbitrary FORMAT lookup in VCF helpers**

In `src/vcf.rs`, make `format_value_bytes` visible inside the crate:

```rust
pub(crate) fn format_value_bytes<'a>(
    format: &'a [u8],
    sample: &'a [u8],
    key: &[u8],
) -> Option<&'a [u8]> {
    // keep the existing implementation body
}
```

Keep `selected_format_values_bytes` for existing callers until `filter.rs` has migrated.

- [ ] **Step 6: Migrate native filter ByteEvalRecord to arbitrary FORMAT lookup**

In `src/engine/filter.rs`, replace the fixed `FormatValueBytes` field in `ByteEvalRecord` with selected FORMAT/sample byte slices:

```rust
struct ByteEvalRecord<'a> {
    view: RecordView<'a>,
    info: Option<InfoView<'a>>,
    format_column: Option<&'a [u8]>,
    selected_sample: Option<&'a [u8]>,
}
```

During parse, populate `format_column` from column 8 and `selected_sample` from the resolved sample column when `required.requires_format()` is true. Implement `EvalContext::format_value`:

```rust
fn format_value(&self, key: &[u8]) -> Option<&[u8]> {
    let format = self.format_column?;
    let sample = self.selected_sample?;
    vcf::format_value_bytes(format, sample, key)
}
```

- [ ] **Step 7: Create the expression parity fixture and selected FORMAT CLI coverage**

Create `tests/data/expression_parity.vcf` with exactly this content:

```text
##fileformat=VCFv4.3
##INFO=<ID=MQ,Number=1,Type=Float,Description="Mapping quality">
##INFO=<ID=FS,Number=A,Type=Float,Description="Strand bias">
##INFO=<ID=CSQ,Number=1,Type=String,Description="Consequence">
##INFO=<ID=SOMATIC,Number=0,Type=Flag,Description="Somatic flag">
##INFO=<ID=EMPTY,Number=1,Type=String,Description="Empty string">
##FORMAT=<ID=AD,Number=R,Type=Integer,Description="Allelic depths">
##FORMAT=<ID=FT,Number=1,Type=String,Description="Sample filter">
##FORMAT=<ID=DP,Number=1,Type=Integer,Description="Sample depth">
##FORMAT=<ID=GQ,Number=1,Type=Integer,Description="Genotype quality">
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO	FORMAT	HG002	HG003
chr1	101	rs1	A	G	60	PASS	MQ=55;FS=12.5,8.2;CSQ=synonymous_variant;SOMATIC	AD:FT:DP:GQ	4,11:PASS:22:35	10,0:LowDP:10:20
chr1	102	rs2	C	T	45	PASS	MQ=40;FS=30.0;CSQ=missense_variant	AD:FT:DP:GQ	9,1:LowDP:12:18	2,18:PASS:30:40
chr1	103	rs3	G	GA	20	q10	MQ=.;EMPTY=;CSQ=.	AD:FT:DP:GQ	.:.:.:.	3,1:PASS:8:25
```

Add this integration test to `tests/filter_cli_tests.rs`:

```rust
#[test]
fn filter_supports_arbitrary_selected_format_field() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("arbitrary_format_selected.vcf");

    let assert = Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/expression_parity.vcf").to_str().unwrap(),
            "--sample",
            "HG002",
            "--where",
            "FORMAT/AD > 8 && FORMAT/FT == \"PASS\"",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert.stderr(predicate::str::is_empty());
    let text = std::fs::read_to_string(output).unwrap();
    assert!(text.contains("chr1\t101\trs1\tA\tG\t60\tPASS"));
    assert!(!text.contains("chr1\t102\trs2"));
    assert!(!text.contains("chr1\t103\trs3"));
}
```

- [ ] **Step 8: Run Task 2 tests**

Run:

```bash
cargo test --test expr_tests arbitrary_format
cargo test --test filter_cli_tests filter_supports_arbitrary_selected_format_field
```

Expected: PASS.

- [ ] **Step 9: Commit Task 2**

Run:

```bash
git add src/expr/mod.rs src/vcf.rs src/engine/filter.rs tests/expr_tests.rs tests/filter_cli_tests.rs tests/data/expression_parity.vcf
git commit -m "feat: support arbitrary selected FORMAT expressions"
```

---

### Task 3: Add ANY And ALL Sample Aggregate Predicates

**Files:**
- Modify: `tests/expr_tests.rs`
- Modify: `tests/filter_cli_tests.rs`
- Modify: `src/expr/mod.rs`
- Modify: `src/vcf.rs`
- Modify: `src/engine/filter.rs`

- [ ] **Step 1: Add failing parser tests for aggregate syntax**

Append to `tests/expr_tests.rs`:

```rust
#[test]
fn parses_any_and_all_format_aggregate_predicates() {
    parse_expression("ANY(FORMAT/DP > 20)").unwrap();
    parse_expression("ALL(FORMAT/GQ >= 30)").unwrap();
    parse_expression("QUAL > 30 && ANY(FORMAT/AD > 12)").unwrap();
}

#[test]
fn rejects_aggregate_predicates_over_non_format_fields() {
    let err = parse_expression("ANY(INFO/DP > 20)").unwrap_err().to_string();
    assert!(err.contains("ANY/ALL predicates require a FORMAT field"));
}
```

- [ ] **Step 2: Run the failing parser tests**

Run:

```bash
cargo test --test expr_tests parses_any_and_all_format_aggregate_predicates rejects_aggregate_predicates_over_non_format_fields -- --nocapture
```

Expected: FAIL because `ANY` and `ALL` are not recognized as expression forms.

- [ ] **Step 3: Add aggregate AST types**

In `src/expr/mod.rs`, extend the AST:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SampleQuantifier {
    Any,
    All,
}

#[derive(Debug, Clone, PartialEq)]
enum ExprNode {
    Comparison(Comparison),
    SampleAggregate {
        quantifier: SampleQuantifier,
        comparison: Comparison,
    },
    And(Box<ExprNode>, Box<ExprNode>),
    Or(Box<ExprNode>, Box<ExprNode>),
}
```

- [ ] **Step 4: Parse aggregate primary expressions**

In the parser primary-expression function, add this branch before normal comparison parsing:

```rust
if self.match_ident("ANY") || self.match_ident("ALL") {
    let quantifier = if self.previous_ident() == "ANY" {
        SampleQuantifier::Any
    } else {
        SampleQuantifier::All
    };
    self.expect_token(TokenKind::LeftParen)?;
    let comparison = self.parse_comparison()?;
    self.expect_token(TokenKind::RightParen)?;
    if !matches!(comparison.field, Field::Format(_)) {
        return Err(anyhow!("ANY/ALL predicates require a FORMAT field"));
    }
    return Ok(ExprNode::SampleAggregate {
        quantifier,
        comparison,
    });
}
```

If the parser does not currently expose `match_ident` and `previous_ident`, implement them with the current token cursor:

```rust
fn match_ident(&mut self, expected: &str) -> bool {
    match self.peek() {
        Some(Token::Ident(actual)) if actual == expected => {
            self.advance();
            true
        }
        _ => false,
    }
}

fn previous_ident(&self) -> &str {
    match self.previous() {
        Some(Token::Ident(value)) => value.as_str(),
        _ => "",
    }
}
```

- [ ] **Step 5: Track aggregate required fields**

Update required-field traversal:

```rust
ExprNode::SampleAggregate { comparison, .. } => {
    required.format_aggregates = true;
    mark_required_field(required, &comparison.field);
}
```

This ensures `filter.rs` resolves sample columns from the header even when no `--sample` is provided.

- [ ] **Step 6: Add aggregate evaluation trait methods**

Add these methods to `EvalContext`:

```rust
fn any_format_value(&self, key: &[u8], predicate: &mut dyn FnMut(&[u8]) -> bool) -> bool;

fn all_format_value(&self, key: &[u8], predicate: &mut dyn FnMut(&[u8]) -> bool) -> bool;
```

Add this comparison helper:

```rust
fn evaluate_format_value(comparison: &Comparison, value: &[u8]) -> bool {
    if !is_present_value(value) {
        return false;
    }
    match &comparison.value {
        Value::Number(threshold) => {
            number_list_any(value, |actual| compare_numbers(actual, comparison.op, *threshold))
        }
        Value::String(expected) => compare_bytes(value, comparison.op, expected.as_bytes()),
    }
}
```

Implement `ExprNode::SampleAggregate` evaluation:

```rust
ExprNode::SampleAggregate {
    quantifier,
    comparison,
} => {
    let Field::Format(key) = &comparison.field else {
        return false;
    };
    let mut predicate = |value: &[u8]| evaluate_format_value(comparison, value);
    match quantifier {
        SampleQuantifier::Any => record.any_format_value(key, &mut predicate),
        SampleQuantifier::All => record.all_format_value(key, &mut predicate),
    }
}
```

- [ ] **Step 7: Add RecordView sample iteration helper**

In `src/vcf.rs`, add:

```rust
impl<'a> RecordView<'a> {
    pub(crate) fn for_each_sample_column<F>(&self, mut visit: F)
    where
        F: FnMut(&'a [u8]),
    {
        let mut column_index = 0usize;
        let mut start = 0usize;
        for (idx, byte) in self.line.iter().enumerate() {
            if *byte == b'\t' {
                if column_index >= 9 {
                    visit(trim_line_end(&self.line[start..idx]));
                }
                column_index += 1;
                start = idx + 1;
            }
        }
        if column_index >= 9 && start <= self.line.len() {
            visit(trim_line_end(&self.line[start..]));
        }
    }
}
```

Use the existing private `trim_line_end` helper in `src/vcf.rs`; the new public surface is only `RecordView::for_each_sample_column`.

- [ ] **Step 8: Implement aggregate evaluation in native filter**

In `src/engine/filter.rs`, implement aggregate methods for `ByteEvalRecord`:

```rust
fn any_format_value(&self, key: &[u8], predicate: &mut dyn FnMut(&[u8]) -> bool) -> bool {
    let Some(format) = self.format_column else {
        return false;
    };
    let mut matched = false;
    self.view.for_each_sample_column(|sample| {
        if matched {
            return;
        }
        if let Some(value) = vcf::format_value_bytes(format, sample, key) {
            matched = predicate(value);
        }
    });
    matched
}

fn all_format_value(&self, key: &[u8], predicate: &mut dyn FnMut(&[u8]) -> bool) -> bool {
    let Some(format) = self.format_column else {
        return false;
    };
    let mut saw_sample = false;
    let mut all_match = true;
    self.view.for_each_sample_column(|sample| {
        saw_sample = true;
        if !all_match {
            return;
        }
        all_match = vcf::format_value_bytes(format, sample, key).is_some_and(|value| predicate(value));
    });
    saw_sample && all_match
}
```

- [ ] **Step 9: Add aggregate CLI tests**

Append to `tests/filter_cli_tests.rs`:

```rust
#[test]
fn filter_supports_any_format_aggregate() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("any_format_dp.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/expression_parity.vcf").to_str().unwrap(),
            "--where",
            "ANY(FORMAT/AD > 15)",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = std::fs::read_to_string(output).unwrap();
    assert!(!text.contains("chr1\t101\trs1"));
    assert!(text.contains("chr1\t102\trs2\tC\tT"));
}

#[test]
fn filter_supports_all_format_aggregate() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("all_format_ft.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/expression_parity.vcf").to_str().unwrap(),
            "--where",
            "ALL(FORMAT/FT != \"LowDP\")",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = std::fs::read_to_string(output).unwrap();
    assert!(!text.contains("chr1\t101\trs1"));
    assert!(!text.contains("chr1\t102\trs2"));
}
```

- [ ] **Step 10: Run Task 3 tests**

Run:

```bash
cargo test --test expr_tests parses_any_and_all_format_aggregate_predicates rejects_aggregate_predicates_over_non_format_fields
cargo test --test filter_cli_tests filter_supports_any_format_aggregate filter_supports_all_format_aggregate
```

Expected: PASS.

- [ ] **Step 11: Commit Task 3**

Run:

```bash
git add src/expr/mod.rs src/vcf.rs src/engine/filter.rs tests/expr_tests.rs tests/filter_cli_tests.rs
git commit -m "feat: add ANY ALL FORMAT aggregate predicates"
```

---

### Task 4: Preserve Existing Behavior And Add Clear htslib Boundaries

**Files:**
- Modify: `src/engine/filter.rs`
- Modify: `src/htslib_backend.rs`
- Modify: `tests/filter_cli_tests.rs`
- Modify: `tests/compatibility_cli_tests.rs`

- [ ] **Step 1: Add regression tests for existing FORMAT behavior**

Run the existing selected-sample tests before modifying htslib:

```bash
cargo test --test filter_cli_tests format -- --nocapture
```

Expected: PASS before this task begins. If a test fails, inspect the failure before changing code.

- [ ] **Step 2: Add htslib aggregate rejection test**

In `tests/compatibility_cli_tests.rs`, add this feature-gated test:

```rust
#[cfg(feature = "htslib")]
#[test]
fn htslib_path_rejects_any_all_format_predicates_in_v09() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("expression_parity.vcf.gz");
    let output = dir.path().join("htslib_any_reject.vcf");
    create_bgzf_vcf(&fixture("tests/data/expression_parity.vcf"), &input);

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            input.to_str().unwrap(),
            "--region",
            "chr1:1-1000",
            "--where",
            "ANY(FORMAT/DP > 20)",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "ANY/ALL FORMAT predicates are not implemented for htslib-backed input in v0.9",
        ));
}
```

- [ ] **Step 3: Expose aggregate requirement detection**

In `src/expr/mod.rs`, add:

```rust
impl RequiredFields {
    pub(crate) fn requires_format_aggregates(&self) -> bool {
        self.format_aggregates
    }
}
```

- [ ] **Step 4: Reject unsupported htslib aggregate expressions**

In `src/htslib_backend.rs`, after parsing the expression and computing required fields in `filter`, add:

```rust
if required.requires_format_aggregates() {
    bail!("ANY/ALL FORMAT predicates are not implemented for htslib-backed input in v0.9");
}
```

Do not add the guard to `convert_to_tsv` or `stats` because those functions do not parse `--where` in this repository.

- [ ] **Step 5: Ensure native selected-sample FORMAT still requires `--sample`**

In `src/engine/filter.rs`, update the sample requirement check:

```rust
if required.requires_format() && !required.requires_format_aggregates() && sample_name.is_none() {
    bail!("FORMAT predicates require --sample");
}
```

If an expression mixes selected-sample `FORMAT/DP > 10` with `ANY(FORMAT/GQ > 20)`, it still requires `--sample` for the selected-sample predicate. Implement that with a separate `requires_selected_format()` method:

```rust
pub(crate) fn requires_selected_format(&self) -> bool {
    !self.format_keys.is_empty()
}
```

Use:

```rust
if required.requires_selected_format() && sample_name.is_none() {
    bail!("FORMAT predicates require --sample");
}
```

- [ ] **Step 6: Run htslib and native regression tests**

Run:

```bash
cargo test --test filter_cli_tests format
cargo test --features htslib-static --test compatibility_cli_tests htslib_path_rejects_any_all_format_predicates_in_v09
```

Expected: PASS.

- [ ] **Step 7: Commit Task 4**

Run:

```bash
git add src/expr/mod.rs src/engine/filter.rs src/htslib_backend.rs tests/filter_cli_tests.rs tests/compatibility_cli_tests.rs
git commit -m "fix: define htslib boundary for aggregate expressions"
```

---

### Task 5: Add Native End-To-End Fixture And bcftools Comparison Notes

**Files:**
- Modify: `tests/data/expression_parity.vcf`
- Modify: `tests/filter_cli_tests.rs`
- Create: `benchmark/reports/v09-expression-parity-benchmark.md`
- Modify: `tests/benchmark_harness_tests.rs`

- [ ] **Step 1: Verify the expression parity fixture has all required edge cases**

Run:

```bash
rg "ID=MQ|ID=FS|ID=CSQ|ID=SOMATIC|ID=EMPTY|ID=AD|ID=FT|ID=DP|ID=GQ|HG002|HG003|MQ=\\.|EMPTY=|SOMATIC" tests/data/expression_parity.vcf
```

Expected: `rg` prints the fixture header and record lines containing every required INFO/FORMAT edge case.

- [ ] **Step 2: Add end-to-end arbitrary INFO CLI test**

Append to `tests/filter_cli_tests.rs`:

```rust
#[test]
fn filter_supports_arbitrary_info_numeric_and_string_fields() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("arbitrary_info.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/expression_parity.vcf").to_str().unwrap(),
            "--where",
            "INFO/MQ >= 50 && INFO/CSQ == \"synonymous_variant\"",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = std::fs::read_to_string(output).unwrap();
    assert!(text.contains("#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tHG002\tHG003"));
    assert!(text.contains("chr1\t101\trs1\tA\tG\t60\tPASS\tMQ=55;FS=12.5,8.2;CSQ=synonymous_variant;SOMATIC\tAD:FT:DP:GQ\t4,11:PASS:22:35\t10,0:LowDP:10:20"));
    assert!(!text.contains("chr1\t102\trs2"));
    assert!(!text.contains("chr1\t103\trs3"));
}
```

- [ ] **Step 3: Add missing-value behavior CLI test**

Append to `tests/filter_cli_tests.rs`:

```rust
#[test]
fn arbitrary_info_missing_empty_flag_and_dot_do_not_match() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("arbitrary_info_missing.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/expression_parity.vcf").to_str().unwrap(),
            "--where",
            "INFO/SOMATIC == \"true\" || INFO/EMPTY == \"\" || INFO/MQ == \".\"",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = std::fs::read_to_string(output).unwrap();
    assert!(text.contains("#CHROM"));
    assert!(!text.contains("chr1\t101\trs1"));
    assert!(!text.contains("chr1\t102\trs2"));
    assert!(!text.contains("chr1\t103\trs3"));
}
```

- [ ] **Step 4: Create v0.9 benchmark report scaffold**

Create `benchmark/reports/v09-expression-parity-benchmark.md`:

```markdown
# VCF-Fast v0.9 Expression Parity Benchmark

## Status

This report tracks correctness and performance for v0.9 expression parity cases. Rows are added only after the command output matches the stated `bcftools` baseline.

## Native Expression Cases

| Case | Dataset | VCF-Fast command | Competitor command | Correctness result | Runtime | Speedup | Caveat |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Arbitrary INFO numeric/string | `tests/data/expression_parity.vcf` | `vcf-fast filter tests/data/expression_parity.vcf --where 'INFO/MQ >= 50 && INFO/CSQ == "synonymous_variant"' -o out.vcf` | `bcftools filter -i 'INFO/MQ >= 50 && INFO/CSQ == "synonymous_variant"' tests/data/expression_parity.vcf -o bcftools.vcf` | Fixture expectation covered by integration tests; public benchmark measurement pending | `n/a` | `n/a` | Small fixture proves semantics, not performance |
| Selected arbitrary FORMAT | `tests/data/expression_parity.vcf` | `vcf-fast filter tests/data/expression_parity.vcf --sample HG002 --where 'FORMAT/AD > 8 && FORMAT/FT == "PASS"' -o out.vcf` | `bcftools view -s HG002 tests/data/expression_parity.vcf | bcftools filter -i 'FMT/AD[*] > 8 && FMT/FT == "PASS"' -o bcftools.vcf` | Fixture expectation covered by integration tests; bcftools vector syntax requires explicit normalization | `n/a` | `n/a` | bcftools FORMAT vector semantics differ for multi-value fields |
| ANY sample aggregate | `tests/data/expression_parity.vcf` | `vcf-fast filter tests/data/expression_parity.vcf --where 'ANY(FORMAT/AD > 15)' -o out.vcf` | `bcftools filter -i 'N_PASS(FMT/AD[*] > 15) > 0' tests/data/expression_parity.vcf -o bcftools.vcf` | Fixture expectation covered by integration tests; normalized bcftools comparison pending | `n/a` | `n/a` | Aggregate semantics are intentionally documented before public performance claims |

## Required Report Fields

- dataset source
- dataset size
- record count
- exact VCF-Fast command
- exact competitor command
- competitor version
- correctness result
- runtime mean and standard deviation
- speedup
- variants per second
- peak RSS
- caveat
```

- [ ] **Step 5: Add benchmark report harness test**

Append to `tests/benchmark_harness_tests.rs`:

```rust
#[test]
fn v09_expression_parity_report_tracks_required_fields() {
    let report = std::fs::read_to_string("benchmark/reports/v09-expression-parity-benchmark.md")
        .expect("read v0.9 report");

    for required in [
        "dataset source",
        "dataset size",
        "record count",
        "exact VCF-Fast command",
        "exact competitor command",
        "competitor version",
        "correctness result",
        "runtime mean",
        "speedup",
        "variants per second",
        "peak RSS",
        "caveat",
    ] {
        assert!(report.contains(required), "missing {required}");
    }
}
```

- [ ] **Step 6: Run Task 5 tests**

Run:

```bash
cargo test --test filter_cli_tests arbitrary_info
cargo test --test benchmark_harness_tests v09_expression_parity_report_tracks_required_fields
```

Expected: PASS.

- [ ] **Step 7: Commit Task 5**

Run:

```bash
git add tests/data/expression_parity.vcf tests/filter_cli_tests.rs benchmark/reports/v09-expression-parity-benchmark.md tests/benchmark_harness_tests.rs
git commit -m "test: add expression parity fixtures and report scaffold"
```

---

### Task 6: Documentation And Claim Matrix

**Files:**
- Modify: `README.md`
- Modify: `docs/contribution-map.md`
- Modify: `benchmark/reports/v09-expression-parity-benchmark.md`

- [ ] **Step 1: Update README expression support**

In `README.md`, update the filter examples section with:

````markdown
### v0.9 Expression Support

Native `.vcf` and `.vcf.gz` filtering supports site fields (`CHROM`, `POS`, `QUAL`, `FILTER`), arbitrary `INFO/<KEY>` fields, selected-sample `FORMAT/<KEY>` fields with `--sample`, and native sample aggregate predicates:

```bash
vcf-fast filter input.vcf.gz --where 'INFO/MQ >= 50 && INFO/CSQ != "missense_variant"' -o filtered.vcf.gz
vcf-fast filter input.vcf.gz --sample HG002 --where 'FORMAT/DP > 10 && FORMAT/GQ >= 20' -o sample.vcf.gz
vcf-fast filter input.vcf.gz --where 'ANY(FORMAT/DP > 20)' -o any_sample.vcf.gz
vcf-fast filter input.vcf.gz --where 'ALL(FORMAT/GQ >= 30)' -o all_samples.vcf.gz
```

`DP` and `AF` remain aliases for `INFO/DP` and `INFO/AF`. Missing values, empty values, flag-only INFO entries, and `.` do not satisfy predicates. htslib-backed BCF or indexed-region paths keep the v0.8 expression subset for this milestone and return a clear error for native-only `ANY`/`ALL` FORMAT predicates.
````

- [ ] **Step 2: Update the contribution map**

In `docs/contribution-map.md`, add a v0.9 row:

```markdown
| Native expression parity | Arbitrary `INFO/<KEY>`, selected-sample `FORMAT/<KEY>`, and `ANY`/`ALL` FORMAT aggregates for native VCF/VCF.GZ filtering | `src/expr/mod.rs`, `src/engine/filter.rs`, `tests/data/expression_parity.vcf`, `tests/filter_cli_tests.rs`, `benchmark/reports/v09-expression-parity-benchmark.md` | `bcftools filter` syntax and behavior notes | Correctness covered by unit and integration fixtures; public performance rows pending measured benchmark runs | htslib-backed aggregate predicates are explicitly rejected in v0.9; bcftools vector FORMAT semantics require normalization for exact comparisons |
```

- [ ] **Step 3: Keep benchmark report claims cautious**

In `benchmark/reports/v09-expression-parity-benchmark.md`, ensure the status section includes this sentence:

```markdown
No runtime win is claimed for v0.9 expression parity until public benchmark rows are measured and correctness-normalized against `bcftools`.
```

- [ ] **Step 4: Run docs and harness checks**

Run:

```bash
cargo test --test benchmark_harness_tests v09_expression_parity_report_tracks_required_fields
rg "ANY\\(FORMAT|INFO/<KEY>|bcftools|No runtime win" README.md docs/contribution-map.md benchmark/reports/v09-expression-parity-benchmark.md
```

Expected: tests PASS and `rg` prints matches from all three files.

- [ ] **Step 5: Commit Task 6**

Run:

```bash
git add README.md docs/contribution-map.md benchmark/reports/v09-expression-parity-benchmark.md
git commit -m "docs: document v09 expression parity scope"
```

---

### Task 7: Full Verification And Merge Readiness

**Files:**
- Verify all touched files

- [ ] **Step 1: Format**

Run:

```bash
cargo fmt --check
```

Expected: PASS. If formatting fails, run `cargo fmt`, inspect `git diff`, and repeat `cargo fmt --check`.

- [ ] **Step 2: Clippy default build**

Run:

```bash
cargo clippy --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 3: Test default build**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 4: Verify Makefile workflow**

Run:

```bash
make verify
```

Expected: PASS.

- [ ] **Step 5: Verify htslib feature boundary**

Run:

```bash
cargo test --features htslib-static
cargo clippy --features htslib-static --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 6: Run a native smoke command**

Run:

```bash
cargo run -- filter tests/data/expression_parity.vcf --where 'INFO/MQ >= 50 && ANY(FORMAT/DP > 20)' -o tests/output/v09-smoke.vcf
```

Expected: command exits 0 and `tests/output/v09-smoke.vcf` contains `chr1	101	rs1` only.

- [ ] **Step 7: Confirm worktree state**

Run:

```bash
git status --short --branch
```

Expected: current branch is the v0.9 implementation branch and no untracked benchmark artifacts outside ignored `tests/output/...` paths are present.

- [ ] **Step 8: Commit verification-only changes if any docs changed during verification**

Run only if `git status --short` shows tracked documentation or test changes:

```bash
git add README.md docs/contribution-map.md benchmark/reports/v09-expression-parity-benchmark.md tests/benchmark_harness_tests.rs
git commit -m "chore: finalize v09 expression parity verification"
```

Expected: commit succeeds or there are no tracked changes to commit.

---

## Implementation Notes

- Do not add a new CLI option for v0.9 sample aggregates. `ANY` and `ALL` operate over all sample columns in native VCF/VCF.GZ input.
- Keep passing native records byte-for-byte identical. All filter tests that assert original record text must continue to compare full lines, including FORMAT and sample columns.
- Keep arbitrary FORMAT parsing lazy. The evaluator should call `format_value_bytes(format, sample, key)` only for keys required by the expression.
- Keep arbitrary INFO parsing lazy. `InfoView::value(key)` scans borrowed INFO bytes and does not allocate record-owned strings.
- Avoid broad performance claims. v0.9 is primarily expression coverage; performance claims belong in measured reports after correctness normalization.
- Keep `||` behavior unchanged if it is already supported. This plan does not expand boolean grammar beyond the existing parser behavior.

## Final Acceptance Checklist

- [ ] `INFO/MQ`, `INFO/CSQ`, and other arbitrary INFO keys parse and evaluate in native filters.
- [ ] Numeric arbitrary INFO values use any-comma-value semantics.
- [ ] Missing, empty, flag-only, and `.` INFO values evaluate false.
- [ ] `FORMAT/AD`, `FORMAT/FT`, and other arbitrary FORMAT keys parse and evaluate for a selected sample.
- [ ] Plain `FORMAT/<KEY>` predicates still require `--sample`.
- [ ] `ANY(FORMAT/<KEY> op literal)` and `ALL(FORMAT/<KEY> op literal)` work over native sample columns.
- [ ] htslib-backed input exits clearly for native-only `ANY`/`ALL` predicates.
- [ ] Native passing records remain line-preserved.
- [ ] `make verify` passes.
- [ ] `cargo test --features htslib-static` passes.
- [ ] README, contribution map, and v0.9 report describe measured and unmeasured claims honestly.
