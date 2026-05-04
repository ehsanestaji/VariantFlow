use anyhow::{Context, Result};
use serde::Serialize;

use crate::expr::{FormatValues, RequiredFormatFields};

#[derive(Debug, Clone, Copy)]
pub struct RecordFields<'a> {
    pub chrom: &'a str,
    pub pos: &'a str,
    pub id: &'a str,
    pub reference: &'a str,
    pub alternate: &'a str,
    pub qual: &'a str,
    pub filter: &'a str,
    pub info: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct VariantKey {
    pub chrom: String,
    pub pos: u64,
    pub reference: String,
    pub alternate: String,
}

#[derive(Debug, Clone)]
pub struct SiteRecord {
    pub chrom: String,
    pub pos: u64,
    pub reference: String,
    pub alternate: String,
    pub qual: Option<f64>,
    pub filter: String,
    pub info: String,
}

impl SiteRecord {
    pub fn key(&self) -> VariantKey {
        VariantKey {
            chrom: self.chrom.clone(),
            pos: self.pos,
            reference: self.reference.clone(),
            alternate: self.alternate.clone(),
        }
    }

    pub fn alt_alleles(&self) -> impl Iterator<Item = &str> {
        self.alternate.split(',')
    }
}

impl<'a> RecordFields<'a> {
    pub fn pos_u64(&self) -> Result<u64> {
        parse_u64_ascii(self.pos).with_context(|| format!("invalid POS value '{}'", self.pos))
    }

    pub fn qual_float(&self) -> Result<Option<f64>> {
        parse_optional_float(self.qual)
            .with_context(|| format!("invalid QUAL value '{}'", self.qual))
    }

    pub fn alt_alleles(&self) -> impl Iterator<Item = &'a str> {
        self.alternate.split(',')
    }
}

pub fn parse_record_fields(line: &str) -> Result<RecordFields<'_>> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    let bytes = trimmed.as_bytes();
    let mut fields = [""; 8];
    let mut start = 0;

    for (index, field) in fields.iter_mut().enumerate() {
        if start > bytes.len() {
            anyhow::bail!("VCF record has fewer than 8 columns: {trimmed}");
        }

        let end = bytes[start..]
            .iter()
            .position(|byte| *byte == b'\t')
            .map_or(bytes.len(), |offset| start + offset);

        if index < 7 && end == bytes.len() {
            anyhow::bail!("VCF record has fewer than 8 columns: {trimmed}");
        }

        *field = &trimmed[start..end];
        start = end.saturating_add(1);
    }

    Ok(RecordFields {
        chrom: fields[0],
        pos: fields[1],
        id: fields[2],
        reference: fields[3],
        alternate: fields[4],
        qual: fields[5],
        filter: fields[6],
        info: fields[7],
    })
}

pub fn parse_record_line(line: &str) -> Result<SiteRecord> {
    let fields = parse_record_fields(line)?;
    let pos = fields.pos_u64()?;
    let qual = fields.qual_float()?;

    Ok(SiteRecord {
        chrom: fields.chrom.to_string(),
        pos,
        reference: fields.reference.to_string(),
        alternate: fields.alternate.to_string(),
        qual,
        filter: fields.filter.to_string(),
        info: fields.info.to_string(),
    })
}

pub fn parse_optional_float(value: &str) -> Result<Option<f64>> {
    if value == "." {
        Ok(None)
    } else {
        Ok(Some(value.parse()?))
    }
}

pub fn parse_u64_ascii(value: &str) -> Result<u64> {
    if value.is_empty() {
        anyhow::bail!("empty unsigned integer");
    }

    let mut result = 0u64;
    for byte in value.bytes() {
        if !byte.is_ascii_digit() {
            anyhow::bail!("invalid unsigned integer '{value}'");
        }
        result = result
            .checked_mul(10)
            .and_then(|current| current.checked_add(u64::from(byte - b'0')))
            .ok_or_else(|| anyhow::anyhow!("unsigned integer overflow '{value}'"))?;
    }

    Ok(result)
}

pub fn resolve_sample_column(chrom_header: &str, sample: &str) -> Result<usize> {
    let header = chrom_header.trim_end_matches(['\r', '\n']);
    let mut found = None;

    for_each_tab_column(header, |column, value| {
        if column >= 9 && value == sample && found.is_none() {
            found = Some(column);
        }
    });

    found.ok_or_else(|| anyhow::anyhow!("sample '{sample}' not found in VCF header"))
}

pub fn column_value(line: &str, target_column: usize) -> Option<&str> {
    let line = line.trim_end_matches(['\r', '\n']);
    let mut found = None;

    for_each_tab_column(line, |column, value| {
        if column == target_column && found.is_none() {
            found = Some(value);
        }
    });

    found
}

pub fn selected_format_values<'a>(
    format: &'a str,
    sample: &'a str,
    required: RequiredFormatFields,
) -> FormatValues<'a> {
    if sample == "." {
        return FormatValues::default();
    }

    FormatValues {
        gt: required
            .gt
            .then(|| format_value(format, sample, "GT"))
            .flatten(),
        dp: required
            .dp
            .then(|| format_value(format, sample, "DP"))
            .flatten(),
        gq: required
            .gq
            .then(|| format_value(format, sample, "GQ"))
            .flatten(),
    }
}

pub fn for_each_info_number(info: &str, key: &str, mut observe: impl FnMut(f64)) {
    for_each_info_value(info, key, |value| {
        for_each_comma_value(value, |part| {
            if part != "."
                && !part.is_empty()
                && let Ok(number) = part.parse::<f64>()
            {
                observe(number);
            }
        });
    });
}

pub fn info_number_any(info: &str, key: &str, mut predicate: impl FnMut(f64) -> bool) -> bool {
    let mut matched = false;
    for_each_info_value(info, key, |value| {
        if !matched {
            matched = comma_value_any(value, |part| {
                part != "." && !part.is_empty() && part.parse::<f64>().is_ok_and(&mut predicate)
            });
        }
    });
    matched
}

pub fn info_value<'a>(info: &'a str, key: &str) -> Option<&'a str> {
    let mut found = None;
    for_each_info_value(info, key, |value| {
        if found.is_none() {
            found = Some(value);
        }
    });
    found
}

fn format_value<'a>(format: &'a str, sample: &'a str, key: &str) -> Option<&'a str> {
    let mut key_index = None;
    let mut index = 0;

    for_each_colon_value(format, |value| {
        if value == key && key_index.is_none() {
            key_index = Some(index);
        }
        index += 1;
    });

    let target_index = key_index?;
    let mut found = None;
    let mut index = 0;

    for_each_colon_value(sample, |value| {
        if index == target_index && found.is_none() {
            found = Some(value);
        }
        index += 1;
    });

    found
}

fn for_each_info_value<'a>(info: &'a str, key: &str, mut observe: impl FnMut(&'a str)) {
    if info == "." || key.is_empty() {
        return;
    }

    let bytes = info.as_bytes();
    let mut entry_start = 0;

    while entry_start <= bytes.len() {
        let entry_end = bytes[entry_start..]
            .iter()
            .position(|byte| *byte == b';')
            .map_or(bytes.len(), |offset| entry_start + offset);

        if entry_start < entry_end
            && let Some(eq_offset) = bytes[entry_start..entry_end]
                .iter()
                .position(|byte| *byte == b'=')
        {
            let key_end = entry_start + eq_offset;
            if &info[entry_start..key_end] == key {
                observe(&info[key_end + 1..entry_end]);
            }
        }

        if entry_end == bytes.len() {
            break;
        }
        entry_start = entry_end + 1;
    }
}

fn for_each_tab_column<'a>(value: &'a str, mut observe: impl FnMut(usize, &'a str)) {
    let bytes = value.as_bytes();
    let mut start = 0;
    let mut column = 0;

    while start <= bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|byte| *byte == b'\t')
            .map_or(bytes.len(), |offset| start + offset);

        observe(column, &value[start..end]);

        if end == bytes.len() {
            break;
        }
        start = end + 1;
        column += 1;
    }
}

fn for_each_colon_value<'a>(value: &'a str, mut observe: impl FnMut(&'a str)) {
    let bytes = value.as_bytes();
    let mut start = 0;

    while start <= bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|byte| *byte == b':')
            .map_or(bytes.len(), |offset| start + offset);

        observe(&value[start..end]);

        if end == bytes.len() {
            break;
        }
        start = end + 1;
    }
}

fn for_each_comma_value<'a>(value: &'a str, mut observe: impl FnMut(&'a str)) {
    let bytes = value.as_bytes();
    let mut start = 0;

    while start <= bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|byte| *byte == b',')
            .map_or(bytes.len(), |offset| start + offset);

        observe(&value[start..end]);

        if end == bytes.len() {
            break;
        }
        start = end + 1;
    }
}

fn comma_value_any(value: &str, mut predicate: impl FnMut(&str) -> bool) -> bool {
    let bytes = value.as_bytes();
    let mut start = 0;

    while start <= bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|byte| *byte == b',')
            .map_or(bytes.len(), |offset| start + offset);

        if predicate(&value[start..end]) {
            return true;
        }

        if end == bytes.len() {
            break;
        }
        start = end + 1;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{
        column_value, for_each_info_number, info_value, parse_record_fields, parse_record_line,
        resolve_sample_column, selected_format_values,
    };

    #[test]
    fn borrowed_record_fields_ignore_sample_columns_without_allocating_column_vec() {
        let fields =
            parse_record_fields("1\t20\trs1\tA\tG\t42\tPASS\tDP=11;AF=0.2\tGT:DP\t0/1:11\n")
                .unwrap();

        assert_eq!(fields.chrom, "1");
        assert_eq!(fields.pos, "20");
        assert_eq!(fields.id, "rs1");
        assert_eq!(fields.reference, "A");
        assert_eq!(fields.alternate, "G");
        assert_eq!(fields.qual, "42");
        assert_eq!(fields.filter, "PASS");
        assert_eq!(fields.info, "DP=11;AF=0.2");
        assert_eq!(fields.pos_u64().unwrap(), 20);
        assert_eq!(fields.qual_float().unwrap(), Some(42.0));
    }

    #[test]
    fn borrowed_record_fields_reject_short_records() {
        let error = parse_record_fields("1\t20\t.\tA\tG\t42\tPASS\n")
            .unwrap_err()
            .to_string();

        assert!(error.contains("fewer than 8 columns"));
    }

    #[test]
    fn parses_site_level_record_fields() {
        let record = parse_record_line("1\t20\t.\tA\tG\t42\tPASS\tDP=11;AF=0.2\n").unwrap();

        assert_eq!(record.chrom, "1");
        assert_eq!(record.pos, 20);
        assert_eq!(record.reference, "A");
        assert_eq!(record.alternate, "G");
        assert_eq!(record.qual, Some(42.0));
        assert_eq!(record.filter, "PASS");
        assert_eq!(record.info, "DP=11;AF=0.2");
    }

    #[test]
    fn dot_qual_is_missing() {
        let record = parse_record_line("1\t20\t.\tA\tG\t.\tPASS\tDP=11\n").unwrap();

        assert_eq!(record.qual, None);
    }

    #[test]
    fn info_value_scans_exact_keys_and_edge_values() {
        let info = "FLAG;XDP=999;DP=18;EMPTY=;AF=0.01,0.2;TRAIL=done";

        assert_eq!(info_value(info, "DP"), Some("18"));
        assert_eq!(info_value(info, "AF"), Some("0.01,0.2"));
        assert_eq!(info_value(info, "EMPTY"), Some(""));
        assert_eq!(info_value(info, "FLAG"), None);
        assert_eq!(info_value(info, "MISSING"), None);
    }

    #[test]
    fn info_number_scanner_visits_comma_separated_numeric_values_only() {
        let mut observed = Vec::new();

        for_each_info_number("AF=.;DP=12;AF2=9;AF=0.005,0.02,bad,.", "AF", |value| {
            observed.push(value);
        });

        assert_eq!(observed, vec![0.005, 0.02]);
    }

    #[test]
    fn resolves_sample_column_from_chrom_header() {
        let header = "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tHG002\tNA12878\n";

        assert_eq!(resolve_sample_column(header, "HG002").unwrap(), 9);
        assert_eq!(resolve_sample_column(header, "NA12878").unwrap(), 10);
    }

    #[test]
    fn unknown_sample_reports_clear_error() {
        let header = "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tHG002\n";
        let error = resolve_sample_column(header, "MISSING")
            .unwrap_err()
            .to_string();

        assert!(error.contains("sample 'MISSING' not found in VCF header"));
    }

    #[test]
    fn reads_absolute_record_columns_by_index() {
        let line = "1\t100\t.\tA\tG\t50\tPASS\t.\tGT:DP:GQ\t0/1:25:40\t0/0:5:10\n";

        assert_eq!(column_value(line, 8), Some("GT:DP:GQ"));
        assert_eq!(column_value(line, 9), Some("0/1:25:40"));
        assert_eq!(column_value(line, 10), Some("0/0:5:10"));
        assert_eq!(column_value(line, 11), None);
    }

    #[test]
    fn extracts_selected_sample_format_values() {
        let values = selected_format_values(
            "GT:DP:GQ",
            "0/1:25:40",
            crate::expr::RequiredFormatFields {
                gt: true,
                dp: true,
                gq: true,
            },
        );

        assert_eq!(values.gt, Some("0/1"));
        assert_eq!(values.dp, Some("25"));
        assert_eq!(values.gq, Some("40"));
    }

    #[test]
    fn missing_format_values_return_none() {
        let values = selected_format_values(
            "GT:DP:GQ",
            "0/1:.",
            crate::expr::RequiredFormatFields {
                gt: true,
                dp: true,
                gq: true,
            },
        );

        assert_eq!(values.gt, Some("0/1"));
        assert_eq!(values.dp, Some("."));
        assert_eq!(values.gq, None);
    }
}
