use std::collections::BTreeMap;
use std::io::BufRead;
use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::io::open_reader;
use crate::vcf::{RecordFields, for_each_info_number, parse_record_fields};

#[derive(Debug, Default, Serialize)]
pub struct StatsSummary {
    variants: u64,
    snps: u64,
    indels: u64,
    variants_per_chromosome: BTreeMap<String, u64>,
    qual: NumericSummary,
    af: NumericSummary,
    missing_filter_values: u64,
    transition_transversion_ratio: Option<f64>,
}

#[derive(Debug, Default, Serialize)]
pub struct NumericSummary {
    count: u64,
    min: Option<f64>,
    max: Option<f64>,
    mean: Option<f64>,
    #[serde(skip)]
    sum: f64,
}

#[derive(Debug, Default)]
struct TiTv {
    transitions: u64,
    transversions: u64,
}

pub fn run(input: &Path) -> Result<()> {
    let summary = collect(input)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

pub fn collect(input: &Path) -> Result<StatsSummary> {
    let mut reader = open_reader(input)?;
    let mut line = String::new();
    let mut summary = StatsSummary::default();
    let mut titv = TiTv::default();

    while reader.read_line(&mut line)? != 0 {
        if !line.starts_with('#') {
            let fields = parse_record_fields(&line)?;
            summary.observe(&fields, &mut titv)?;
        }
        line.clear();
    }

    summary.qual.finish();
    summary.af.finish();
    summary.transition_transversion_ratio = titv.ratio();
    Ok(summary)
}

impl StatsSummary {
    fn observe(&mut self, record: &RecordFields<'_>, titv: &mut TiTv) -> Result<()> {
        self.variants += 1;
        *self
            .variants_per_chromosome
            .entry(record.chrom.to_string())
            .or_default() += 1;

        if record.filter == "." {
            self.missing_filter_values += 1;
        }

        if let Some(qual) = record.qual_float()? {
            self.qual.observe(qual);
        }

        for_each_info_number(record.info, "AF", |af| self.af.observe(af));

        for alt in record.alt_alleles() {
            if is_snp(record.reference, alt) {
                self.snps += 1;
                titv.observe(record.reference, alt);
            } else {
                self.indels += 1;
            }
        }

        Ok(())
    }
}

impl NumericSummary {
    fn observe(&mut self, value: f64) {
        self.count += 1;
        self.min = Some(self.min.map_or(value, |current| current.min(value)));
        self.max = Some(self.max.map_or(value, |current| current.max(value)));
        self.sum += value;
    }

    fn finish(&mut self) {
        self.mean = if self.count == 0 {
            None
        } else {
            Some(self.sum / self.count as f64)
        };
    }
}

impl TiTv {
    fn observe(&mut self, reference: &str, alternate: &str) {
        if is_transition(reference, alternate) {
            self.transitions += 1;
        } else {
            self.transversions += 1;
        }
    }

    fn ratio(&self) -> Option<f64> {
        if self.transversions == 0 {
            None
        } else {
            Some(self.transitions as f64 / self.transversions as f64)
        }
    }
}

fn is_snp(reference: &str, alternate: &str) -> bool {
    reference.len() == 1 && alternate.len() == 1
}

fn is_transition(reference: &str, alternate: &str) -> bool {
    matches!(
        (reference, alternate),
        ("A", "G") | ("G", "A") | ("C", "T") | ("T", "C")
    )
}
