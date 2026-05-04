use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::{Context, Result};

use crate::expr::{EvalRecord, parse_expression};
use crate::io::{open_reader, open_writer};

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

        let record = parse_record(&line)?;
        if expr.evaluate(&record) {
            writer.write_all(line.as_bytes())?;
        }
        line.clear();
    }

    writer.flush()?;
    Ok(())
}

fn parse_record(line: &str) -> Result<EvalRecord<'_>> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    let columns: Vec<&str> = trimmed.split('\t').collect();

    if columns.len() < 8 {
        anyhow::bail!("VCF record has fewer than 8 columns: {trimmed}");
    }

    let pos = columns[1]
        .parse::<u64>()
        .with_context(|| format!("invalid POS value '{}'", columns[1]))?;
    let qual = parse_optional_float(columns[5])
        .with_context(|| format!("invalid QUAL value '{}'", columns[5]))?;

    Ok(EvalRecord {
        chrom: columns[0],
        pos,
        qual,
        filter: columns[6],
        info: columns[7],
    })
}

fn parse_optional_float(value: &str) -> Result<Option<f64>> {
    if value == "." {
        Ok(None)
    } else {
        Ok(Some(value.parse()?))
    }
}

#[cfg(test)]
mod tests {
    use super::parse_record;

    #[test]
    fn parses_site_level_record_fields() {
        let record = parse_record("1\t20\t.\tA\tG\t42\tPASS\tDP=11;AF=0.2\n").unwrap();

        assert_eq!(record.chrom, "1");
        assert_eq!(record.pos, 20);
        assert_eq!(record.qual, Some(42.0));
        assert_eq!(record.filter, "PASS");
        assert_eq!(record.info, "DP=11;AF=0.2");
    }

    #[test]
    fn dot_qual_is_missing() {
        let record = parse_record("1\t20\t.\tA\tG\t.\tPASS\tDP=11\n").unwrap();

        assert_eq!(record.qual, None);
    }
}
