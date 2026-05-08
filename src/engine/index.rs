use std::fs::File;
use std::io::{BufRead, BufWriter};
use std::path::Path;

use anyhow::{Context, Result};

use crate::io::open_reader;
use crate::vcf::RecordView;

mod bgzf;
mod metadata;
#[allow(dead_code)]
pub(crate) mod planner;
pub(crate) mod schema;

use bgzf::for_each_bgzf_block;
pub(crate) use bgzf::{
    first_record_virtual_start, for_each_virtual_range_slice, read_decoded_bgzf_blocks,
};
use metadata::ChunkMetadataBuilder;
pub(crate) use planner::{SkipDecision, plan_chunk};
pub(crate) use schema::{
    OffsetModel, VariantFlowIndex, default_index_path, read_index, source_matches,
};
use schema::{set_metadata_sha256, source_identity};

const DEFAULT_CHUNK_RECORDS: u64 = 8192;

pub fn run(input: &Path, output: &Path) -> Result<()> {
    if has_gz_extension(input) {
        match write_bgzf_index(input, output, DEFAULT_CHUNK_RECORDS) {
            Ok(()) => return Ok(()),
            Err(error) if error.to_string().contains("not a BGZF file") => {}
            Err(error) => return Err(error),
        }
    }

    write_index(input, output, DEFAULT_CHUNK_RECORDS)
}

fn write_index(input: &Path, output: &Path, chunk_record_target: u64) -> Result<()> {
    let mut reader = open_reader(input)?;
    let mut line = Vec::new();
    let mut record_count = 0_u64;
    let mut chunks = Vec::new();
    let mut current = ChunkMetadataBuilder::new(0, 0, None);

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

        if current.record_count() >= chunk_record_target {
            if let Some(chunk) = current.finish(None) {
                chunks.push(chunk);
            }
            current = ChunkMetadataBuilder::new(chunks.len() as u64, record_count, None);
        }
    }

    if let Some(chunk) = current.finish(None) {
        chunks.push(chunk);
    }

    let index = VariantFlowIndex {
        schema_version: 2,
        index_kind: "variantflow-vfi".to_string(),
        index_metadata_sha256: String::new(),
        offset_model: OffsetModel::RecordChunk,
        virtual_offsets_available: false,
        source: source_identity(input)?,
        chunk_record_target,
        record_count,
        chunks,
    };

    write_index_json(output, &index)
}

fn write_bgzf_index(input: &Path, output: &Path, chunk_record_target: u64) -> Result<()> {
    let mut record_count = 0_u64;
    let mut chunks = Vec::new();
    let mut current = None;
    let mut line = Vec::new();
    let mut line_virtual_start = None;
    let mut line_virtual_end = None;
    let mut current_virtual_end = None;

    for_each_bgzf_block(input, |block| {
        for (offset, byte) in block.uncompressed.iter().copied().enumerate() {
            if line.is_empty() && line_virtual_start.is_none() {
                line_virtual_start = Some(virtual_offset_at(&block, offset));
            }
            line.push(byte);
            line_virtual_end = Some(virtual_offset_after(&block, offset + 1));

            if byte == b'\n' {
                observe_bgzf_line(
                    &line,
                    line_virtual_start.unwrap_or_else(|| virtual_offset_at(&block, 0)),
                    line_virtual_end.unwrap_or_else(|| virtual_offset_at(&block, 0)),
                    chunks.len() as u64,
                    &mut record_count,
                    &mut current,
                    &mut current_virtual_end,
                )?;
                line.clear();
                line_virtual_start = None;
                line_virtual_end = None;
            }
        }

        if current
            .as_ref()
            .is_some_and(|builder| builder.record_count() >= chunk_record_target)
            && line.is_empty()
            && let Some(chunk) = current
                .take()
                .and_then(|builder| builder.finish(current_virtual_end.take()))
        {
            chunks.push(chunk);
        }
        Ok(())
    })?;

    if !line.is_empty() {
        observe_bgzf_line(
            &line,
            line_virtual_start.unwrap_or(0),
            line_virtual_end.unwrap_or_else(|| line_virtual_start.unwrap_or(0)),
            chunks.len() as u64,
            &mut record_count,
            &mut current,
            &mut current_virtual_end,
        )?;
    }

    if let Some(chunk) = current
        .take()
        .and_then(|builder| builder.finish(current_virtual_end))
    {
        chunks.push(chunk);
    }

    let index = VariantFlowIndex {
        schema_version: 2,
        index_kind: "variantflow-vfi".to_string(),
        index_metadata_sha256: String::new(),
        offset_model: OffsetModel::BgzfVirtual,
        virtual_offsets_available: true,
        source: source_identity(input)?,
        chunk_record_target,
        record_count,
        chunks,
    };

    write_index_json(output, &index)
}

fn observe_bgzf_line(
    line: &[u8],
    line_virtual_start: u64,
    line_virtual_end: u64,
    chunk_ordinal: u64,
    record_count: &mut u64,
    current: &mut Option<ChunkMetadataBuilder>,
    current_virtual_end: &mut Option<u64>,
) -> Result<()> {
    if line.starts_with(b"#") {
        return Ok(());
    }

    if current.is_none() {
        *current = Some(ChunkMetadataBuilder::new(
            chunk_ordinal,
            *record_count,
            Some(line_virtual_start),
        ));
    }

    let record = RecordView::parse(line)?;
    current
        .as_mut()
        .expect("chunk builder exists")
        .observe(&record)?;
    *record_count += 1;
    *current_virtual_end = Some(line_virtual_end);
    Ok(())
}

fn virtual_offset_at(block: &bgzf::BgzfBlock, uncompressed_offset: usize) -> u64 {
    block.virtual_start() | uncompressed_offset as u64
}

fn virtual_offset_after(block: &bgzf::BgzfBlock, uncompressed_offset: usize) -> u64 {
    if uncompressed_offset == 65_536 {
        block.virtual_end()
    } else {
        virtual_offset_at(block, uncompressed_offset)
    }
}

fn write_index_json(output: &Path, index: &VariantFlowIndex) -> Result<()> {
    let mut index = VariantFlowIndex {
        schema_version: index.schema_version,
        index_kind: index.index_kind.clone(),
        index_metadata_sha256: String::new(),
        offset_model: match index.offset_model {
            OffsetModel::RecordChunk => OffsetModel::RecordChunk,
            OffsetModel::BgzfVirtual => OffsetModel::BgzfVirtual,
        },
        virtual_offsets_available: index.virtual_offsets_available,
        source: index.source.clone(),
        chunk_record_target: index.chunk_record_target,
        record_count: index.record_count,
        chunks: index.chunks.clone(),
    };
    set_metadata_sha256(&mut index)?;
    let file = File::create(output)
        .with_context(|| format!("failed to create index {}", output.display()))?;
    serde_json::to_writer_pretty(BufWriter::new(file), &index)
        .with_context(|| format!("failed to write index {}", output.display()))?;
    Ok(())
}

fn has_gz_extension(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "gz")
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
