use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct VariantFlowIndex {
    pub(crate) schema_version: u32,
    pub(crate) index_kind: String,
    #[serde(default)]
    pub(crate) index_metadata_sha256: String,
    pub(crate) offset_model: OffsetModel,
    pub(crate) virtual_offsets_available: bool,
    pub(crate) source: SourceIdentity,
    pub(crate) chunk_record_target: u64,
    pub(crate) record_count: u64,
    pub(crate) chunks: Vec<IndexChunk>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum OffsetModel {
    RecordChunk,
    #[allow(dead_code)]
    BgzfVirtual,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(crate) struct SourceIdentity {
    pub(crate) path: String,
    pub(crate) size_bytes: u64,
    pub(crate) modified_unix_seconds: u64,
    pub(crate) modified_unix_nanoseconds: u128,
    pub(crate) metadata_changed_unix_nanoseconds: Option<u128>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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
    #[serde(default)]
    pub(crate) filter_values: Vec<String>,
    pub(crate) info_dp_min: Option<i64>,
    pub(crate) info_dp_max: Option<i64>,
    pub(crate) has_info_af: bool,
    pub(crate) info_af_min: Option<f64>,
    pub(crate) info_af_max: Option<f64>,
    pub(crate) info_af_complete: bool,
    #[serde(default)]
    pub(crate) info_numeric: BTreeMap<String, NumericBounds>,
    pub(crate) format_keys: Vec<String>,
    pub(crate) virtual_start: Option<u64>,
    pub(crate) virtual_end: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct NumericBounds {
    pub(crate) min: Option<f64>,
    pub(crate) max: Option<f64>,
    pub(crate) complete: bool,
}

pub(crate) fn read_index(path: &Path) -> Result<VariantFlowIndex> {
    let file =
        std::fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let index: VariantFlowIndex = serde_json::from_reader(file)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    ensure!(
        !index.index_metadata_sha256.is_empty()
            && index.index_metadata_sha256 == metadata_sha256(&index)?,
        "VFI index metadata checksum mismatch"
    );
    Ok(index)
}

pub(crate) fn default_index_path(input: &Path) -> PathBuf {
    let mut path = input.as_os_str().to_os_string();
    path.push(".vfi");
    PathBuf::from(path)
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
    let modified_unix_nanoseconds = modified
        .duration_since(UNIX_EPOCH)
        .with_context(|| format!("mtime for {} is before UNIX epoch", path.display()))?
        .as_nanos();

    Ok(SourceIdentity {
        path: path.display().to_string(),
        size_bytes: metadata.len(),
        modified_unix_seconds,
        modified_unix_nanoseconds,
        metadata_changed_unix_nanoseconds: metadata_changed_unix_nanoseconds(&metadata),
    })
}

#[allow(dead_code)]
pub(crate) fn source_matches(index: &VariantFlowIndex, path: &Path) -> Result<bool> {
    Ok(index.source == source_identity(path)?)
}

pub(crate) fn set_metadata_sha256(index: &mut VariantFlowIndex) -> Result<()> {
    index.index_metadata_sha256 = metadata_sha256(index)?;
    Ok(())
}

fn metadata_sha256(index: &VariantFlowIndex) -> Result<String> {
    let payload = serde_json::to_vec(&(
        index.schema_version,
        &index.index_kind,
        &index.offset_model,
        index.virtual_offsets_available,
        &index.source,
        index.chunk_record_target,
        index.record_count,
        &index.chunks,
    ))
    .context("failed to serialize VFI index metadata for checksum")?;
    let mut hasher = Sha256::new();
    hasher.update(payload);
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(unix)]
fn metadata_changed_unix_nanoseconds(metadata: &std::fs::Metadata) -> Option<u128> {
    use std::os::unix::fs::MetadataExt;

    Some((metadata.ctime() as u128) * 1_000_000_000 + metadata.ctime_nsec() as u128)
}

#[cfg(not(unix))]
fn metadata_changed_unix_nanoseconds(_metadata: &std::fs::Metadata) -> Option<u128> {
    None
}
