use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead, BufWriter};
use std::path::Path;

use anyhow::{Context, Result};
use memchr::memchr;

use crate::io::open_reader;
use crate::vcf::{InfoView, RecordView};

mod schema;

use schema::{IndexChunk, OffsetModel, VariantFlowIndex, source_identity};

const DEFAULT_CHUNK_RECORDS: u64 = 8192;

#[derive(Debug)]
struct ChunkBuilder {
    ordinal: u64,
    first_record: u64,
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
    has_info_af: bool,
    format_keys: BTreeSet<String>,
}

impl ChunkBuilder {
    fn new(ordinal: u64, first_record: u64) -> Self {
        Self {
            ordinal,
            first_record,
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
            has_info_af: false,
            format_keys: BTreeSet::new(),
        }
    }

    fn observe(&mut self, record: &RecordView<'_>) -> Result<()> {
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

        observe_filters(record.filter(), &mut self.filters);

        let info = InfoView::scan(record.info());
        if let Some(value) = info.value(b"DP") {
            for_each_comma_i64(value, |number| {
                update_i64_minmax(&mut self.info_dp_min, &mut self.info_dp_max, number);
            });
        }
        if info
            .value(b"AF")
            .is_some_and(|value| value != b"." && !value.is_empty())
        {
            self.has_info_af = true;
        }

        if let Some(format) = record.column(8) {
            observe_format_keys(format, &mut self.format_keys);
        }

        self.record_count += 1;
        Ok(())
    }

    fn is_full(&self, chunk_record_target: u64) -> bool {
        self.record_count >= chunk_record_target
    }

    fn finish(self) -> Option<IndexChunk> {
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
            has_info_af: self.has_info_af,
            info_af_min: None,
            info_af_max: None,
            info_af_complete: false,
            format_keys: self.format_keys.into_iter().collect(),
            virtual_start: None,
            virtual_end: None,
        })
    }
}

pub fn run(input: &Path, output: &Path) -> Result<()> {
    write_index(input, output, DEFAULT_CHUNK_RECORDS)
}

fn write_index(input: &Path, output: &Path, chunk_record_target: u64) -> Result<()> {
    let mut reader = open_reader(input)?;
    let mut line = Vec::new();
    let mut record_count = 0_u64;
    let mut chunks = Vec::new();
    let mut current = ChunkBuilder::new(0, 0);

    loop {
        line.clear();
        let bytes_read = reader
            .read_until(b'\n', &mut line)
            .with_context(|| format!("failed reading {}", input.display()))?;
        if bytes_read == 0 {
            break;
        }
        if line.starts_with(b"#") {
            continue;
        }

        let record = RecordView::parse(&line)?;
        current.observe(&record)?;
        record_count += 1;

        if current.is_full(chunk_record_target) {
            if let Some(chunk) = current.finish() {
                chunks.push(chunk);
            }
            current = ChunkBuilder::new(chunks.len() as u64, record_count);
        }
    }

    if let Some(chunk) = current.finish() {
        chunks.push(chunk);
    }

    let index = VariantFlowIndex {
        schema_version: 2,
        index_kind: "variantflow-vfi",
        offset_model: OffsetModel::RecordChunk,
        virtual_offsets_available: false,
        source: source_identity(input)?,
        chunk_record_target,
        record_count,
        chunks,
    };

    let file = File::create(output)
        .with_context(|| format!("failed to create index {}", output.display()))?;
    serde_json::to_writer_pretty(BufWriter::new(file), &index)
        .with_context(|| format!("failed to write index {}", output.display()))?;
    Ok(())
}

fn observe_filters(filter: &[u8], filters: &mut BTreeSet<String>) {
    if filter.is_empty() || filter == b"." {
        return;
    }

    for_each_delimited(filter, b';', |part| {
        if !part.is_empty() && part != b"." {
            filters.insert(String::from_utf8_lossy(part).into_owned());
        }
    });
}

fn observe_format_keys(format: &[u8], format_keys: &mut BTreeSet<String>) {
    if format.is_empty() || format == b"." {
        return;
    }

    for_each_delimited(format, b':', |part| {
        if !part.is_empty() {
            format_keys.insert(String::from_utf8_lossy(part).into_owned());
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
    if !value.is_finite() {
        return;
    }

    *min = Some(min.map_or(value, |current| current.min(value)));
    *max = Some(max.map_or(value, |current| current.max(value)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use tempfile::tempdir;

    #[test]
    fn write_index_splits_chunks_by_record_target() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("mini.vcf");
        let output = dir.path().join("mini.vcf.vfi");
        std::fs::write(
            &input,
            "##fileformat=VCFv4.3\n\
             #CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n\
             chr1\t1\t.\tA\tG\t1\tPASS\tDP=1\n\
             chr1\t2\t.\tA\tG\t2\tPASS\tDP=2\n\
             chr1\t3\t.\tA\tG\t3\tPASS\tDP=3\n",
        )
        .unwrap();

        write_index(&input, &output, 2).unwrap();

        let json: Value = serde_json::from_str(&std::fs::read_to_string(output).unwrap()).unwrap();
        assert_eq!(json["record_count"], 3);
        assert_eq!(json["chunks"].as_array().unwrap().len(), 2);
        assert_eq!(json["chunks"][0]["record_count"], 2);
        assert_eq!(json["chunks"][1]["first_record"], 2);
    }
}
