use std::collections::BTreeMap;
use std::io::BufRead;
use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::compat::{Backend, Region, select_backend};
use crate::io::open_reader;
use crate::vcf::{RecordFields, for_each_info_number, parse_record_fields};

#[derive(Debug, Default, Serialize)]
pub struct StatsSummary {
    pub(crate) variants: u64,
    pub(crate) snps: u64,
    pub(crate) indels: u64,
    pub(crate) variants_per_chromosome: BTreeMap<String, u64>,
    pub(crate) qual: NumericSummary,
    pub(crate) af: NumericSummary,
    pub(crate) missing_filter_values: u64,
    pub(crate) transition_transversion_ratio: Option<f64>,
}

#[derive(Debug, Default, Serialize)]
pub struct NumericSummary {
    pub(crate) count: u64,
    pub(crate) min: Option<f64>,
    pub(crate) max: Option<f64>,
    pub(crate) mean: Option<f64>,
    #[serde(skip)]
    pub(crate) sum: f64,
}

#[derive(Debug, Default)]
pub(crate) struct TiTv {
    transitions: u64,
    transversions: u64,
}

pub fn run(input: &Path, region: Option<&Region>) -> Result<()> {
    let summary = collect(input, region)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

pub fn collect(input: &Path, region: Option<&Region>) -> Result<StatsSummary> {
    let selected = select_backend(input, region, Default::default());
    if selected.backend == Backend::Htslib {
        #[cfg(feature = "htslib")]
        {
            return crate::htslib_backend::stats(input, region);
        }

        #[cfg(not(feature = "htslib"))]
        {
            anyhow::bail!(selected.reason.unwrap().unavailable_message());
        }
    }

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
    pub(crate) fn observe(&mut self, record: &RecordFields<'_>, titv: &mut TiTv) -> Result<()> {
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
    pub(crate) fn observe(&mut self, value: f64) {
        self.count += 1;
        self.min = Some(self.min.map_or(value, |current| current.min(value)));
        self.max = Some(self.max.map_or(value, |current| current.max(value)));
        self.sum += value;
    }

    pub(crate) fn finish(&mut self) {
        self.mean = if self.count == 0 {
            None
        } else {
            Some(self.sum / self.count as f64)
        };
    }
}

impl TiTv {
    pub(crate) fn observe(&mut self, reference: &str, alternate: &str) {
        if is_transition(reference, alternate) {
            self.transitions += 1;
        } else {
            self.transversions += 1;
        }
    }

    pub(crate) fn ratio(&self) -> Option<f64> {
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
