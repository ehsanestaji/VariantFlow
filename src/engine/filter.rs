use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::Result;

use crate::expr::{EvalRecord, parse_expression};
use crate::io::{open_reader, open_writer};
use crate::vcf::{SiteRecord, parse_record_line};

pub fn run(input: &Path, where_expr: &str, output: &Path) -> Result<()> {
    let expr = parse_expression(where_expr)?;
    let mut reader = open_reader(input)?;
    let mut writer = open_writer(output)?;
    let mut line = String::new();

    while reader.read_line(&mut line)? != 0 {
        if line.starts_with('#') {
            writer.write_all(line.as_bytes())?;
            line.clear();
            continue;
        }

        let record = parse_record_line(&line)?;
        if expr.evaluate(&EvalRecord::from(&record)) {
            writer.write_all(line.as_bytes())?;
        }
        line.clear();
    }

    writer.flush()?;
    Ok(())
}

impl<'a> From<&'a SiteRecord> for EvalRecord<'a> {
    fn from(record: &'a SiteRecord) -> Self {
        Self {
            chrom: &record.chrom,
            pos: record.pos,
            qual: record.qual,
            filter: &record.filter,
            info: &record.info,
        }
    }
}
