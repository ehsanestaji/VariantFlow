use anyhow::{Context, Result};
use serde::Serialize;

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

pub fn parse_record_line(line: &str) -> Result<SiteRecord> {
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

    Ok(SiteRecord {
        chrom: columns[0].to_string(),
        pos,
        reference: columns[3].to_string(),
        alternate: columns[4].to_string(),
        qual,
        filter: columns[6].to_string(),
        info: columns[7].to_string(),
    })
}

pub fn parse_optional_float(value: &str) -> Result<Option<f64>> {
    if value == "." {
        Ok(None)
    } else {
        Ok(Some(value.parse()?))
    }
}

pub fn info_numbers(info: &str, key: &str) -> Vec<f64> {
    info.split(';')
        .filter_map(|entry| entry.split_once('='))
        .find(|(entry_key, _)| *entry_key == key)
        .map(|(_, value)| {
            value
                .split(',')
                .filter_map(|part| {
                    if part == "." {
                        None
                    } else {
                        part.parse::<f64>().ok()
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn info_value<'a>(info: &'a str, key: &str) -> Option<&'a str> {
    info.split(';')
        .filter_map(|entry| entry.split_once('='))
        .find(|(entry_key, _)| *entry_key == key)
        .map(|(_, value)| value)
}

#[cfg(test)]
mod tests {
    use super::parse_record_line;

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
