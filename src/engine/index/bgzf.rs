use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail, ensure};
use flate2::read::DeflateDecoder;

const MAX_BGZF_UNCOMPRESSED_BLOCK_SIZE: usize = 65_536;

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

pub(crate) fn for_each_bgzf_block(
    path: &Path,
    mut visit: impl FnMut(BgzfBlock) -> Result<()>,
) -> Result<()> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let file_len = file
        .metadata()
        .with_context(|| format!("failed to stat {}", path.display()))?
        .len();

    loop {
        let compressed_start = file
            .stream_position()
            .with_context(|| format!("failed to seek {}", path.display()))?;
        if compressed_start == file_len {
            break;
        }

        let block = read_block(path, &mut file, compressed_start)?;
        visit(block)?;
    }

    Ok(())
}

#[cfg(test)]
pub(crate) fn read_bgzf_blocks(path: &Path) -> Result<Vec<BgzfBlock>> {
    let mut blocks = Vec::new();
    for_each_bgzf_block(path, |block| {
        blocks.push(block);
        Ok(())
    })?;
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
    let footer = &remaining[payload_end..];
    let expected_crc32 = u32::from_le_bytes([footer[0], footer[1], footer[2], footer[3]]);
    let expected_isize = u32::from_le_bytes([footer[4], footer[5], footer[6], footer[7]]);
    ensure!(
        expected_isize as usize <= MAX_BGZF_UNCOMPRESSED_BLOCK_SIZE,
        "BGZF block at compressed offset {compressed_start} exceeds BGZF maximum uncompressed block size"
    );

    let mut decoder = DeflateDecoder::new(&remaining[..payload_end]);
    let mut uncompressed = Vec::new();
    decoder.read_to_end(&mut uncompressed).with_context(|| {
        format!("failed to inflate BGZF block at compressed offset {compressed_start}")
    })?;
    ensure!(
        uncompressed.len() <= MAX_BGZF_UNCOMPRESSED_BLOCK_SIZE,
        "BGZF block at compressed offset {compressed_start} exceeds BGZF maximum uncompressed block size"
    );
    ensure!(
        uncompressed.len() as u32 == expected_isize,
        "BGZF ISIZE mismatch at compressed offset {compressed_start}: expected {expected_isize}, decoded {}",
        uncompressed.len()
    );
    let actual_crc32 = crc32fast::hash(&uncompressed);
    ensure!(
        actual_crc32 == expected_crc32,
        "BGZF CRC32 mismatch at compressed offset {compressed_start}: expected {expected_crc32:#010x}, decoded {actual_crc32:#010x}"
    );

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

    fn write_bgzf(path: &Path) {
        let file = std::fs::File::create(path).unwrap();
        let mut writer = noodles_bgzf::io::Writer::new(file);
        writer.write_all(TINY_VCF).unwrap();
        writer.finish().unwrap();
    }

    #[test]
    fn bgzf_block_reader_reports_block_boundary_virtual_offsets() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("tiny.vcf.gz");
        write_bgzf(&input);

        let blocks = read_bgzf_blocks(&input).unwrap();
        let first_non_empty = blocks
            .iter()
            .find(|block| !block.uncompressed.is_empty())
            .expect("expected at least one non-empty BGZF block");

        assert_eq!(first_non_empty.virtual_start(), 0);
        assert!(first_non_empty.virtual_end() > first_non_empty.virtual_start());
    }

    #[test]
    fn bgzf_streaming_walker_visits_blocks_without_collecting() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("tiny.vcf.gz");
        write_bgzf(&input);

        let mut block_count = 0;
        let mut non_empty_count = 0;
        for_each_bgzf_block(&input, |block| {
            block_count += 1;
            if !block.uncompressed.is_empty() {
                non_empty_count += 1;
            }
            Ok(())
        })
        .unwrap();

        assert!(block_count >= 1);
        assert!(non_empty_count >= 1);
    }

    #[test]
    fn bgzf_reader_rejects_crc32_mismatch() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("tiny.vcf.gz");
        write_bgzf(&input);
        let first_block_end = read_bgzf_blocks(&input)
            .unwrap()
            .into_iter()
            .find(|block| !block.uncompressed.is_empty())
            .unwrap()
            .compressed_end as usize;

        let mut bytes = std::fs::read(&input).unwrap();
        bytes[first_block_end - 8] ^= 0xff;
        std::fs::write(&input, bytes).unwrap();

        let error = read_bgzf_blocks(&input).unwrap_err();
        assert!(
            error.to_string().contains("CRC32"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn bgzf_reader_rejects_oversized_isize() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("tiny.vcf.gz");
        write_bgzf(&input);
        let first_block_end = read_bgzf_blocks(&input)
            .unwrap()
            .into_iter()
            .find(|block| !block.uncompressed.is_empty())
            .unwrap()
            .compressed_end as usize;

        let mut bytes = std::fs::read(&input).unwrap();
        bytes[first_block_end - 4..first_block_end].copy_from_slice(&65_537_u32.to_le_bytes());
        std::fs::write(&input, bytes).unwrap();

        let error = read_bgzf_blocks(&input).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("exceeds BGZF maximum uncompressed block size"),
            "unexpected error: {error:#}"
        );
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
