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
    if required.has_non_legacy_format_keys() {
        bail!("arbitrary FORMAT predicates are not implemented for htslib-backed input in v0.9");
    }
    if required.requires_format() && sample.is_none() {
        bail!("FORMAT predicates require --sample <name>");
    }

    if let Some(region) = region {
        let mut reader = indexed_reader(input, region)?;
        apply_reader_threads(&mut reader)?;
        let header = Header::from_template(reader.header());
        let sample_id = sample_id(reader.header(), sample, &required)?;
        let mut writer = vcf_writer(output, &header, compression)?;
        apply_writer_threads(&mut writer)?;
        for_each_record(&mut reader, |record| {
            if evaluate_record(record, &required, sample_id, &expr)? {
                writer.write(record)?;
            }
            Ok(())
        })?;
    } else {
        let mut reader = Reader::from_path(input)
            .with_context(|| format!("failed to open input {}", input.display()))?;
        apply_reader_threads(&mut reader)?;
        let header = Header::from_template(reader.header());
        let sample_id = sample_id(reader.header(), sample, &required)?;
        let mut writer = vcf_writer(output, &header, compression)?;
        apply_writer_threads(&mut writer)?;
        for_each_record(&mut reader, |record| {
            if evaluate_record(record, &required, sample_id, &expr)? {
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
            observe_stats_record(record, &mut summary, &mut titv)?;
            Ok(())
        })?;
    } else {
        let mut reader = Reader::from_path(input)
            .with_context(|| format!("failed to open input {}", input.display()))?;
        apply_reader_threads(&mut reader)?;
        for_each_record(&mut reader, |record| {
            observe_stats_record(record, &mut summary, &mut titv)?;
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
    required: &RequiredFields,
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
    required: &RequiredFields,
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
    let info = if required.requires_info() {
        info_string(record)?
    } else {
        String::new()
    };
    let required_format = required.legacy_format_fields();
    let gt = if required_format.gt {
        sample_id
            .and_then(|index| genotype_string(record, index).transpose())
            .transpose()?
    } else {
        None
    };
    let dp = if required_format.dp {
        sample_id
            .and_then(|index| format_integer_string(record, b"DP", index).transpose())
            .transpose()?
    } else {
        None
    };
    let gq = if required_format.gq {
        sample_id
            .and_then(|index| format_integer_string(record, b"GQ", index).transpose())
            .transpose()?
    } else {
        None
    };

    let mut format = FormatValues::default();
    if let Some(value) = gt.as_deref() {
        format = format.with_gt(value.as_bytes());
    }
    if let Some(value) = dp.as_deref() {
        format = format.with_dp(value.as_bytes());
    }
    if let Some(value) = gq.as_deref() {
        format = format.with_gq(value.as_bytes());
    }

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
        format,
    };

    Ok(expr.evaluate(&eval))
}

fn write_tsv_record(record: &Record, writer: &mut dyn Write) -> Result<()> {
    write_chrom_cell(record, writer)?;
    write!(writer, "{}\t", record.pos() + 1)?;
    write_bytes_cell(writer, &record.id())?;
    write_allele_cell(record, 0, writer)?;
    write_alternate_cell(record, writer)?;
    write_qual_cell(record, writer)?;
    write_filter_cell(record, writer)?;
    write_info_numeric_cell(record, b"DP", writer)?;
    write_info_numeric_value_or_dot(record, b"AF", writer)?;
    writer.write_all(b"\n")?;
    Ok(())
}

fn write_chrom_cell(record: &Record, writer: &mut dyn Write) -> Result<()> {
    let rid = record
        .rid()
        .ok_or_else(|| anyhow::anyhow!("record is missing RID"))?;
    writer.write_all(record.header().rid2name(rid)?)?;
    writer.write_all(b"\t")?;
    Ok(())
}

fn write_allele_cell(record: &Record, index: usize, writer: &mut dyn Write) -> Result<()> {
    let alleles = record.alleles();
    let allele = alleles
        .get(index)
        .ok_or_else(|| anyhow::anyhow!("record is missing allele {index}"))?;
    write_bytes_cell(writer, allele)
}

fn write_alternate_cell(record: &Record, writer: &mut dyn Write) -> Result<()> {
    let alleles = record.alleles();
    if alleles.len() <= 1 {
        writer.write_all(b".")?;
    } else {
        for (index, allele) in alleles.iter().skip(1).enumerate() {
            if index > 0 {
                writer.write_all(b",")?;
            }
            writer.write_all(allele)?;
        }
    }
    writer.write_all(b"\t")?;
    Ok(())
}

fn write_qual_cell(record: &Record, writer: &mut dyn Write) -> Result<()> {
    if let Some(qual) = qual(record) {
        write!(writer, "{qual}")?;
    } else {
        writer.write_all(b".")?;
    }
    writer.write_all(b"\t")?;
    Ok(())
}

fn write_filter_cell(record: &Record, writer: &mut dyn Write) -> Result<()> {
    if record.inner().d.n_flt == 0 {
        writer.write_all(b".")?;
    } else {
        let mut wrote = false;
        for id in record.filters() {
            if wrote {
                writer.write_all(b";")?;
            }
            writer.write_all(&record.header().id_to_name(id))?;
            wrote = true;
        }
        if !wrote {
            writer.write_all(b"PASS")?;
        }
    }
    writer.write_all(b"\t")?;
    Ok(())
}

fn write_info_numeric_cell(record: &Record, tag: &[u8], writer: &mut dyn Write) -> Result<()> {
    write_info_numeric_value_or_dot(record, tag, writer)?;
    writer.write_all(b"\t")?;
    Ok(())
}

fn write_info_numeric_value_or_dot(
    record: &Record,
    tag: &[u8],
    writer: &mut dyn Write,
) -> Result<()> {
    if write_info_integer_values(record, tag, writer)? {
        return Ok(());
    }
    if write_info_float_values(record, tag, writer)? {
        return Ok(());
    }
    writer.write_all(b".")?;
    Ok(())
}

fn write_info_integer_values(record: &Record, tag: &[u8], writer: &mut dyn Write) -> Result<bool> {
    match record.info(tag).integer() {
        Ok(Some(values)) => {
            let mut wrote = false;
            for value in values.iter().filter(|value| !value.is_missing()) {
                if wrote {
                    writer.write_all(b",")?;
                }
                write!(writer, "{value}")?;
                wrote = true;
            }
            Ok(wrote)
        }
        Ok(None)
        | Err(HtslibError::BcfUnexpectedType { .. })
        | Err(HtslibError::BcfUndefinedTag { .. }) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn write_info_float_values(record: &Record, tag: &[u8], writer: &mut dyn Write) -> Result<bool> {
    match record.info(tag).float() {
        Ok(Some(values)) => {
            let mut wrote = false;
            for value in values
                .iter()
                .filter(|value| !value.is_missing() && !value.is_nan())
            {
                if wrote {
                    writer.write_all(b",")?;
                }
                write!(writer, "{value}")?;
                wrote = true;
            }
            Ok(wrote)
        }
        Ok(None)
        | Err(HtslibError::BcfUnexpectedType { .. })
        | Err(HtslibError::BcfUndefinedTag { .. }) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn write_bytes_cell(writer: &mut dyn Write, bytes: &[u8]) -> Result<()> {
    writer.write_all(bytes)?;
    writer.write_all(b"\t")?;
    Ok(())
}

fn observe_stats_record(
    record: &Record,
    summary: &mut StatsSummary,
    titv: &mut TiTv,
) -> Result<()> {
    summary.variants += 1;
    *summary
        .variants_per_chromosome
        .entry(chrom(record)?)
        .or_default() += 1;

    if record.inner().d.n_flt == 0 {
        summary.missing_filter_values += 1;
    }

    if let Some(qual) = qual(record) {
        summary.qual.observe(f64::from(qual));
    }

    observe_info_numeric(record, b"AF", |af| summary.af.observe(af))?;

    let alleles = record.alleles();
    let reference = alleles
        .first()
        .ok_or_else(|| anyhow::anyhow!("record is missing allele 0"))?;
    for alternate in alleles.iter().skip(1) {
        if is_snp_allele(reference, alternate) {
            summary.snps += 1;
            if let (Ok(reference), Ok(alternate)) =
                (str::from_utf8(reference), str::from_utf8(alternate))
            {
                titv.observe(reference, alternate);
            }
        } else {
            summary.indels += 1;
        }
    }

    Ok(())
}

fn chrom(record: &Record) -> Result<String> {
    let rid = record
        .rid()
        .ok_or_else(|| anyhow::anyhow!("record is missing RID"))?;
    Ok(bytes_to_string(record.header().rid2name(rid)?))
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
    Ok(stats_filter_string(record))
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
                .map(|value| format_float32(*value))
                .collect::<Vec<_>>();
            Ok(non_empty_join(parts))
        }
        Ok(None)
        | Err(HtslibError::BcfUnexpectedType { .. })
        | Err(HtslibError::BcfUndefinedTag { .. }) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn observe_info_numeric(record: &Record, tag: &[u8], mut observe: impl FnMut(f64)) -> Result<()> {
    match record.info(tag).integer() {
        Ok(Some(values)) => {
            for value in values.iter().filter(|value| !value.is_missing()) {
                observe(f64::from(*value));
            }
            return Ok(());
        }
        Ok(None)
        | Err(HtslibError::BcfUnexpectedType { .. })
        | Err(HtslibError::BcfUndefinedTag { .. }) => {}
        Err(error) => return Err(error.into()),
    }

    match record.info(tag).float() {
        Ok(Some(values)) => {
            for value in values
                .iter()
                .filter(|value| !value.is_missing() && !value.is_nan())
            {
                observe(f64::from(*value));
            }
            Ok(())
        }
        Ok(None)
        | Err(HtslibError::BcfUnexpectedType { .. })
        | Err(HtslibError::BcfUndefinedTag { .. }) => Ok(()),
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

fn format_float32(value: f32) -> String {
    value.to_string()
}

fn is_snp_allele(reference: &[u8], alternate: &[u8]) -> bool {
    reference.len() == 1 && alternate.len() == 1
}

fn bytes_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}
