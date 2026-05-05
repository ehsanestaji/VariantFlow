use std::env;
use std::io::{BufRead, Write};
use std::num::NonZeroUsize;
use std::path::Path;

use anyhow::{Context, Result, bail};
use rayon::ThreadPool;
use rayon::prelude::*;

use crate::compat::{Backend, CompressionMode, Region, select_backend};
use crate::expr::{EvalContext, Expression, RequiredFields, parse_expression};
use crate::io::{open_reader, open_vcf_writer};
use crate::vcf::{self, InfoView, RecordView, column_value, resolve_sample_column};

pub const NATIVE_FILTER_THREADS_ENV: &str = "VCF_FAST_NATIVE_FILTER_THREADS";
pub const NATIVE_FILTER_BATCH_RECORDS_ENV: &str = "VCF_FAST_NATIVE_FILTER_BATCH_RECORDS";
const DEFAULT_PARALLEL_BATCH_RECORDS: usize = 8192;

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
        vcf::format_value_bytes(format, sample, key)
    }

    fn any_format_value(&self, key: &[u8], predicate: &mut dyn FnMut(&[u8]) -> bool) -> bool {
        let Some(format) = self.format_column else {
            return false;
        };

        let mut matched = false;
        self.record.for_each_sample_column(|sample| {
            if matched {
                return;
            }
            if let Some(value) = vcf::format_value_bytes(format, sample, key) {
                matched = predicate(value);
            }
        });
        matched
    }

    fn all_format_value(&self, key: &[u8], mut predicate: &mut dyn FnMut(&[u8]) -> bool) -> bool {
        let Some(format) = self.format_column else {
            return false;
        };

        let mut saw_sample = false;
        let mut all_match = true;
        self.record.for_each_sample_column(|sample| {
            saw_sample = true;
            if !all_match {
                return;
            }
            all_match = vcf::format_value_bytes(format, sample, key).is_some_and(&mut predicate);
        });
        saw_sample && all_match
    }

    fn count_format_value(&self, key: &[u8], predicate: &mut dyn FnMut(&[u8]) -> bool) -> u64 {
        let Some(format) = self.format_column else {
            return 0;
        };

        let mut count = 0;
        self.record.for_each_sample_column(|sample| {
            if let Some(value) = vcf::format_value_bytes(format, sample, key)
                && predicate(value)
            {
                count += 1;
            }
        });
        count
    }
}
