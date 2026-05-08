use std::env;
use std::fs::File;
use std::io::{BufRead, BufWriter, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::thread;

use anyhow::{Context, Result, bail};
use rayon::ThreadPool;
use rayon::prelude::*;

use crate::compat::{Backend, CompressionMode, Region, select_backend};
use crate::engine::index::{
    OffsetModel, SkipDecision, VariantFlowIndex, default_index_path, first_record_virtual_start,
    for_each_decoded_bgzf_block_with_threads, for_each_virtual_range_slice, plan_chunk, read_index,
    source_matches,
};
use crate::engine::pipeline::{
    AcceptedBatch, LineCarry, OrderedBatchWriter, PipelineConfig, RecordBatch,
    evaluate_batches_ordered,
};
use crate::expr::{EvalContext, Expression, RequiredFields, parse_expression};
use crate::io::{
    NATIVE_BGZF_THREADS_ENV, native_bgzf_threads_from_env, open_reader_with_native_bgzf_threads,
    open_vcf_writer,
};
use crate::vcf::{self, InfoView, RecordView, column_value, resolve_sample_column};

#[cfg(test)]
type BatchThreadObserver = dyn Fn(thread::ThreadId, &RecordBatch) + Send + Sync + 'static;

#[cfg(test)]
static BGZF_BATCH_THREAD_OBSERVER: std::sync::Mutex<Option<std::sync::Arc<BatchThreadObserver>>> =
    std::sync::Mutex::new(None);

pub const NATIVE_FILTER_THREADS_ENV: &str = "VCF_FAST_NATIVE_FILTER_THREADS";
pub const NATIVE_FILTER_BATCH_RECORDS_ENV: &str = "VCF_FAST_NATIVE_FILTER_BATCH_RECORDS";
pub const NATIVE_FILTER_QUEUE_BATCHES_ENV: &str = "VCF_FAST_NATIVE_FILTER_QUEUE_BATCHES";
const DISABLE_VFI_ENV: &str = "VCF_FAST_DISABLE_VFI";
const INDEX_REPORT_ENV: &str = "VCF_FAST_INDEX_REPORT";
const INDEX_MIN_SKIP_RATE_ENV: &str = "VCF_FAST_INDEX_MIN_SKIP_RATE";
const DEFAULT_INDEX_MIN_SKIP_RATE: f64 = 0.80;
const DEFAULT_PARALLEL_BATCH_RECORDS: usize = 2048;
const DEFAULT_PARALLEL_QUEUE_BATCHES: usize = 2;
const DEFAULT_AUTO_FILTER_THREAD_CAP: usize = 4;

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
    let bgzf_threads = native_filter_bgzf_threads(&required)?;
    let parallel_config = NativeParallelFilterConfig::from_env(&required)?;
    let pipeline_config = native_pipeline_config_from_env(&required, bgzf_threads)?;

    let mut reader = open_reader_with_native_bgzf_threads(input, bgzf_threads)?;
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

    if pipeline_config.enabled()
        && try_native_bgzf_pipeline(
            input,
            output,
            compression,
            &expr,
            &required,
            sample_column,
            pipeline_config,
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

    let min_skip_rate = index_min_skip_rate()?;
    let skip_rate = if index.record_count == 0 {
        1.0
    } else {
        records_skipped_estimate as f64 / index.record_count as f64
    };
    if skip_rate < min_skip_rate {
        maybe_write_index_report(&IndexFilterReport {
            indexed: false,
            fallback_reason: Some(format!(
                "VFI skip estimate {skip_rate:.3} is below required minimum {min_skip_rate:.3}"
            )),
            chunks_total: index.chunks.len() as u64,
            chunks_skipped,
            chunks_scanned,
            records_indexed: index.record_count,
            records_skipped_estimate,
        })?;
        return Ok(false);
    }

    let mut writer = open_vcf_writer(output, compression)?;
    for header in headers {
        writer.write_all(header)?;
    }

    let mut pending_scan_start = None;
    let mut pending_scan_end = None;
    for (decision, virtual_start, virtual_end) in chunk_plan {
        if decision == SkipDecision::CanSkip {
            if let (Some(start), Some(end)) = (pending_scan_start.take(), pending_scan_end.take()) {
                scan_indexed_virtual_range(
                    input,
                    start,
                    end,
                    &mut *writer,
                    expr,
                    required,
                    sample_column,
                )?;
            }
            continue;
        }

        if pending_scan_start.is_none() {
            pending_scan_start = Some(virtual_start);
        }
        pending_scan_end = Some(virtual_end);
    }

    if let (Some(start), Some(end)) = (pending_scan_start, pending_scan_end) {
        scan_indexed_virtual_range(
            input,
            start,
            end,
            &mut *writer,
            expr,
            required,
            sample_column,
        )?;
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

fn index_min_skip_rate() -> Result<f64> {
    let Some(value) = env::var_os(INDEX_MIN_SKIP_RATE_ENV) else {
        return Ok(DEFAULT_INDEX_MIN_SKIP_RATE);
    };
    let value = value
        .to_str()
        .context("VCF_FAST_INDEX_MIN_SKIP_RATE must be valid UTF-8")?;
    let rate: f64 = value
        .parse()
        .context("VCF_FAST_INDEX_MIN_SKIP_RATE must be a number between 0 and 1")?;
    if !(0.0..=1.0).contains(&rate) {
        bail!("VCF_FAST_INDEX_MIN_SKIP_RATE must be between 0 and 1");
    }
    Ok(rate)
}

fn maybe_write_index_report(report: &IndexFilterReport) -> Result<()> {
    let Some(path) = env::var_os(INDEX_REPORT_ENV) else {
        return Ok(());
    };

    let path = PathBuf::from(path);
    let file = File::create(&path)
        .with_context(|| format!("failed to create index report {}", path.display()))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, report)
        .with_context(|| format!("failed to write index report {}", path.display()))?;
    writer
        .flush()
        .with_context(|| format!("failed to flush index report {}", path.display()))?;
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
    let mut previous_virtual_end = None;
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
                && previous_virtual_end.is_some_and(|previous| virtual_start < previous))
        {
            return Ok(false);
        }

        let Some(next) = next_first_record.checked_add(chunk.record_count) else {
            return Ok(false);
        };
        next_first_record = next;
        previous_virtual_end = Some(virtual_end);
    }

    if index.record_count != next_first_record {
        return Ok(false);
    }

    Ok(previous_virtual_end.is_some() || index.record_count == 0)
}

fn scan_indexed_virtual_range(
    input: &Path,
    virtual_start: u64,
    virtual_end: u64,
    writer: &mut dyn Write,
    expr: &Expression,
    required: &RequiredFields,
    sample_column: Option<usize>,
) -> Result<()> {
    let mut line = Vec::new();
    for_each_virtual_range_slice(input, virtual_start, virtual_end, |bytes| {
        for segment in bytes.split_inclusive(|byte| *byte == b'\n') {
            line.extend_from_slice(segment);
            if segment.ends_with(b"\n") {
                scan_indexed_record_line(&line, writer, expr, required, sample_column)?;
                line.clear();
            }
        }
        Ok(())
    })?;

    if !line.is_empty() {
        scan_indexed_record_line(&line, writer, expr, required, sample_column)?;
    }

    Ok(())
}

fn scan_indexed_record_line(
    line: &[u8],
    writer: &mut dyn Write,
    expr: &Expression,
    required: &RequiredFields,
    sample_column: Option<usize>,
) -> Result<()> {
    if !is_vcf_record_line(line) {
        return Ok(());
    }

    let record = ByteEvalRecord::parse(line, required, sample_column)?;
    if expr.evaluate_context(&record) {
        writer.write_all(line)?;
    }
    Ok(())
}

fn is_vcf_record_line(line: &[u8]) -> bool {
    !line.is_empty()
        && !line.starts_with(b"#")
        && !line.iter().all(|byte| matches!(byte, b'\n' | b'\r'))
}

fn try_native_bgzf_pipeline(
    input: &Path,
    output: &Path,
    compression: CompressionMode,
    expr: &Expression,
    required: &RequiredFields,
    sample_column: Option<usize>,
    config: PipelineConfig,
) -> Result<bool> {
    if !has_gz_extension(input) {
        return Ok(false);
    }

    let mut writer = open_vcf_writer(output, compression)?;
    let result =
        stream_native_bgzf_pipeline(input, &mut *writer, expr, required, sample_column, config);
    match result {
        Ok(()) => {
            writer.flush()?;
            Ok(true)
        }
        Err(error) if is_not_bgzf_error(&error) => Ok(false),
        Err(error) => Err(error),
    }
}

fn stream_native_bgzf_pipeline(
    input: &Path,
    writer: &mut dyn Write,
    expr: &Expression,
    required: &RequiredFields,
    sample_column: Option<usize>,
    config: PipelineConfig,
) -> Result<()> {
    let mut carry = LineCarry {
        pending: Vec::new(),
        next_batch_sequence: 0,
    };
    let mut ordered_writer = OrderedBatchWriter::new(writer);
    let mut header_open = true;
    let group_capacity = config.queue_batches.max(1);
    let mut ready_batches = Vec::with_capacity(group_capacity);

    for_each_decoded_bgzf_block_with_threads(input, config.bgzf_threads, |block| {
        for batch in carry.push_block(block, config.batch_records) {
            ready_batches.push(batch);
            if ready_batches.len() >= group_capacity {
                flush_bgzf_batch_group(
                    &mut ready_batches,
                    &mut ordered_writer,
                    expr,
                    required,
                    sample_column,
                    &mut header_open,
                    config.filter_threads,
                )?;
            }
        }
        Ok(())
    })?;

    ready_batches.extend(carry.finish());
    flush_bgzf_batch_group(
        &mut ready_batches,
        &mut ordered_writer,
        expr,
        required,
        sample_column,
        &mut header_open,
        config.filter_threads,
    )?;

    ordered_writer.finish()?;
    Ok(())
}

fn flush_bgzf_batch_group<W: Write + ?Sized>(
    ready_batches: &mut Vec<RecordBatch>,
    ordered_writer: &mut OrderedBatchWriter<'_, W>,
    expr: &Expression,
    required: &RequiredFields,
    sample_column: Option<usize>,
    header_open: &mut bool,
    filter_threads: usize,
) -> Result<()> {
    if ready_batches.is_empty() {
        return Ok(());
    }

    let batches = std::mem::take(ready_batches);
    let mut batch_iter = batches.into_iter();
    let mut record_batches = Vec::new();

    for batch in batch_iter.by_ref() {
        if *header_open {
            let accepted =
                evaluate_original_byte_batch(batch, expr, required, sample_column, header_open)?;
            ordered_writer.write_batch(accepted)?;
        } else {
            record_batches.push(batch);
            record_batches.extend(batch_iter);
            break;
        }
    }

    if !record_batches.is_empty() {
        let accepted_batches = evaluate_batches_ordered(record_batches, filter_threads, |batch| {
            let mut record_header_open = false;
            evaluate_original_byte_batch(
                batch,
                expr,
                required,
                sample_column,
                &mut record_header_open,
            )
        })?;
        for accepted in accepted_batches {
            ordered_writer.write_batch(accepted)?;
        }
    }

    Ok(())
}

fn is_not_bgzf_error(error: &anyhow::Error) -> bool {
    error.to_string().contains("not a BGZF file")
}

fn has_gz_extension(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "gz")
}

fn evaluate_original_byte_batch(
    batch: RecordBatch,
    expr: &Expression,
    required: &RequiredFields,
    sample_column: Option<usize>,
    header_open: &mut bool,
) -> Result<AcceptedBatch> {
    #[cfg(test)]
    if let Some(observer) = BGZF_BATCH_THREAD_OBSERVER.lock().unwrap().clone() {
        observer(thread::current().id(), &batch);
    }

    let mut accepted = Vec::with_capacity(batch.bytes.len());
    for line in batch.bytes.split_inclusive(|byte| *byte == b'\n') {
        if *header_open && line.starts_with(b"#") {
            accepted.extend_from_slice(line);
            continue;
        }

        *header_open = false;
        let record = ByteEvalRecord::parse(line, required, sample_column)?;
        if expr.evaluate_context(&record) {
            accepted.extend_from_slice(line);
        }
    }

    Ok(AcceptedBatch {
        sequence: batch.sequence,
        bytes: accepted,
    })
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
    let (work_tx, work_rx) = sync_channel::<Vec<Vec<u8>>>(config.queue_batches.get());
    let (result_tx, result_rx) = sync_channel::<ParallelBatchResult>(config.queue_batches.get());

    thread::scope(|scope| -> Result<()> {
        let evaluator = scope.spawn(move || -> Result<()> {
            run_parallel_evaluator(work_rx, result_tx, &pool, expr, required, sample_column)
        });

        loop {
            if !line.is_empty() {
                batch.push(std::mem::take(&mut line));
                if batch.len() >= config.batch_records.get() {
                    send_parallel_work_batch(
                        &work_tx,
                        &result_rx,
                        std::mem::take(&mut batch),
                        writer,
                    )?;
                }
            }

            if reader.read_until(b'\n', &mut line)? == 0 {
                break;
            }
        }

        if !batch.is_empty() {
            send_parallel_work_batch(&work_tx, &result_rx, batch, writer)?;
        }
        drop(work_tx);
        drain_parallel_results(&result_rx, writer)?;
        evaluator
            .join()
            .map_err(|_| anyhow::anyhow!("native filter evaluator thread panicked"))??;
        Ok(())
    })
}

type ParallelBatchResult = Result<Vec<Option<Vec<u8>>>>;

fn run_parallel_evaluator(
    work_rx: Receiver<Vec<Vec<u8>>>,
    result_tx: SyncSender<ParallelBatchResult>,
    pool: &ThreadPool,
    expr: &Expression,
    required: &RequiredFields,
    sample_column: Option<usize>,
) -> Result<()> {
    for lines in work_rx {
        let evaluated = evaluate_parallel_batch(lines, pool, expr, required, sample_column);
        result_tx
            .send(evaluated)
            .map_err(|_| anyhow::anyhow!("native filter result receiver disconnected"))?;
    }
    Ok(())
}

fn evaluate_parallel_batch(
    lines: Vec<Vec<u8>>,
    pool: &ThreadPool,
    expr: &Expression,
    required: &RequiredFields,
    sample_column: Option<usize>,
) -> ParallelBatchResult {
    pool.install(|| {
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
            .collect::<Result<Vec<Option<Vec<u8>>>>>()
    })
}

fn send_parallel_work_batch(
    work_tx: &SyncSender<Vec<Vec<u8>>>,
    result_rx: &Receiver<ParallelBatchResult>,
    mut batch: Vec<Vec<u8>>,
    writer: &mut dyn Write,
) -> Result<()> {
    loop {
        match work_tx.try_send(batch) {
            Ok(()) => return drain_available_parallel_results(result_rx, writer),
            Err(TrySendError::Full(returned_batch)) => {
                batch = returned_batch;
                let result = result_rx
                    .recv()
                    .context("native filter evaluator disconnected while queue was full")?;
                write_parallel_batch_result(result, writer)?;
            }
            Err(TrySendError::Disconnected(_)) => {
                bail!("native filter evaluator disconnected before accepting work");
            }
        }
    }
}

fn drain_available_parallel_results(
    result_rx: &Receiver<ParallelBatchResult>,
    writer: &mut dyn Write,
) -> Result<()> {
    while let Ok(result) = result_rx.try_recv() {
        write_parallel_batch_result(result, writer)?;
    }
    Ok(())
}

fn drain_parallel_results(
    result_rx: &Receiver<ParallelBatchResult>,
    writer: &mut dyn Write,
) -> Result<()> {
    while let Ok(result) = result_rx.recv() {
        write_parallel_batch_result(result, writer)?;
    }
    Ok(())
}

fn write_parallel_batch_result(result: ParallelBatchResult, writer: &mut dyn Write) -> Result<()> {
    for line in result?.into_iter().flatten() {
        writer.write_all(&line)?;
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct NativeParallelFilterConfig {
    threads: NonZeroUsize,
    batch_records: NonZeroUsize,
    queue_batches: NonZeroUsize,
}

impl NativeParallelFilterConfig {
    fn from_env(required: &RequiredFields) -> Result<Self> {
        let available = thread::available_parallelism().map_or(1, NonZeroUsize::get);
        Self::from_env_with_available_parallelism(required, available)
    }

    fn from_env_with_available_parallelism(
        required: &RequiredFields,
        available_parallelism: usize,
    ) -> Result<Self> {
        Ok(Self {
            threads: parse_filter_threads_env(required, available_parallelism)?,
            batch_records: parse_positive_env(
                NATIVE_FILTER_BATCH_RECORDS_ENV,
                NonZeroUsize::new(DEFAULT_PARALLEL_BATCH_RECORDS),
            )?,
            queue_batches: parse_positive_env(
                NATIVE_FILTER_QUEUE_BATCHES_ENV,
                NonZeroUsize::new(DEFAULT_PARALLEL_QUEUE_BATCHES),
            )?,
        })
    }

    fn enabled(&self) -> bool {
        self.threads.get() > 1
    }
}

fn native_pipeline_config_from_env(
    required: &RequiredFields,
    bgzf_threads: Option<NonZeroUsize>,
) -> Result<PipelineConfig> {
    let available = thread::available_parallelism().map_or(1, NonZeroUsize::get);
    let filter_config =
        NativeParallelFilterConfig::from_env_with_available_parallelism(required, available)?;

    Ok(PipelineConfig {
        bgzf_threads: bgzf_threads.map_or(1, NonZeroUsize::get),
        filter_threads: filter_config.threads.get(),
        batch_records: filter_config.batch_records.get(),
        queue_batches: filter_config.queue_batches.get(),
    })
}

fn native_filter_bgzf_threads(required: &RequiredFields) -> Result<Option<NonZeroUsize>> {
    if required.requires_format_aggregates() && env::var_os(NATIVE_BGZF_THREADS_ENV).is_none() {
        return Ok(NonZeroUsize::new(1));
    }

    native_bgzf_threads_from_env()
}

fn parse_filter_threads_env(
    required: &RequiredFields,
    available_parallelism: usize,
) -> Result<NonZeroUsize> {
    match env::var(NATIVE_FILTER_THREADS_ENV) {
        Ok(raw) if raw.eq_ignore_ascii_case("auto") => Ok(auto_filter_threads(
            required.requires_format_aggregates(),
            available_parallelism,
        )),
        Ok(raw) => {
            let value = raw.parse::<usize>().map_err(|_| {
                anyhow::anyhow!("{NATIVE_FILTER_THREADS_ENV} must be auto or a positive integer")
            })?;
            NonZeroUsize::new(value).ok_or_else(|| {
                anyhow::anyhow!("{NATIVE_FILTER_THREADS_ENV} must be auto or a positive integer")
            })
        }
        Err(env::VarError::NotPresent) => Ok(NonZeroUsize::new(1).unwrap()),
        Err(env::VarError::NotUnicode(_)) => {
            bail!("{NATIVE_FILTER_THREADS_ENV} must be valid UTF-8")
        }
    }
}

fn auto_filter_threads(
    enable_for_cpu_heavy_expression: bool,
    available_parallelism: usize,
) -> NonZeroUsize {
    if !enable_for_cpu_heavy_expression {
        return NonZeroUsize::new(1).unwrap();
    }
    let threads = available_parallelism.clamp(1, DEFAULT_AUTO_FILTER_THREAD_CAP);
    NonZeroUsize::new(threads).unwrap()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    fn aggregate_required_fields() -> RequiredFields {
        RequiredFields {
            format_aggregates: true,
            format_keys: vec![b"AD".to_vec()],
            ..RequiredFields::default()
        }
    }

    #[test]
    fn auto_filter_threads_stay_single_thread_for_site_only_predicates() {
        assert_eq!(auto_filter_threads(false, 8).get(), 1);
    }

    #[test]
    fn explicit_auto_filter_threads_enable_workers_for_format_aggregates() {
        assert_eq!(auto_filter_threads(true, 1).get(), 1);
        assert_eq!(auto_filter_threads(true, 2).get(), 2);
        assert_eq!(
            auto_filter_threads(true, 16).get(),
            DEFAULT_AUTO_FILTER_THREAD_CAP
        );
    }

    #[test]
    fn native_parallel_filter_config_default_is_conservative_for_format_aggregates() {
        let site_only = RequiredFields {
            qual: true,
            ..RequiredFields::default()
        };
        let aggregate = aggregate_required_fields();

        let site_config =
            NativeParallelFilterConfig::from_env_with_available_parallelism(&site_only, 8).unwrap();
        let aggregate_config =
            NativeParallelFilterConfig::from_env_with_available_parallelism(&aggregate, 8).unwrap();

        assert_eq!(site_config.threads.get(), 1);
        assert_eq!(aggregate_config.threads.get(), 1);
    }

    #[test]
    fn native_pipeline_config_defaults_to_single_bgzf_for_format_aggregates() {
        let site_only = RequiredFields {
            qual: true,
            ..RequiredFields::default()
        };
        let aggregate = aggregate_required_fields();

        let site_bgzf = native_filter_bgzf_threads(&site_only)
            .unwrap()
            .map(NonZeroUsize::get)
            .unwrap_or(0);
        let aggregate_bgzf = native_filter_bgzf_threads(&aggregate)
            .unwrap()
            .map(NonZeroUsize::get)
            .unwrap_or(0);

        assert!(site_bgzf >= 1);
        assert_eq!(aggregate_bgzf, 1);
    }

    #[test]
    fn original_byte_batch_rejects_header_after_records_start() {
        let expr = parse_expression("QUAL > 30").unwrap();
        let required = expr.required_fields();
        let batch = RecordBatch {
            sequence: 0,
            bytes: b"##fileformat=VCFv4.3\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n1\t100\trs1\tA\tG\t42\tPASS\tDP=1\n#late\n".to_vec(),
            record_count: 4,
        };
        let mut header_open = true;

        let error = evaluate_original_byte_batch(batch, &expr, &required, None, &mut header_open)
            .unwrap_err();

        assert!(
            error.to_string().contains("VCF record"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn native_bgzf_pipeline_uses_predicate_workers_for_bounded_batch_groups() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.vcf.gz");
        let mut plain = b"##fileformat=VCFv4.3\n\
#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1\tS2\n"
            .to_vec();
        for position in 1..=24 {
            plain.extend_from_slice(
                format!(
                    "1\t{position}\ttask5probe{position}\tA\tG\t42\tPASS\tDP=1\tAD\t10,90\t5,6\n"
                )
                .as_bytes(),
            );
        }
        let file = std::fs::File::create(&input).unwrap();
        let mut writer = noodles_bgzf::io::Writer::new(file);
        writer.write_all(&plain).unwrap();
        writer.finish().unwrap();

        let expr = parse_expression("ANY(FORMAT/AD[1] > 80)").unwrap();
        let required = expr.required_fields();
        let threads_seen = Arc::new(Mutex::new(HashSet::new()));
        *BGZF_BATCH_THREAD_OBSERVER.lock().unwrap() = Some(Arc::new({
            let threads_seen = Arc::clone(&threads_seen);
            move |thread_id, batch| {
                if batch
                    .bytes
                    .windows(b"task5probe".len())
                    .any(|window| window == b"task5probe")
                {
                    threads_seen.lock().unwrap().insert(thread_id);
                }
            }
        }));

        let mut output = Vec::new();
        let result = stream_native_bgzf_pipeline(
            &input,
            &mut output,
            &expr,
            &required,
            None,
            PipelineConfig {
                bgzf_threads: 1,
                filter_threads: 4,
                batch_records: 1,
                queue_batches: 4,
            },
        );
        *BGZF_BATCH_THREAD_OBSERVER.lock().unwrap() = None;
        result.unwrap();

        assert_eq!(output, plain);
        assert!(
            threads_seen.lock().unwrap().len() > 1,
            "expected real BGZF pipeline predicate batches to run on worker threads"
        );
    }
}
