use std::collections::BTreeMap;
use std::io::BufRead;
use std::path::Path;

use anyhow::Result;
use memchr::memchr;
use serde::Serialize;

use crate::compat::{Backend, Region, select_backend};
use crate::io::open_reader;
use crate::vcf::{InfoView, RecordView};

#[derive(Debug, Default, Serialize)]
pub struct StatsSummary {
    pub(crate) variants: u64,
    pub(crate) snps: u64,
    pub(crate) indels: u64,
    pub(crate) variants_per_chromosome: BTreeMap<String, u64>,
    pub(crate) filter_counts: BTreeMap<String, u64>,
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
    let mut line = Vec::new();
    let mut summary = StatsSummary::default();
    let mut titv = TiTv::default();
    let mut chrom_cache = ChromCache::default();

    while reader.read_until(b'\n', &mut line)? != 0 {
        if !line.starts_with(b"#") {
            let record = RecordView::parse(&line)?;
            summary.observe_view(&record, &mut titv, &mut chrom_cache)?;
        }
        line.clear();
    }

    summary.qual.finish();
    summary.af.finish();
    summary.transition_transversion_ratio = titv.ratio();
    Ok(summary)
}

impl StatsSummary {
    pub(crate) fn observe_view(
        &mut self,
        record: &RecordView<'_>,
        titv: &mut TiTv,
        chrom_cache: &mut ChromCache,
    ) -> Result<()> {
        self.variants += 1;
        self.observe_chromosome(record.chrom(), chrom_cache)?;

        if record.filter() == b"." {
            self.missing_filter_values += 1;
        }
        self.observe_filter(record.filter())?;

        if let Some(qual) = record.qual_float()? {
            self.qual.observe(qual);
        }

        InfoView::scan(record.info()).for_each_number(b"AF", |af| self.af.observe(af));

        for_each_alt_allele(record.alternate(), |alt| {
            if is_snp_bytes(record.reference(), alt) {
                self.snps += 1;
                titv.observe_bytes(record.reference(), alt);
            } else {
                self.indels += 1;
            }
        });

        Ok(())
    }

    fn observe_chromosome(&mut self, chrom: &[u8], cache: &mut ChromCache) -> Result<()> {
        let key = cache.key_for(chrom)?;
        if let Some(count) = self.variants_per_chromosome.get_mut(key) {
            *count += 1;
        } else {
            self.variants_per_chromosome.insert(key.to_owned(), 1);
        }
        Ok(())
    }

    pub(crate) fn observe_filter(&mut self, filter: &[u8]) -> Result<()> {
        let key = std::str::from_utf8(filter)?;
        *self.filter_counts.entry(key.to_owned()).or_default() += 1;
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
    #[cfg(feature = "htslib")]
    pub(crate) fn observe(&mut self, reference: &str, alternate: &str) {
        if is_transition(reference, alternate) {
            self.transitions += 1;
        } else {
            self.transversions += 1;
        }
    }

    pub(crate) fn observe_bytes(&mut self, reference: &[u8], alternate: &[u8]) {
        if is_transition_bytes(reference, alternate) {
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

#[derive(Debug, Default)]
pub(crate) struct ChromCache {
    raw: Vec<u8>,
    key: String,
}

impl ChromCache {
    fn key_for(&mut self, chrom: &[u8]) -> Result<&str> {
        if self.raw != chrom {
            self.raw.clear();
            self.raw.extend_from_slice(chrom);
            self.key = std::str::from_utf8(chrom)?.to_owned();
        }
        Ok(&self.key)
    }
}

fn for_each_alt_allele<'a>(alternate: &'a [u8], mut observe: impl FnMut(&'a [u8])) {
    let mut start = 0;

    while start <= alternate.len() {
        let end =
            memchr(b',', &alternate[start..]).map_or(alternate.len(), |offset| start + offset);
        observe(&alternate[start..end]);

        if end == alternate.len() {
            break;
        }
        start = end + 1;
    }
}

fn is_snp_bytes(reference: &[u8], alternate: &[u8]) -> bool {
    reference.len() == 1 && alternate.len() == 1
}

#[cfg(feature = "htslib")]
fn is_transition(reference: &str, alternate: &str) -> bool {
    matches!(
        (reference, alternate),
        ("A", "G") | ("G", "A") | ("C", "T") | ("T", "C")
    )
}

fn is_transition_bytes(reference: &[u8], alternate: &[u8]) -> bool {
    matches!(
        (reference, alternate),
        (b"A", b"G") | (b"G", b"A") | (b"C", b"T") | (b"T", b"C")
    )
}
