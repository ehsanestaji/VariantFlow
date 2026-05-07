use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::io::open_reader;
use crate::vcf::{RecordView, format_key_index, sample_format_value_at};

#[derive(Debug, Clone)]
struct SampleColumn {
    name: String,
    column: usize,
}

#[derive(Debug, Default)]
struct SampleSelection {
    keep: Option<HashSet<String>>,
    remove: HashSet<String>,
}

#[derive(Debug, Clone, Copy, Default)]
struct SampleMissingness {
    n_data: u64,
    n_missing: u64,
}

pub fn run_freq(
    input: &Path,
    keep: Option<&Path>,
    remove: Option<&Path>,
    output: &Path,
) -> Result<()> {
    let selection = SampleSelection::from_files(keep, remove)?;
    let mut reader = open_reader(input)?;
    let mut line = Vec::new();
    let mut samples: Option<Vec<SampleColumn>> = None;
    let mut writer = BufWriter::new(
        File::create(output).with_context(|| format!("failed to create {}", output.display()))?,
    );

    writeln!(writer, "CHROM\tPOS\tN_ALLELES\tN_CHR\t{{ALLELE:FREQ}}")?;

    while reader.read_until(b'\n', &mut line)? != 0 {
        if line.starts_with(b"#CHROM") {
            samples = Some(selection.resolve(&line)?);
        } else if !line.starts_with(b"#") {
            let samples = samples
                .as_ref()
                .context("VCF header is missing #CHROM sample line")?;
            let record = RecordView::parse(&line)?;
            write_frequency_row(&mut writer, &record, samples)?;
        }
        line.clear();
    }

    writer.flush()?;
    Ok(())
}

pub fn run_missingness(
    input: &Path,
    keep: Option<&Path>,
    remove: Option<&Path>,
    output_prefix: &Path,
) -> Result<()> {
    let selection = SampleSelection::from_files(keep, remove)?;
    let mut reader = open_reader(input)?;
    let mut line = Vec::new();
    let mut samples: Option<Vec<SampleColumn>> = None;
    let mut sample_missingness = Vec::new();
    let lmiss_path = output_prefix.with_extension("lmiss");
    let imiss_path = output_prefix.with_extension("imiss");
    let mut lmiss = BufWriter::new(
        File::create(&lmiss_path)
            .with_context(|| format!("failed to create {}", lmiss_path.display()))?,
    );

    writeln!(
        lmiss,
        "CHR\tPOS\tN_DATA\tN_GENOTYPE_FILTERED\tN_MISS\tF_MISS"
    )?;

    while reader.read_until(b'\n', &mut line)? != 0 {
        if line.starts_with(b"#CHROM") {
            let resolved = selection.resolve(&line)?;
            sample_missingness = vec![SampleMissingness::default(); resolved.len()];
            samples = Some(resolved);
        } else if !line.starts_with(b"#") {
            let samples = samples
                .as_ref()
                .context("VCF header is missing #CHROM sample line")?;
            let record = RecordView::parse(&line)?;
            write_site_missingness_row(&mut lmiss, &record, samples, &mut sample_missingness)?;
        }
        line.clear();
    }
    lmiss.flush()?;

    let samples = samples.context("VCF header is missing #CHROM sample line")?;
    let mut imiss = BufWriter::new(
        File::create(&imiss_path)
            .with_context(|| format!("failed to create {}", imiss_path.display()))?,
    );
    writeln!(imiss, "INDV\tN_DATA\tN_GENOTYPE_FILTERED\tN_MISS\tF_MISS")?;
    for (sample, counts) in samples.iter().zip(sample_missingness) {
        writeln!(
            imiss,
            "{}\t{}\t0\t{}\t{}",
            sample.name,
            counts.n_data,
            counts.n_missing,
            format_fraction(counts.n_missing, counts.n_data)
        )?;
    }
    imiss.flush()?;

    Ok(())
}

fn write_frequency_row(
    writer: &mut impl Write,
    record: &RecordView<'_>,
    samples: &[SampleColumn],
) -> Result<()> {
    let alleles = allele_labels(record.reference(), record.alternate())?;
    let mut counts = vec![0_u64; alleles.len()];
    let Some(gt_index) = record
        .column(8)
        .and_then(|format| format_key_index(format, b"GT"))
    else {
        write_empty_frequency_row(writer, record, &alleles)?;
        return Ok(());
    };

    for sample in samples {
        if let Some(gt) = record
            .column(sample.column)
            .and_then(|value| sample_format_value_at(value, gt_index))
        {
            for_each_called_allele(gt, |allele| {
                if let Some(count) = counts.get_mut(allele) {
                    *count += 1;
                }
            });
        }
    }

    let n_chr = counts.iter().sum::<u64>();
    write!(
        writer,
        "{}\t{}\t{}\t{}",
        bytes_text(record.chrom())?,
        bytes_text(record.pos())?,
        alleles.len(),
        n_chr
    )?;
    for (allele, count) in alleles.iter().zip(counts) {
        write!(
            writer,
            "\t{}:{}",
            allele,
            format_optional_frequency(count, n_chr)
        )?;
    }
    writeln!(writer)?;
    Ok(())
}

fn write_empty_frequency_row(
    writer: &mut impl Write,
    record: &RecordView<'_>,
    alleles: &[String],
) -> Result<()> {
    write!(
        writer,
        "{}\t{}\t{}\t0",
        bytes_text(record.chrom())?,
        bytes_text(record.pos())?,
        alleles.len()
    )?;
    for allele in alleles {
        write!(writer, "\t{allele}:.")?;
    }
    writeln!(writer)?;
    Ok(())
}

fn write_site_missingness_row(
    writer: &mut impl Write,
    record: &RecordView<'_>,
    samples: &[SampleColumn],
    sample_missingness: &mut [SampleMissingness],
) -> Result<()> {
    let gt_index = record
        .column(8)
        .and_then(|format| format_key_index(format, b"GT"));
    let mut missing = 0_u64;

    for (sample, counts) in samples.iter().zip(sample_missingness) {
        counts.n_data += 1;
        let is_missing = gt_index
            .and_then(|index| {
                record
                    .column(sample.column)
                    .and_then(|value| sample_format_value_at(value, index))
            })
            .is_none_or(genotype_is_missing);
        if is_missing {
            counts.n_missing += 1;
            missing += 1;
        }
    }

    let n_data = samples.len() as u64;
    writeln!(
        writer,
        "{}\t{}\t{}\t0\t{}\t{}",
        bytes_text(record.chrom())?,
        bytes_text(record.pos())?,
        n_data,
        missing,
        format_fraction(missing, n_data)
    )?;
    Ok(())
}

impl SampleSelection {
    fn from_files(keep: Option<&Path>, remove: Option<&Path>) -> Result<Self> {
        if keep.is_some() && remove.is_some() {
            bail!("--keep and --remove cannot be used together");
        }

        Ok(Self {
            keep: keep.map(read_sample_set).transpose()?,
            remove: remove.map(read_sample_set).transpose()?.unwrap_or_default(),
        })
    }

    fn resolve(&self, chrom_header: &[u8]) -> Result<Vec<SampleColumn>> {
        let header = bytes_text(trim_line_end(chrom_header))?;
        let mut selected = Vec::new();
        let mut seen = HashSet::new();

        for (column, name) in header.split('\t').enumerate().skip(9) {
            seen.insert(name.to_owned());
            let kept = self.keep.as_ref().is_none_or(|keep| keep.contains(name));
            if kept && !self.remove.contains(name) {
                selected.push(SampleColumn {
                    name: name.to_owned(),
                    column,
                });
            }
        }

        if let Some(keep) = &self.keep {
            let missing: Vec<_> = keep
                .iter()
                .filter(|sample| !seen.contains(*sample))
                .cloned()
                .collect();
            if !missing.is_empty() {
                bail!("sample(s) not found in VCF header: {}", missing.join(","));
            }
        }

        let missing_remove: Vec<_> = self
            .remove
            .iter()
            .filter(|sample| !seen.contains(*sample))
            .cloned()
            .collect();
        if !missing_remove.is_empty() {
            bail!(
                "sample(s) not found in VCF header: {}",
                missing_remove.join(",")
            );
        }

        Ok(selected)
    }
}

fn read_sample_set(path: &Path) -> Result<HashSet<String>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read sample file {}", path.display()))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect())
}

fn allele_labels(reference: &[u8], alternate: &[u8]) -> Result<Vec<String>> {
    let mut labels = vec![bytes_text(reference)?.to_owned()];
    let mut start = 0;
    while start <= alternate.len() {
        let end = memchr::memchr(b',', &alternate[start..])
            .map_or(alternate.len(), |offset| start + offset);
        labels.push(bytes_text(&alternate[start..end])?.to_owned());
        if end == alternate.len() {
            break;
        }
        start = end + 1;
    }
    Ok(labels)
}

fn for_each_called_allele(gt: &[u8], mut observe: impl FnMut(usize)) {
    if genotype_is_missing(gt) {
        return;
    }

    let mut start = 0;
    while start <= gt.len() {
        let end = gt[start..]
            .iter()
            .position(|byte| *byte == b'/' || *byte == b'|')
            .map_or(gt.len(), |offset| start + offset);
        if let Ok(allele) = parse_allele_index(&gt[start..end]) {
            observe(allele);
        }
        if end == gt.len() {
            break;
        }
        start = end + 1;
    }
}

fn parse_allele_index(value: &[u8]) -> Result<usize> {
    if value.is_empty() {
        bail!("empty allele index");
    }
    let mut result = 0_usize;
    for byte in value {
        if !byte.is_ascii_digit() {
            bail!("invalid allele index");
        }
        result = result * 10 + usize::from(byte - b'0');
    }
    Ok(result)
}

fn genotype_is_missing(gt: &[u8]) -> bool {
    gt.is_empty() || gt == b"." || gt.contains(&b'.')
}

fn format_optional_frequency(count: u64, total: u64) -> String {
    if total == 0 {
        ".".to_owned()
    } else {
        format_ratio(count as f64 / total as f64)
    }
}

fn format_fraction(numerator: u64, denominator: u64) -> String {
    if denominator == 0 {
        ".".to_owned()
    } else {
        format_ratio(numerator as f64 / denominator as f64)
    }
}

fn format_ratio(value: f64) -> String {
    let mut text = format!("{value:.6}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn bytes_text(value: &[u8]) -> Result<&str> {
    Ok(std::str::from_utf8(value)?)
}

fn trim_line_end(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r").unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use super::{for_each_called_allele, format_ratio, genotype_is_missing};

    #[test]
    fn genotype_missing_detects_dot_and_partial_missing_values() {
        assert!(genotype_is_missing(b"."));
        assert!(genotype_is_missing(b"./."));
        assert!(genotype_is_missing(b"0/."));
        assert!(!genotype_is_missing(b"0/1"));
        assert!(!genotype_is_missing(b"1|2"));
    }

    #[test]
    fn called_allele_iterator_handles_diploid_phased_and_haploid_genotypes() {
        let mut observed = Vec::new();
        for_each_called_allele(b"10|2", |allele| observed.push(allele));
        assert_eq!(observed, vec![10, 2]);

        observed.clear();
        for_each_called_allele(b"1", |allele| observed.push(allele));
        assert_eq!(observed, vec![1]);

        observed.clear();
        for_each_called_allele(b"./.", |allele| observed.push(allele));
        assert!(observed.is_empty());
    }

    #[test]
    fn ratio_formatting_matches_vcftools_style_plain_tsv() {
        assert_eq!(format_ratio(0.0), "0");
        assert_eq!(format_ratio(1.0), "1");
        assert_eq!(format_ratio(1.0 / 6.0), "0.166667");
        assert_eq!(format_ratio(1.0 / 3.0), "0.333333");
    }
}
