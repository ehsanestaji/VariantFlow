use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct VariantFlowIndex {
    pub(crate) schema_version: u32,
    pub(crate) index_kind: String,
    pub(crate) offset_model: OffsetModel,
    pub(crate) virtual_offsets_available: bool,
    pub(crate) source: SourceIdentity,
    pub(crate) chunk_record_target: u64,
    pub(crate) record_count: u64,
    pub(crate) chunks: Vec<IndexChunk>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum OffsetModel {
    RecordChunk,
    #[allow(dead_code)]
    BgzfVirtual,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(crate) struct SourceIdentity {
    pub(crate) path: String,
    pub(crate) size_bytes: u64,
    pub(crate) modified_unix_seconds: u64,
    pub(crate) content_sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
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

pub(crate) fn read_index(path: &Path) -> Result<VariantFlowIndex> {
    let file =
        std::fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    serde_json::from_reader(file).with_context(|| format!("failed to parse {}", path.display()))
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

    Ok(SourceIdentity {
        path: path.display().to_string(),
        size_bytes: metadata.len(),
        modified_unix_seconds,
        content_sha256: file_sha256(path)?,
    })
}

#[allow(dead_code)]
pub(crate) fn source_matches(index: &VariantFlowIndex, path: &Path) -> Result<bool> {
    Ok(index.source == source_identity(path)?)
}

fn file_sha256(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("failed to hash source {}", path.display()))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)
        .with_context(|| format!("failed reading source hash for {}", path.display()))?;
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn source_identity_includes_content_hash_for_same_size_files() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("first.vcf.gz");
        let second = dir.path().join("second.vcf.gz");
        std::fs::write(&first, b"aaaa").unwrap();
        std::fs::write(&second, b"bbbb").unwrap();

        let first_identity = source_identity(&first).unwrap();
        let second_identity = source_identity(&second).unwrap();

        assert_eq!(first_identity.size_bytes, second_identity.size_bytes);
        assert_eq!(first_identity.content_sha256.len(), 64);
        assert_ne!(
            first_identity.content_sha256,
            second_identity.content_sha256
        );
    }
}
