use std::fs::File;
use std::io::{BufRead, Write};
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use arrow_array::builder::{Float64Builder, Int64Builder, StringBuilder};
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use memchr::{memchr, memchr2};
use parquet::arrow::ArrowWriter;

use crate::compat::{Backend, Region, select_backend};
use crate::io::{open_reader, open_writer};

const PARQUET_BATCH_ROWS: usize = 8192;

pub fn run(input: &Path, target: &str, output: &Path, region: Option<&Region>) -> Result<()> {
    match target {
        "tsv" => convert_to_tsv(input, output, region),
        "parquet" => convert_to_parquet(input, output, region),
        other => bail!("unsupported convert target '{other}'; supported targets: tsv, parquet"),
    }
}

fn convert_to_tsv(input: &Path, output: &Path, region: Option<&Region>) -> Result<()> {
    let selected = select_backend(input, region, Default::default());
    if selected.backend == Backend::Htslib {
        #[cfg(feature = "htslib")]
        {
            return crate::htslib_backend::convert_to_tsv(input, output, region);
        }

        #[cfg(not(feature = "htslib"))]
        {
            bail!(selected.reason.unwrap().unavailable_message());
        }
    }

    let mut reader = open_reader(input)?;
    let mut writer = open_writer(output)?;
    let mut line = Vec::new();

    writer.write_all(b"CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO/DP\tINFO/AF\n")?;

    write_tsv_records_streaming(&mut reader, &mut writer, &mut line)?;

    writer.flush()?;
    Ok(())
}

fn convert_to_parquet(input: &Path, output: &Path, region: Option<&Region>) -> Result<()> {
    let selected = select_backend(input, region, Default::default());
    if selected.backend == Backend::Htslib {
        bail!("convert --to parquet is supported only for native .vcf and .vcf.gz input in v1.0");
    }

    let mut reader = open_reader(input)?;
    let file = File::create(output)
        .with_context(|| format!("failed to create output {}", output.display()))?;
    let schema = parquet_schema();
    let mut writer = ArrowWriter::try_new(file, schema.clone(), None)?;
    let mut fields: [Vec<u8>; 8] = std::array::from_fn(|_| Vec::new());
    let mut batch = ParquetBatch::new(schema);

    loop {
        match peek_byte(&mut *reader)? {
            Some(b'#') => skip_line_tail(&mut *reader)?,
            Some(_) => {
                append_next_parquet_record(&mut *reader, &mut fields, &mut batch)?;
                if batch.len() >= PARQUET_BATCH_ROWS {
                    batch.flush(&mut writer)?;
                }
            }
            None => break,
        }
    }

    batch.flush(&mut writer)?;
    writer.close()?;
    Ok(())
}

fn write_tsv_records_streaming(
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
    field: &mut Vec<u8>,
) -> Result<()> {
    loop {
        match peek_byte(reader)? {
            Some(b'#') => skip_line_tail(reader)?,
            Some(_) => write_next_tsv_record(reader, writer, field)?,
            None => break,
        }
    }

    Ok(())
}

fn peek_byte(reader: &mut dyn BufRead) -> Result<Option<u8>> {
    Ok(reader.fill_buf()?.first().copied())
}

fn write_next_tsv_record(
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
    field: &mut Vec<u8>,
) -> Result<()> {
    for column in 0..8 {
        field.clear();
        let delimiter = read_field(reader, field)?;
        if matches!(delimiter, Some(b'\n') | None) {
            strip_trailing_carriage_return(field);
        }

        if column < 7 {
            if delimiter != Some(b'\t') {
                anyhow::bail!("VCF record has fewer than 8 columns");
            }
            write_cell_bytes(writer, field)?;
        } else {
            let info = TsvInfoBytes::scan(field);
            write_cell_bytes(writer, info.dp.unwrap_or(b"."))?;
            writer.write_all(info.af.unwrap_or(b"."))?;
            writer.write_all(b"\n")?;

            if delimiter == Some(b'\t') {
                skip_line_tail(reader)?;
            }
        }
    }

    Ok(())
}

fn read_field(reader: &mut dyn BufRead, field: &mut Vec<u8>) -> Result<Option<u8>> {
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return Ok(None);
        }

        if let Some(delimiter_index) = memchr2(b'\t', b'\n', buffer) {
            let delimiter = buffer[delimiter_index];
            field.extend_from_slice(&buffer[..delimiter_index]);
            reader.consume(delimiter_index + 1);
            return Ok(Some(delimiter));
        }

        let consumed = buffer.len();
        field.extend_from_slice(buffer);
        reader.consume(consumed);
    }
}

fn skip_line_tail(reader: &mut dyn BufRead) -> Result<()> {
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return Ok(());
        }

        if let Some(newline_index) = memchr(b'\n', buffer) {
            reader.consume(newline_index + 1);
            return Ok(());
        }

        let consumed = buffer.len();
        reader.consume(consumed);
    }
}

fn strip_trailing_carriage_return(field: &mut Vec<u8>) {
    if field.last() == Some(&b'\r') {
        field.pop();
    }
}

fn write_cell_bytes(writer: &mut dyn Write, value: &[u8]) -> Result<()> {
    writer.write_all(value)?;
    writer.write_all(b"\t")?;
    Ok(())
}

#[derive(Debug, Default, PartialEq, Eq)]
struct TsvInfoBytes<'a> {
    dp: Option<&'a [u8]>,
    af: Option<&'a [u8]>,
}

impl<'a> TsvInfoBytes<'a> {
    fn scan(info: &'a [u8]) -> Self {
        if info == b"." {
            return Self::default();
        }

        let mut values = Self::default();
        let mut entry_start = 0;

        while entry_start <= info.len() {
            let entry_end = info[entry_start..]
                .iter()
                .position(|byte| *byte == b';')
                .map_or(info.len(), |offset| entry_start + offset);

            if entry_start < entry_end
                && let Some(eq_offset) = info[entry_start..entry_end]
                    .iter()
                    .position(|byte| *byte == b'=')
            {
                let key_end = entry_start + eq_offset;
                let value = &info[key_end + 1..entry_end];
                match &info[entry_start..key_end] {
                    b"DP" if values.dp.is_none() => values.dp = Some(value),
                    b"AF" if values.af.is_none() => values.af = Some(value),
                    _ => {}
                }

                if values.dp.is_some() && values.af.is_some() {
                    break;
                }
            }

            if entry_end == info.len() {
                break;
            }
            entry_start = entry_end + 1;
        }

        values
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct ParquetInfoBytes<'a> {
    dp: Option<&'a [u8]>,
    af: Option<&'a [u8]>,
}

impl<'a> From<TsvInfoBytes<'a>> for ParquetInfoBytes<'a> {
    fn from(value: TsvInfoBytes<'a>) -> Self {
        Self {
            dp: present_bytes(value.dp),
            af: present_bytes(value.af),
        }
    }
}

struct ParquetBatch {
    schema: SchemaRef,
    chrom: StringBuilder,
    pos: Int64Builder,
    id: StringBuilder,
    reference: StringBuilder,
    alternate: StringBuilder,
    qual: Float64Builder,
    filter: StringBuilder,
    info_dp: Int64Builder,
    info_af: StringBuilder,
    rows: usize,
}

impl ParquetBatch {
    fn new(schema: SchemaRef) -> Self {
        Self {
            schema,
            chrom: StringBuilder::new(),
            pos: Int64Builder::new(),
            id: StringBuilder::new(),
            reference: StringBuilder::new(),
            alternate: StringBuilder::new(),
            qual: Float64Builder::new(),
            filter: StringBuilder::new(),
            info_dp: Int64Builder::new(),
            info_af: StringBuilder::new(),
            rows: 0,
        }
    }

    fn len(&self) -> usize {
        self.rows
    }

    fn append(&mut self, record: ParquetRecord<'_>) -> Result<()> {
        self.chrom.append_value(utf8_cell(record.chrom, "CHROM")?);
        let pos =
            parse_i64_cell(record.pos, "POS")?.context("POS is required for Parquet export")?;
        self.pos.append_value(pos);
        self.id.append_value(utf8_cell(record.id, "ID")?);
        self.reference
            .append_value(utf8_cell(record.reference, "REF")?);
        self.alternate
            .append_value(utf8_cell(record.alternate, "ALT")?);
        append_optional_f64(&mut self.qual, record.qual, "QUAL")?;
        self.filter
            .append_value(utf8_cell(record.filter, "FILTER")?);
        append_optional_i64(&mut self.info_dp, record.info.dp, "INFO/DP")?;
        append_optional_utf8(&mut self.info_af, record.info.af, "INFO/AF")?;
        self.rows += 1;
        Ok(())
    }

    fn flush<W: Write + Send>(&mut self, writer: &mut ArrowWriter<W>) -> Result<()> {
        if self.rows == 0 {
            return Ok(());
        }

        let batch = RecordBatch::try_new(
            self.schema.clone(),
            vec![
                Arc::new(self.chrom.finish()) as ArrayRef,
                Arc::new(self.pos.finish()) as ArrayRef,
                Arc::new(self.id.finish()) as ArrayRef,
                Arc::new(self.reference.finish()) as ArrayRef,
                Arc::new(self.alternate.finish()) as ArrayRef,
                Arc::new(self.qual.finish()) as ArrayRef,
                Arc::new(self.filter.finish()) as ArrayRef,
                Arc::new(self.info_dp.finish()) as ArrayRef,
                Arc::new(self.info_af.finish()) as ArrayRef,
            ],
        )?;
        writer.write(&batch)?;
        self.rows = 0;
        Ok(())
    }
}

#[derive(Debug)]
struct ParquetRecord<'a> {
    chrom: &'a [u8],
    pos: &'a [u8],
    id: &'a [u8],
    reference: &'a [u8],
    alternate: &'a [u8],
    qual: &'a [u8],
    filter: &'a [u8],
    info: ParquetInfoBytes<'a>,
}

fn parquet_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("CHROM", DataType::Utf8, false),
        Field::new("POS", DataType::Int64, false),
        Field::new("ID", DataType::Utf8, false),
        Field::new("REF", DataType::Utf8, false),
        Field::new("ALT", DataType::Utf8, false),
        Field::new("QUAL", DataType::Float64, true),
        Field::new("FILTER", DataType::Utf8, false),
        Field::new("INFO/DP", DataType::Int64, true),
        Field::new("INFO/AF", DataType::Utf8, true),
    ]))
}

fn append_next_parquet_record(
    reader: &mut dyn BufRead,
    fields: &mut [Vec<u8>; 8],
    batch: &mut ParquetBatch,
) -> Result<()> {
    for (column, field) in fields.iter_mut().enumerate() {
        field.clear();
        let delimiter = read_field(reader, field)?;
        if matches!(delimiter, Some(b'\n') | None) {
            strip_trailing_carriage_return(field);
        }
        if column < 7 && delimiter != Some(b'\t') {
            bail!("VCF record has fewer than 8 columns");
        }

        if column == 7 && delimiter == Some(b'\t') {
            skip_line_tail(reader)?;
        }
    }

    let info = ParquetInfoBytes::from(TsvInfoBytes::scan(&fields[7]));
    batch.append(ParquetRecord {
        chrom: &fields[0],
        pos: &fields[1],
        id: &fields[2],
        reference: &fields[3],
        alternate: &fields[4],
        qual: &fields[5],
        filter: &fields[6],
        info,
    })
}

fn utf8_cell<'a>(value: &'a [u8], column: &str) -> Result<&'a str> {
    std::str::from_utf8(value).with_context(|| format!("{column} is not valid UTF-8"))
}

fn present_bytes(value: Option<&[u8]>) -> Option<&[u8]> {
    value.filter(|bytes| !bytes.is_empty() && *bytes != b".")
}

fn append_optional_utf8(
    builder: &mut StringBuilder,
    value: Option<&[u8]>,
    column: &str,
) -> Result<()> {
    if let Some(value) = present_bytes(value) {
        builder.append_value(utf8_cell(value, column)?);
    } else {
        builder.append_null();
    }
    Ok(())
}

fn append_optional_i64(
    builder: &mut Int64Builder,
    value: Option<&[u8]>,
    column: &str,
) -> Result<()> {
    match value {
        Some(value) => {
            if let Some(parsed) = parse_i64_cell(value, column)? {
                builder.append_value(parsed);
            } else {
                builder.append_null();
            }
        }
        None => builder.append_null(),
    }
    Ok(())
}

fn append_optional_f64(builder: &mut Float64Builder, value: &[u8], column: &str) -> Result<()> {
    if let Some(value) = parse_f64_cell(value, column)? {
        builder.append_value(value);
    } else {
        builder.append_null();
    }
    Ok(())
}

fn parse_i64_cell(value: &[u8], column: &str) -> Result<Option<i64>> {
    let Some(value) = present_bytes(Some(value)) else {
        return Ok(None);
    };
    let raw = utf8_cell(value, column)?;
    Ok(Some(raw.parse::<i64>().with_context(|| {
        format!("{column} value '{raw}' is not an integer")
    })?))
}

fn parse_f64_cell(value: &[u8], column: &str) -> Result<Option<f64>> {
    let Some(value) = present_bytes(Some(value)) else {
        return Ok(None);
    };
    let raw = utf8_cell(value, column)?;
    Ok(Some(raw.parse::<f64>().with_context(|| {
        format!("{column} value '{raw}' is not numeric")
    })?))
}

#[cfg(test)]
mod tests {
    use super::{TsvInfoBytes, write_tsv_records_streaming};
    use std::io::Cursor;

    fn convert_body(input: &[u8]) -> String {
        let mut reader = Cursor::new(input);
        let mut field = Vec::new();
        let mut output = Vec::new();

        write_tsv_records_streaming(&mut reader, &mut output, &mut field).unwrap();

        String::from_utf8(output).unwrap()
    }

    #[test]
    fn writes_missing_info_values_as_dot() {
        assert_eq!(
            convert_body(b"1\t20\t.\tA\tG\t.\tPASS\tAF=0.2\n"),
            "1\t20\t.\tA\tG\t.\tPASS\t.\t0.2\n"
        );
    }

    #[test]
    fn writes_tsv_record_without_materializing_sample_tail() {
        assert_eq!(
            convert_body(b"1\t20\trs1\tA\tG\t42\tPASS\tZZ=1;DP=18;AF=0.01,0.2\tGT:DP\t0/1:18\n",),
            "1\t20\trs1\tA\tG\t42\tPASS\t18\t0.01,0.2\n"
        );
    }

    #[test]
    fn skips_headers_and_handles_final_record_without_newline() {
        assert_eq!(
            convert_body(
                b"##fileformat=VCFv4.2\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n1\t20\t.\tA\tG\t42\tPASS\tDP=18"
            ),
            "1\t20\t.\tA\tG\t42\tPASS\t18\t.\n"
        );
    }

    #[test]
    fn scans_byte_info_values_once_and_preserves_raw_values() {
        assert_eq!(
            TsvInfoBytes::scan(b"ZZ=1;DP=18;FLAG;AF=0.01,0.2;DP=99"),
            TsvInfoBytes {
                dp: Some(b"18".as_slice()),
                af: Some(b"0.01,0.2".as_slice())
            }
        );
        assert_eq!(TsvInfoBytes::scan(b"."), TsvInfoBytes::default());
        assert_eq!(
            TsvInfoBytes::scan(b"AF=.;EMPTY=;DP="),
            TsvInfoBytes {
                dp: Some(b"".as_slice()),
                af: Some(b".".as_slice())
            }
        );
    }
}
