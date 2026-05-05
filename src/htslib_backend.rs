use std::io::Write;
use std::path::Path;
use std::str;

use anyhow::{Context, Result, bail};
use rust_htslib::bcf::header::Header;
use rust_htslib::bcf::record::{Numeric, Record};
use rust_htslib::bcf::{Format, IndexedReader, Read, Reader, Writer};
use rust_htslib::errors::Error as HtslibError;

use crate::compat::{CompressionMode, Region, htslib_threads_from_env};
use crate::engine::stats::{StatsSummary, TiTv};
use crate::expr::{EvalRecord, FormatValues, RequiredFields, parse_expression};
use crate::io::open_writer;
use crate::vcf::{RecordFields, info_value};

pub fn filter(
    input: &Path,
    where_expr: &str,
    sample: Option<&str>,
    output: &Path,
    region: Option<&Region>,
    compression: CompressionMode,
) -> Result<()> {
    let expr = parse_expression(where_expr)?;
    let required = expr.required_fields();
    if required.requires_format() && sample.is_none() {
        bail!("FORMAT predicates require --sample <name>");
    }

    if let Some(region) = region {
        let mut reader = indexed_reader(input, region)?;
        apply_reader_threads(&mut reader)?;
        let header = Header::from_template(reader.header());
        let sample_id = sample_id(reader.header(), sample, required)?;
        let mut writer = vcf_writer(output, &header, compression)?;
        apply_writer_threads(&mut writer)?;
        for_each_record(&mut reader, |record| {
            if evaluate_record(record, required, sample_id, &expr)? {
                writer.write(record)?;
            }
            Ok(())
        })?;
    } else {
        let mut reader = Reader::from_path(input)
            .with_context(|| format!("failed to open input {}", input.display()))?;
        apply_reader_threads(&mut reader)?;
        let header = Header::from_template(reader.header());
        let sample_id = sample_id(reader.header(), sample, required)?;
        let mut writer = vcf_writer(output, &header, compression)?;
        apply_writer_threads(&mut writer)?;
        for_each_record(&mut reader, |record| {
            if evaluate_record(record, required, sample_id, &expr)? {
                writer.write(record)?;
            }
            Ok(())
        })?;
    }

    Ok(())
}

pub fn convert_to_tsv(input: &Path, output: &Path, region: Option<&Region>) -> Result<()> {
    let mut writer = open_writer(output)?;
    writer.write_all(b"CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO/DP\tINFO/AF\n")?;

    if let Some(region) = region {
        let mut reader = indexed_reader(input, region)?;
        apply_reader_threads(&mut reader)?;
        for_each_record(&mut reader, |record| write_tsv_record(record, &mut writer))?;
    } else {
        let mut reader = Reader::from_path(input)
            .with_context(|| format!("failed to open input {}", input.display()))?;
        apply_reader_threads(&mut reader)?;
        for_each_record(&mut reader, |record| write_tsv_record(record, &mut writer))?;
    }

    writer.flush()?;
    Ok(())
}

pub fn stats(input: &Path, region: Option<&Region>) -> Result<StatsSummary> {
    let mut summary = StatsSummary::default();
    let mut titv = TiTv::default();

    if let Some(region) = region {
        let mut reader = indexed_reader(input, region)?;
        apply_reader_threads(&mut reader)?;
        for_each_record(&mut reader, |record| {
            let fields = stats_record_fields(record)?;
            summary.observe(&fields.as_fields(), &mut titv)?;
            Ok(())
        })?;
    } else {
        let mut reader = Reader::from_path(input)
            .with_context(|| format!("failed to open input {}", input.display()))?;
        apply_reader_threads(&mut reader)?;
        for_each_record(&mut reader, |record| {
            let fields = stats_record_fields(record)?;
            summary.observe(&fields.as_fields(), &mut titv)?;
            Ok(())
        })?;
    }

    summary.qual.finish();
    summary.af.finish();
    summary.transition_transversion_ratio = titv.ratio();
    Ok(summary)
}

fn indexed_reader(input: &Path, region: &Region) -> Result<IndexedReader> {
    let mut reader = IndexedReader::from_path(input)
        .with_context(|| format!("failed to open indexed input {}", input.display()))?;
    let rid = reader
        .header()
        .name2rid(region.contig.as_bytes())
        .with_context(|| format!("region contig '{}' not found in header", region.contig))?;
    let (start, end) = region.htslib_interval();
    reader.fetch(rid, start, end)?;
    Ok(reader)
}

fn vcf_writer(output: &Path, header: &Header, compression: CompressionMode) -> Result<Writer> {
    let uncompressed = match compression {
        CompressionMode::Plain => true,
        CompressionMode::Gzip | CompressionMode::Bgzf => false,
        CompressionMode::Auto => output.extension().is_none_or(|extension| extension != "gz"),
    };
    Writer::from_path(output, header, uncompressed, Format::Vcf)
        .with_context(|| format!("failed to create output {}", output.display()))
}

fn apply_reader_threads<R: Read>(reader: &mut R) -> Result<()> {
    if let Some(threads) = htslib_threads_from_env()? {
        reader.set_threads(threads)?;
    }
    Ok(())
}

fn apply_writer_threads(writer: &mut Writer) -> Result<()> {
    if let Some(threads) = htslib_threads_from_env()? {
        writer.set_threads(threads)?;
    }
    Ok(())
}

fn for_each_record<R: Read>(
    reader: &mut R,
    mut observe: impl FnMut(&Record) -> Result<()>,
) -> Result<()> {
    let mut record = reader.empty_record();
    while let Some(result) = reader.read(&mut record) {
        result?;
        observe(&record)?;
    }
    Ok(())
}

fn sample_id(
    header: &rust_htslib::bcf::header::HeaderView,
    sample: Option<&str>,
    required: RequiredFields,
) -> Result<Option<usize>> {
    if !required.requires_format() {
        return Ok(None);
    }

    let sample = sample.expect("sample presence checked by caller");
    header
        .sample_id(sample.as_bytes())
        .map(Some)
        .ok_or_else(|| anyhow::anyhow!("sample '{sample}' not found in VCF header"))
}

fn evaluate_record(
    record: &Record,
    required: RequiredFields,
    sample_id: Option<usize>,
    expr: &crate::expr::Expression,
) -> Result<bool> {
    let chrom = if required.chrom {
        chrom(record)?
    } else {
        String::new()
    };
    let filter = if required.filter {
        filter_string(record)?
    } else {
        String::new()
    };
    let info = if required.info {
        info_string(record)?
    } else {
        String::new()
    };
    let gt = if required.format.gt {
        sample_id
            .and_then(|index| genotype_string(record, index).transpose())
            .transpose()?
    } else {
        None
    };
    let dp = if required.format.dp {
        sample_id
            .and_then(|index| format_integer_string(record, b"DP", index).transpose())
            .transpose()?
    } else {
        None
    };
    let gq = if required.format.gq {
        sample_id
            .and_then(|index| format_integer_string(record, b"GQ", index).transpose())
            .transpose()?
    } else {
        None
    };

    let eval = EvalRecord {
        chrom: &chrom,
        pos: if required.pos {
            (record.pos() + 1) as u64
        } else {
            0
        },
        qual: if required.qual {
            qual(record).map(f64::from)
        } else {
            None
        },
        filter: &filter,
        info: &info,
        format: FormatValues {
            gt: gt.as_deref(),
            dp: dp.as_deref(),
            gq: gq.as_deref(),
        },
    };

    Ok(expr.evaluate(&eval))
}

fn write_tsv_record(record: &Record, writer: &mut dyn Write) -> Result<()> {
    let fields = owned_record_fields(record)?;
    writeln!(
        writer,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        fields.chrom,
        fields.pos,
        fields.id,
        fields.reference,
        fields.alternate,
        fields.qual,
        fields.filter,
        fields.dp.unwrap_or_else(|| ".".to_string()),
        fields.af.unwrap_or_else(|| ".".to_string())
    )?;
    Ok(())
}

struct OwnedRecordFields {
    chrom: String,
    pos: String,
    id: String,
    reference: String,
    alternate: String,
    qual: String,
    filter: String,
    dp: Option<String>,
    af: Option<String>,
}

struct HtslibStatsFields {
    chrom: String,
    pos: String,
    reference: String,
    alternate: String,
    qual: String,
    filter: String,
    info: String,
}

impl HtslibStatsFields {
    fn as_fields(&self) -> RecordFields<'_> {
        RecordFields {
            chrom: &self.chrom,
            pos: &self.pos,
            id: ".",
            reference: &self.reference,
            alternate: &self.alternate,
            qual: &self.qual,
            filter: &self.filter,
            info: &self.info,
        }
    }
}

fn stats_record_fields(record: &Record) -> Result<HtslibStatsFields> {
    Ok(HtslibStatsFields {
        chrom: chrom(record)?,
        pos: (record.pos() + 1).to_string(),
        reference: allele_string(record, 0)?,
        alternate: alternate_string(record)?,
        qual: qual(record)
            .map(|value| format_float(value as f64))
            .unwrap_or_else(|| ".".to_string()),
        filter: stats_filter_string(record),
        info: info_string(record)?,
    })
}

fn owned_record_fields(record: &Record) -> Result<OwnedRecordFields> {
    let info = record_vcf_column(record, 7)?;
    let dp = raw_info_value(&info, "DP");
    let af = raw_info_value(&info, "AF");

    Ok(OwnedRecordFields {
        chrom: chrom(record)?,
        pos: (record.pos() + 1).to_string(),
        id: bytes_to_string(&record.id()),
        reference: allele_string(record, 0)?,
        alternate: alternate_string(record)?,
        qual: qual(record)
            .map(|value| format_float(value as f64))
            .unwrap_or_else(|| ".".to_string()),
        filter: filter_string(record)?,
        dp,
        af,
    })
}

fn raw_info_value(info: &str, key: &str) -> Option<String> {
    info_value(info, key)
        .filter(|value| !value.is_empty() && *value != ".")
        .map(ToOwned::to_owned)
}

fn chrom(record: &Record) -> Result<String> {
    let rid = record
        .rid()
        .ok_or_else(|| anyhow::anyhow!("record is missing RID"))?;
    Ok(bytes_to_string(record.header().rid2name(rid)?))
}

fn allele_string(record: &Record, index: usize) -> Result<String> {
    record
        .alleles()
        .get(index)
        .map(|allele| bytes_to_string(allele))
        .ok_or_else(|| anyhow::anyhow!("record is missing allele {index}"))
}

fn alternate_string(record: &Record) -> Result<String> {
    let alleles = record.alleles();
    if alleles.len() <= 1 {
        return Ok(".".to_string());
    }
    Ok(alleles[1..]
        .iter()
        .map(|allele| bytes_to_string(allele))
        .collect::<Vec<_>>()
        .join(","))
}

fn qual(record: &Record) -> Option<f32> {
    let value = record.qual();
    if value.is_missing() || value.is_nan() {
        None
    } else {
        Some(value)
    }
}

fn filter_string(record: &Record) -> Result<String> {
    record_vcf_column(record, 6)
}

fn stats_filter_string(record: &Record) -> String {
    if record.inner().d.n_flt == 0 {
        return ".".to_string();
    }

    fallback_filter_string(record)
}

fn fallback_filter_string(record: &Record) -> String {
    let mut filters = Vec::new();
    for id in record.filters() {
        filters.push(bytes_to_string(&record.header().id_to_name(id)));
    }
    if filters.is_empty() {
        "PASS".to_string()
    } else {
        filters.join(";")
    }
}

fn info_string(record: &Record) -> Result<String> {
    let dp = info_numeric_string(record, b"DP")?;
    let af = info_numeric_string(record, b"AF")?;
    let mut parts = Vec::new();
    if let Some(dp) = dp {
        parts.push(format!("DP={dp}"));
    }
    if let Some(af) = af {
        parts.push(format!("AF={af}"));
    }
    Ok(parts.join(";"))
}

fn info_numeric_string(record: &Record, tag: &[u8]) -> Result<Option<String>> {
    match record.info(tag).integer() {
        Ok(Some(values)) => {
            let parts = values
                .iter()
                .filter(|value| !value.is_missing())
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            return Ok(non_empty_join(parts));
        }
        Ok(None)
        | Err(HtslibError::BcfUnexpectedType { .. })
        | Err(HtslibError::BcfUndefinedTag { .. }) => {}
        Err(error) => return Err(error.into()),
    }

    match record.info(tag).float() {
        Ok(Some(values)) => {
            let parts = values
                .iter()
                .filter(|value| !value.is_missing() && !value.is_nan())
                .map(|value| format_float(f64::from(*value)))
                .collect::<Vec<_>>();
            Ok(non_empty_join(parts))
        }
        Ok(None)
        | Err(HtslibError::BcfUnexpectedType { .. })
        | Err(HtslibError::BcfUndefinedTag { .. }) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn format_integer_string(
    record: &Record,
    tag: &[u8],
    sample_index: usize,
) -> Result<Option<String>> {
    let values = record.format(tag).integer()?;
    Ok(values.get(sample_index).and_then(|sample_values| {
        sample_values
            .first()
            .filter(|value| !value.is_missing())
            .map(ToString::to_string)
    }))
}

fn genotype_string(record: &Record, sample_index: usize) -> Result<Option<String>> {
    let genotypes = record.genotypes()?;
    Ok(Some(genotypes.get(sample_index).to_string()))
}

fn non_empty_join(parts: Vec<String>) -> Option<String> {
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(","))
    }
}

fn format_float(value: f64) -> String {
    let raw = format!("{value:.6}");
    raw.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn bytes_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn record_vcf_column(record: &Record, column: usize) -> Result<String> {
    let text = record.to_vcf_string()?;
    Ok(text
        .trim_end_matches(['\r', '\n'])
        .split('\t')
        .nth(column)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| fallback_filter_string(record)))
}
