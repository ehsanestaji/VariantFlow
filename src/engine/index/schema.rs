use std::path::Path;
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct VariantFlowIndex {
    pub(crate) schema_version: u32,
    pub(crate) index_kind: &'static str,
    pub(crate) offset_model: OffsetModel,
    pub(crate) virtual_offsets_available: bool,
    pub(crate) source: SourceIdentity,
    pub(crate) chunk_record_target: u64,
    pub(crate) record_count: u64,
    pub(crate) chunks: Vec<IndexChunk>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum OffsetModel {
    RecordChunk,
    #[allow(dead_code)]
    BgzfVirtual,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct SourceIdentity {
    pub(crate) path: String,
    pub(crate) size_bytes: u64,
    pub(crate) modified_unix_seconds: u64,
}

#[derive(Debug, Serialize)]
pub(crate) struct IndexChunk {
    pub(crate) ordinal: u64,
    pub(crate) first_record: u64,
    pub(crate) record_count: u64,
    pub(crate) chrom_start: String,
    pub(crate) chrom_end: String,
    pub(crate) pos_min: u64,
    pub(crate) pos_max: u64,
    pub(crate) qual_min: Option<f64>,
    pub(crate) qual_max: Option<f64>,
    pub(crate) filters: Vec<String>,
    pub(crate) info_dp_min: Option<i64>,
    pub(crate) info_dp_max: Option<i64>,
    pub(crate) has_info_af: bool,
    pub(crate) info_af_min: Option<f64>,
    pub(crate) info_af_max: Option<f64>,
    pub(crate) info_af_complete: bool,
    pub(crate) format_keys: Vec<String>,
    pub(crate) virtual_start: Option<u64>,
    pub(crate) virtual_end: Option<u64>,
}

pub(crate) fn source_identity(path: &Path) -> Result<SourceIdentity> {
    let metadata =
        std::fs::metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
    let modified = metadata
        .modified()
        .with_context(|| format!("failed to read mtime for {}", path.display()))?;
    let modified_unix_seconds = modified
        .duration_since(UNIX_EPOCH)
        .with_context(|| format!("mtime for {} is before UNIX epoch", path.display()))?
        .as_secs();

    Ok(SourceIdentity {
        path: path.display().to_string(),
        size_bytes: metadata.len(),
        modified_unix_seconds,
    })
}

#[allow(dead_code)]
pub(crate) fn source_matches(index: &VariantFlowIndex, path: &Path) -> Result<bool> {
    Ok(index.source == source_identity(path)?)
}
