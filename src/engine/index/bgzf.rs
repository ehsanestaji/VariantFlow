use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail, ensure};
use flate2::read::DeflateDecoder;

#[derive(Debug)]
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
    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let file_len = file
        .metadata()
        .with_context(|| format!("failed to stat {}", path.display()))?
        .len();
    let mut blocks = Vec::new();

    loop {
        let compressed_start = file
            .stream_position()
            .with_context(|| format!("failed to seek {}", path.display()))?;
        if compressed_start == file_len {
            break;
        }

        let block = read_block(path, &mut file, compressed_start)?;
        blocks.push(block);
    }

    Ok(blocks)
}

fn read_block(path: &Path, file: &mut File, compressed_start: u64) -> Result<BgzfBlock> {
    let mut fixed = [0_u8; 10];
    read_exact_bgzf(file, &mut fixed, path, compressed_start)?;

    let is_gzip = fixed[0] == 0x1f && fixed[1] == 0x8b && fixed[2] == 8;
    let has_extra = fixed[3] & 0x04 != 0;
    ensure!(
        is_gzip && has_extra,
        "{} is not a BGZF file",
        path.display()
    );

    let mut extra_len_bytes = [0_u8; 2];
    read_exact_bgzf(file, &mut extra_len_bytes, path, compressed_start)?;
    let extra_len = u16::from_le_bytes(extra_len_bytes) as usize;
    let mut extra = vec![0_u8; extra_len];
    read_exact_bgzf(file, &mut extra, path, compressed_start)?;

    let bsize = find_bgzf_bsize(&extra)
        .with_context(|| format!("{} is not a BGZF file", path.display()))?;
    let block_size = u64::from(bsize) + 1;
    let header_len = 12_u64 + extra_len as u64;
    ensure!(
        block_size >= header_len + 8,
        "invalid BGZF block at compressed offset {compressed_start}: block size is too small"
    );

    let remaining_len = usize::try_from(block_size - header_len)
        .context("BGZF block is too large to fit in memory")?;
    let mut remaining = vec![0_u8; remaining_len];
    read_exact_bgzf(file, &mut remaining, path, compressed_start)?;
    let payload_end = remaining
        .len()
        .checked_sub(8)
        .ok_or_else(|| anyhow!("invalid BGZF block at compressed offset {compressed_start}"))?;

    let mut decoder = DeflateDecoder::new(&remaining[..payload_end]);
    let mut uncompressed = Vec::new();
    decoder.read_to_end(&mut uncompressed).with_context(|| {
        format!("failed to inflate BGZF block at compressed offset {compressed_start}")
    })?;

    Ok(BgzfBlock {
        compressed_start,
        compressed_end: compressed_start + block_size,
        uncompressed,
    })
}

fn read_exact_bgzf(
    file: &mut File,
    buffer: &mut [u8],
    path: &Path,
    compressed_start: u64,
) -> Result<()> {
    file.read_exact(buffer).with_context(|| {
        format!(
            "truncated BGZF block in {} at compressed offset {compressed_start}",
            path.display()
        )
    })
}

fn find_bgzf_bsize(extra: &[u8]) -> Result<u16> {
    let mut index = 0;
    while index + 4 <= extra.len() {
        let subfield_id = &extra[index..index + 2];
        let subfield_len = u16::from_le_bytes([extra[index + 2], extra[index + 3]]) as usize;
        index += 4;
        if index + subfield_len > extra.len() {
            bail!("invalid BGZF extra field");
        }
        if subfield_id == b"BC" && subfield_len == 2 {
            return Ok(u16::from_le_bytes([extra[index], extra[index + 1]]));
        }
        index += subfield_len;
    }

    bail!("missing BGZF BC extra field")
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use std::io::Write;
    use tempfile::tempdir;

    const TINY_VCF: &[u8] = b"##fileformat=VCFv4.3\n\
#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n\
chr1\t1\t.\tA\tG\t1\tPASS\tDP=1\n";

    #[test]
    fn bgzf_block_reader_reports_block_boundary_virtual_offsets() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("tiny.vcf.gz");
        let file = std::fs::File::create(&input).unwrap();
        let mut writer = noodles_bgzf::io::Writer::new(file);
        writer.write_all(TINY_VCF).unwrap();
        writer.finish().unwrap();

        let blocks = read_bgzf_blocks(&input).unwrap();
        let first_non_empty = blocks
            .iter()
            .find(|block| !block.uncompressed.is_empty())
            .expect("expected at least one non-empty BGZF block");

        assert_eq!(first_non_empty.virtual_start(), 0);
        assert!(first_non_empty.virtual_end() > first_non_empty.virtual_start());
    }

    #[test]
    fn non_bgzf_gzip_is_rejected_by_block_reader() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("ordinary.vcf.gz");
        let file = std::fs::File::create(&input).unwrap();
        let mut encoder = GzEncoder::new(file, flate2::Compression::default());
        encoder.write_all(TINY_VCF).unwrap();
        encoder.finish().unwrap();

        let error = read_bgzf_blocks(&input).unwrap_err();
        assert!(
            error.to_string().contains("not a BGZF file"),
            "unexpected error: {error:#}"
        );
    }
}
