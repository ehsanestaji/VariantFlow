use std::collections::{HashSet, VecDeque};
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

#[derive(Debug, Clone)]
struct LdSite {
    chrom: String,
    pos: u64,
    dosages: PackedDosages,
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

/// Per-population, per-window accumulator for nucleotide diversity (pi).
///
/// `count_diffs` and `count_comparisons` are the summed un-doubled pairwise
/// differences and comparisons across all sites in the window (variant AND
/// invariant). `no_sites` counts sites where the population had at least two
/// non-missing gametes (n >= 2), i.e. sites that could contribute a comparison.
#[derive(Debug, Clone, Default)]
struct PixyPiWindow {
    chrom: String,
    start: u64,
    end: u64,
    no_sites: u64,
    count_diffs: u128,
    count_comparisons: u128,
}

/// Per-population-pair, per-window accumulator for between-population divergence (dxy).
#[derive(Debug, Clone, Default)]
struct PixyDxyWindow {
    chrom: String,
    start: u64,
    end: u64,
    no_sites: u64,
    count_diffs: u128,
    count_comparisons: u128,
}

/// A parsed pixy-style populations file: an ordered, de-duplicated list of
/// population names plus a mapping from each population to its member samples.
#[derive(Debug, Default)]
struct PixyPopulations {
    order: Vec<String>,
    members: std::collections::HashMap<String, HashSet<String>>,
}

/// Pairwise differences and comparisons for pi at a single site within one
/// population, given per-allele counts over the non-missing gametes.
///
/// Let `n = sum(counts)`. Then:
/// - comparisons = n * (n - 1) / 2
/// - differences = (n^2 - sum(counts[a]^2)) / 2
///
/// Both are always exact integers. Returns `(diffs, comparisons)`.
fn pi_site_counts(counts: &[u64]) -> (u128, u128) {
    let n: u128 = counts.iter().map(|c| u128::from(*c)).sum();
    if n < 2 {
        return (0, 0);
    }
    let comparisons = n * (n - 1) / 2;
    let sum_sq: u128 = counts.iter().map(|c| u128::from(*c) * u128::from(*c)).sum();
    let differences = (n * n - sum_sq) / 2;
    (differences, comparisons)
}

/// Pairwise differences and comparisons for dxy at a single site between two
/// populations, given per-allele counts over each population's non-missing
/// gametes.
///
/// Let `nP = sum(a)`, `nQ = sum(b)`. Then:
/// - comparisons = nP * nQ
/// - differences = nP * nQ - sum_a(a[a] * b[a])
///
/// Returns `(diffs, comparisons)`.
fn dxy_site_counts(a: &[u64], b: &[u64]) -> (u128, u128) {
    let n_a: u128 = a.iter().map(|c| u128::from(*c)).sum();
    let n_b: u128 = b.iter().map(|c| u128::from(*c)).sum();
    if n_a == 0 || n_b == 0 {
        return (0, 0);
    }
    let comparisons = n_a * n_b;
    let width = a.len().min(b.len());
    let mut shared = 0_u128;
    for idx in 0..width {
        shared += u128::from(a[idx]) * u128::from(b[idx]);
    }
    let differences = comparisons - shared;
    (differences, comparisons)
}

/// Gather per-allele counts over the non-missing gametes for the samples in a
/// population at a single site. The returned vector is sized to the number of
/// alleles observed at the site (`site.allele_counts.len()`), so allele indices
/// are stable across populations at the same site.
fn population_allele_counts(site: &SiteAlleleSummary, indices: &[usize]) -> Vec<u64> {
    let mut counts = vec![0_u64; site.allele_counts.len()];
    for &index in indices {
        if let Some(Some(alleles)) = site.genotypes.get(index) {
            for &allele in alleles {
                if let Some(slot) = counts.get_mut(allele) {
                    *slot += 1;
                }
            }
        }
    }
    counts
}

/// Compute the window [start, end] that `pos` falls into for a given
/// `window_size`, matching the existing `add_window_pi` binning:
/// `start = ((pos - 1) / window_size) * window_size + 1`.
fn pixy_window_bounds(pos: u64, window_size: u64) -> (u64, u64) {
    let start = ((pos - 1) / window_size) * window_size + 1;
    let end = start + window_size - 1;
    (start, end)
}

/// Parse a pixy-style populations file. Each non-blank, non-comment line has two
/// whitespace/tab separated columns: `sample_id  population_name`. Populations
/// are recorded in first-seen order, de-duplicated.
fn read_pixy_populations(path: &Path) -> Result<PixyPopulations> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read populations file {}", path.display()))?;
    let mut result = PixyPopulations::default();
    for (line_no, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let sample = fields.next().with_context(|| {
            format!(
                "populations file line {} is missing a sample id",
                line_no + 1
            )
        })?;
        let population = fields.next().with_context(|| {
            format!(
                "populations file line {} is missing a population name",
                line_no + 1
            )
        })?;
        if !result.members.contains_key(population) {
            result.order.push(population.to_owned());
        }
        result
            .members
            .entry(population.to_owned())
            .or_default()
            .insert(sample.to_owned());
    }
    Ok(result)
}

/// Compute pixy-equivalent nucleotide diversity (pi) per population and
/// between-population divergence (dxy) per population pair, over fixed-size
/// windows. Handles missing genotypes correctly: at every site only non-missing
/// gametes are counted, and windowed statistics are ratios of summed pairwise
/// counts across all sites present (including invariant sites).
///
/// Rows are ordered by window (in first-seen order) then by population /
/// population-pair order.
pub fn run_pixy(
    input: &Path,
    populations: &Path,
    window_size: u64,
    out_pi: &Path,
    out_dxy: &Path,
) -> Result<()> {
    if window_size == 0 {
        bail!("--window-size must be positive");
    }
    let pops = read_pixy_populations(populations)?;
    if pops.order.is_empty() {
        bail!("populations file must define at least one population");
    }

    // Unordered pairs of distinct populations, in population order.
    let pop_pairs: Vec<(usize, usize)> = {
        let mut pairs = Vec::new();
        for i in 0..pops.order.len() {
            for j in (i + 1)..pops.order.len() {
                pairs.push((i, j));
            }
        }
        pairs
    };

    // Resolved column indices per population, populated on the first site.
    let mut pop_indices: Vec<Vec<usize>> = Vec::new();
    // Windows in first-seen order; one accumulator per population / pair.
    let mut pi_windows: Vec<Vec<PixyPiWindow>> = Vec::new();
    let mut dxy_windows: Vec<Vec<PixyDxyWindow>> = Vec::new();
    // Track the current window per (chrom, start) so all pops share ordering.
    let mut window_keys: Vec<(String, u64)> = Vec::new();

    let selection = SampleSelection::default();
    stream_site_summaries_with_samples(input, &selection, |samples, site| {
        if pop_indices.is_empty() {
            for pop in &pops.order {
                let members = pops
                    .members
                    .get(pop)
                    .expect("population present in order must have members");
                pop_indices.push(population_indices(samples, members)?);
            }
        }

        let (start, end) = pixy_window_bounds(site.pos, window_size);
        // Find or create the window slot for this (chrom, start).
        let key = (site.chrom.clone(), start);
        let window_idx = match window_keys.last() {
            Some(last) if *last == key => window_keys.len() - 1,
            _ => match window_keys.iter().position(|k| *k == key) {
                Some(idx) => idx,
                None => {
                    window_keys.push(key.clone());
                    for (pop_idx, _) in pops.order.iter().enumerate() {
                        let entry = pi_windows.get_mut(pop_idx);
                        let vec = match entry {
                            Some(vec) => vec,
                            None => {
                                pi_windows.push(Vec::new());
                                pi_windows.last_mut().unwrap()
                            }
                        };
                        vec.push(PixyPiWindow {
                            chrom: site.chrom.clone(),
                            start,
                            end,
                            ..Default::default()
                        });
                    }
                    for (pair_idx, _) in pop_pairs.iter().enumerate() {
                        let vec = match dxy_windows.get_mut(pair_idx) {
                            Some(vec) => vec,
                            None => {
                                dxy_windows.push(Vec::new());
                                dxy_windows.last_mut().unwrap()
                            }
                        };
                        vec.push(PixyDxyWindow {
                            chrom: site.chrom.clone(),
                            start,
                            end,
                            ..Default::default()
                        });
                    }
                    window_keys.len() - 1
                }
            },
        };

        // Per-population allele counts at this site (computed once, reused for dxy).
        let per_pop_counts: Vec<Vec<u64>> = pop_indices
            .iter()
            .map(|indices| population_allele_counts(site, indices))
            .collect();

        for (pop_idx, counts) in per_pop_counts.iter().enumerate() {
            let n: u64 = counts.iter().sum();
            let (diffs, comps) = pi_site_counts(counts);
            let window = &mut pi_windows[pop_idx][window_idx];
            if n >= 2 {
                window.no_sites += 1;
            }
            window.count_diffs += diffs;
            window.count_comparisons += comps;
        }

        for (pair_idx, &(a, b)) in pop_pairs.iter().enumerate() {
            let counts_a = &per_pop_counts[a];
            let counts_b = &per_pop_counts[b];
            let n_a: u64 = counts_a.iter().sum();
            let n_b: u64 = counts_b.iter().sum();
            let (diffs, comps) = dxy_site_counts(counts_a, counts_b);
            let window = &mut dxy_windows[pair_idx][window_idx];
            if n_a >= 1 && n_b >= 1 {
                window.no_sites += 1;
            }
            window.count_diffs += diffs;
            window.count_comparisons += comps;
        }

        Ok(())
    })?;

    write_pixy_pi(out_pi, &pops.order, &pi_windows)?;
    write_pixy_dxy(out_dxy, &pops.order, &pop_pairs, &dxy_windows)?;
    Ok(())
}

fn write_pixy_pi(out_pi: &Path, order: &[String], pi_windows: &[Vec<PixyPiWindow>]) -> Result<()> {
    let mut writer = BufWriter::new(
        File::create(out_pi).with_context(|| format!("failed to create {}", out_pi.display()))?,
    );
    writeln!(
        writer,
        "pop\tchromosome\twindow_pos_1\twindow_pos_2\tavg_pi\tno_sites\tcount_diffs\tcount_comparisons"
    )?;
    let window_count = pi_windows.first().map_or(0, Vec::len);
    // `w` indexes the per-population parallel window vectors; range loop is intentional.
    #[allow(clippy::needless_range_loop)]
    for w in 0..window_count {
        for (pop_idx, pop) in order.iter().enumerate() {
            let window = &pi_windows[pop_idx][w];
            writeln!(
                writer,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                pop,
                window.chrom,
                window.start,
                window.end,
                format_pixy_ratio(window.count_diffs, window.count_comparisons),
                window.no_sites,
                window.count_diffs,
                window.count_comparisons,
            )?;
        }
    }
    writer.flush()?;
    Ok(())
}

fn write_pixy_dxy(
    out_dxy: &Path,
    order: &[String],
    pop_pairs: &[(usize, usize)],
    dxy_windows: &[Vec<PixyDxyWindow>],
) -> Result<()> {
    let mut writer = BufWriter::new(
        File::create(out_dxy).with_context(|| format!("failed to create {}", out_dxy.display()))?,
    );
    writeln!(
        writer,
        "pop1\tpop2\tchromosome\twindow_pos_1\twindow_pos_2\tavg_dxy\tno_sites\tcount_diffs\tcount_comparisons"
    )?;
    let window_count = dxy_windows.first().map_or(0, Vec::len);
    // `w` indexes the per-pair parallel window vectors; range loop is intentional.
    #[allow(clippy::needless_range_loop)]
    for w in 0..window_count {
        for (pair_idx, &(a, b)) in pop_pairs.iter().enumerate() {
            let window = &dxy_windows[pair_idx][w];
            writeln!(
                writer,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                order[a],
                order[b],
                window.chrom,
                window.start,
                window.end,
                format_pixy_ratio(window.count_diffs, window.count_comparisons),
                window.no_sites,
                window.count_diffs,
                window.count_comparisons,
            )?;
        }
    }
    writer.flush()?;
    Ok(())
}

/// Format an average pi/dxy value as `count_diffs / count_comparisons`, or `NA`
/// when there are no comparisons (matching pixy's convention for empty windows).
fn format_pixy_ratio(diffs: u128, comparisons: u128) -> String {
    if comparisons == 0 {
        "NA".to_owned()
    } else {
        format!("{:.8}", diffs as f64 / comparisons as f64)
    }
}

pub fn run_ld(
    input: &Path,
    keep: Option<&Path>,
    remove: Option<&Path>,
    max_distance: Option<u64>,
    output: &Path,
) -> Result<()> {
    let selection = SampleSelection::from_files(keep, remove)?;
    let mut reader = open_reader(input)?;
    let mut line = Vec::new();
    let mut samples: Option<Vec<SampleColumn>> = None;
    let mut window: VecDeque<LdSite> = VecDeque::new();
    let mut pending_position = Vec::new();
    let mut writer = BufWriter::new(
        File::create(output).with_context(|| format!("failed to create {}", output.display()))?,
    );
    writeln!(writer, "CHR\tPOS1\tPOS2\tN_INDV\tR^2")?;

    while reader.read_until(b'\n', &mut line)? != 0 {
        if line.starts_with(b"#CHROM") {
            samples = Some(selection.resolve(&line)?);
        } else if !line.starts_with(b"#") {
            let samples = samples
                .as_ref()
                .context("VCF header is missing #CHROM sample line")?;
            let record = RecordView::parse(&line)?;
            let Some(site) = ld_site_from_record(&record, samples)? else {
                line.clear();
                continue;
            };

            if pending_position.first().is_some_and(|pending: &LdSite| {
                pending.chrom != site.chrom || pending.pos != site.pos
            }) {
                flush_ld_position(
                    &mut writer,
                    &mut window,
                    &mut pending_position,
                    max_distance,
                )?;
            }

            pending_position.push(site);
        }
        line.clear();
    }

    if samples.is_none() {
        bail!("VCF header is missing #CHROM sample line");
    }
    flush_ld_position(
        &mut writer,
        &mut window,
        &mut pending_position,
        max_distance,
    )?;
    writer.flush()?;
    Ok(())
}

const MISSING_DOSAGE: u8 = 3;

#[derive(Debug, Clone)]
struct PackedDosages {
    sample_count: usize,
    bytes: Vec<u8>,
}

impl PackedDosages {
    fn new_missing(sample_count: usize) -> Self {
        Self {
            sample_count,
            bytes: vec![0xff; sample_count.div_ceil(4)],
        }
    }

    fn set(&mut self, index: usize, dosage: u8) {
        debug_assert!(index < self.sample_count);
        debug_assert!(dosage <= MISSING_DOSAGE);
        let byte_index = index / 4;
        let shift = (index % 4) * 2;
        let mask = !(0b11_u8 << shift);
        self.bytes[byte_index] = (self.bytes[byte_index] & mask) | (dosage << shift);
    }

    fn get(&self, index: usize) -> u8 {
        debug_assert!(index < self.sample_count);
        let byte_index = index / 4;
        let shift = (index % 4) * 2;
        (self.bytes[byte_index] >> shift) & 0b11
    }

    #[cfg(test)]
    fn byte_len(&self) -> usize {
        self.bytes.len()
    }
}

fn ld_site_from_record(
    record: &RecordView<'_>,
    samples: &[SampleColumn],
) -> Result<Option<LdSite>> {
    if memchr::memchr(b',', record.alternate()).is_some() {
        return Ok(None);
    }

    let gt_index = record
        .column(8)
        .and_then(|format| format_key_index(format, b"GT"));
    let mut dosages = PackedDosages::new_missing(samples.len());

    for_each_selected_sample_value(record, samples, |index, _sample, value| {
        let dosage = gt_index
            .and_then(|index| sample_format_value_at(value, index))
            .and_then(parse_ld_alt_dosage)
            .unwrap_or(MISSING_DOSAGE);
        dosages.set(index, dosage);
    });

    Ok(Some(LdSite {
        chrom: bytes_text(record.chrom())?.to_owned(),
        pos: record.pos_u64()?,
        dosages,
    }))
}

fn flush_ld_position(
    writer: &mut impl Write,
    window: &mut VecDeque<LdSite>,
    pending: &mut Vec<LdSite>,
    max_distance: Option<u64>,
) -> Result<()> {
    let Some(first_pending) = pending.first() else {
        return Ok(());
    };

    if let Some(distance) = max_distance {
        while let Some(left) = window.front() {
            if left.chrom != first_pending.chrom
                || first_pending.pos.saturating_sub(left.pos) > distance
            {
                window.pop_front();
            } else {
                break;
            }
        }
    }

    for left in window.iter() {
        if left.chrom != first_pending.chrom {
            continue;
        }
        if max_distance
            .is_some_and(|distance| first_pending.pos.saturating_sub(left.pos) > distance)
        {
            continue;
        }
        for right in pending.iter() {
            write_ld_pair(writer, left, right)?;
        }
    }

    for left_index in 0..pending.len() {
        for right_index in left_index + 1..pending.len() {
            write_ld_pair(writer, &pending[left_index], &pending[right_index])?;
        }
    }

    window.extend(pending.drain(..));
    Ok(())
}

fn write_ld_pair(writer: &mut impl Write, left: &LdSite, right: &LdSite) -> Result<()> {
    if let Some((n, r2)) = genotype_dosage_r2(&left.dosages, &right.dosages) {
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

fn genotype_dosage_r2(left: &PackedDosages, right: &PackedDosages) -> Option<(u64, f64)> {
    let mut n = 0_u64;
    let mut sum_left = 0.0;
    let mut sum_right = 0.0;
    let mut sum_left_sq = 0.0;
    let mut sum_right_sq = 0.0;
    let mut sum_product = 0.0;

    for index in 0..left.sample_count.min(right.sample_count) {
        let left = left.get(index);
        let right = right.get(index);
        if left == MISSING_DOSAGE || right == MISSING_DOSAGE {
            continue;
        }
        let left = f64::from(left);
        let right = f64::from(right);
        n += 1;
        sum_left += left;
        sum_right += right;
        sum_left_sq += left * left;
        sum_right_sq += right * right;
        sum_product += left * right;
    }

    if n < 2 {
        return None;
    }

    let n_f64 = n as f64;
    let covariance = sum_product - (sum_left * sum_right / n_f64);
    let left_ss = sum_left_sq - (sum_left * sum_left / n_f64);
    let right_ss = sum_right_sq - (sum_right * sum_right / n_f64);
    let denominator = left_ss * right_ss;
    (denominator > 0.0).then_some((n, covariance * covariance / denominator))
}

fn parse_ld_alt_dosage(gt: &[u8]) -> Option<u8> {
    if genotype_is_missing(gt) {
        return None;
    }

    let mut observed = false;
    let mut invalid = false;
    let mut dosage = 0_u8;
    for_each_called_allele(gt, |allele| {
        observed = true;
        match allele {
            0 => {}
            1 => dosage = dosage.saturating_add(1),
            _ => invalid = true,
        }
    });

    (observed && !invalid).then_some(dosage)
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
    use super::{
        MISSING_DOSAGE, PackedDosages, dxy_site_counts, for_each_called_allele, format_ratio,
        genotype_dosage_r2, genotype_is_missing, pi_site_counts, run_pixy,
    };
    use std::io::Write;

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

    #[test]
    fn packed_dosages_store_four_samples_per_byte_with_missing_default() {
        let mut dosages = PackedDosages::new_missing(5);

        assert_eq!(dosages.byte_len(), 2);
        for index in 0..5 {
            assert_eq!(dosages.get(index), MISSING_DOSAGE);
        }

        dosages.set(0, 0);
        dosages.set(1, 1);
        dosages.set(2, 2);
        dosages.set(3, MISSING_DOSAGE);
        dosages.set(4, 1);

        assert_eq!(dosages.get(0), 0);
        assert_eq!(dosages.get(1), 1);
        assert_eq!(dosages.get(2), 2);
        assert_eq!(dosages.get(3), MISSING_DOSAGE);
        assert_eq!(dosages.get(4), 1);
    }

    #[test]
    fn packed_dosage_r2_ignores_missing_pairs() {
        let mut left = PackedDosages::new_missing(4);
        let mut right = PackedDosages::new_missing(4);

        for (index, dosage) in [0, 1, 2, MISSING_DOSAGE].into_iter().enumerate() {
            left.set(index, dosage);
        }
        for (index, dosage) in [0, 1, 1, 2].into_iter().enumerate() {
            right.set(index, dosage);
        }

        let (n, r2) = genotype_dosage_r2(&left, &right).unwrap();
        assert_eq!(n, 3);
        assert!((r2 - 0.75).abs() < 1e-12);
    }

    #[test]
    fn pi_site_counts_matches_hand_computed_values() {
        // counts=[2,2], n=4 -> comps=6, diffs=4
        assert_eq!(pi_site_counts(&[2, 2]), (4, 6));
        // counts=[4,0] (invariant), n=4 -> comps=6, diffs=0
        assert_eq!(pi_site_counts(&[4, 0]), (0, 6));
        // counts=[3,1], n=4 -> comps=6, diffs=3
        assert_eq!(pi_site_counts(&[3, 1]), (3, 6));
        // n < 2 yields no comparisons.
        assert_eq!(pi_site_counts(&[1, 0]), (0, 0));
        assert_eq!(pi_site_counts(&[0, 0]), (0, 0));
        // multiallelic: counts=[2,1,1], n=4 -> comps=6, sum_sq=6, diffs=(16-6)/2=5
        assert_eq!(pi_site_counts(&[2, 1, 1]), (5, 6));
    }

    #[test]
    fn dxy_site_counts_matches_hand_computed_values() {
        // P=[2,0], Q=[0,2] -> comps=4, diffs=4
        assert_eq!(dxy_site_counts(&[2, 0], &[0, 2]), (4, 4));
        // P=[2,0], Q=[2,0] -> comps=4, diffs=0
        assert_eq!(dxy_site_counts(&[2, 0], &[2, 0]), (0, 4));
        // P=[1,1], Q=[2,0] -> comps=4, shared=1*2=2, diffs=2
        assert_eq!(dxy_site_counts(&[1, 1], &[2, 0]), (2, 4));
        // empty on one side yields no comparisons.
        assert_eq!(dxy_site_counts(&[0, 0], &[2, 0]), (0, 0));
    }

    #[test]
    fn run_pixy_handles_invariant_and_missing_sites_in_one_window() {
        let dir = tempfile::tempdir().unwrap();
        let vcf = dir.path().join("allsites.vcf");
        let pops = dir.path().join("pops.txt");
        let out_pi = dir.path().join("pi.txt");
        let out_dxy = dir.path().join("dxy.txt");

        // 4 samples, 2 populations (P1: S1,S2; P2: S3,S4). Single window (size 1000).
        //
        // Site 1 (pos 100), variant:   S1=0/1 S2=0/0 S3=1/1 S4=0/1
        // Site 2 (pos 200), invariant: all 0/0 (ALT ".")
        // Site 3 (pos 300), missing:   S1=0/1 S2=./. S3=1/1 S4=0/0
        let mut f = std::fs::File::create(&vcf).unwrap();
        writeln!(f, "##fileformat=VCFv4.3").unwrap();
        writeln!(
            f,
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1\tS2\tS3\tS4"
        )
        .unwrap();
        writeln!(f, "1\t100\t.\tA\tG\t.\t.\t.\tGT\t0/1\t0/0\t1/1\t0/1").unwrap();
        writeln!(f, "1\t200\t.\tA\t.\t.\t.\t.\tGT\t0/0\t0/0\t0/0\t0/0").unwrap();
        writeln!(f, "1\t300\t.\tA\tG\t.\t.\t.\tGT\t0/1\t./.\t1/1\t0/0").unwrap();
        f.flush().unwrap();
        drop(f);

        let mut p = std::fs::File::create(&pops).unwrap();
        writeln!(p, "S1\tP1").unwrap();
        writeln!(p, "S2\tP1").unwrap();
        writeln!(p, "S3\tP2").unwrap();
        writeln!(p, "S4\tP2").unwrap();
        p.flush().unwrap();
        drop(p);

        run_pixy(&vcf, &pops, 1000, &out_pi, &out_dxy).unwrap();

        let pi_text = std::fs::read_to_string(&out_pi).unwrap();
        let dxy_text = std::fs::read_to_string(&out_dxy).unwrap();

        // --- Hand-computed pi ---
        //
        // P1 (S1,S2) allele counts per site (non-missing gametes):
        //  site1: S1=0/1, S2=0/0 -> counts=[3,1], n=4 -> diffs=3, comps=6, n>=2
        //  site2: all 0/0        -> counts=[4],   n=4 -> diffs=0, comps=6, n>=2
        //  site3: S1=0/1, S2=./. -> counts=[1,1], n=2 -> diffs=1, comps=1, n>=2
        //  total diffs=4, comps=13, no_sites=3, avg_pi = 4/13 = 0.30769231
        //
        // P2 (S3,S4):
        //  site1: S3=1/1, S4=0/1 -> counts=[1,3], n=4 -> diffs=3, comps=6, n>=2
        //  site2: all 0/0        -> counts=[4],   n=4 -> diffs=0, comps=6, n>=2
        //  site3: S3=1/1, S4=0/0 -> counts=[2,2], n=4 -> diffs=4, comps=6, n>=2
        //  total diffs=7, comps=18, no_sites=3, avg_pi = 7/18 = 0.38888889
        let p1_row = pi_text
            .lines()
            .find(|l| l.starts_with("P1\t"))
            .expect("P1 pi row present");
        assert_eq!(
            p1_row, "P1\t1\t1\t1000\t0.30769231\t3\t4\t13",
            "P1 pi row mismatch"
        );
        let p2_row = pi_text
            .lines()
            .find(|l| l.starts_with("P2\t"))
            .expect("P2 pi row present");
        assert_eq!(
            p2_row, "P2\t1\t1\t1000\t0.38888889\t3\t7\t18",
            "P2 pi row mismatch"
        );

        // --- Hand-computed dxy (P1 vs P2) ---
        //
        //  site1: P1=[3,1], P2=[1,3] -> comps=16, shared=3*1+1*3=6, diffs=10, both>=1
        //  site2: P1=[4],   P2=[4]   -> comps=16, shared=16,        diffs=0,  both>=1
        //  site3: P1=[1,1], P2=[2,2] -> comps=8,  shared=1*2+1*2=4, diffs=4,  both>=1
        //  total diffs=14, comps=40, no_sites=3, avg_dxy = 14/40 = 0.35000000
        let dxy_row = dxy_text
            .lines()
            .find(|l| l.starts_with("P1\tP2\t"))
            .expect("P1/P2 dxy row present");
        assert_eq!(
            dxy_row, "P1\tP2\t1\t1\t1000\t0.35000000\t3\t14\t40",
            "dxy row mismatch"
        );
    }

    #[test]
    fn run_pixy_reports_na_for_empty_windows() {
        let dir = tempfile::tempdir().unwrap();
        let vcf = dir.path().join("empty.vcf");
        let pops = dir.path().join("pops.txt");
        let out_pi = dir.path().join("pi.txt");
        let out_dxy = dir.path().join("dxy.txt");

        // Single site fully missing for P1 -> no comparisons -> NA avg_pi.
        let mut f = std::fs::File::create(&vcf).unwrap();
        writeln!(f, "##fileformat=VCFv4.3").unwrap();
        writeln!(
            f,
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1\tS2\tS3\tS4"
        )
        .unwrap();
        writeln!(f, "1\t100\t.\tA\tG\t.\t.\t.\tGT\t./.\t./.\t0/1\t0/1").unwrap();
        f.flush().unwrap();
        drop(f);

        let mut p = std::fs::File::create(&pops).unwrap();
        writeln!(p, "S1\tP1").unwrap();
        writeln!(p, "S2\tP1").unwrap();
        writeln!(p, "S3\tP2").unwrap();
        writeln!(p, "S4\tP2").unwrap();
        p.flush().unwrap();
        drop(p);

        run_pixy(&vcf, &pops, 1000, &out_pi, &out_dxy).unwrap();

        let pi_text = std::fs::read_to_string(&out_pi).unwrap();
        let p1_row = pi_text
            .lines()
            .find(|l| l.starts_with("P1\t"))
            .expect("P1 pi row present");
        assert_eq!(p1_row, "P1\t1\t1\t1000\tNA\t0\t0\t0", "empty P1 pi row");
    }
}
