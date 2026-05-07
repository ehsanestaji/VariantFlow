# BGZF Virtual-Offset Index Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade VariantFlow `.vfi` indexes from record-chunk metadata to BGZF virtual-offset metadata and add safe index-aware native filtering that matches default output byte-for-byte.

**Architecture:** Keep ordinary streaming unchanged. Add a focused BGZF block walker, a versioned index schema, a conservative expression skip planner, and an indexed filter branch that only skips chunks proven impossible. Existing native record evaluation remains the source of truth.

**Tech Stack:** Rust, `flate2` raw deflate decoding, existing `memchr`, `serde`/`serde_json`, existing `RecordView`/`InfoView`, existing native filter evaluator, existing CLI binaries `variantflow` and `vcf-fast`.

---

## File Structure

- Modify `src/engine/index.rs`: keep CLI entrypoint, delegate schema writing to submodules, preserve plain VCF record-chunk behavior.
- Create `src/engine/index/bgzf.rs`: parse BGZF blocks, expose block-start virtual offsets, and read chunk ranges by virtual offset.
- Create `src/engine/index/schema.rs`: shared `.vfi` structs, source identity, chunk metadata, load/save helpers.
- Create `src/engine/index/metadata.rs`: reusable record metadata aggregation for index writer and tests.
- Create `src/engine/index/planner.rs`: expression-to-index skip planner.
- Modify `src/expr/mod.rs`: expose a small crate-visible visitor or skip-planner hook without making AST public API.
- Modify `src/engine/filter.rs`: auto-discover matching `.vfi`, validate it, and use indexed filtering when safe.
- Modify `tests/index_cli_tests.rs`: v2 schema, BGZF virtual-offset, stale-source, and plain fallback tests.
- Modify `tests/filter_cli_tests.rs`: default-vs-indexed byte-for-byte tests.
- Add `benchmark/run_v21_indexed_filter_benchmarks.sh`: skip-heavy synthetic and public IGSR benchmark harness.
- Modify `Makefile`: add `bench-v21-index`.
- Add `benchmark/reports/v21-indexed-filter-benchmark.md`: tracked report template.

## Task 1: Versioned `.vfi` Schema And Source Identity

**Files:**
- Create: `src/engine/index/schema.rs`
- Modify: `src/engine/index.rs`
- Test: `tests/index_cli_tests.rs`

- [ ] **Step 1: Write failing schema tests**

Add this test to `tests/index_cli_tests.rs`:

```rust
#[test]
fn plain_vcf_index_records_source_identity_and_record_chunk_offset_model() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("plain.vcf");
    let output = dir.path().join("plain.vcf.vfi");
    fs::write(&input, FORMAT_RICH_VCF).unwrap();

    Command::cargo_bin("variantflow")
        .unwrap()
        .args(["index", input.to_str().unwrap(), "-o", output.to_str().unwrap()])
        .assert()
        .success();

    let json: Value = serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    assert_eq!(json["schema_version"], 2);
    assert_eq!(json["index_kind"], "variantflow-vfi");
    assert_eq!(json["offset_model"], "record-chunk");
    assert_eq!(json["virtual_offsets_available"], false);
    assert_eq!(json["source"]["size_bytes"].as_u64().unwrap(), fs::metadata(&input).unwrap().len());
    assert!(json["source"]["modified_unix_seconds"].as_u64().unwrap() > 0);
    assert_eq!(json["record_count"], 2);
}
```

- [ ] **Step 2: Run the test and verify it fails**

Run:

```bash
cargo test --test index_cli_tests plain_vcf_index_records_source_identity_and_record_chunk_offset_model
```

Expected: FAIL because the current schema version is `1` and `source` is a string.

- [ ] **Step 3: Create schema structs**

Create `src/engine/index/schema.rs` with:

```rust
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct VariantFlowIndex {
    pub schema_version: u32,
    pub index_kind: String,
    pub offset_model: OffsetModel,
    pub virtual_offsets_available: bool,
    pub source: SourceIdentity,
    pub chunk_record_target: u64,
    pub record_count: u64,
    pub chunks: Vec<IndexChunk>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum OffsetModel {
    RecordChunk,
    BgzfVirtual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SourceIdentity {
    pub path: String,
    pub size_bytes: u64,
    pub modified_unix_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct IndexChunk {
    pub ordinal: u64,
    pub first_record: u64,
    pub record_count: u64,
    pub chrom_start: String,
    pub chrom_end: String,
    pub pos_min: u64,
    pub pos_max: u64,
    pub qual_min: Option<f64>,
    pub qual_max: Option<f64>,
    pub filters: Vec<String>,
    pub info_dp_min: Option<i64>,
    pub info_dp_max: Option<i64>,
    pub info_af_min: Option<f64>,
    pub info_af_max: Option<f64>,
    pub info_af_complete: bool,
    pub has_info_af: bool,
    pub format_keys: Vec<String>,
    pub virtual_start: Option<u64>,
    pub virtual_end: Option<u64>,
}

pub(crate) fn source_identity(path: &Path) -> Result<SourceIdentity> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("failed to inspect source {}", path.display()))?;
    let modified = metadata
        .modified()
        .with_context(|| format!("failed to read modified time for {}", path.display()))?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(SourceIdentity {
        path: path.display().to_string(),
        size_bytes: metadata.len(),
        modified_unix_seconds: modified,
    })
}

pub(crate) fn source_matches(index: &VariantFlowIndex, path: &Path) -> Result<bool> {
    Ok(index.source == source_identity(path)?)
}
```

- [ ] **Step 4: Update `src/engine/index.rs` to use schema structs**

Replace the local `VariantFlowIndex` and `IndexChunk` structs with imports:

```rust
mod schema;

use schema::{IndexChunk, OffsetModel, VariantFlowIndex, source_identity};
```

Build the plain index with:

```rust
let index = VariantFlowIndex {
    schema_version: 2,
    index_kind: "variantflow-vfi".to_string(),
    offset_model: OffsetModel::RecordChunk,
    virtual_offsets_available: false,
    source: source_identity(input)?,
    chunk_record_target,
    record_count,
    chunks,
};
```

Set these new chunk fields for plain VCF chunks:

```rust
info_af_min: None,
info_af_max: None,
info_af_complete: false,
virtual_start: None,
virtual_end: None,
```

- [ ] **Step 5: Run test and commit**

Run:

```bash
cargo test --test index_cli_tests plain_vcf_index_records_source_identity_and_record_chunk_offset_model
cargo fmt
```

Expected: PASS.

Commit:

```bash
git add src/engine/index.rs src/engine/index/schema.rs tests/index_cli_tests.rs
git commit -m "feat: version VFI schema with source identity"
```

## Task 2: Shared Chunk Metadata Aggregator

**Files:**
- Create: `src/engine/index/metadata.rs`
- Modify: `src/engine/index.rs`
- Test: `src/engine/index/metadata.rs`

- [ ] **Step 1: Write failing metadata unit tests**

Create `src/engine/index/metadata.rs` with only tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::vcf::RecordView;

    #[test]
    fn chunk_builder_tracks_qual_filter_info_and_format_metadata() {
        let mut builder = ChunkMetadataBuilder::new(0, 0, None);
        let line = b"chr1\t10\t.\tA\tG\t50\tPASS\tDP=12;AF=0.2,0.3\tGT:DP:AD\t0/1:12:6,6\n";
        let record = RecordView::parse(line).unwrap();

        builder.observe(&record).unwrap();
        let chunk = builder.finish(Some(65536)).unwrap();

        assert_eq!(chunk.record_count, 1);
        assert_eq!(chunk.chrom_start, "chr1");
        assert_eq!(chunk.pos_min, 10);
        assert_eq!(chunk.qual_min, Some(50.0));
        assert_eq!(chunk.qual_max, Some(50.0));
        assert_eq!(chunk.info_dp_min, Some(12));
        assert_eq!(chunk.info_dp_max, Some(12));
        assert_eq!(chunk.info_af_min, Some(0.2));
        assert_eq!(chunk.info_af_max, Some(0.3));
        assert!(chunk.info_af_complete);
        assert_eq!(chunk.filters, vec!["PASS"]);
        assert_eq!(chunk.format_keys, vec!["AD", "DP", "GT"]);
        assert_eq!(chunk.virtual_start, None);
        assert_eq!(chunk.virtual_end, Some(65536));
    }

    #[test]
    fn chunk_builder_marks_af_incomplete_when_any_value_is_non_numeric() {
        let mut builder = ChunkMetadataBuilder::new(0, 0, Some(131072));
        let line = b"chr1\t20\t.\tA\tT\t10\tq10\tDP=5;AF=.\tGT:DP\t0/1:5\n";
        let record = RecordView::parse(line).unwrap();

        builder.observe(&record).unwrap();
        let chunk = builder.finish(Some(196608)).unwrap();

        assert!(chunk.has_info_af);
        assert!(!chunk.info_af_complete);
        assert_eq!(chunk.info_af_min, None);
        assert_eq!(chunk.info_af_max, None);
        assert_eq!(chunk.virtual_start, Some(131072));
        assert_eq!(chunk.virtual_end, Some(196608));
    }
}
```

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
cargo test metadata::tests::chunk_builder_tracks_qual_filter_info_and_format_metadata
```

Expected: FAIL because `ChunkMetadataBuilder` does not exist.

- [ ] **Step 3: Implement `ChunkMetadataBuilder`**

Fill `src/engine/index/metadata.rs` with:

```rust
use std::collections::BTreeSet;

use anyhow::Result;
use memchr::memchr;

use crate::engine::index::schema::IndexChunk;
use crate::vcf::{InfoView, RecordView};

#[derive(Debug)]
pub(crate) struct ChunkMetadataBuilder {
    ordinal: u64,
    first_record: u64,
    virtual_start: Option<u64>,
    record_count: u64,
    chrom_start: String,
    chrom_end: String,
    pos_min: u64,
    pos_max: u64,
    qual_min: Option<f64>,
    qual_max: Option<f64>,
    filters: BTreeSet<String>,
    info_dp_min: Option<i64>,
    info_dp_max: Option<i64>,
    info_af_min: Option<f64>,
    info_af_max: Option<f64>,
    info_af_seen: bool,
    info_af_complete: bool,
    format_keys: BTreeSet<String>,
}

impl ChunkMetadataBuilder {
    pub(crate) fn new(ordinal: u64, first_record: u64, virtual_start: Option<u64>) -> Self {
        Self {
            ordinal,
            first_record,
            virtual_start,
            record_count: 0,
            chrom_start: String::new(),
            chrom_end: String::new(),
            pos_min: u64::MAX,
            pos_max: 0,
            qual_min: None,
            qual_max: None,
            filters: BTreeSet::new(),
            info_dp_min: None,
            info_dp_max: None,
            info_af_min: None,
            info_af_max: None,
            info_af_seen: false,
            info_af_complete: true,
            format_keys: BTreeSet::new(),
        }
    }

    pub(crate) fn observe(&mut self, record: &RecordView<'_>) -> Result<()> {
        let chrom = String::from_utf8_lossy(record.chrom()).into_owned();
        if self.record_count == 0 {
            self.chrom_start = chrom.clone();
        }
        self.chrom_end = chrom;

        let pos = record.pos_u64()?;
        self.pos_min = self.pos_min.min(pos);
        self.pos_max = self.pos_max.max(pos);

        if let Some(qual) = record.qual_float()? {
            update_f64_minmax(&mut self.qual_min, &mut self.qual_max, qual);
        }

        observe_delimited_strings(record.filter(), b';', &mut self.filters);

        let info = InfoView::scan(record.info());
        if let Some(value) = info.value(b"DP") {
            for_each_comma_i64(value, |number| {
                update_i64_minmax(&mut self.info_dp_min, &mut self.info_dp_max, number);
            });
        }
        if let Some(value) = info.value(b"AF") {
            self.info_af_seen = true;
            let mut parsed_any = false;
            let mut all_numeric = true;
            for_each_delimited(value, b',', |part| {
                if part.is_empty() || part == b"." {
                    all_numeric = false;
                    return;
                }
                match std::str::from_utf8(part).ok().and_then(|text| text.parse::<f64>().ok()) {
                    Some(number) => {
                        parsed_any = true;
                        update_f64_minmax(&mut self.info_af_min, &mut self.info_af_max, number);
                    }
                    None => all_numeric = false,
                }
            });
            self.info_af_complete &= parsed_any && all_numeric;
        }

        if let Some(format) = record.column(8) {
            observe_delimited_strings(format, b':', &mut self.format_keys);
        }

        self.record_count += 1;
        Ok(())
    }

    pub(crate) fn record_count(&self) -> u64 {
        self.record_count
    }

    pub(crate) fn finish(self, virtual_end: Option<u64>) -> Option<IndexChunk> {
        (self.record_count > 0).then(|| IndexChunk {
            ordinal: self.ordinal,
            first_record: self.first_record,
            record_count: self.record_count,
            chrom_start: self.chrom_start,
            chrom_end: self.chrom_end,
            pos_min: self.pos_min,
            pos_max: self.pos_max,
            qual_min: self.qual_min,
            qual_max: self.qual_max,
            filters: self.filters.into_iter().collect(),
            info_dp_min: self.info_dp_min,
            info_dp_max: self.info_dp_max,
            info_af_min: self.info_af_min,
            info_af_max: self.info_af_max,
            info_af_complete: self.info_af_seen && self.info_af_complete,
            has_info_af: self.info_af_seen,
            format_keys: self.format_keys.into_iter().collect(),
            virtual_start: self.virtual_start,
            virtual_end,
        })
    }
}

fn observe_delimited_strings(value: &[u8], delimiter: u8, set: &mut BTreeSet<String>) {
    if value.is_empty() || value == b"." {
        return;
    }
    for_each_delimited(value, delimiter, |part| {
        if !part.is_empty() && part != b"." {
            set.insert(String::from_utf8_lossy(part).into_owned());
        }
    });
}

fn for_each_comma_i64(value: &[u8], mut observe: impl FnMut(i64)) {
    if value.is_empty() || value == b"." {
        return;
    }
    for_each_delimited(value, b',', |part| {
        if !part.is_empty()
            && part != b"."
            && let Ok(text) = std::str::from_utf8(part)
            && let Ok(number) = text.parse::<i64>()
        {
            observe(number);
        }
    });
}

fn for_each_delimited(mut value: &[u8], delimiter: u8, mut observe: impl FnMut(&[u8])) {
    loop {
        let end = memchr(delimiter, value).unwrap_or(value.len());
        observe(&value[..end]);
        if end == value.len() {
            break;
        }
        value = &value[end + 1..];
    }
}

fn update_i64_minmax(min: &mut Option<i64>, max: &mut Option<i64>, value: i64) {
    *min = Some(min.map_or(value, |current| current.min(value)));
    *max = Some(max.map_or(value, |current| current.max(value)));
}

fn update_f64_minmax(min: &mut Option<f64>, max: &mut Option<f64>, value: f64) {
    if value.is_finite() {
        *min = Some(min.map_or(value, |current| current.min(value)));
        *max = Some(max.map_or(value, |current| current.max(value)));
    }
}
```

- [ ] **Step 4: Refactor `src/engine/index.rs` to use the builder**

At the top of `src/engine/index.rs`, add:

```rust
mod metadata;

use metadata::ChunkMetadataBuilder;
```

Remove the old local `ChunkBuilder`, `observe_filters`, `observe_format_keys`,
`for_each_comma_i64`, `for_each_delimited`, `update_i64_minmax`, and
`update_f64_minmax` helpers. In `write_index`, create:

```rust
let mut current = ChunkMetadataBuilder::new(0, 0, None);
```

When finishing a full chunk:

```rust
if let Some(chunk) = current.finish(None) {
    chunks.push(chunk);
}
current = ChunkMetadataBuilder::new(chunks.len() as u64, record_count, None);
```

- [ ] **Step 5: Run tests and commit**

Run:

```bash
cargo test metadata::tests
cargo test --test index_cli_tests
cargo fmt
```

Expected: PASS.

Commit:

```bash
git add src/engine/index.rs src/engine/index/metadata.rs tests/index_cli_tests.rs
git commit -m "refactor: share VFI chunk metadata aggregation"
```

## Task 3: Native BGZF Block Walker With Virtual Offsets

**Files:**
- Create: `src/engine/index/bgzf.rs`
- Modify: `src/engine/index.rs`
- Test: `src/engine/index/bgzf.rs`, `tests/index_cli_tests.rs`

- [ ] **Step 1: Write failing BGZF unit tests**

Create `src/engine/index/bgzf.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn bgzf_block_reader_reports_block_boundary_virtual_offsets() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("mini.vcf.gz");
        let file = std::fs::File::create(&input).unwrap();
        let mut writer = noodles_bgzf::io::Writer::new(file);
        writer.write_all(b"##fileformat=VCFv4.3\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\nchr1\t1\t.\tA\tG\t50\tPASS\tDP=12\n").unwrap();
        writer.finish().unwrap();

        let blocks = read_bgzf_blocks(&input).unwrap();
        assert!(blocks.iter().any(|block| !block.uncompressed.is_empty()));
        assert_eq!(blocks[0].virtual_start(), 0);
        assert!(blocks[0].virtual_end() > blocks[0].virtual_start());
    }

    #[test]
    fn non_bgzf_gzip_is_rejected_by_block_reader() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("ordinary.vcf.gz");
        let file = std::fs::File::create(&input).unwrap();
        let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        encoder.write_all(b"plain gzip").unwrap();
        encoder.finish().unwrap();

        let error = read_bgzf_blocks(&input).unwrap_err();
        assert!(error.to_string().contains("not a BGZF file"));
    }
}
```

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
cargo test bgzf::tests
```

Expected: FAIL because `read_bgzf_blocks` does not exist.

- [ ] **Step 3: Implement BGZF parsing**

Fill `src/engine/index/bgzf.rs` with:

```rust
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::{Context, Result, bail};
use flate2::read::DeflateDecoder;

#[derive(Debug, Clone)]
pub(crate) struct BgzfBlock {
    pub(crate) compressed_start: u64,
    pub(crate) compressed_end: u64,
    pub(crate) uncompressed: Vec<u8>,
}

impl BgzfBlock {
    pub(crate) fn virtual_start(&self) -> u64 {
        self.compressed_start << 16
    }

    pub(crate) fn virtual_end(&self) -> u64 {
        self.compressed_end << 16
    }
}

pub(crate) fn read_bgzf_blocks(path: &Path) -> Result<Vec<BgzfBlock>> {
    let mut file = File::open(path)
        .with_context(|| format!("failed to open BGZF input {}", path.display()))?;
    let mut blocks = Vec::new();

    loop {
        let compressed_start = file.stream_position()?;
        let mut fixed = [0_u8; 12];
        match file.read_exact(&mut fixed) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error.into()),
        }

        if fixed[0] != 0x1f || fixed[1] != 0x8b || fixed[2] != 8 || fixed[3] & 0x04 == 0 {
            bail!("{} is not a BGZF file", path.display());
        }

        let extra_len = u16::from_le_bytes([fixed[10], fixed[11]]) as usize;
        let mut extra = vec![0_u8; extra_len];
        file.read_exact(&mut extra)?;

        let block_size = parse_bgzf_block_size(&extra)
            .ok_or_else(|| anyhow::anyhow!("{} is not a BGZF file", path.display()))?;
        let compressed_end = compressed_start + u64::from(block_size);
        let remaining = block_size as usize - 12 - extra_len;
        let mut rest = vec![0_u8; remaining];
        file.read_exact(&mut rest)?;

        let payload_len = remaining
            .checked_sub(8)
            .ok_or_else(|| anyhow::anyhow!("invalid BGZF block in {}", path.display()))?;
        let mut decoder = DeflateDecoder::new(&rest[..payload_len]);
        let mut uncompressed = Vec::new();
        decoder.read_to_end(&mut uncompressed)?;

        blocks.push(BgzfBlock {
            compressed_start,
            compressed_end,
            uncompressed,
        });

        file.seek(SeekFrom::Start(compressed_end))?;
    }

    Ok(blocks)
}

fn parse_bgzf_block_size(extra: &[u8]) -> Option<u16> {
    let mut index = 0;
    while index + 4 <= extra.len() {
        let id = &extra[index..index + 2];
        let len = u16::from_le_bytes([extra[index + 2], extra[index + 3]]) as usize;
        index += 4;
        if index + len > extra.len() {
            return None;
        }
        if id == b"BC" && len == 2 {
            let bsize = u16::from_le_bytes([extra[index], extra[index + 1]]);
            return Some(bsize + 1);
        }
        index += len;
    }
    None
}
```

- [ ] **Step 4: Add BGZF index CLI test**

In `tests/index_cli_tests.rs`, update `index_accepts_bgzf_vcf_input` assertions:

```rust
assert_eq!(json["schema_version"], 2);
assert_eq!(json["offset_model"], "bgzf-virtual");
assert_eq!(json["virtual_offsets_available"], true);
assert!(json["chunks"][0]["virtual_start"].as_u64().is_some());
assert!(json["chunks"][0]["virtual_end"].as_u64().unwrap() > json["chunks"][0]["virtual_start"].as_u64().unwrap());
```

- [ ] **Step 5: Wire BGZF indexing in `src/engine/index.rs`**

At the top:

```rust
mod bgzf;

use bgzf::read_bgzf_blocks;
use schema::OffsetModel;
```

In `run`, dispatch `.vcf.gz` inputs to a new `write_bgzf_index` first:

```rust
pub fn run(input: &Path, output: &Path) -> Result<()> {
    if input.extension().is_some_and(|extension| extension == "gz") {
        return write_bgzf_index(input, output, DEFAULT_CHUNK_RECORDS)
            .or_else(|_| write_index(input, output, DEFAULT_CHUNK_RECORDS));
    }
    write_index(input, output, DEFAULT_CHUNK_RECORDS)
}
```

Add `write_bgzf_index` that:

```rust
fn write_bgzf_index(input: &Path, output: &Path, chunk_record_target: u64) -> Result<()> {
    let blocks = read_bgzf_blocks(input)?;
    let mut chunks = Vec::new();
    let mut record_count = 0_u64;
    let mut pending_line = Vec::new();
    let mut current = ChunkMetadataBuilder::new(0, 0, blocks.first().map(|block| block.virtual_start()));

    for block in &blocks {
        for byte in &block.uncompressed {
            pending_line.push(*byte);
            if *byte == b'\n' {
                if !pending_line.starts_with(b"#") {
                    let record = RecordView::parse(&pending_line)?;
                    current.observe(&record)?;
                    record_count += 1;
                }
                pending_line.clear();
            }
        }

        if pending_line.is_empty() && current.record_count() >= chunk_record_target {
            if let Some(chunk) = current.finish(Some(block.virtual_end())) {
                chunks.push(chunk);
            }
            current = ChunkMetadataBuilder::new(
                chunks.len() as u64,
                record_count,
                Some(block.virtual_end()),
            );
        }
    }

    if !pending_line.is_empty() && !pending_line.starts_with(b"#") {
        let record = RecordView::parse(&pending_line)?;
        current.observe(&record)?;
        record_count += 1;
    }

    if let Some(last_block) = blocks.last()
        && let Some(chunk) = current.finish(Some(last_block.virtual_end()))
    {
        chunks.push(chunk);
    }

    let index = VariantFlowIndex {
        schema_version: 2,
        index_kind: "variantflow-vfi".to_string(),
        offset_model: OffsetModel::BgzfVirtual,
        virtual_offsets_available: true,
        source: source_identity(input)?,
        chunk_record_target,
        record_count,
        chunks,
    };

    write_index_json(output, &index)
}
```

Extract the JSON write logic into:

```rust
fn write_index_json(output: &Path, index: &VariantFlowIndex) -> Result<()> {
    let file = File::create(output)
        .with_context(|| format!("failed to create index {}", output.display()))?;
    serde_json::to_writer_pretty(BufWriter::new(file), index)
        .with_context(|| format!("failed to write index {}", output.display()))?;
    Ok(())
}
```

- [ ] **Step 6: Run tests and commit**

Run:

```bash
cargo test bgzf::tests
cargo test --test index_cli_tests
cargo fmt
```

Expected: PASS.

Commit:

```bash
git add src/engine/index.rs src/engine/index/bgzf.rs tests/index_cli_tests.rs
git commit -m "feat: write BGZF virtual offsets to VFI"
```

## Task 4: Safe Skip Planner

**Files:**
- Create: `src/engine/index/planner.rs`
- Modify: `src/expr/mod.rs`
- Test: `src/engine/index/planner.rs`

- [ ] **Step 1: Write failing planner tests**

Create `src/engine/index/planner.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::index::schema::IndexChunk;
    use crate::expr::parse_expression;

    fn chunk() -> IndexChunk {
        IndexChunk {
            ordinal: 0,
            first_record: 0,
            record_count: 10,
            chrom_start: "chr1".to_string(),
            chrom_end: "chr1".to_string(),
            pos_min: 1,
            pos_max: 10,
            qual_min: Some(1.0),
            qual_max: Some(20.0),
            filters: vec!["q10".to_string()],
            info_dp_min: Some(3),
            info_dp_max: Some(10),
            info_af_min: Some(0.01),
            info_af_max: Some(0.05),
            info_af_complete: true,
            has_info_af: true,
            format_keys: vec!["GT".to_string()],
            virtual_start: Some(0),
            virtual_end: Some(65536),
        }
    }

    #[test]
    fn skips_qual_gt_when_chunk_max_cannot_pass() {
        let expr = parse_expression("QUAL > 30").unwrap();
        assert_eq!(plan_chunk(&expr, &chunk()), SkipDecision::CanSkip);
    }

    #[test]
    fn scans_filter_eq_when_value_is_present() {
        let expr = parse_expression("FILTER == \"q10\"").unwrap();
        assert_eq!(plan_chunk(&expr, &chunk()), SkipDecision::MustScan);
    }

    #[test]
    fn skips_and_if_either_side_is_impossible() {
        let expr = parse_expression("QUAL > 30 && FILTER == \"q10\"").unwrap();
        assert_eq!(plan_chunk(&expr, &chunk()), SkipDecision::CanSkip);
    }

    #[test]
    fn skips_or_only_when_both_sides_are_impossible() {
        let expr = parse_expression("QUAL > 30 || INFO/DP > 40").unwrap();
        assert_eq!(plan_chunk(&expr, &chunk()), SkipDecision::CanSkip);
    }

    #[test]
    fn unsupported_format_aggregate_falls_back() {
        let expr = parse_expression("ANY(FORMAT/AD > 80)").unwrap();
        assert_eq!(plan_chunk(&expr, &chunk()), SkipDecision::UnsupportedForIndex);
    }
}
```

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
cargo test planner::tests
```

Expected: FAIL because `plan_chunk` does not exist and expression internals are private.

- [ ] **Step 3: Add crate-visible expression visitor**

Change the existing declarations in `src/expr/mod.rs` so they become crate-visible:

```rust
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ExprNode {
    Comparison(Comparison),
    SampleAggregate {
        quantifier: SampleQuantifier,
        comparison: Comparison,
    },
    CountAggregate {
        comparison: Comparison,
        op: Operator,
        literal: Literal,
    },
    And(Box<ExprNode>, Box<ExprNode>),
    Or(Box<ExprNode>, Box<ExprNode>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Comparison {
    field: Field,
    op: Operator,
    literal: Literal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Field {
    Chrom,
    Pos,
    Qual,
    Filter,
    Info { key: Vec<u8>, index: Option<usize> },
    Format { key: Vec<u8>, index: Option<usize> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operator {
    Gt,
    Gte,
    Lt,
    Lte,
    Eq,
    Ne,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Literal {
    Number(f64),
    String(String),
}
```

Add:

```rust
impl Expression {
    pub(crate) fn root_node(&self) -> &ExprNode {
        &self.root
    }
}
```

- [ ] **Step 4: Implement planner**

Fill `src/engine/index/planner.rs` with:

```rust
use crate::engine::index::schema::IndexChunk;
use crate::expr::{ExprNode, Expression, Field, Literal, Operator};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkipDecision {
    CanSkip,
    MustScan,
    UnsupportedForIndex,
}

pub(crate) fn plan_chunk(expr: &Expression, chunk: &IndexChunk) -> SkipDecision {
    plan_node(expr.root_node(), chunk)
}

fn plan_node(node: &ExprNode, chunk: &IndexChunk) -> SkipDecision {
    match node {
        ExprNode::Comparison(comparison) => plan_comparison(comparison.field(), comparison.op(), comparison.literal(), chunk),
        ExprNode::And(left, right) => combine_and(plan_node(left, chunk), plan_node(right, chunk)),
        ExprNode::Or(left, right) => combine_or(plan_node(left, chunk), plan_node(right, chunk)),
        ExprNode::SampleAggregate { .. } | ExprNode::CountAggregate { .. } => SkipDecision::UnsupportedForIndex,
    }
}

fn combine_and(left: SkipDecision, right: SkipDecision) -> SkipDecision {
    match (left, right) {
        (SkipDecision::CanSkip, _) | (_, SkipDecision::CanSkip) => SkipDecision::CanSkip,
        (SkipDecision::UnsupportedForIndex, _) | (_, SkipDecision::UnsupportedForIndex) => SkipDecision::UnsupportedForIndex,
        _ => SkipDecision::MustScan,
    }
}

fn combine_or(left: SkipDecision, right: SkipDecision) -> SkipDecision {
    match (left, right) {
        (SkipDecision::CanSkip, SkipDecision::CanSkip) => SkipDecision::CanSkip,
        (SkipDecision::UnsupportedForIndex, _) | (_, SkipDecision::UnsupportedForIndex) => SkipDecision::UnsupportedForIndex,
        _ => SkipDecision::MustScan,
    }
}
```

Add crate-visible accessors to `Comparison` in `src/expr/mod.rs`:

```rust
impl Comparison {
    pub(crate) fn field(&self) -> &Field {
        &self.field
    }

    pub(crate) fn op(&self) -> Operator {
        self.op
    }

    pub(crate) fn literal(&self) -> &Literal {
        &self.literal
    }
}
```

Continue `planner.rs` with:

```rust
fn plan_comparison(field: &Field, op: Operator, literal: &Literal, chunk: &IndexChunk) -> SkipDecision {
    match (field, literal) {
        (Field::Qual, Literal::Number(value)) => plan_numeric_range(chunk.qual_min, chunk.qual_max, op, *value),
        (Field::Filter, Literal::String(value)) => plan_filter(&chunk.filters, op, value),
        (Field::Info { key, index: None }, Literal::Number(value)) if key == b"DP" => {
            plan_i64_range(chunk.info_dp_min, chunk.info_dp_max, op, *value)
        }
        (Field::Info { key, index: None }, Literal::Number(value)) if key == b"AF" && chunk.info_af_complete => {
            plan_numeric_range(chunk.info_af_min, chunk.info_af_max, op, *value)
        }
        (Field::Info { key, index: None }, Literal::Number(value)) if key == b"AF" && !chunk.info_af_complete => {
            let _ = value;
            SkipDecision::MustScan
        }
        _ => SkipDecision::UnsupportedForIndex,
    }
}

fn plan_i64_range(min: Option<i64>, max: Option<i64>, op: Operator, value: f64) -> SkipDecision {
    plan_numeric_range(min.map(|v| v as f64), max.map(|v| v as f64), op, value)
}

fn plan_numeric_range(min: Option<f64>, max: Option<f64>, op: Operator, value: f64) -> SkipDecision {
    let (Some(min), Some(max)) = (min, max) else {
        return SkipDecision::MustScan;
    };
    match op {
        Operator::Gt => (max <= value).then_some(SkipDecision::CanSkip).unwrap_or(SkipDecision::MustScan),
        Operator::Gte => (max < value).then_some(SkipDecision::CanSkip).unwrap_or(SkipDecision::MustScan),
        Operator::Lt => (min >= value).then_some(SkipDecision::CanSkip).unwrap_or(SkipDecision::MustScan),
        Operator::Lte => (min > value).then_some(SkipDecision::CanSkip).unwrap_or(SkipDecision::MustScan),
        Operator::Eq => (value < min || value > max).then_some(SkipDecision::CanSkip).unwrap_or(SkipDecision::MustScan),
        Operator::Ne => {
            if min == max && min == value {
                SkipDecision::CanSkip
            } else {
                SkipDecision::MustScan
            }
        }
    }
}

fn plan_filter(filters: &[String], op: Operator, value: &str) -> SkipDecision {
    match op {
        Operator::Eq => {
            if filters.iter().any(|filter| filter == value) {
                SkipDecision::MustScan
            } else {
                SkipDecision::CanSkip
            }
        }
        Operator::Ne => {
            if filters.len() == 1 && filters[0] == value {
                SkipDecision::CanSkip
            } else {
                SkipDecision::MustScan
            }
        }
        _ => SkipDecision::UnsupportedForIndex,
    }
}
```

- [ ] **Step 5: Run tests and commit**

Run:

```bash
cargo test planner::tests
cargo test --test expr_tests
cargo fmt
```

Expected: PASS.

Commit:

```bash
git add src/expr/mod.rs src/engine/index/planner.rs
git commit -m "feat: add conservative VFI skip planner"
```

## Task 5: Indexed BGZF Range Reader

**Files:**
- Modify: `src/engine/index/bgzf.rs`
- Test: `src/engine/index/bgzf.rs`

- [ ] **Step 1: Write failing range-reader test**

Add to `src/engine/index/bgzf.rs` tests:

```rust
#[test]
fn bgzf_range_reader_returns_text_between_block_boundary_offsets() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("mini.vcf.gz");
    let file = std::fs::File::create(&input).unwrap();
    let mut writer = noodles_bgzf::io::Writer::new(file);
    writer.write_all(b"##fileformat=VCFv4.3\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\nchr1\t1\t.\tA\tG\t50\tPASS\tDP=12\n").unwrap();
    writer.finish().unwrap();

    let blocks = read_bgzf_blocks(&input).unwrap();
    let start = blocks[0].virtual_start();
    let end = blocks.last().unwrap().virtual_end();
    let text = read_virtual_range(&input, start, end).unwrap();

    assert!(String::from_utf8(text).unwrap().contains("chr1\t1"));
}
```

- [ ] **Step 2: Run test and verify failure**

Run:

```bash
cargo test bgzf::tests::bgzf_range_reader_returns_text_between_block_boundary_offsets
```

Expected: FAIL because `read_virtual_range` does not exist.

- [ ] **Step 3: Implement range reader**

Add to `src/engine/index/bgzf.rs`:

```rust
pub(crate) fn read_virtual_range(path: &Path, virtual_start: u64, virtual_end: u64) -> Result<Vec<u8>> {
    let start_compressed = virtual_start >> 16;
    let start_uncompressed = (virtual_start & 0xffff) as usize;
    let end_compressed = virtual_end >> 16;
    let end_uncompressed = (virtual_end & 0xffff) as usize;

    let mut file = File::open(path)
        .with_context(|| format!("failed to open BGZF input {}", path.display()))?;
    file.seek(SeekFrom::Start(start_compressed))?;

    let mut output = Vec::new();
    loop {
        let block_start = file.stream_position()?;
        if block_start >= end_compressed && end_uncompressed == 0 {
            break;
        }

        let block = read_one_bgzf_block(path, &mut file, block_start)?;
        let slice_start = if block.compressed_start == start_compressed {
            start_uncompressed
        } else {
            0
        };
        let slice_end = if block.compressed_start == end_compressed {
            end_uncompressed.min(block.uncompressed.len())
        } else {
            block.uncompressed.len()
        };

        if slice_start < slice_end {
            output.extend_from_slice(&block.uncompressed[slice_start..slice_end]);
        }

        if block.compressed_end >= end_compressed {
            break;
        }
    }

    Ok(output)
}
```

Refactor `read_bgzf_blocks` to use:

```rust
fn read_one_bgzf_block(path: &Path, file: &mut File, compressed_start: u64) -> Result<BgzfBlock> {
    file.seek(SeekFrom::Start(compressed_start))?;
    /* move the existing single-block parsing body here */
}
```

- [ ] **Step 4: Run tests and commit**

Run:

```bash
cargo test bgzf::tests
cargo fmt
```

Expected: PASS.

Commit:

```bash
git add src/engine/index/bgzf.rs
git commit -m "feat: read BGZF ranges by virtual offset"
```

## Task 6: Indexed Native Filter Fallback And Byte-Exact Output

**Files:**
- Modify: `src/engine/filter.rs`
- Modify: `src/engine/index.rs`
- Modify: `src/engine/index/schema.rs`
- Test: `tests/filter_cli_tests.rs`

- [ ] **Step 1: Write failing indexed filter tests**

Add to `tests/filter_cli_tests.rs`:

```rust
#[test]
fn indexed_bgzf_filter_matches_default_output_byte_for_byte() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("stress.vcf.gz");
    let index = dir.path().join("stress.vcf.gz.vfi");
    let default_output = dir.path().join("default.vcf");
    let indexed_output = dir.path().join("indexed.vcf");
    bgzf_fixture(&fixture("tests/data/stress_small.vcf"), &input);

    Command::cargo_bin("variantflow")
        .unwrap()
        .args(["index", input.to_str().unwrap(), "-o", index.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("variantflow")
        .unwrap()
        .args(["filter", input.to_str().unwrap(), "--where", "QUAL > 30", "-o", default_output.to_str().unwrap()])
        .env("VCF_FAST_DISABLE_VFI", "1")
        .assert()
        .success();

    Command::cargo_bin("variantflow")
        .unwrap()
        .args(["filter", input.to_str().unwrap(), "--where", "QUAL > 30", "-o", indexed_output.to_str().unwrap()])
        .assert()
        .success();

    assert_eq!(fs::read(&default_output).unwrap(), fs::read(&indexed_output).unwrap());
}

#[test]
fn stale_index_falls_back_to_default_streaming_output() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("stress.vcf.gz");
    let index = dir.path().join("stress.vcf.gz.vfi");
    let output = dir.path().join("out.vcf");
    bgzf_fixture(&fixture("tests/data/stress_small.vcf"), &input);

    fs::write(&index, r#"{"schema_version":2,"index_kind":"variantflow-vfi","offset_model":"bgzf-virtual","virtual_offsets_available":true,"source":{"path":"wrong","size_bytes":1,"modified_unix_seconds":1},"chunk_record_target":8192,"record_count":0,"chunks":[]}"#).unwrap();

    Command::cargo_bin("variantflow")
        .unwrap()
        .args(["filter", input.to_str().unwrap(), "--where", "QUAL > 30", "-o", output.to_str().unwrap()])
        .assert()
        .success();

    assert!(fs::read_to_string(output).unwrap().contains("#CHROM"));
}
```

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
cargo test --test filter_cli_tests indexed_bgzf_filter_matches_default_output_byte_for_byte
cargo test --test filter_cli_tests stale_index_falls_back_to_default_streaming_output
```

Expected: first test may pass without proving index use, but the next task adds trace verification. If the command does not understand schema v2, fix that in Step 3.

- [ ] **Step 3: Add index loading helpers**

In `src/engine/index/schema.rs`, add:

```rust
pub(crate) fn read_index(path: &Path) -> Result<VariantFlowIndex> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to open index {}", path.display()))?;
    serde_json::from_reader(file)
        .with_context(|| format!("failed to parse index {}", path.display()))
}

pub(crate) fn default_index_path(input: &Path) -> std::path::PathBuf {
    let mut text = input.as_os_str().to_os_string();
    text.push(".vfi");
    std::path::PathBuf::from(text)
}
```

In `src/engine/index.rs`, re-export for crate use:

```rust
pub(crate) use bgzf::read_virtual_range;
pub(crate) use planner::{SkipDecision, plan_chunk};
pub(crate) use schema::{OffsetModel, VariantFlowIndex, default_index_path, read_index, source_matches};
```

- [ ] **Step 4: Add indexed filter path**

In `src/engine/filter.rs`, after headers are parsed and before `let mut writer = open_vcf_writer(output, compression)?;`, add:

```rust
if std::env::var_os("VCF_FAST_DISABLE_VFI").is_none()
    && try_indexed_filter(input, &expr, &required, sample_column, &headers, output, compression)?
{
    return Ok(());
}
```

Use a helper that returns `Result<bool>`:

```rust
fn try_indexed_filter(
    input: &Path,
    expr: &Expression,
    required: &RequiredFields,
    sample_column: Option<usize>,
    headers: &[Vec<u8>],
    output: &Path,
    compression: CompressionMode,
) -> Result<bool> {
    let index_path = crate::engine::index::default_index_path(input);
    if !index_path.exists() {
        return Ok(false);
    }
    let index = crate::engine::index::read_index(&index_path)?;
    if index.offset_model != crate::engine::index::OffsetModel::BgzfVirtual
        || !index.virtual_offsets_available
        || !crate::engine::index::source_matches(&index, input)?
    {
        return Ok(false);
    }

    if index.chunks.iter().any(|chunk| {
        crate::engine::index::plan_chunk(expr, chunk)
            == crate::engine::index::SkipDecision::UnsupportedForIndex
    }) {
        return Ok(false);
    }

    let mut writer = open_vcf_writer(output, compression)?;
    for header in headers {
        writer.write_all(header)?;
    }

    for chunk in &index.chunks {
        match crate::engine::index::plan_chunk(expr, chunk) {
            crate::engine::index::SkipDecision::CanSkip => continue,
            crate::engine::index::SkipDecision::UnsupportedForIndex => return Ok(false),
            crate::engine::index::SkipDecision::MustScan => {
                let (Some(start), Some(end)) = (chunk.virtual_start, chunk.virtual_end) else {
                    return Ok(false);
                };
                let bytes = crate::engine::index::read_virtual_range(input, start, end)?;
                scan_indexed_chunk_bytes(&bytes, &mut *writer, expr, required, sample_column)?;
            }
        }
    }

    writer.flush()?;
    Ok(true)
}
```

Add:

```rust
fn scan_indexed_chunk_bytes(
    bytes: &[u8],
    writer: &mut dyn Write,
    expr: &Expression,
    required: &RequiredFields,
    sample_column: Option<usize>,
) -> Result<()> {
    for line in bytes.split_inclusive(|byte| *byte == b'\n') {
        if line.is_empty() || line.starts_with(b"#") {
            continue;
        }
        let record = ByteEvalRecord::parse(line, required, sample_column)?;
        if expr.evaluate_context(&record) {
            writer.write_all(line)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 5: Run tests and commit**

Run:

```bash
cargo test --test filter_cli_tests indexed_bgzf_filter_matches_default_output_byte_for_byte
cargo test --test filter_cli_tests stale_index_falls_back_to_default_streaming_output
cargo test --test filter_cli_tests combined_bgzf_and_predicate_threads_preserve_native_output_order
cargo fmt
```

Expected: PASS.

Commit:

```bash
git add src/engine/filter.rs src/engine/index.rs src/engine/index/schema.rs tests/filter_cli_tests.rs
git commit -m "feat: use VFI for safe indexed BGZF filtering"
```

## Task 7: Index Usage Report For Benchmarks

**Files:**
- Modify: `src/engine/filter.rs`
- Test: `tests/filter_cli_tests.rs`

- [ ] **Step 1: Write failing index report test**

Add to `tests/filter_cli_tests.rs`:

```rust
#[test]
fn indexed_filter_can_write_skip_report() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("stress.vcf.gz");
    let index = dir.path().join("stress.vcf.gz.vfi");
    let output = dir.path().join("indexed.vcf");
    let report = dir.path().join("index-report.json");
    bgzf_fixture(&fixture("tests/data/stress_small.vcf"), &input);

    Command::cargo_bin("variantflow")
        .unwrap()
        .args(["index", input.to_str().unwrap(), "-o", index.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("variantflow")
        .unwrap()
        .args(["filter", input.to_str().unwrap(), "--where", "QUAL > 1000", "-o", output.to_str().unwrap()])
        .env("VCF_FAST_INDEX_REPORT", report.to_str().unwrap())
        .assert()
        .success();

    let json: Value = serde_json::from_str(&fs::read_to_string(report).unwrap()).unwrap();
    assert_eq!(json["indexed"], true);
    assert!(json["chunks_total"].as_u64().unwrap() >= 1);
    assert!(json["chunks_skipped"].as_u64().unwrap() >= 1);
}
```

- [ ] **Step 2: Run test and verify failure**

Run:

```bash
cargo test --test filter_cli_tests indexed_filter_can_write_skip_report
```

Expected: FAIL because `VCF_FAST_INDEX_REPORT` is ignored.

- [ ] **Step 3: Add report writer**

In `src/engine/filter.rs`, add:

```rust
#[derive(Debug, serde::Serialize)]
struct IndexFilterReport {
    indexed: bool,
    fallback_reason: Option<String>,
    chunks_total: u64,
    chunks_skipped: u64,
    chunks_scanned: u64,
    records_indexed: u64,
    records_skipped_estimate: u64,
}

fn maybe_write_index_report(report: &IndexFilterReport) -> Result<()> {
    if let Some(path) = std::env::var_os("VCF_FAST_INDEX_REPORT") {
        let file = std::fs::File::create(&path)
            .with_context(|| format!("failed to create index report {}", std::path::PathBuf::from(&path).display()))?;
        serde_json::to_writer_pretty(file, report)?;
    }
    Ok(())
}
```

Track counters in `try_indexed_filter`:

```rust
let mut chunks_skipped = 0_u64;
let mut chunks_scanned = 0_u64;
let mut records_skipped_estimate = 0_u64;
```

On skip:

```rust
chunks_skipped += 1;
records_skipped_estimate += chunk.record_count;
```

On scan:

```rust
chunks_scanned += 1;
```

Before returning `Ok(true)`, call:

```rust
maybe_write_index_report(&IndexFilterReport {
    indexed: true,
    fallback_reason: None,
    chunks_total: index.chunks.len() as u64,
    chunks_skipped,
    chunks_scanned,
    records_indexed: index.record_count,
    records_skipped_estimate,
})?;
```

When falling back after an index was seen, call:

```rust
maybe_write_index_report(&IndexFilterReport {
    indexed: false,
    fallback_reason: Some("unsupported-or-stale-index".to_string()),
    chunks_total: index.chunks.len() as u64,
    chunks_skipped: 0,
    chunks_scanned: 0,
    records_indexed: index.record_count,
    records_skipped_estimate: 0,
})?;
```

- [ ] **Step 4: Run tests and commit**

Run:

```bash
cargo test --test filter_cli_tests indexed_filter_can_write_skip_report
cargo fmt
```

Expected: PASS.

Commit:

```bash
git add src/engine/filter.rs tests/filter_cli_tests.rs
git commit -m "feat: report VFI indexed filter skip counts"
```

## Task 8: Benchmark Harness And Report Template

**Files:**
- Create: `benchmark/run_v21_indexed_filter_benchmarks.sh`
- Create: `benchmark/reports/v21-indexed-filter-benchmark.md`
- Modify: `Makefile`
- Modify: `tests/benchmark_harness_tests.rs`

- [ ] **Step 1: Write failing harness test**

Add to `tests/benchmark_harness_tests.rs`:

```rust
#[test]
fn v21_indexed_filter_harness_tracks_skip_rate_and_correctness() {
    let root = repo_root();
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap();
    let script = fs::read_to_string(root.join("benchmark/run_v21_indexed_filter_benchmarks.sh")).unwrap();
    let report = fs::read_to_string(root.join("benchmark/reports/v21-indexed-filter-benchmark.md")).unwrap();

    assert!(makefile.contains("bench-v21-index"));
    for needle in [
        "VCF_FAST_INDEX_REPORT",
        "chunks skipped",
        "records skipped estimate",
        "default native",
        "indexed native",
        "bcftools filter",
        "correctness result",
    ] {
        assert!(script.contains(needle) || report.contains(needle), "missing {needle}");
    }
}
```

- [ ] **Step 2: Run test and verify failure**

Run:

```bash
cargo test --test benchmark_harness_tests v21_indexed_filter_harness_tracks_skip_rate_and_correctness
```

Expected: FAIL because files/target do not exist.

- [ ] **Step 3: Add Make target**

In `Makefile`, add `bench-v21-index` to `.PHONY`, then add:

```make
bench-v21-index:
	./benchmark/run_v21_indexed_filter_benchmarks.sh
```

- [ ] **Step 4: Add report template**

Create `benchmark/reports/v21-indexed-filter-benchmark.md`:

```markdown
# VariantFlow v2.1 Indexed BGZF Filter Benchmark

This report tracks `.vfi` BGZF virtual-offset filtering. Claims require
byte-for-byte equality between default native and indexed native output, plus
core-record comparison against `bcftools filter`.

| case | dataset | record count | chunks total | chunks skipped | records skipped estimate | exact default native command | exact indexed native command | exact competitor command | correctness result | runtime mean/stddev | speedup | variants/sec | peak RSS | caveat | claim decision |
| --- | --- | ---: | ---: | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| template row | generated dataset | 0 | 0 | 0 | 0 | command generated by benchmark | command generated by benchmark | command generated by benchmark | not measured | generated by benchmark | generated by benchmark | generated by benchmark | generated by benchmark | report rows are generated by `make bench-v21-index` | no claim |
```

- [ ] **Step 5: Add benchmark script skeleton**

Create `benchmark/run_v21_indexed_filter_benchmarks.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${VCF_FAST_V21_OUT_DIR:-tests/output/benchmark-results/v21-indexed-filter}"
REPORT="${VCF_FAST_V21_REPORT:-benchmark/reports/v21-indexed-filter-benchmark.md}"
SIZES="${VCF_FAST_V21_SIZES:-10000 100000}"
RUNS="${VCF_FAST_BENCH_RUNS:-3}"
WARMUP="${VCF_FAST_BENCH_WARMUP:-1}"

mkdir -p "$OUT_DIR/data"
cargo build --release

echo "# VariantFlow v2.1 Indexed BGZF Filter Benchmark" >"$REPORT"
echo "" >>"$REPORT"
echo "This report tracks .vfi BGZF virtual-offset filtering." >>"$REPORT"
echo "" >>"$REPORT"
echo "| case | dataset | record count | chunks total | chunks skipped | records skipped estimate | exact default native command | exact indexed native command | exact competitor command | correctness result | runtime mean/stddev | speedup | variants/sec | peak RSS | caveat | claim decision |" >>"$REPORT"
echo "| --- | --- | ---: | ---: | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |" >>"$REPORT"

for records in $SIZES; do
  dataset="$OUT_DIR/data/skip-heavy-${records}.vcf"
  bgzf="$dataset.gz"
  index="$bgzf.vfi"
  default_out="$OUT_DIR/default-qual-${records}.vcf"
  indexed_out="$OUT_DIR/indexed-qual-${records}.vcf"
  bcftools_out="$OUT_DIR/bcftools-qual-${records}.vcf"
  index_report="$OUT_DIR/index-report-${records}.json"
  hyperfine_json="$OUT_DIR/hyperfine-${records}.json"

  VCF_FAST_STRESS_INFO_COUNT=20 VCF_FAST_STRESS_SAMPLE_COUNT=20 \
    ./benchmark/generate_stress_vcf.sh "$records" "$dataset"
  bgzip -c "$dataset" > "$bgzf"
  ./target/release/variantflow index "$bgzf" -o "$index"

  VCF_FAST_DISABLE_VFI=1 ./target/release/variantflow filter "$bgzf" --where "QUAL > 1000" -o "$default_out"
  VCF_FAST_INDEX_REPORT="$index_report" ./target/release/variantflow filter "$bgzf" --where "QUAL > 1000" -o "$indexed_out"
  bcftools filter -i 'QUAL>1000' "$bgzf" -o "$bcftools_out"
  cmp "$default_out" "$indexed_out"

  hyperfine --warmup "$WARMUP" --runs "$RUNS" --export-json "$hyperfine_json" \
    "VCF_FAST_DISABLE_VFI=1 ./target/release/variantflow filter $bgzf --where 'QUAL > 1000' -o $OUT_DIR/default-qual-${records}.timed.vcf" \
    "VCF_FAST_INDEX_REPORT=$OUT_DIR/index-report-${records}.timed.json ./target/release/variantflow filter $bgzf --where 'QUAL > 1000' -o $OUT_DIR/indexed-qual-${records}.timed.vcf" \
    "bcftools filter -i 'QUAL>1000' $bgzf -o $OUT_DIR/bcftools-qual-${records}.timed.vcf"

  chunks_total="$(python3 -c "import json; print(json.load(open('$index_report'))['chunks_total'])")"
  chunks_skipped="$(python3 -c "import json; print(json.load(open('$index_report'))['chunks_skipped'])")"
  records_skipped="$(python3 -c "import json; print(json.load(open('$index_report'))['records_skipped_estimate'])")"

  echo "| skip-heavy QUAL impossible | $bgzf | $records | $chunks_total | $chunks_skipped | $records_skipped | \`VCF_FAST_DISABLE_VFI=1 ./target/release/variantflow filter $bgzf --where 'QUAL > 1000' -o $default_out\` | \`VCF_FAST_INDEX_REPORT=$index_report ./target/release/variantflow filter $bgzf --where 'QUAL > 1000' -o $indexed_out\` | \`bcftools filter -i 'QUAL>1000' $bgzf -o $bcftools_out\` | default and indexed outputs match byte-for-byte | see $hyperfine_json | generated by hyperfine | reported in hyperfine JSON | RSS helper not recorded in first script slice | synthetic skip-heavy evidence | measured row requires summary parser |" >>"$REPORT"
done
```

Make it executable:

```bash
chmod +x benchmark/run_v21_indexed_filter_benchmarks.sh
```

- [ ] **Step 6: Run tests and commit**

Run:

```bash
bash -n benchmark/run_v21_indexed_filter_benchmarks.sh
cargo test --test benchmark_harness_tests v21_indexed_filter_harness_tracks_skip_rate_and_correctness
cargo fmt
```

Expected: PASS.

Commit:

```bash
git add Makefile benchmark/run_v21_indexed_filter_benchmarks.sh benchmark/reports/v21-indexed-filter-benchmark.md tests/benchmark_harness_tests.rs
git commit -m "bench: add indexed BGZF filter harness"
```

## Task 9: Full Verification

**Files:**
- All files changed by prior tasks.

- [ ] **Step 1: Run focused tests**

Run:

```bash
cargo test --test index_cli_tests
cargo test --test filter_cli_tests indexed_bgzf_filter_matches_default_output_byte_for_byte
cargo test --test filter_cli_tests indexed_filter_can_write_skip_report
cargo test bgzf::tests
cargo test planner::tests
cargo test metadata::tests
```

Expected: PASS.

- [ ] **Step 2: Run full default verification**

Run:

```bash
make verify
```

Expected: PASS.

- [ ] **Step 3: Run htslib feature verification**

Run:

```bash
cargo test --features htslib-static
cargo clippy --features htslib-static --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 4: Run smoke indexed benchmark**

Run:

```bash
VCF_FAST_V21_SIZES="100" VCF_FAST_BENCH_RUNS=1 VCF_FAST_BENCH_WARMUP=0 make bench-v21-index
```

Expected: PASS, with `benchmark/reports/v21-indexed-filter-benchmark.md` containing a measured smoke row and `tests/output/benchmark-results/v21-indexed-filter/index-report-100.json` showing `indexed: true`.

- [ ] **Step 5: Commit verification report update if benchmark report changed**

If the smoke run updates the tracked report and the row is correct:

```bash
git add benchmark/reports/v21-indexed-filter-benchmark.md
git commit -m "bench: record indexed filter smoke evidence"
```

If the smoke run only changes ignored `tests/output/...` artifacts, do not commit artifacts.

- [ ] **Step 6: Final status**

Run:

```bash
git status --short --branch
```

Expected: clean branch after committed tracked changes.

## Execution Notes

- Use TDD for each task: write the test first, confirm it fails for the expected reason, then implement.
- Keep every commit small and coherent.
- Do not update README performance claims unless repeated benchmark rows show correctness-matched wins.
- If virtual-offset range reading is uncertain, stop and inspect BGZF block boundaries rather than guessing.
- If indexed filtering ever differs from default output, treat it as a correctness bug and fix before benchmarking.
