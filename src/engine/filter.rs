use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::{Result, bail};

use crate::expr::{EvalRecord, FormatValues, RequiredFields, parse_expression};
use crate::io::{open_reader, open_writer};
use crate::vcf::{SiteRecord, parse_record_fields};

pub fn run(input: &Path, where_expr: &str, output: &Path) -> Result<()> {
    let expr = parse_expression(where_expr)?;
    let required = expr.required_fields();
    if required.requires_format() {
        bail!("FORMAT predicates require --sample <name>");
    }

    let mut reader = open_reader(input)?;
    let mut writer = open_writer(output)?;
    let mut line = String::new();

    while reader.read_line(&mut line)? != 0 {
        if line.starts_with('#') {
            writer.write_all(line.as_bytes())?;
            line.clear();
            continue;
        }

        let record = parse_eval_record_line(&line, required)?;
        if expr.evaluate(&record) {
            writer.write_all(line.as_bytes())?;
        }
        line.clear();
    }

    writer.flush()?;
    Ok(())
}

fn parse_eval_record_line(line: &str, required: RequiredFields) -> Result<EvalRecord<'_>> {
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

    Ok(EvalRecord {
        chrom,
        pos,
        qual,
        filter,
        info,
        format: FormatValues::default(),
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
        )
        .unwrap();

        assert_eq!(record.qual, Some(42.0));
        assert_eq!(record.info, "");
    }
}
