use anyhow::{Context, Result};
use memchr::memchr;
use serde::Serialize;

use crate::expr::RequiredFormatFields;

const CORE_FIELD_COUNT: usize = 8;

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

#[derive(Debug, Clone, Copy)]
pub(crate) struct RecordView<'a> {
    line: &'a [u8],
    fields: [(usize, usize); CORE_FIELD_COUNT],
    line_end: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct InfoView<'a> {
    info: &'a [u8],
    dp: Option<&'a [u8]>,
    af: Option<&'a [u8]>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct FormatValueBytes<'a> {
    pub(crate) gt: Option<&'a [u8]>,
    pub(crate) dp: Option<&'a [u8]>,
    pub(crate) gq: Option<&'a [u8]>,
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

impl<'a> RecordView<'a> {
    pub(crate) fn parse(line: &'a [u8]) -> Result<Self> {
        let line_end = trim_line_end(line).len();
        let mut fields = [(0usize, 0usize); CORE_FIELD_COUNT];
        let mut start = 0;

        for (index, field) in fields.iter_mut().enumerate() {
            if start > line_end {
                anyhow::bail!(
                    "VCF record has fewer than 8 columns: {}",
                    String::from_utf8_lossy(trim_line_end(line))
                );
            }

            let end =
                memchr(b'\t', &line[start..line_end]).map_or(line_end, |offset| start + offset);

            if index < CORE_FIELD_COUNT - 1 && end == line_end {
                anyhow::bail!(
                    "VCF record has fewer than 8 columns: {}",
                    String::from_utf8_lossy(trim_line_end(line))
                );
            }

            *field = (start, end);
            start = end.saturating_add(1);
        }

        Ok(Self {
            line,
            fields,
            line_end,
        })
    }

    pub(crate) fn chrom(&self) -> &'a [u8] {
        self.core_field(0)
    }

    pub(crate) fn pos(&self) -> &'a [u8] {
        self.core_field(1)
    }

    #[allow(dead_code)]
    pub(crate) fn id(&self) -> &'a [u8] {
        self.core_field(2)
    }

    pub(crate) fn reference(&self) -> &'a [u8] {
        self.core_field(3)
    }

    pub(crate) fn alternate(&self) -> &'a [u8] {
        self.core_field(4)
    }

    pub(crate) fn qual(&self) -> &'a [u8] {
        self.core_field(5)
    }

    pub(crate) fn filter(&self) -> &'a [u8] {
        self.core_field(6)
    }

    pub(crate) fn info(&self) -> &'a [u8] {
        self.core_field(7)
    }

    pub(crate) fn core_field(&self, index: usize) -> &'a [u8] {
        let (start, end) = self.fields[index];
        &self.line[start..end]
    }

    pub(crate) fn column(&self, target_column: usize) -> Option<&'a [u8]> {
        if target_column < CORE_FIELD_COUNT {
            return Some(self.core_field(target_column));
        }

        let mut column = CORE_FIELD_COUNT;
        let mut start = self.fields[CORE_FIELD_COUNT - 1].1.saturating_add(1);

        while start <= self.line_end {
            let end = memchr(b'\t', &self.line[start..self.line_end])
                .map_or(self.line_end, |offset| start + offset);

            if column == target_column {
                return Some(&self.line[start..end]);
            }

            if end == self.line_end {
                break;
            }

            start = end + 1;
            column += 1;
        }

        None
    }

    pub(crate) fn pos_u64(&self) -> Result<u64> {
        parse_u64_ascii_bytes(self.pos()).with_context(|| {
            format!(
                "invalid POS value '{}'",
                String::from_utf8_lossy(self.pos())
            )
        })
    }

    pub(crate) fn qual_float(&self) -> Result<Option<f64>> {
        parse_optional_float_bytes(self.qual()).with_context(|| {
            format!(
                "invalid QUAL value '{}'",
                String::from_utf8_lossy(self.qual())
            )
        })
    }
}

impl<'a> InfoView<'a> {
    pub(crate) fn scan(info: &'a [u8]) -> Self {
        if info == b"." {
            return Self {
                info,
                ..Self::default()
            };
        }

        let mut view = Self {
            info,
            ..Self::default()
        };
        let mut entry_start = 0;

        while entry_start <= info.len() {
            let entry_end = memchr(b';', &info[entry_start..])
                .map_or(info.len(), |offset| entry_start + offset);

            if entry_start < entry_end
                && let Some(eq_offset) = memchr(b'=', &info[entry_start..entry_end])
            {
                let key_end = entry_start + eq_offset;
                let value = &info[key_end + 1..entry_end];
                match &info[entry_start..key_end] {
                    b"DP" if view.dp.is_none() => view.dp = Some(value),
                    b"AF" if view.af.is_none() => view.af = Some(value),
                    _ => {}
                }
            }

            if entry_end == info.len() {
                break;
            }
            entry_start = entry_end + 1;
        }

        view
    }

    pub(crate) fn value(&self, key: &[u8]) -> Option<&'a [u8]> {
        match key {
            b"DP" => self.dp,
            b"AF" => self.af,
            _ => self.scan_value(key),
        }
    }

    pub(crate) fn number_any(&self, key: &[u8], mut predicate: impl FnMut(f64) -> bool) -> bool {
        self.value(key)
            .is_some_and(|value| comma_number_any_bytes(value, &mut predicate))
    }

    pub(crate) fn for_each_number(&self, key: &[u8], mut observe: impl FnMut(f64)) {
        if let Some(value) = self.value(key) {
            for_each_comma_number_bytes(value, &mut observe);
        }
    }

    fn scan_value(&self, key: &[u8]) -> Option<&'a [u8]> {
        if self.info == b"." || key.is_empty() {
            return None;
        }

        let mut entry_start = 0;

        while entry_start <= self.info.len() {
            let entry_end = memchr(b';', &self.info[entry_start..])
                .map_or(self.info.len(), |offset| entry_start + offset);

            if entry_start < entry_end
                && let Some(eq_offset) = memchr(b'=', &self.info[entry_start..entry_end])
            {
                let key_end = entry_start + eq_offset;
                if &self.info[entry_start..key_end] == key {
                    return Some(&self.info[key_end + 1..entry_end]);
                }
            }

            if entry_end == self.info.len() {
                break;
            }
            entry_start = entry_end + 1;
        }

        None
    }
}

fn trim_line_end(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r").unwrap_or(line)
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

pub(crate) fn parse_optional_float_bytes(value: &[u8]) -> Result<Option<f64>> {
    if value == b"." {
        Ok(None)
    } else {
        Ok(Some(std::str::from_utf8(value)?.parse()?))
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

pub(crate) fn parse_u64_ascii_bytes(value: &[u8]) -> Result<u64> {
    if value.is_empty() {
        anyhow::bail!("empty unsigned integer");
    }

    let mut result = 0u64;
    for byte in value {
        if !byte.is_ascii_digit() {
            anyhow::bail!(
                "invalid unsigned integer '{}'",
                String::from_utf8_lossy(value)
            );
        }
        result = result
            .checked_mul(10)
            .and_then(|current| current.checked_add(u64::from(byte - b'0')))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "unsigned integer overflow '{}'",
                    String::from_utf8_lossy(value)
                )
            })?;
    }

    Ok(result)
}

pub(crate) fn resolve_sample_column(chrom_header: &str, sample: &str) -> Result<usize> {
    let header = chrom_header.trim_end_matches(['\r', '\n']);
    let bytes = header.as_bytes();
    let mut start = 0;
    let mut column = 0;

    while start <= bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|byte| *byte == b'\t')
            .map_or(bytes.len(), |offset| start + offset);

        if column >= 9 && &header[start..end] == sample {
            return Ok(column);
        }

        if end == bytes.len() {
            break;
        }
        start = end + 1;
        column += 1;
    }

    Err(anyhow::anyhow!("sample '{sample}' not found in VCF header"))
}

pub(crate) fn column_value(line: &str, target_column: usize) -> Option<&str> {
    let line = line.trim_end_matches(['\r', '\n']);
    let bytes = line.as_bytes();
    let mut start = 0;
    let mut column = 0;

    while start <= bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|byte| *byte == b'\t')
            .map_or(bytes.len(), |offset| start + offset);

        if column == target_column {
            return Some(&line[start..end]);
        }

        if end == bytes.len() {
            break;
        }
        start = end + 1;
        column += 1;
    }

    None
}

#[allow(dead_code)]
pub(crate) fn selected_format_values_bytes<'sample>(
    format: &[u8],
    sample: &'sample [u8],
    required: RequiredFormatFields,
) -> FormatValueBytes<'sample> {
    if sample == b"." {
        return FormatValueBytes::default();
    }

    FormatValueBytes {
        gt: required
            .gt
            .then(|| format_value_bytes(format, sample, b"GT"))
            .flatten(),
        dp: required
            .dp
            .then(|| format_value_bytes(format, sample, b"DP"))
            .flatten(),
        gq: required
            .gq
            .then(|| format_value_bytes(format, sample, b"GQ"))
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

pub(crate) fn format_value_bytes<'sample>(
    format: &[u8],
    sample: &'sample [u8],
    key: &[u8],
) -> Option<&'sample [u8]> {
    let mut key_index = None;
    let mut index = 0;

    for_each_delimited_byte_value(format, b':', |value| {
        if value == key && key_index.is_none() {
            key_index = Some(index);
        }
        index += 1;
    });

    let target_index = key_index?;
    let mut found = None;
    let mut index = 0;

    for_each_delimited_byte_value(sample, b':', |value| {
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

fn for_each_delimited_byte_value<'a>(
    value: &'a [u8],
    delimiter: u8,
    mut observe: impl FnMut(&'a [u8]),
) {
    let mut start = 0;

    while start <= value.len() {
        let end = memchr(delimiter, &value[start..]).map_or(value.len(), |offset| start + offset);

        observe(&value[start..end]);

        if end == value.len() {
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

fn comma_number_any_bytes(value: &[u8], predicate: &mut impl FnMut(f64) -> bool) -> bool {
    let mut start = 0;

    while start <= value.len() {
        let end = memchr(b',', &value[start..]).map_or(value.len(), |offset| start + offset);
        let part = &value[start..end];

        if part != b"."
            && !part.is_empty()
            && let Ok(text) = std::str::from_utf8(part)
            && let Ok(number) = text.parse::<f64>()
            && predicate(number)
        {
            return true;
        }

        if end == value.len() {
            break;
        }
        start = end + 1;
    }

    false
}

fn for_each_comma_number_bytes(value: &[u8], observe: &mut impl FnMut(f64)) {
    let mut start = 0;

    while start <= value.len() {
        let end = memchr(b',', &value[start..]).map_or(value.len(), |offset| start + offset);
        let part = &value[start..end];

        if part != b"."
            && !part.is_empty()
            && let Ok(text) = std::str::from_utf8(part)
            && let Ok(number) = text.parse::<f64>()
        {
            observe(number);
        }

        if end == value.len() {
            break;
        }
        start = end + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        InfoView, RecordView, column_value, for_each_info_number, info_value, parse_record_fields,
        parse_record_line, resolve_sample_column, selected_format_values_bytes,
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
    fn byte_record_view_handles_core_fields_and_crlf() {
        let view = RecordView::parse(b"1\t20\trs1\tA\tC,G\t.\tPASS\tDP=11;AF=0.1,0.2\r\n").unwrap();

        assert_eq!(view.chrom(), b"1");
        assert_eq!(view.pos_u64().unwrap(), 20);
        assert_eq!(view.id(), b"rs1");
        assert_eq!(view.reference(), b"A");
        assert_eq!(view.alternate(), b"C,G");
        assert_eq!(view.qual_float().unwrap(), None);
        assert_eq!(view.filter(), b"PASS");
        assert_eq!(view.info(), b"DP=11;AF=0.1,0.2");
        assert_eq!(view.column(8), None);
    }

    #[test]
    fn byte_record_view_accesses_format_and_selected_samples_without_collecting_columns() {
        let view =
            RecordView::parse(b"1\t20\t.\tA\tG\t42\tPASS\tDP=11\tGT:DP:GQ\t0/1:25:40\t0/0:5:10\n")
                .unwrap();

        assert_eq!(view.column(8), Some(b"GT:DP:GQ".as_slice()));
        assert_eq!(view.column(9), Some(b"0/1:25:40".as_slice()));
        assert_eq!(view.column(10), Some(b"0/0:5:10".as_slice()));
        assert_eq!(view.column(11), None);
    }

    #[test]
    fn byte_record_view_rejects_short_records() {
        let error = RecordView::parse(b"1\t20\t.\tA\tG\t42\tPASS\n")
            .unwrap_err()
            .to_string();

        assert!(error.contains("fewer than 8 columns"));
    }

    #[test]
    fn byte_info_view_scans_exact_values_once() {
        let info = InfoView::scan(b"FLAG;XDP=999;DP=18;EMPTY=;AF=0.005,0.02;TRAIL=done");

        assert_eq!(info.value(b"DP"), Some(b"18".as_slice()));
        assert_eq!(info.value(b"AF"), Some(b"0.005,0.02".as_slice()));
        assert_eq!(info.value(b"EMPTY"), Some(b"".as_slice()));
        assert_eq!(info.value(b"FLAG"), None);
        assert_eq!(info.value(b"MISSING"), None);
        assert!(info.number_any(b"AF", |value| value > 0.01));
        assert!(!InfoView::scan(b"AF=.;DP=bad").number_any(b"AF", |value| value > 0.01));
    }

    #[test]
    fn byte_format_values_extract_only_required_sample_fields() {
        let values = selected_format_values_bytes(
            b"GT:DP:GQ:AD",
            b"0/1:25:40:10,15",
            crate::expr::RequiredFormatFields {
                gt: true,
                dp: true,
                gq: true,
            },
        );

        assert_eq!(values.gt, Some(b"0/1".as_slice()));
        assert_eq!(values.dp, Some(b"25".as_slice()));
        assert_eq!(values.gq, Some(b"40".as_slice()));

        let missing = selected_format_values_bytes(
            b"GT:DP:GQ",
            b"0/1:.",
            crate::expr::RequiredFormatFields {
                gt: true,
                dp: true,
                gq: true,
            },
        );
        assert_eq!(missing.gt, Some(b"0/1".as_slice()));
        assert_eq!(missing.dp, Some(b".".as_slice()));
        assert_eq!(missing.gq, None);
    }
}
