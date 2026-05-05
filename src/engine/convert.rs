use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::{Result, bail};
use memchr::{memchr, memchr2};

use crate::compat::{Backend, Region, select_backend};
use crate::io::{open_reader, open_writer};
pub fn run(input: &Path, target: &str, output: &Path, region: Option<&Region>) -> Result<()> {
    match target {
        "tsv" => convert_to_tsv(input, output, region),
        other => bail!("unsupported convert target '{other}'; supported targets: tsv"),
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
