use std::env;
use std::fs::File;
use std::io::{BufRead, BufWriter, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rayon::ThreadPool;
use rayon::prelude::*;

use crate::compat::{Backend, CompressionMode, Region, select_backend};
use crate::engine::index::{
    OffsetModel, SkipDecision, VariantFlowIndex, bgzf_data_virtual_end, default_index_path,
    first_record_virtual_start, plan_chunk, read_index, read_virtual_range, source_matches,
    virtual_ends_are_record_boundaries,
};
use crate::expr::{EvalContext, Expression, RequiredFields, parse_expression};
use crate::io::{open_reader, open_vcf_writer};
use crate::vcf::{self, InfoView, RecordView, column_value, resolve_sample_column};

pub const NATIVE_FILTER_THREADS_ENV: &str = "VCF_FAST_NATIVE_FILTER_THREADS";
pub const NATIVE_FILTER_BATCH_RECORDS_ENV: &str = "VCF_FAST_NATIVE_FILTER_BATCH_RECORDS";
const DISABLE_VFI_ENV: &str = "VCF_FAST_DISABLE_VFI";
const INDEX_REPORT_ENV: &str = "VCF_FAST_INDEX_REPORT";
const DEFAULT_PARALLEL_BATCH_RECORDS: usize = 8192;

#[derive(Debug, serde::Serialize)]
struct IndexFilterReport {
    indexed: bool,
    fallback_reason: Option<String>,
    chunks_total: u64,
    chunks_skipped: u64,
    chunks_scanned: u64,
    records_indexed: u64,
    records_skipped_estimate: u64,
}

pub fn run(
    input: &Path,
    where_expr: &str,
    sample: Option<&str>,
    output: &Path,
    region: Option<&Region>,
    compression: CompressionMode,
) -> Result<()> {
    let selected = select_backend(input, region, compression);
    if selected.backend == Backend::Htslib {
        #[cfg(feature = "htslib")]
        {
            return crate::htslib_backend::filter(
                input,
                where_expr,
                sample,
                output,
                region,
                compression,
            );
        }

        #[cfg(not(feature = "htslib"))]
        {
            bail!(selected.reason.unwrap().unavailable_message());
        }
    }

    let expr = parse_expression(where_expr)?;
    let required = expr.required_fields();
    if required.requires_selected_format() && sample.is_none() {
        bail!("FORMAT predicates require --sample <name>");
    }
    let parallel_config = NativeParallelFilterConfig::from_env()?;

    let mut reader = open_reader(input)?;
    let mut headers = Vec::new();
    let mut line = Vec::new();
    let mut sample_column = None;
    let mut saw_chrom_header = false;

    while reader.read_until(b'\n', &mut line)? != 0 {
        if !line.starts_with(b"#") {
            break;
        }

        if line.starts_with(b"#CHROM\t") {
            saw_chrom_header = true;
            if required.requires_format() {
                let header = std::str::from_utf8(&line)?;
                if column_value(header, 9).is_none() {
                    bail!("FORMAT predicates require #CHROM header with sample columns");
                }
                if required.requires_selected_format() {
                    sample_column = Some(resolve_sample_column(header, sample.unwrap())?);
                }
            }
        }

        headers.push(std::mem::take(&mut line));
    }

    if required.requires_format() && !saw_chrom_header {
        bail!("FORMAT predicates require #CHROM header with sample columns");
    }

    if env::var_os(DISABLE_VFI_ENV).is_none()
        && try_indexed_filter(
            input,
            output,
            compression,
            &headers,
            &expr,
            &required,
            sample_column,
        )?
    {
        return Ok(());
    }

    let mut writer = open_vcf_writer(output, compression)?;
    for header in &headers {
        writer.write_all(header)?;
    }

    if parallel_config.enabled() {
        run_parallel_filter(
            &mut *reader,
            &mut *writer,
            line,
            &expr,
            &required,
            sample_column,
            parallel_config,
        )?;
    } else {
        run_streaming_filter(
            &mut *reader,
            &mut *writer,
            line,
            &expr,
            &required,
            sample_column,
        )?;
    }

    writer.flush()?;
    Ok(())
}

fn try_indexed_filter(
    input: &Path,
    output: &Path,
    compression: CompressionMode,
    headers: &[Vec<u8>],
    expr: &Expression,
    required: &RequiredFields,
    sample_column: Option<usize>,
) -> Result<bool> {
    let index_path = default_index_path(input);
    if !index_path.exists() {
        return Ok(false);
    }

    let index = match read_index(&index_path) {
        Ok(index) => index,
        Err(_) => {
            maybe_write_index_report(&IndexFilterReport {
                indexed: false,
                fallback_reason: Some("failed to read or parse VFI index".to_string()),
                chunks_total: 0,
                chunks_skipped: 0,
                chunks_scanned: 0,
                records_indexed: 0,
                records_skipped_estimate: 0,
            })?;
            return Ok(false);
        }
    };
    if !is_usable_bgzf_index(&index, input)? {
        maybe_write_index_report(&IndexFilterReport {
            indexed: false,
            fallback_reason: Some(
                "VFI index is stale or invalid for BGZF virtual-offset filtering".to_string(),
            ),
            chunks_total: index.chunks.len() as u64,
            chunks_skipped: 0,
            chunks_scanned: 0,
            records_indexed: index.record_count,
            records_skipped_estimate: 0,
        })?;
        return Ok(false);
    }

    let mut chunk_plan = Vec::with_capacity(index.chunks.len());
    let mut chunks_skipped = 0_u64;
    let mut chunks_scanned = 0_u64;
    let mut records_skipped_estimate = 0_u64;
    for chunk in &index.chunks {
        let decision = plan_chunk(expr, chunk);
        if decision == SkipDecision::UnsupportedForIndex {
            maybe_write_index_report(&IndexFilterReport {
                indexed: false,
                fallback_reason: Some(
                    "filter predicate is unsupported by VFI chunk planning".to_string(),
                ),
                chunks_total: index.chunks.len() as u64,
                chunks_skipped: 0,
                chunks_scanned: 0,
                records_indexed: index.record_count,
                records_skipped_estimate: 0,
            })?;
            return Ok(false);
        }

        let (Some(virtual_start), Some(virtual_end)) = (chunk.virtual_start, chunk.virtual_end)
        else {
            maybe_write_index_report(&IndexFilterReport {
                indexed: false,
                fallback_reason: Some("VFI index chunk is missing virtual offsets".to_string()),
                chunks_total: index.chunks.len() as u64,
                chunks_skipped: 0,
                chunks_scanned: 0,
                records_indexed: index.record_count,
                records_skipped_estimate: 0,
            })?;
            return Ok(false);
        };

        match decision {
            SkipDecision::CanSkip => {
                chunks_skipped += 1;
                records_skipped_estimate += chunk.record_count;
            }
            SkipDecision::MustScan => {
                chunks_scanned += 1;
            }
            SkipDecision::UnsupportedForIndex => unreachable!("unsupported decisions return early"),
        }
        chunk_plan.push((decision, virtual_start, virtual_end));
    }

    let mut writer = open_vcf_writer(output, compression)?;
    for header in headers {
        writer.write_all(header)?;
    }

    for (decision, virtual_start, virtual_end) in chunk_plan {
        if decision == SkipDecision::CanSkip {
            continue;
        }

        let bytes = read_virtual_range(input, virtual_start, virtual_end)?;
        scan_indexed_chunk_bytes(&bytes, &mut *writer, expr, required, sample_column)?;
    }

    writer.flush()?;
    maybe_write_index_report(&IndexFilterReport {
        indexed: true,
        fallback_reason: None,
        chunks_total: index.chunks.len() as u64,
        chunks_skipped,
        chunks_scanned,
        records_indexed: index.record_count,
        records_skipped_estimate,
    })?;
    Ok(true)
}

fn maybe_write_index_report(report: &IndexFilterReport) -> Result<()> {
    let Some(path) = env::var_os(INDEX_REPORT_ENV) else {
        return Ok(());
    };

    let path = PathBuf::from(path);
    let file = File::create(&path)
        .with_context(|| format!("failed to create index report {}", path.display()))?;
    serde_json::to_writer_pretty(BufWriter::new(file), report)
        .with_context(|| format!("failed to write index report {}", path.display()))?;
    Ok(())
}

fn is_usable_bgzf_index(index: &VariantFlowIndex, input: &Path) -> Result<bool> {
    if index.index_kind != "variantflow-vfi"
        || index.offset_model != OffsetModel::BgzfVirtual
        || !index.virtual_offsets_available
        || !source_matches(index, input)?
        || (index.record_count > 0 && index.chunks.is_empty())
    {
        return Ok(false);
    }

    let terminal_virtual_end = bgzf_data_virtual_end(input)?;
    let first_record_virtual_start = if index.record_count == 0 || index.chunks.is_empty() {
        let first_record_virtual_start = first_record_virtual_start(input)?;
        if first_record_virtual_start.is_some() {
            return Ok(false);
        }
        first_record_virtual_start
    } else {
        first_record_virtual_start(input)?
    };

    if index.chunks.is_empty() {
        return Ok(index.record_count == 0);
    }

    let mut next_first_record = 0_u64;
    let mut counted_records = 0_u64;
    let mut previous_virtual_end = None;
    let mut boundary_virtual_ends = Vec::new();
    for (expected_ordinal, chunk) in index.chunks.iter().enumerate() {
        let Some(virtual_start) = chunk.virtual_start else {
            return Ok(false);
        };
        let Some(virtual_end) = chunk.virtual_end else {
            return Ok(false);
        };

        if chunk.ordinal != expected_ordinal as u64
            || chunk.first_record != next_first_record
            || chunk.record_count == 0
            || virtual_end <= virtual_start
            || (expected_ordinal == 0 && Some(virtual_start) != first_record_virtual_start)
            || (expected_ordinal > 0
                && previous_virtual_end.is_some_and(|previous| virtual_start != previous))
        {
            return Ok(false);
        }

        if Some(virtual_end) != terminal_virtual_end {
            boundary_virtual_ends.push(virtual_end);
        }

        let Ok(bytes) = read_virtual_range(input, virtual_start, virtual_end) else {
            return Ok(false);
        };
        let range_record_count = count_vcf_record_lines(&bytes);
        if range_record_count != chunk.record_count {
            return Ok(false);
        }
        let Some(counted_next) = counted_records.checked_add(range_record_count) else {
            return Ok(false);
        };
        counted_records = counted_next;

        let Some(next) = next_first_record.checked_add(chunk.record_count) else {
            return Ok(false);
        };
        next_first_record = next;
        previous_virtual_end = Some(virtual_end);
    }

    if index.record_count != next_first_record {
        return Ok(false);
    }

    if index.record_count != counted_records {
        return Ok(false);
    }

    if !virtual_ends_are_record_boundaries(input, &boundary_virtual_ends)? {
        return Ok(false);
    }

    if let Some(last_virtual_end) = previous_virtual_end {
        return Ok(terminal_virtual_end == Some(last_virtual_end));
    }

    Ok(index.record_count == 0)
}

fn scan_indexed_chunk_bytes(
    bytes: &[u8],
    writer: &mut dyn Write,
    expr: &Expression,
    required: &RequiredFields,
    sample_column: Option<usize>,
) -> Result<()> {
    for line in bytes.split_inclusive(|byte| *byte == b'\n') {
        if !is_vcf_record_line(line) {
            continue;
        }

        let record = ByteEvalRecord::parse(line, required, sample_column)?;
        if expr.evaluate_context(&record) {
            writer.write_all(line)?;
        }
    }

    Ok(())
}

fn count_vcf_record_lines(bytes: &[u8]) -> u64 {
    bytes
        .split_inclusive(|byte| *byte == b'\n')
        .filter(|line| is_vcf_record_line(line))
        .count() as u64
}

fn is_vcf_record_line(line: &[u8]) -> bool {
    !line.is_empty()
        && !line.starts_with(b"#")
        && !line.iter().all(|byte| matches!(byte, b'\n' | b'\r'))
}

fn run_streaming_filter(
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
    mut line: Vec<u8>,
    expr: &Expression,
    required: &RequiredFields,
    sample_column: Option<usize>,
) -> Result<()> {
    loop {
        if !line.is_empty() {
            let record = ByteEvalRecord::parse(&line, required, sample_column)?;
            if expr.evaluate_context(&record) {
                writer.write_all(&line)?;
            }
            line.clear();
        }

        if reader.read_until(b'\n', &mut line)? == 0 {
            break;
        }
    }

    Ok(())
}

fn run_parallel_filter(
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
    mut line: Vec<u8>,
    expr: &Expression,
    required: &RequiredFields,
    sample_column: Option<usize>,
    config: NativeParallelFilterConfig,
) -> Result<()> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.threads.get())
        .build()
        .context("failed to build native filter thread pool")?;
    let mut batch = Vec::with_capacity(config.batch_records.get());

    loop {
        if !line.is_empty() {
            batch.push(std::mem::take(&mut line));
            if batch.len() >= config.batch_records.get() {
                flush_parallel_batch(&mut batch, &pool, expr, required, sample_column, writer)?;
            }
        }

        if reader.read_until(b'\n', &mut line)? == 0 {
            break;
        }
    }

    flush_parallel_batch(&mut batch, &pool, expr, required, sample_column, writer)?;
    Ok(())
}

fn flush_parallel_batch(
    batch: &mut Vec<Vec<u8>>,
    pool: &ThreadPool,
    expr: &Expression,
    required: &RequiredFields,
    sample_column: Option<usize>,
    writer: &mut dyn Write,
) -> Result<()> {
    if batch.is_empty() {
        return Ok(());
    }

    let lines = std::mem::take(batch);
    let evaluated = pool.install(|| {
        lines
            .into_par_iter()
            .map(|line| {
                let record = ByteEvalRecord::parse(&line, required, sample_column)?;
                Ok(if expr.evaluate_context(&record) {
                    Some(line)
                } else {
                    None
                })
            })
            .collect::<Vec<Result<Option<Vec<u8>>>>>()
    });

    for maybe_line in evaluated {
        if let Some(line) = maybe_line? {
            writer.write_all(&line)?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct NativeParallelFilterConfig {
    threads: NonZeroUsize,
    batch_records: NonZeroUsize,
}

impl NativeParallelFilterConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            threads: parse_positive_env(NATIVE_FILTER_THREADS_ENV, None)?,
            batch_records: parse_positive_env(
                NATIVE_FILTER_BATCH_RECORDS_ENV,
                NonZeroUsize::new(DEFAULT_PARALLEL_BATCH_RECORDS),
            )?,
        })
    }

    fn enabled(&self) -> bool {
        self.threads.get() > 1
    }
}

fn parse_positive_env(name: &str, default: Option<NonZeroUsize>) -> Result<NonZeroUsize> {
    match env::var(name) {
        Ok(raw) => {
            let value = raw
                .parse::<usize>()
                .map_err(|_| anyhow::anyhow!("{name} must be a positive integer"))?;
            NonZeroUsize::new(value)
                .ok_or_else(|| anyhow::anyhow!("{name} must be a positive integer"))
        }
        Err(env::VarError::NotPresent) => {
            Ok(default.unwrap_or_else(|| NonZeroUsize::new(1).unwrap()))
        }
        Err(env::VarError::NotUnicode(_)) => bail!("{name} must be valid UTF-8"),
    }
}

struct ByteEvalRecord<'a> {
    record: RecordView<'a>,
    info: InfoView<'a>,
    format_column: Option<&'a [u8]>,
    selected_sample: Option<&'a [u8]>,
}

impl<'a> ByteEvalRecord<'a> {
    fn parse(
        line: &'a [u8],
        required: &RequiredFields,
        sample_column: Option<usize>,
    ) -> Result<Self> {
        let record = RecordView::parse(line)?;
        let info = if required.requires_info() {
            InfoView::scan(record.info())
        } else {
            InfoView::default()
        };
        let (format_column, selected_sample) = if required.requires_format() {
            (
                Some(record.column(8).unwrap_or(b"")),
                Some(
                    sample_column
                        .and_then(|column| record.column(column))
                        .unwrap_or(b"."),
                ),
            )
        } else {
            (None, None)
        };

        Ok(Self {
            record,
            info,
            format_column,
            selected_sample,
        })
    }
}

impl EvalContext for ByteEvalRecord<'_> {
    fn chrom(&self) -> Option<&[u8]> {
        Some(self.record.chrom())
    }

    fn pos(&self) -> Option<u64> {
        self.record.pos_u64().ok()
    }

    fn qual(&self) -> Option<f64> {
        self.record.qual_float().ok().flatten()
    }

    fn filter(&self) -> Option<&[u8]> {
        Some(self.record.filter())
    }

    fn info_number_any(&self, key: &[u8], predicate: &mut dyn FnMut(f64) -> bool) -> bool {
        self.info.number_any(key, predicate)
    }

    fn info_value(&self, key: &[u8]) -> Option<&[u8]> {
        self.info.value(key)
    }

    fn format_value(&self, key: &[u8]) -> Option<&[u8]> {
        let format = self.format_column?;
        let sample = self.selected_sample?;
        let index = vcf::format_key_index(format, key)?;
        vcf::sample_format_value_at(sample, index)
    }

    fn any_format_value(&self, key: &[u8], predicate: &mut dyn FnMut(&[u8]) -> bool) -> bool {
        let Some(format) = self.format_column else {
            return false;
        };
        let Some(index) = vcf::format_key_index(format, key) else {
            return false;
        };

        let mut matched = false;
        self.record.for_each_sample_column(|sample| {
            if matched {
                return;
            }
            if let Some(value) = vcf::sample_format_value_at(sample, index) {
                matched = predicate(value);
            }
        });
        matched
    }

    fn all_format_value(&self, key: &[u8], mut predicate: &mut dyn FnMut(&[u8]) -> bool) -> bool {
        let Some(format) = self.format_column else {
            return false;
        };
        let Some(index) = vcf::format_key_index(format, key) else {
            return false;
        };

        let mut saw_sample = false;
        let mut all_match = true;
        self.record.for_each_sample_column(|sample| {
            saw_sample = true;
            if !all_match {
                return;
            }
            all_match = vcf::sample_format_value_at(sample, index).is_some_and(&mut predicate);
        });
        saw_sample && all_match
    }

    fn count_format_value(&self, key: &[u8], predicate: &mut dyn FnMut(&[u8]) -> bool) -> u64 {
        let Some(format) = self.format_column else {
            return 0;
        };
        let Some(index) = vcf::format_key_index(format, key) else {
            return 0;
        };

        let mut count = 0;
        self.record.for_each_sample_column(|sample| {
            if let Some(value) = vcf::sample_format_value_at(sample, index)
                && predicate(value)
            {
                count += 1;
            }
        });
        count
    }
}
