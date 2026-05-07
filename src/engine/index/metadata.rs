use std::collections::BTreeSet;

use anyhow::Result;
use memchr::memchr;

use crate::engine::index::schema::IndexChunk;
use crate::vcf::RecordView;

#[derive(Debug)]
pub(crate) struct ChunkMetadataBuilder {
    ordinal: u64,
    first_record: u64,
    virtual_start: Option<u64>,
    record_count: u64,
    chrom_start: String,
    chrom_end: String,
    pos_min: u64,
    pos_max: u64,
    qual_min: Option<f64>,
    qual_max: Option<f64>,
    filters: BTreeSet<String>,
    info_dp_min: Option<i64>,
    info_dp_max: Option<i64>,
    info_af_min: Option<f64>,
    info_af_max: Option<f64>,
    info_af_seen: bool,
    info_af_complete: bool,
    format_keys: BTreeSet<String>,
}

impl ChunkMetadataBuilder {
    pub(crate) fn new(ordinal: u64, first_record: u64, virtual_start: Option<u64>) -> Self {
        Self {
            ordinal,
            first_record,
            virtual_start,
            record_count: 0,
            chrom_start: String::new(),
            chrom_end: String::new(),
            pos_min: u64::MAX,
            pos_max: 0,
            qual_min: None,
            qual_max: None,
            filters: BTreeSet::new(),
            info_dp_min: None,
            info_dp_max: None,
            info_af_min: None,
            info_af_max: None,
            info_af_seen: false,
            info_af_complete: true,
            format_keys: BTreeSet::new(),
        }
    }

    pub(crate) fn observe(&mut self, record: &RecordView<'_>) -> Result<()> {
        let chrom = String::from_utf8_lossy(record.chrom()).into_owned();
        if self.record_count == 0 {
            self.chrom_start = chrom.clone();
        }
        self.chrom_end = chrom;

        let pos = record.pos_u64()?;
        self.pos_min = self.pos_min.min(pos);
        self.pos_max = self.pos_max.max(pos);

        if let Some(qual) = record.qual_float()? {
            update_f64_minmax(&mut self.qual_min, &mut self.qual_max, qual);
        }

        observe_filters(record.filter(), &mut self.filters);
        observe_info(record.info(), self);

        if let Some(format) = record.column(8) {
            observe_format_keys(format, &mut self.format_keys);
        }

        self.record_count += 1;
        Ok(())
    }

    pub(crate) fn record_count(&self) -> u64 {
        self.record_count
    }

    pub(crate) fn finish(self, virtual_end: Option<u64>) -> Option<IndexChunk> {
        (self.record_count > 0).then(|| IndexChunk {
            ordinal: self.ordinal,
            first_record: self.first_record,
            record_count: self.record_count,
            chrom_start: self.chrom_start,
            chrom_end: self.chrom_end,
            pos_min: self.pos_min,
            pos_max: self.pos_max,
            qual_min: self.qual_min,
            qual_max: self.qual_max,
            filters: self.filters.into_iter().collect(),
            info_dp_min: self.info_dp_min,
            info_dp_max: self.info_dp_max,
            has_info_af: self.info_af_seen,
            info_af_min: self.info_af_min,
            info_af_max: self.info_af_max,
            info_af_complete: self.info_af_seen && self.info_af_complete,
            format_keys: self.format_keys.into_iter().collect(),
            virtual_start: self.virtual_start,
            virtual_end,
        })
    }
}

fn observe_info(info: &[u8], builder: &mut ChunkMetadataBuilder) {
    if info.is_empty() || info == b"." {
        return;
    }

    for_each_delimited(info, b';', |entry| {
        if entry.is_empty() {
            return;
        }
        let Some(eq_offset) = memchr(b'=', entry) else {
            return;
        };
        let key = &entry[..eq_offset];
        let value = &entry[eq_offset + 1..];
        match key {
            b"DP" => {
                for_each_comma_i64(value, |number| {
                    update_i64_minmax(&mut builder.info_dp_min, &mut builder.info_dp_max, number);
                });
            }
            b"AF" => {
                builder.info_af_seen = true;
                observe_af_values(value, builder);
            }
            _ => {}
        }
    });
}

fn observe_af_values(value: &[u8], builder: &mut ChunkMetadataBuilder) {
    let mut saw_part = false;
    for_each_delimited(value, b',', |part| {
        saw_part = true;
        if part.is_empty() || part == b"." {
            builder.info_af_complete = false;
            return;
        }
        match parse_f64(part) {
            Some(number) if number.is_finite() => {
                update_f64_minmax(&mut builder.info_af_min, &mut builder.info_af_max, number);
            }
            _ => {
                builder.info_af_complete = false;
            }
        }
    });
    if !saw_part {
        builder.info_af_complete = false;
    }
}

fn observe_filters(filter: &[u8], filters: &mut BTreeSet<String>) {
    if filter.is_empty() || filter == b"." {
        return;
    }

    for_each_delimited(filter, b';', |part| {
        if !part.is_empty() && part != b"." {
            filters.insert(String::from_utf8_lossy(part).into_owned());
        }
    });
}

fn observe_format_keys(format: &[u8], format_keys: &mut BTreeSet<String>) {
    if format.is_empty() || format == b"." {
        return;
    }

    for_each_delimited(format, b':', |part| {
        if !part.is_empty() {
            format_keys.insert(String::from_utf8_lossy(part).into_owned());
        }
    });
}

fn for_each_comma_i64(value: &[u8], mut observe: impl FnMut(i64)) {
    if value.is_empty() || value == b"." {
        return;
    }

    for_each_delimited(value, b',', |part| {
        if !part.is_empty()
            && part != b"."
            && let Ok(text) = std::str::from_utf8(part)
            && let Ok(number) = text.parse::<i64>()
        {
            observe(number);
        }
    });
}

fn for_each_delimited(mut value: &[u8], delimiter: u8, mut observe: impl FnMut(&[u8])) {
    loop {
        let end = memchr(delimiter, value).unwrap_or(value.len());
        observe(&value[..end]);
        if end == value.len() {
            break;
        }
        value = &value[end + 1..];
    }
}

fn parse_f64(value: &[u8]) -> Option<f64> {
    std::str::from_utf8(value).ok()?.parse::<f64>().ok()
}

fn update_i64_minmax(min: &mut Option<i64>, max: &mut Option<i64>, value: i64) {
    *min = Some(min.map_or(value, |current| current.min(value)));
    *max = Some(max.map_or(value, |current| current.max(value)));
}

fn update_f64_minmax(min: &mut Option<f64>, max: &mut Option<f64>, value: f64) {
    if !value.is_finite() {
        return;
    }

    *min = Some(min.map_or(value, |current| current.min(value)));
    *max = Some(max.map_or(value, |current| current.max(value)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vcf::RecordView;

    #[test]
    fn chunk_builder_tracks_qual_filter_info_and_format_metadata() {
        let record = RecordView::parse(
            b"chr1\t10\t.\tA\tG\t50\tPASS\tDP=12;AF=0.2,0.3\tGT:DP:AD\t0/1:12:6,6\n",
        )
        .unwrap();

        let mut builder = ChunkMetadataBuilder::new(0, 0, None);
        builder.observe(&record).unwrap();
        assert_eq!(builder.record_count(), 1);
        let chunk = builder.finish(Some(65_536)).unwrap();

        assert_eq!(chunk.record_count, 1);
        assert_eq!(chunk.chrom_start, "chr1");
        assert_eq!(chunk.pos_min, 10);
        assert_eq!(chunk.qual_min, Some(50.0));
        assert_eq!(chunk.qual_max, Some(50.0));
        assert_eq!(chunk.info_dp_min, Some(12));
        assert_eq!(chunk.info_dp_max, Some(12));
        assert_eq!(chunk.info_af_min, Some(0.2));
        assert_eq!(chunk.info_af_max, Some(0.3));
        assert!(chunk.info_af_complete);
        assert_eq!(chunk.filters, vec!["PASS"]);
        assert_eq!(chunk.format_keys, vec!["AD", "DP", "GT"]);
        assert_eq!(chunk.virtual_start, None);
        assert_eq!(chunk.virtual_end, Some(65_536));
    }

    #[test]
    fn chunk_builder_marks_af_incomplete_when_any_value_is_non_numeric() {
        let record = RecordView::parse(b"chr1\t10\t.\tA\tG\t50\tPASS\tAF=.\n").unwrap();

        let mut builder = ChunkMetadataBuilder::new(0, 0, Some(131_072));
        builder.observe(&record).unwrap();
        let chunk = builder.finish(Some(196_608)).unwrap();

        assert!(chunk.has_info_af);
        assert!(!chunk.info_af_complete);
        assert_eq!(chunk.info_af_min, None);
        assert_eq!(chunk.info_af_max, None);
        assert_eq!(chunk.virtual_start, Some(131_072));
        assert_eq!(chunk.virtual_end, Some(196_608));
    }
}
