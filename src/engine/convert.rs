use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::{Result, bail};

use crate::io::{open_reader, open_writer};
use crate::vcf::info_value;

pub fn run(input: &Path, target: &str, output: &Path) -> Result<()> {
    match target {
        "tsv" => convert_to_tsv(input, output),
        other => bail!("unsupported convert target '{other}'; supported targets: tsv"),
    }
}

fn convert_to_tsv(input: &Path, output: &Path) -> Result<()> {
    let mut reader = open_reader(input)?;
    let mut writer = open_writer(output)?;
    let mut line = String::new();

    writer.write_all(b"CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO/DP\tINFO/AF\n")?;

    while reader.read_line(&mut line)? != 0 {
        if !line.starts_with('#') {
            write_tsv_record(&line, &mut writer)?;
        }
        line.clear();
    }

    writer.flush()?;
    Ok(())
}

fn write_tsv_record(line: &str, writer: &mut dyn Write) -> Result<()> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    let columns: Vec<&str> = trimmed.split('\t').collect();

    if columns.len() < 8 {
        bail!("VCF record has fewer than 8 columns: {trimmed}");
    }

    let dp = info_value(columns[7], "DP").unwrap_or(".");
    let af = info_value(columns[7], "AF").unwrap_or(".");

    writeln!(
        writer,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        columns[0], columns[1], columns[2], columns[3], columns[4], columns[5], columns[6], dp, af
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::write_tsv_record;

    #[test]
    fn writes_missing_info_values_as_dot() {
        let mut output = Vec::new();

        write_tsv_record("1\t20\t.\tA\tG\t.\tPASS\tAF=0.2\n", &mut output).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "1\t20\t.\tA\tG\t.\tPASS\t.\t0.2\n"
        );
    }
}
