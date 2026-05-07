use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use clap::ValueEnum;

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

#[derive(Debug, Clone, Default)]
struct SiteAlleleSummary {
    chrom: String,
    pos: u64,
    allele_counts: Vec<u64>,
    genotypes: Vec<Option<Vec<usize>>>,
}

#[derive(Debug, Clone, Copy, Default)]
struct BiallelicSummary {
    hom_ref: u64,
    het: u64,
    hom_alt: u64,
    ref_count: u64,
    alt_count: u64,
    n_chr: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct HetSampleSummary {
    observed_hom: u64,
    expected_hom: f64,
    n_sites: u64,
}

#[derive(Debug, Clone, Default)]
struct WindowSummary {
    chrom: String,
    start: u64,
    end: u64,
    pi_sum: f64,
    segregating_sites: u64,
    site_pairs: u64,
    mismatches: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FstEstimator {
    Hudson,
    WeirCockerham,
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
    writeln!(imiss, "INDV\tN_DATA\tN_GENOTYPES_FILTERED\tN_MISS\tF_MISS")?;
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

pub fn run_hardy(
    input: &Path,
    keep: Option<&Path>,
    remove: Option<&Path>,
    output: &Path,
) -> Result<()> {
    let selection = SampleSelection::from_files(keep, remove)?;
    let mut writer = BufWriter::new(
        File::create(output).with_context(|| format!("failed to create {}", output.display()))?,
    );
    writeln!(
        writer,
        "CHROM\tPOS\tOBS_HOM_REF\tOBS_HET\tOBS_HOM_ALT\tE_HOM_REF\tE_HET\tE_HOM_ALT\tCHISQ_HWE"
    )?;

    stream_site_summaries(input, &selection, |site| {
        if let Some(summary) = site.biallelic_summary() {
            let called = (summary.hom_ref + summary.het + summary.hom_alt) as f64;
            if called > 0.0 && summary.n_chr > 0 {
                let p_ref = summary.ref_count as f64 / summary.n_chr as f64;
                let p_alt = summary.alt_count as f64 / summary.n_chr as f64;
                let e_hom_ref = called * p_ref * p_ref;
                let e_het = called * 2.0 * p_ref * p_alt;
                let e_hom_alt = called * p_alt * p_alt;
                let chisq = chi_square([
                    (summary.hom_ref as f64, e_hom_ref),
                    (summary.het as f64, e_het),
                    (summary.hom_alt as f64, e_hom_alt),
                ]);
                writeln!(
                    writer,
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    site.chrom,
                    site.pos,
                    summary.hom_ref,
                    summary.het,
                    summary.hom_alt,
                    format_ratio(e_hom_ref),
                    format_ratio(e_het),
                    format_ratio(e_hom_alt),
                    format_ratio(chisq)
                )?;
            }
        }
        Ok(())
    })?;
    writer.flush()?;
    Ok(())
}

pub fn run_het(
    input: &Path,
    keep: Option<&Path>,
    remove: Option<&Path>,
    output: &Path,
) -> Result<()> {
    let selection = SampleSelection::from_files(keep, remove)?;
    let mut sample_names = Vec::new();
    let mut summaries = Vec::new();

    stream_site_summaries_with_samples(input, &selection, |samples, site| {
        if sample_names.is_empty() {
            sample_names = samples.iter().map(|sample| sample.name.clone()).collect();
            summaries = vec![HetSampleSummary::default(); samples.len()];
        }
        let Some(biallelic) = site.biallelic_summary() else {
            return Ok(());
        };
        if biallelic.n_chr == 0 {
            return Ok(());
        }
        let Some(pi) = site_pi(site) else {
            return Ok(());
        };
        let expected_hom = 1.0 - pi;

        for (genotype, summary) in site.genotypes.iter().zip(&mut summaries) {
            let Some(genotype) = genotype else {
                continue;
            };
            if genotype.len() != 2 || genotype.iter().any(|allele| *allele > 1) {
                continue;
            }
            summary.n_sites += 1;
            summary.expected_hom += expected_hom;
            if genotype[0] == genotype[1] {
                summary.observed_hom += 1;
            }
        }
        Ok(())
    })?;

    let mut writer = BufWriter::new(
        File::create(output).with_context(|| format!("failed to create {}", output.display()))?,
    );
    writeln!(writer, "INDV\tO_HOM\tE_HOM\tN_SITES\tF")?;
    for (sample, summary) in sample_names.iter().zip(summaries) {
        let denominator = summary.n_sites as f64 - summary.expected_hom;
        let f = if denominator == 0.0 {
            ".".to_owned()
        } else {
            format_fixed_trimmed(
                (summary.observed_hom as f64 - summary.expected_hom) / denominator,
                5,
            )
        };
        writeln!(
            writer,
            "{}\t{}\t{}\t{}\t{}",
            sample,
            summary.observed_hom,
            format_fixed_trimmed(summary.expected_hom, 1),
            summary.n_sites,
            f
        )?;
    }
    writer.flush()?;
    Ok(())
}

pub fn run_fst(
    input: &Path,
    populations: &[std::path::PathBuf],
    estimator: FstEstimator,
    output: &Path,
) -> Result<()> {
    if populations.len() != 2 {
        bail!("fst requires exactly two --pop files");
    }
    let pop1 = read_sample_set(&populations[0])?;
    let pop2 = read_sample_set(&populations[1])?;
    let selection = SampleSelection::default();
    let mut pop1_indices = Vec::new();
    let mut pop2_indices = Vec::new();
    let mut writer = BufWriter::new(
        File::create(output).with_context(|| format!("failed to create {}", output.display()))?,
    );
    match estimator {
        FstEstimator::Hudson => writeln!(writer, "CHROM\tPOS\tHUDSON_FST")?,
        FstEstimator::WeirCockerham => writeln!(writer, "CHROM\tPOS\tWEIR_AND_COCKERHAM_FST")?,
    }

    stream_site_summaries_with_samples(input, &selection, |samples, site| {
        if pop1_indices.is_empty() && pop2_indices.is_empty() {
            pop1_indices = population_indices(samples, &pop1)?;
            pop2_indices = population_indices(samples, &pop2)?;
        }
        let fst = match estimator {
            FstEstimator::Hudson => hudson_fst(site, &pop1_indices, &pop2_indices),
            FstEstimator::WeirCockerham => weir_cockerham_fst(site, &pop1_indices, &pop2_indices)?,
        };
        match (estimator, fst) {
            (_, Some(fst)) => {
                writeln!(
                    writer,
                    "{}\t{}\t{}",
                    site.chrom,
                    site.pos,
                    format_ratio(fst)
                )?;
            }
            (FstEstimator::WeirCockerham, None) => {
                writeln!(writer, "{}\t{}\tnan", site.chrom, site.pos)?;
            }
            (FstEstimator::Hudson, None) => {}
        }
        Ok(())
    })?;
    writer.flush()?;
    Ok(())
}

pub fn run_pi(
    input: &Path,
    keep: Option<&Path>,
    remove: Option<&Path>,
    window_size: Option<u64>,
    output: &Path,
) -> Result<()> {
    let selection = SampleSelection::from_files(keep, remove)?;
    let mut writer = BufWriter::new(
        File::create(output).with_context(|| format!("failed to create {}", output.display()))?,
    );
    if let Some(window_size) = window_size {
        if window_size == 0 {
            bail!("--window-size must be positive");
        }
        let mut windows = Vec::new();
        let mut n_chr = 0_u64;
        stream_site_summaries_with_samples(input, &selection, |samples, site| {
            if n_chr == 0 {
                n_chr = samples.len() as u64 * 2;
            }
            add_window_pi(&mut windows, site, window_pi_counts(site), window_size);
            Ok(())
        })?;
        writeln!(writer, "CHROM\tBIN_START\tBIN_END\tN_VARIANTS\tPI")?;
        for window in windows {
            let monomorphic_sites = window_size.saturating_sub(window.segregating_sites);
            let total_site_pairs = n_chr * n_chr.saturating_sub(1);
            let denominator = window.site_pairs + monomorphic_sites * total_site_pairs;
            writeln!(
                writer,
                "{}\t{}\t{}\t{}\t{}",
                window.chrom,
                window.start,
                window.end,
                window.segregating_sites,
                format_fraction(window.mismatches, denominator)
            )?;
        }
    } else {
        writeln!(writer, "CHROM\tPOS\tPI")?;
        stream_site_summaries(input, &selection, |site| {
            if let Some(pi) = site_pi(site) {
                writeln!(writer, "{}\t{}\t{}", site.chrom, site.pos, format_ratio(pi))?;
            }
            Ok(())
        })?;
    }
    writer.flush()?;
    Ok(())
}

pub fn run_tajima_d(
    input: &Path,
    keep: Option<&Path>,
    remove: Option<&Path>,
    window_size: u64,
    output: &Path,
) -> Result<()> {
    if window_size == 0 {
        bail!("--window-size must be positive");
    }
    let selection = SampleSelection::from_files(keep, remove)?;
    let mut windows = Vec::new();
    let mut n_chr = 0_u64;
    stream_site_summaries_with_samples(input, &selection, |samples, site| {
        if n_chr == 0 {
            n_chr = samples.len() as u64 * 2;
        }
        let pi = tajima_site_pi_component(site, n_chr)?;
        add_tajima_window(&mut windows, site, pi, window_size);
        Ok(())
    })?;

    let mut writer = BufWriter::new(
        File::create(output).with_context(|| format!("failed to create {}", output.display()))?,
    );
    writeln!(writer, "CHROM\tBIN_START\tN_SNPS\tTajimaD")?;
    for window in windows {
        let value = tajima_d(window.pi_sum, window.segregating_sites, n_chr)
            .map(format_ratio)
            .unwrap_or_else(|| "nan".to_string());
        writeln!(
            writer,
            "{}\t{}\t{}\t{}",
            window.chrom, window.start, window.segregating_sites, value
        )?;
    }
    writer.flush()?;
    Ok(())
}

pub fn run_ld(
    input: &Path,
    keep: Option<&Path>,
    remove: Option<&Path>,
    max_distance: Option<u64>,
    output: &Path,
) -> Result<()> {
    let selection = SampleSelection::from_files(keep, remove)?;
    let mut sites = Vec::new();
    stream_site_summaries(input, &selection, |site| {
        if site.biallelic_summary().is_some() {
            sites.push(site.clone());
        }
        Ok(())
    })?;

    let mut writer = BufWriter::new(
        File::create(output).with_context(|| format!("failed to create {}", output.display()))?,
    );
    writeln!(writer, "CHR\tPOS1\tPOS2\tN_INDV\tR^2")?;
    for left_index in 0..sites.len() {
        for right_index in left_index + 1..sites.len() {
            let left = &sites[left_index];
            let right = &sites[right_index];
            if left.chrom != right.chrom {
                continue;
            }
            if max_distance.is_some_and(|distance| right.pos.saturating_sub(left.pos) > distance) {
                continue;
            }
            if let Some((n, r2)) = genotype_dosage_r2(left, right) {
                writeln!(
                    writer,
                    "{}\t{}\t{}\t{}\t{}",
                    left.chrom,
                    left.pos,
                    right.pos,
                    n,
                    format_ratio(r2)
                )?;
            }
        }
    }
    writer.flush()?;
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

    for_each_selected_sample_value(record, samples, |_index, _sample, value| {
        if let Some(gt) = sample_format_value_at(value, gt_index) {
            for_each_called_allele(gt, |allele| {
                if let Some(count) = counts.get_mut(allele) {
                    *count += 1;
                }
            });
        }
    });

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
    let mut n_data = 0_u64;

    let mut visited = vec![false; samples.len()];
    for counts in sample_missingness.iter_mut() {
        counts.n_data += 1;
    }

    for_each_selected_sample_value(record, samples, |index, _sample, value| {
        visited[index] = true;
        let counts = &mut sample_missingness[index];
        let genotype = gt_index.and_then(|index| sample_format_value_at(value, index));
        let allele_slots = genotype.map_or(2, genotype_allele_slots);
        n_data += allele_slots;
        let missing_alleles = genotype.map_or(allele_slots, genotype_missing_allele_slots);
        if missing_alleles > 0 {
            counts.n_missing += 1;
            missing += missing_alleles;
        }
    });

    for (was_visited, counts) in visited.into_iter().zip(sample_missingness.iter_mut()) {
        if !was_visited {
            n_data += 2;
            missing += 2;
            counts.n_missing += 1;
        }
    }

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

fn genotype_allele_slots(gt: &[u8]) -> u64 {
    if gt.is_empty() {
        return 1;
    }
    1 + gt
        .iter()
        .filter(|byte| **byte == b'/' || **byte == b'|')
        .count() as u64
}

fn for_each_selected_sample_value<'a>(
    record: &RecordView<'a>,
    samples: &[SampleColumn],
    mut visit: impl FnMut(usize, &SampleColumn, &'a [u8]),
) {
    let mut selected = 0_usize;
    record.for_each_sample_column_with_index(|column, value| {
        while selected < samples.len() && samples[selected].column < column {
            selected += 1;
        }
        if selected < samples.len() && samples[selected].column == column {
            visit(selected, &samples[selected], value);
            selected += 1;
        }
    });
}

impl SiteAlleleSummary {
    fn biallelic_summary(&self) -> Option<BiallelicSummary> {
        if self.allele_counts.len() != 2 {
            return None;
        }
        let mut summary = BiallelicSummary {
            ref_count: self.allele_counts[0],
            alt_count: self.allele_counts[1],
            n_chr: self.allele_counts.iter().sum(),
            ..BiallelicSummary::default()
        };

        for genotype in self.genotypes.iter().flatten() {
            if genotype.len() != 2 || genotype.iter().any(|allele| *allele > 1) {
                continue;
            }
            match (genotype[0], genotype[1]) {
                (0, 0) => summary.hom_ref += 1,
                (1, 1) => summary.hom_alt += 1,
                _ => summary.het += 1,
            }
        }

        Some(summary)
    }
}

fn stream_site_summaries(
    input: &Path,
    selection: &SampleSelection,
    mut visit: impl FnMut(&SiteAlleleSummary) -> Result<()>,
) -> Result<()> {
    stream_site_summaries_with_samples(input, selection, |_samples, site| visit(site))
}

fn stream_site_summaries_with_samples(
    input: &Path,
    selection: &SampleSelection,
    mut visit: impl FnMut(&[SampleColumn], &SiteAlleleSummary) -> Result<()>,
) -> Result<()> {
    let mut reader = open_reader(input)?;
    let mut line = Vec::new();
    let mut samples: Option<Vec<SampleColumn>> = None;

    while reader.read_until(b'\n', &mut line)? != 0 {
        if line.starts_with(b"#CHROM") {
            samples = Some(selection.resolve(&line)?);
        } else if !line.starts_with(b"#") {
            let samples = samples
                .as_ref()
                .context("VCF header is missing #CHROM sample line")?;
            let record = RecordView::parse(&line)?;
            let site = site_allele_summary(&record, samples)?;
            visit(samples, &site)?;
        }
        line.clear();
    }

    if samples.is_none() {
        bail!("VCF header is missing #CHROM sample line");
    }
    Ok(())
}

fn site_allele_summary(
    record: &RecordView<'_>,
    samples: &[SampleColumn],
) -> Result<SiteAlleleSummary> {
    let alleles = allele_labels(record.reference(), record.alternate())?;
    let mut allele_counts = vec![0_u64; alleles.len()];
    let gt_index = record
        .column(8)
        .and_then(|format| format_key_index(format, b"GT"));
    let mut genotypes = vec![None; samples.len()];

    for_each_selected_sample_value(record, samples, |index, _sample, value| {
        let genotype = gt_index
            .and_then(|index| sample_format_value_at(value, index))
            .and_then(parse_called_genotype);
        if let Some(genotype) = &genotype {
            for allele in genotype {
                if let Some(count) = allele_counts.get_mut(*allele) {
                    *count += 1;
                }
            }
        }
        genotypes[index] = genotype;
    });

    Ok(SiteAlleleSummary {
        chrom: bytes_text(record.chrom())?.to_owned(),
        pos: record.pos_u64()?,
        allele_counts,
        genotypes,
    })
}

fn parse_called_genotype(gt: &[u8]) -> Option<Vec<usize>> {
    if genotype_is_missing(gt) {
        return None;
    }

    let mut alleles = Vec::new();
    for_each_called_allele(gt, |allele| alleles.push(allele));
    (!alleles.is_empty()).then_some(alleles)
}

fn chi_square(values: [(f64, f64); 3]) -> f64 {
    values
        .into_iter()
        .filter(|(_observed, expected)| *expected > 0.0)
        .map(|(observed, expected)| {
            let delta = observed - expected;
            delta * delta / expected
        })
        .sum()
}

fn population_indices(
    samples: &[SampleColumn],
    population: &HashSet<String>,
) -> Result<Vec<usize>> {
    let mut indices = Vec::new();
    let seen: HashSet<_> = samples.iter().map(|sample| sample.name.as_str()).collect();
    let missing: Vec<_> = population
        .iter()
        .filter(|sample| !seen.contains(sample.as_str()))
        .cloned()
        .collect();
    if !missing.is_empty() {
        bail!("sample(s) not found in VCF header: {}", missing.join(","));
    }

    for (index, sample) in samples.iter().enumerate() {
        if population.contains(&sample.name) {
            indices.push(index);
        }
    }
    Ok(indices)
}

fn hudson_fst(site: &SiteAlleleSummary, pop1: &[usize], pop2: &[usize]) -> Option<f64> {
    let (alt1, n1) = population_alt_counts(site, pop1)?;
    let (alt2, n2) = population_alt_counts(site, pop2)?;
    if n1 < 2 || n2 < 2 {
        return None;
    }
    let p1 = alt1 as f64 / n1 as f64;
    let p2 = alt2 as f64 / n2 as f64;
    let numerator = (p1 - p2).powi(2)
        - p1 * (1.0 - p1) / (n1 as f64 - 1.0)
        - p2 * (1.0 - p2) / (n2 as f64 - 1.0);
    let denominator = p1 * (1.0 - p2) + p2 * (1.0 - p1);
    (denominator != 0.0).then_some(numerator / denominator)
}

fn population_alt_counts(site: &SiteAlleleSummary, indices: &[usize]) -> Option<(u64, u64)> {
    let mut alt = 0_u64;
    let mut n = 0_u64;
    for index in indices {
        let Some(genotype) = site.genotypes.get(*index).and_then(Option::as_ref) else {
            continue;
        };
        if genotype.iter().any(|allele| *allele > 1) {
            continue;
        }
        for allele in genotype {
            n += 1;
            if *allele == 1 {
                alt += 1;
            }
        }
    }
    (n > 0).then_some((alt, n))
}

#[derive(Debug, Clone, Copy, Default)]
struct PopulationWeirSummary {
    called_individuals: u64,
    alt_count: u64,
    heterozygotes: u64,
}

fn weir_cockerham_fst(
    site: &SiteAlleleSummary,
    pop1: &[usize],
    pop2: &[usize],
) -> Result<Option<f64>> {
    if site.allele_counts.len() != 2 {
        bail!(
            "weir-cockerham fst supports only biallelic sites; multiallelic sites are not supported"
        );
    }

    let pop1 = population_weir_summary(site, pop1)?;
    let pop2 = population_weir_summary(site, pop2)?;
    let n1 = pop1.called_individuals as f64;
    let n2 = pop2.called_individuals as f64;
    if n1 == 0.0 || n2 == 0.0 {
        return Ok(None);
    }

    let r = 2.0;
    let n_sum = n1 + n2;
    let nbar = n_sum / r;
    if nbar <= 1.0 {
        return Ok(None);
    }
    let sum_nsqr = n1.powi(2) + n2.powi(2);
    let nc = (n_sum - (sum_nsqr / n_sum)) / (r - 1.0);
    if nc == 0.0 {
        return Ok(None);
    }

    let p1 = pop1.alt_count as f64 / (2.0 * n1);
    let p2 = pop2.alt_count as f64 / (2.0 * n2);
    let pbar = (pop1.alt_count + pop2.alt_count) as f64 / (2.0 * n_sum);
    let hbar = (pop1.heterozygotes + pop2.heterozygotes) as f64 / n_sum;
    let ssqr = (n1 * (p1 - pbar).powi(2) + n2 * (p2 - pbar).powi(2)) / ((r - 1.0) * nbar);

    // VCFtools' --weir-fst-pop implementation of Weir and Cockerham's biallelic
    // variance components. This path is intentionally scoped to called diploid
    // biallelic genotypes.
    let a = (ssqr - (pbar * (1.0 - pbar) - (((r - 1.0) * ssqr) / r) - (hbar / 4.0)) / (nbar - 1.0))
        * nbar
        / nc;
    let b = (pbar * (1.0 - pbar)
        - (ssqr * (r - 1.0) / r)
        - hbar * (((2.0 * nbar) - 1.0) / (4.0 * nbar)))
        * nbar
        / (nbar - 1.0);
    let c = hbar / 2.0;
    let denominator = a + b + c;
    if denominator.is_nan() || denominator == 0.0 {
        return Ok(None);
    }
    Ok(Some(a / denominator))
}

fn population_weir_summary(
    site: &SiteAlleleSummary,
    indices: &[usize],
) -> Result<PopulationWeirSummary> {
    let mut summary = PopulationWeirSummary::default();
    for index in indices {
        let Some(genotype) = site.genotypes.get(*index).and_then(Option::as_ref) else {
            continue;
        };
        if genotype.len() != 2 || genotype.iter().any(|allele| *allele > 1) {
            bail!("weir-cockerham fst supports only diploid biallelic called genotypes");
        }
        summary.called_individuals += 1;
        summary.alt_count += genotype.iter().filter(|allele| **allele == 1).count() as u64;
        if genotype[0] != genotype[1] {
            summary.heterozygotes += 1;
        }
    }
    Ok(summary)
}

fn site_pi(site: &SiteAlleleSummary) -> Option<f64> {
    let n_chr = site.allele_counts.iter().sum::<u64>();
    if n_chr < 2 {
        return None;
    }
    let homozygosity = site
        .allele_counts
        .iter()
        .map(|count| {
            let frequency = *count as f64 / n_chr as f64;
            frequency * frequency
        })
        .sum::<f64>();
    Some(n_chr as f64 / (n_chr as f64 - 1.0) * (1.0 - homozygosity))
}

#[derive(Debug, Clone, Copy)]
struct WindowPiCounts {
    site_pairs: u64,
    mismatches: u64,
}

fn window_pi_counts(site: &SiteAlleleSummary) -> Option<WindowPiCounts> {
    let n_chr = site.allele_counts.iter().sum::<u64>();
    if n_chr < 2 {
        return None;
    }

    let mismatches = site
        .allele_counts
        .iter()
        .map(|count| count * (n_chr - count))
        .sum::<u64>();
    (mismatches > 0).then_some(WindowPiCounts {
        site_pairs: n_chr * (n_chr - 1),
        mismatches,
    })
}

fn tajima_site_pi_component(site: &SiteAlleleSummary, n_chr: u64) -> Result<Option<f64>> {
    if site.allele_counts.len() != 2 {
        bail!("tajima-d supports only biallelic sites");
    }
    if n_chr < 2 {
        return Ok(None);
    }
    for genotype in site.genotypes.iter().flatten() {
        if genotype.len() != 2 || genotype.iter().any(|allele| *allele > 1) {
            bail!("tajima-d supports only diploid biallelic called genotypes");
        }
    }

    let observed_n_chr = site.allele_counts.iter().sum::<u64>();
    if observed_n_chr == 0 || site.allele_counts[0] == 0 || site.allele_counts[1] == 0 {
        return Ok(None);
    }

    let p_ref = site.allele_counts[0] as f64 / observed_n_chr as f64;
    Ok(Some(
        2.0 * p_ref * (1.0 - p_ref) * n_chr as f64 / (n_chr as f64 - 1.0),
    ))
}

fn add_window_pi(
    windows: &mut Vec<WindowSummary>,
    site: &SiteAlleleSummary,
    counts: Option<WindowPiCounts>,
    window_size: u64,
) {
    let start = ((site.pos - 1) / window_size) * window_size + 1;
    let end = start + window_size - 1;
    if let Some(window) = windows
        .last_mut()
        .filter(|window| window.chrom == site.chrom && window.start == start)
    {
        if let Some(counts) = counts {
            window.segregating_sites += 1;
            window.site_pairs += counts.site_pairs;
            window.mismatches += counts.mismatches;
        }
        return;
    }

    let (segregating_sites, site_pairs, mismatches) = counts
        .map(|counts| (1, counts.site_pairs, counts.mismatches))
        .unwrap_or((0, 0, 0));
    windows.push(WindowSummary {
        chrom: site.chrom.clone(),
        start,
        end,
        pi_sum: 0.0,
        segregating_sites,
        site_pairs,
        mismatches,
    });
}

fn add_tajima_window(
    windows: &mut Vec<WindowSummary>,
    site: &SiteAlleleSummary,
    pi: Option<f64>,
    window_size: u64,
) {
    let start = (site.pos / window_size) * window_size;
    let end = start + window_size - 1;
    if let Some(window) = windows
        .last_mut()
        .filter(|window| window.chrom == site.chrom && window.start == start)
    {
        if let Some(pi) = pi {
            window.pi_sum += pi;
            window.segregating_sites += 1;
        }
        return;
    }

    if let Some(last) = windows.last().filter(|window| window.chrom == site.chrom) {
        let mut gap_start = last.start + window_size;
        while gap_start < start {
            windows.push(WindowSummary {
                chrom: site.chrom.clone(),
                start: gap_start,
                end: gap_start + window_size - 1,
                pi_sum: 0.0,
                segregating_sites: 0,
                site_pairs: 0,
                mismatches: 0,
            });
            gap_start += window_size;
        }
    }

    windows.push(WindowSummary {
        chrom: site.chrom.clone(),
        start,
        end,
        pi_sum: pi.unwrap_or(0.0),
        segregating_sites: u64::from(pi.is_some()),
        site_pairs: 0,
        mismatches: 0,
    });
}

fn tajima_d(pi_sum: f64, segregating_sites: u64, n_chr: u64) -> Option<f64> {
    let n = n_chr as usize;
    let s = segregating_sites as f64;
    if n < 2 || segregating_sites < 1 {
        return None;
    }

    let a1 = (1..n).map(|i| 1.0 / i as f64).sum::<f64>();
    let a2 = (1..n).map(|i| 1.0 / (i * i) as f64).sum::<f64>();
    let n = n as f64;
    let b1 = (n + 1.0) / (3.0 * (n - 1.0));
    let b2 = 2.0 * (n * n + n + 3.0) / (9.0 * n * (n - 1.0));
    let c1 = b1 - 1.0 / a1;
    let c2 = b2 - (n + 2.0) / (a1 * n) + a2 / (a1 * a1);
    let e1 = c1 / a1;
    let e2 = c2 / (a1 * a1 + a2);
    let denominator = (e1 * s + e2 * s * (s - 1.0)).sqrt();
    (denominator > 0.0).then_some((pi_sum - s / a1) / denominator)
}

fn genotype_dosage_r2(left: &SiteAlleleSummary, right: &SiteAlleleSummary) -> Option<(u64, f64)> {
    let mut pairs = Vec::new();
    for (left_gt, right_gt) in left.genotypes.iter().zip(&right.genotypes) {
        if let (Some(left_dosage), Some(right_dosage)) = (
            left_gt.as_deref().and_then(alt_dosage),
            right_gt.as_deref().and_then(alt_dosage),
        ) {
            pairs.push((left_dosage, right_dosage));
        }
    }

    if pairs.len() < 2 {
        return None;
    }

    let n = pairs.len() as f64;
    let mean_left = pairs.iter().map(|(left, _right)| left).sum::<f64>() / n;
    let mean_right = pairs.iter().map(|(_left, right)| right).sum::<f64>() / n;
    let mut covariance = 0.0;
    let mut left_ss = 0.0;
    let mut right_ss = 0.0;
    for (left, right) in &pairs {
        let left_delta = left - mean_left;
        let right_delta = right - mean_right;
        covariance += left_delta * right_delta;
        left_ss += left_delta * left_delta;
        right_ss += right_delta * right_delta;
    }
    let denominator = left_ss * right_ss;
    (denominator > 0.0).then_some((pairs.len() as u64, covariance * covariance / denominator))
}

fn alt_dosage(genotype: &[usize]) -> Option<f64> {
    if genotype.iter().any(|allele| *allele > 1) {
        return None;
    }
    Some(genotype.iter().filter(|allele| **allele == 1).count() as f64)
}

impl SampleSelection {
    fn from_files(keep: Option<&Path>, remove: Option<&Path>) -> Result<Self> {
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

fn genotype_missing_allele_slots(gt: &[u8]) -> u64 {
    if gt.is_empty() {
        return 1;
    }

    let mut missing = 0;
    let mut start = 0;
    while start <= gt.len() {
        let end = gt[start..]
            .iter()
            .position(|byte| *byte == b'/' || *byte == b'|')
            .map_or(gt.len(), |offset| start + offset);
        let allele = &gt[start..end];
        if allele.is_empty() || allele == b"." {
            missing += 1;
        }
        if end == gt.len() {
            break;
        }
        start = end + 1;
    }
    missing
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
    format_fixed_trimmed(value, 6)
}

fn format_fixed_trimmed(value: f64, precision: usize) -> String {
    let mut text = format!("{value:.6}");
    if precision != 6 {
        text = format!("{value:.precision$}");
    }
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
