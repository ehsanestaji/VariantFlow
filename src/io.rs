use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, ErrorKind, Read, Seek, SeekFrom, Write};
use std::num::NonZeroUsize;
use std::path::Path;
use std::thread;

use anyhow::{Context, Result, bail};
use flate2::Compression;
use flate2::read::MultiGzDecoder;
use flate2::write::GzEncoder;
use noodles_bgzf::io::MultithreadedReader;

use crate::compat::CompressionMode;

pub const NATIVE_BGZF_THREADS_ENV: &str = "VCF_FAST_NATIVE_BGZF_THREADS";
const DEFAULT_AUTO_BGZF_THREAD_CAP: usize = 4;

pub fn open_reader(path: &Path) -> Result<Box<dyn std::io::BufRead>> {
    open_reader_with_native_bgzf_threads(path, native_bgzf_threads_from_env()?)
}

fn open_reader_with_native_bgzf_threads(
    path: &Path,
    bgzf_threads: Option<NonZeroUsize>,
) -> Result<Box<dyn std::io::BufRead>> {
    let file =
        File::open(path).with_context(|| format!("failed to open input {}", path.display()))?;

    if has_gz_extension(path) {
        if let Some(worker_count) = bgzf_threads.filter(|threads| threads.get() > 1) {
            return open_compressed_reader(path, file, worker_count);
        }
        let decoder = MultiGzDecoder::new(file);
        Ok(Box::new(BufReader::new(decoder)))
    } else {
        Ok(Box::new(BufReader::new(file)))
    }
}

fn open_compressed_reader(
    path: &Path,
    mut file: File,
    worker_count: NonZeroUsize,
) -> Result<Box<dyn std::io::BufRead>> {
    if has_bgzf_header(&mut file)
        .with_context(|| format!("failed to inspect {}", path.display()))?
    {
        let reader = MultithreadedReader::with_worker_count(worker_count, file);
        Ok(Box::new(reader))
    } else {
        let decoder = MultiGzDecoder::new(file);
        Ok(Box::new(BufReader::new(decoder)))
    }
}

pub fn native_bgzf_threads_from_env() -> Result<Option<NonZeroUsize>> {
    let available = thread::available_parallelism().map_or(1, NonZeroUsize::get);
    match env::var(NATIVE_BGZF_THREADS_ENV) {
        Ok(raw) => resolve_native_bgzf_threads(Some(raw.as_str()), available),
        Err(env::VarError::NotPresent) => resolve_native_bgzf_threads(None, available),
        Err(env::VarError::NotUnicode(_)) => {
            bail!("{NATIVE_BGZF_THREADS_ENV} must be valid UTF-8")
        }
    }
}

pub fn parse_native_bgzf_threads(raw: Option<&str>) -> Result<Option<NonZeroUsize>> {
    resolve_native_bgzf_threads(raw, DEFAULT_AUTO_BGZF_THREAD_CAP)
}

fn resolve_native_bgzf_threads(
    raw: Option<&str>,
    available_parallelism: usize,
) -> Result<Option<NonZeroUsize>> {
    let Some(raw) = raw else {
        return Ok(auto_native_bgzf_threads(available_parallelism));
    };
    if raw.eq_ignore_ascii_case("auto") {
        return Ok(auto_native_bgzf_threads(available_parallelism));
    }

    let value = raw.parse::<usize>().map_err(|_| {
        anyhow::anyhow!("{NATIVE_BGZF_THREADS_ENV} must be auto or a positive integer")
    })?;
    let Some(value) = NonZeroUsize::new(value) else {
        bail!("{NATIVE_BGZF_THREADS_ENV} must be auto or a positive integer");
    };

    Ok(Some(value))
}

fn auto_native_bgzf_threads(available_parallelism: usize) -> Option<NonZeroUsize> {
    let threads = available_parallelism.clamp(1, DEFAULT_AUTO_BGZF_THREAD_CAP);
    NonZeroUsize::new(threads)
}

pub fn open_writer(path: &Path) -> Result<Box<dyn Write>> {
    open_writer_with_compression(path, CompressionMode::Auto)
}

pub fn open_writer_with_compression(
    path: &Path,
    compression: CompressionMode,
) -> Result<Box<dyn Write>> {
    let file = File::create(path)
        .with_context(|| format!("failed to create output {}", path.display()))?;

    match compression {
        CompressionMode::Bgzf => anyhow::bail!("--compression bgzf requires the htslib feature"),
        CompressionMode::Gzip => {
            let encoder = GzEncoder::new(file, Compression::default());
            Ok(Box::new(BufWriter::new(encoder)))
        }
        CompressionMode::Plain => Ok(Box::new(BufWriter::new(file))),
        CompressionMode::Auto if has_gz_extension(path) => {
            let encoder = GzEncoder::new(file, Compression::default());
            Ok(Box::new(BufWriter::new(encoder)))
        }
        CompressionMode::Auto => Ok(Box::new(BufWriter::new(file))),
    }
}

#[cfg(feature = "htslib")]
pub fn open_bgzf_writer(path: &Path) -> Result<Box<dyn Write>> {
    let writer = rust_htslib::bgzf::Writer::from_path(path)
        .with_context(|| format!("failed to create BGZF output {}", path.display()))?;
    Ok(Box::new(writer))
}

#[cfg(feature = "htslib")]
pub fn open_vcf_writer(path: &Path, compression: CompressionMode) -> Result<Box<dyn Write>> {
    match compression {
        CompressionMode::Bgzf => open_bgzf_writer(path),
        _ => open_writer_with_compression(path, compression),
    }
}

#[cfg(not(feature = "htslib"))]
pub fn open_vcf_writer(path: &Path, compression: CompressionMode) -> Result<Box<dyn Write>> {
    open_writer_with_compression(path, compression)
}

fn has_gz_extension(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "gz")
}

fn has_bgzf_header(file: &mut File) -> Result<bool> {
    file.seek(SeekFrom::Start(0))?;
    let result = read_bgzf_header(file);
    file.seek(SeekFrom::Start(0))?;
    result
}

fn read_bgzf_header(file: &mut File) -> Result<bool> {
    let mut fixed = [0_u8; 12];
    if let Err(error) = file.read_exact(&mut fixed) {
        return if error.kind() == ErrorKind::UnexpectedEof {
            Ok(false)
        } else {
            Err(error.into())
        };
    }

    let is_gzip = fixed[0] == 0x1f && fixed[1] == 0x8b && fixed[2] == 8;
    let has_extra = fixed[3] & 0x04 != 0;
    if !is_gzip || !has_extra {
        return Ok(false);
    }

    let extra_len = u16::from_le_bytes([fixed[10], fixed[11]]) as usize;
    let mut extra = vec![0_u8; extra_len];
    if let Err(error) = file.read_exact(&mut extra) {
        return if error.kind() == ErrorKind::UnexpectedEof {
            Ok(false)
        } else {
            Err(error.into())
        };
    }

    let mut index = 0;
    while index + 4 <= extra.len() {
        let subfield_id = &extra[index..index + 2];
        let subfield_len = u16::from_le_bytes([extra[index + 2], extra[index + 3]]) as usize;
        index += 4;
        if index + subfield_len > extra.len() {
            return Ok(false);
        }
        if subfield_id == b"BC" && subfield_len == 2 {
            return Ok(true);
        }
        index += subfield_len;
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use std::io::{BufRead, Write};

    #[test]
    fn native_bgzf_thread_parser_accepts_unset_and_positive_values() {
        assert_eq!(
            parse_native_bgzf_threads(None).unwrap().unwrap().get(),
            DEFAULT_AUTO_BGZF_THREAD_CAP
        );
        assert_eq!(
            parse_native_bgzf_threads(Some("auto"))
                .unwrap()
                .unwrap()
                .get(),
            DEFAULT_AUTO_BGZF_THREAD_CAP
        );
        assert_eq!(
            parse_native_bgzf_threads(Some("1")).unwrap().unwrap().get(),
            1
        );
        assert_eq!(
            parse_native_bgzf_threads(Some("4")).unwrap().unwrap().get(),
            4
        );
    }

    #[test]
    fn native_bgzf_thread_parser_rejects_invalid_values() {
        for raw in ["0", "-2", "fast"] {
            let error = parse_native_bgzf_threads(Some(raw)).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("VCF_FAST_NATIVE_BGZF_THREADS must be auto or a positive integer")
            );
        }
    }

    #[test]
    fn auto_bgzf_thread_policy_caps_available_parallelism() {
        assert_eq!(
            resolve_native_bgzf_threads(None, 1).unwrap().unwrap().get(),
            1
        );
        assert_eq!(
            resolve_native_bgzf_threads(None, 2).unwrap().unwrap().get(),
            2
        );
        assert_eq!(
            resolve_native_bgzf_threads(None, 8).unwrap().unwrap().get(),
            4
        );
        assert_eq!(
            resolve_native_bgzf_threads(Some("auto"), 16)
                .unwrap()
                .unwrap()
                .get(),
            4
        );
    }

    #[test]
    fn threaded_bgzf_reader_reads_bgzf_input() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("input.vcf.gz");
        {
            let file = File::create(&path).unwrap();
            let mut writer = noodles_bgzf::io::Writer::new(file);
            writer.write_all(b"##fileformat=VCFv4.3\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n1\t1\t.\tA\tG\t50\tPASS\t.\n").unwrap();
            writer.finish().unwrap();
        }

        let mut reader = open_reader_with_native_bgzf_threads(&path, NonZeroUsize::new(2)).unwrap();
        let mut line = Vec::new();
        reader.read_until(b'\n', &mut line).unwrap();
        assert_eq!(line, b"##fileformat=VCFv4.3\n");
    }

    #[test]
    fn threaded_bgzf_request_falls_back_for_ordinary_gzip() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("input.vcf.gz");
        {
            let file = File::create(&path).unwrap();
            let mut writer = GzEncoder::new(file, Compression::default());
            writer.write_all(b"plain gzip\n").unwrap();
            writer.finish().unwrap();
        }

        let mut reader = open_reader_with_native_bgzf_threads(&path, NonZeroUsize::new(4)).unwrap();
        let mut line = Vec::new();
        reader.read_until(b'\n', &mut line).unwrap();
        assert_eq!(line, b"plain gzip\n");
    }
}
