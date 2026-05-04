use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};
use flate2::Compression;
use flate2::read::MultiGzDecoder;
use flate2::write::GzEncoder;

use crate::compat::CompressionMode;

pub fn open_reader(path: &Path) -> Result<Box<dyn std::io::BufRead>> {
    let file =
        File::open(path).with_context(|| format!("failed to open input {}", path.display()))?;

    if has_gz_extension(path) {
        let decoder = MultiGzDecoder::new(file);
        Ok(Box::new(BufReader::new(decoder)))
    } else {
        Ok(Box::new(BufReader::new(file)))
    }
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
