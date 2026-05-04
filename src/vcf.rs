use anyhow::{Context, Result};
use serde::Serialize;

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

pub fn for_each_info_number(info: &str, key: &str, mut observe: impl FnMut(f64)) {
    if let Some((_, value)) = info
        .split(';')
        .filter_map(|entry| entry.split_once('='))
        .find(|(entry_key, _)| *entry_key == key)
    {
        let mut remaining = value;
        while !remaining.is_empty() {
            let (part, rest) = remaining.split_once(',').unwrap_or((remaining, ""));
            if part != "."
                && let Ok(number) = part.parse::<f64>()
            {
                observe(number);
            }
            remaining = rest;
            if remaining.is_empty() {
                break;
            }
        }
    }
}

pub fn info_value<'a>(info: &'a str, key: &str) -> Option<&'a str> {
    info.split(';')
        .filter_map(|entry| entry.split_once('='))
        .find(|(entry_key, _)| *entry_key == key)
        .map(|(_, value)| value)
}

#[cfg(test)]
mod tests {
    use super::{parse_record_fields, parse_record_line};

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
}
