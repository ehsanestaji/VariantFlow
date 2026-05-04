use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::{Result, bail};

use crate::expr::{EvalRecord, FormatValues, RequiredFields, parse_expression};
use crate::io::{open_reader, open_writer};
use crate::vcf::{
    SiteRecord, column_value, parse_record_fields, resolve_sample_column, selected_format_values,
};

pub fn run(input: &Path, where_expr: &str, sample: Option<&str>, output: &Path) -> Result<()> {
    let expr = parse_expression(where_expr)?;
    let required = expr.required_fields();
    if required.requires_format() && sample.is_none() {
        bail!("FORMAT predicates require --sample <name>");
    }

    let mut reader = open_reader(input)?;
    let mut writer = open_writer(output)?;
    let mut line = String::new();
    let mut sample_column = None;

    while reader.read_line(&mut line)? != 0 {
        if line.starts_with('#') {
            if required.requires_format() && line.starts_with("#CHROM\t") {
                if column_value(&line, 9).is_none() {
                    bail!("FORMAT predicates require #CHROM header with sample columns");
                }
                sample_column = Some(resolve_sample_column(&line, sample.unwrap())?);
            }
            writer.write_all(line.as_bytes())?;
            line.clear();
            continue;
        }

        if required.requires_format() && sample_column.is_none() {
            bail!("FORMAT predicates require #CHROM header with sample columns");
        }

        let record = parse_eval_record_line(&line, required, sample_column)?;
        if expr.evaluate(&record) {
            writer.write_all(line.as_bytes())?;
        }
        line.clear();
    }

    writer.flush()?;
    Ok(())
}

fn parse_eval_record_line(
    line: &str,
    required: RequiredFields,
    sample_column: Option<usize>,
) -> Result<EvalRecord<'_>> {
    let fields = parse_record_fields(line)?;
    let chrom = if required.chrom { fields.chrom } else { "" };
    let pos = if required.pos { fields.pos_u64()? } else { 0 };
    let qual = if required.qual {
        fields.qual_float()?
    } else {
        None
    };
    let filter = if required.filter { fields.filter } else { "" };
    let info = if required.info { fields.info } else { "" };
    let format = if required.requires_format() {
        let format_column = column_value(line, 8).unwrap_or("");
        let sample_value = sample_column
            .and_then(|column| column_value(line, column))
            .unwrap_or(".");
        selected_format_values(format_column, sample_value, required.format)
    } else {
        FormatValues::default()
    };

    Ok(EvalRecord {
        chrom,
        pos,
        qual,
        filter,
        info,
        format,
    })
}

impl<'a> From<&'a SiteRecord> for EvalRecord<'a> {
    fn from(record: &'a SiteRecord) -> Self {
        Self {
            chrom: &record.chrom,
            pos: record.pos,
            qual: record.qual,
            filter: &record.filter,
            info: &record.info,
            format: FormatValues::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_eval_record_line;
    use crate::expr::RequiredFields;

    #[test]
    fn parses_borrowed_eval_record_without_reconstructing_site_record() {
        let record = parse_eval_record_line(
            "1\t20\t.\tA\tG\t42\tPASS\tDP=11;AF=0.2\n",
            RequiredFields {
                chrom: true,
                pos: true,
                qual: true,
                filter: true,
                info: true,
                format: Default::default(),
            },
            None,
        )
        .unwrap();

        assert_eq!(record.chrom, "1");
        assert_eq!(record.pos, 20);
        assert_eq!(record.qual, Some(42.0));
        assert_eq!(record.filter, "PASS");
        assert_eq!(record.info, "DP=11;AF=0.2");
    }

    #[test]
    fn borrowed_eval_record_treats_dot_qual_as_missing() {
        let record = parse_eval_record_line(
            "1\t20\t.\tA\tG\t.\tPASS\tDP=11\n",
            RequiredFields {
                qual: true,
                ..RequiredFields::default()
            },
            None,
        )
        .unwrap();

        assert_eq!(record.qual, None);
    }

    #[test]
    fn borrowed_eval_record_skips_unneeded_info_column() {
        let record = parse_eval_record_line(
            "1\t20\t.\tA\tG\t42\tPASS\tDP=11\n",
            RequiredFields {
                qual: true,
                ..RequiredFields::default()
            },
            None,
        )
        .unwrap();

        assert_eq!(record.qual, Some(42.0));
        assert_eq!(record.info, "");
    }
}
