use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
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

        let block = read_one_bgzf_block(path, &mut file, compressed_start)?;
        visit(block)?;
    }

    Ok(())
}

pub(crate) fn first_record_virtual_start(path: &Path) -> Result<Option<u64>> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let file_len = file
        .metadata()
        .with_context(|| format!("failed to stat {}", path.display()))?
        .len();
    let mut first_record_start = None;
    let mut line_start = None;

    loop {
        if first_record_start.is_some() {
            break;
        }
        let compressed_start = file
            .stream_position()
            .with_context(|| format!("failed to seek {}", path.display()))?;
        if compressed_start == file_len {
            break;
        }

        let block = read_one_bgzf_block(path, &mut file, compressed_start)?;

        for (offset, byte) in block.uncompressed.iter().copied().enumerate() {
            if line_start.is_none() {
                let start = virtual_offset_at(&block, offset);
                line_start = Some(start);

                if byte != b'#' {
                    first_record_start = Some(start);
                    break;
                }
            }

            if byte == b'\n' {
                line_start = None;
            }
        }
    }

    Ok(first_record_start)
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

#[allow(dead_code)]
pub(crate) fn read_virtual_range(
    path: &Path,
    virtual_start: u64,
    virtual_end: u64,
) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    for_each_virtual_range_slice(path, virtual_start, virtual_end, |slice| {
        output.extend_from_slice(slice);
        Ok(())
    })?;
    Ok(output)
}

pub(crate) fn for_each_virtual_range_slice(
    path: &Path,
    virtual_start: u64,
    virtual_end: u64,
    mut visit: impl FnMut(&[u8]) -> Result<()>,
) -> Result<()> {
    ensure!(
        virtual_end >= virtual_start,
        "invalid BGZF virtual range: end {virtual_end} is before start {virtual_start}"
    );
    if virtual_start == virtual_end {
        return Ok(());
    }

    let start_compressed = virtual_start >> 16;
    let start_uncompressed = (virtual_start & 0xffff) as usize;
    let end_compressed = virtual_end >> 16;
    let end_uncompressed = (virtual_end & 0xffff) as usize;

    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut compressed_start = start_compressed;

    loop {
        let block = read_one_bgzf_block(path, &mut file, compressed_start)?;
        let slice_start = if block.compressed_start == start_compressed {
            start_uncompressed
        } else {
            0
        };
        let (slice_end, range_complete) = if end_compressed == block.compressed_start {
            (end_uncompressed, true)
        } else if end_compressed == block.compressed_end && end_uncompressed == 0 {
            (block.uncompressed.len(), true)
        } else if end_compressed < block.compressed_end {
            bail!(
                "invalid BGZF virtual range: end offset {virtual_end} does not align with a BGZF block"
            );
        } else {
            (block.uncompressed.len(), false)
        };

        ensure!(
            slice_start <= block.uncompressed.len(),
            "invalid BGZF virtual range: start uncompressed offset {slice_start} exceeds block size {} at compressed offset {}",
            block.uncompressed.len(),
            block.compressed_start
        );
        ensure!(
            slice_end <= block.uncompressed.len(),
            "invalid BGZF virtual range: end uncompressed offset {slice_end} exceeds block size {} at compressed offset {}",
            block.uncompressed.len(),
            block.compressed_start
        );
        ensure!(
            slice_start <= slice_end,
            "invalid BGZF virtual range: start uncompressed offset {slice_start} is after end offset {slice_end} in block at compressed offset {}",
            block.compressed_start
        );

        visit(&block.uncompressed[slice_start..slice_end])?;

        if range_complete {
            break;
        }
        compressed_start = block.compressed_end;
    }

    Ok(())
}

fn read_one_bgzf_block(path: &Path, file: &mut File, compressed_start: u64) -> Result<BgzfBlock> {
    file.seek(SeekFrom::Start(compressed_start))
        .with_context(|| format!("failed to seek {} to {compressed_start}", path.display()))?;

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

    let mut decoder = DeflateDecoder::new(&remaining[..payload_end]);
    let mut capped_decoder = decoder
        .by_ref()
        .take((MAX_BGZF_UNCOMPRESSED_BLOCK_SIZE + 1) as u64);
    let mut uncompressed = Vec::with_capacity(MAX_BGZF_UNCOMPRESSED_BLOCK_SIZE);
    capped_decoder
        .read_to_end(&mut uncompressed)
        .with_context(|| {
            format!("failed to inflate BGZF block at compressed offset {compressed_start}")
        })?;
    ensure!(
        uncompressed.len() <= MAX_BGZF_UNCOMPRESSED_BLOCK_SIZE,
        "decoded BGZF block exceeds maximum uncompressed block size at compressed offset {compressed_start}"
    );
    ensure!(
        expected_isize as usize <= MAX_BGZF_UNCOMPRESSED_BLOCK_SIZE,
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

fn virtual_offset_at(block: &BgzfBlock, uncompressed_offset: usize) -> u64 {
    block.virtual_start() | uncompressed_offset as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::DeflateEncoder;
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

    fn write_single_bgzf_block(path: &Path, uncompressed: &[u8]) {
        let mut encoder = DeflateEncoder::new(Vec::new(), flate2::Compression::best());
        encoder.write_all(uncompressed).unwrap();
        let payload = encoder.finish().unwrap();

        let total_size = 18 + payload.len() + 8;
        let bsize = u16::try_from(total_size - 1).unwrap();

        let mut block = Vec::with_capacity(total_size);
        block.extend_from_slice(&[0x1f, 0x8b, 8, 4, 0, 0, 0, 0, 0, 255]);
        block.extend_from_slice(&6_u16.to_le_bytes());
        block.extend_from_slice(b"BC");
        block.extend_from_slice(&2_u16.to_le_bytes());
        block.extend_from_slice(&bsize.to_le_bytes());
        block.extend_from_slice(&payload);
        block.extend_from_slice(&crc32fast::hash(uncompressed).to_le_bytes());
        block.extend_from_slice(&(uncompressed.len() as u32).to_le_bytes());
        std::fs::write(path, block).unwrap();
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
    fn bgzf_range_reader_returns_text_between_block_boundary_offsets() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("tiny.vcf.gz");
        write_bgzf(&input);

        let blocks = read_bgzf_blocks(&input).unwrap();
        let start = blocks.first().unwrap().virtual_start();
        let end = blocks.last().unwrap().virtual_end();

        let bytes = read_virtual_range(&input, start, end).unwrap();
        let text = String::from_utf8(bytes).unwrap();

        assert!(text.contains("chr1\t1"));
    }

    #[test]
    fn bgzf_range_walker_visits_text_without_collecting() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("tiny.vcf.gz");
        write_bgzf(&input);

        let blocks = read_bgzf_blocks(&input).unwrap();
        let start = blocks.first().unwrap().virtual_start();
        let end = blocks.last().unwrap().virtual_end();
        let mut visited = Vec::new();

        for_each_virtual_range_slice(&input, start, end, |slice| {
            visited.extend_from_slice(slice);
            Ok(())
        })
        .unwrap();

        let text = String::from_utf8(visited).unwrap();
        assert!(text.contains("chr1\t1"));
    }

    #[test]
    fn bgzf_range_reader_respects_intra_block_start_offset() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("tiny.vcf.gz");
        write_bgzf(&input);

        let blocks = read_bgzf_blocks(&input).unwrap();
        let block = blocks
            .iter()
            .find(|block| !block.uncompressed.is_empty())
            .unwrap();
        let first_record_offset = block
            .uncompressed
            .windows(b"chr1\t1".len())
            .position(|window| window == b"chr1\t1")
            .unwrap();
        let start = block.virtual_start() | first_record_offset as u64;
        let end = block.virtual_end();

        let bytes = read_virtual_range(&input, start, end).unwrap();
        let text = String::from_utf8(bytes).unwrap();

        assert!(!text.starts_with('#'));
        assert!(text.starts_with("chr1\t1"));
    }

    #[test]
    fn bgzf_range_reader_rejects_end_before_start() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("tiny.vcf.gz");
        write_bgzf(&input);

        let error = read_virtual_range(&input, 2, 1).unwrap_err();

        assert!(
            error.to_string().contains("before start"),
            "unexpected error: {error:#}"
        );
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
    fn bgzf_reader_rejects_oversized_decoded_block_before_isize() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("oversized.vcf.gz");
        let uncompressed = vec![b'A'; MAX_BGZF_UNCOMPRESSED_BLOCK_SIZE + 1];
        write_single_bgzf_block(&input, &uncompressed);

        let error = read_bgzf_blocks(&input).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("decoded BGZF block exceeds maximum uncompressed block size"),
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
