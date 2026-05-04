use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};
use flate2::Compression;
use flate2::read::MultiGzDecoder;
use flate2::write::GzEncoder;

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
    let file = File::create(path)
        .with_context(|| format!("failed to create output {}", path.display()))?;

    if has_gz_extension(path) {
        let encoder = GzEncoder::new(file, Compression::default());
        Ok(Box::new(BufWriter::new(encoder)))
    } else {
        Ok(Box::new(BufWriter::new(file)))
    }
}

fn has_gz_extension(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "gz")
}
