use std::collections::BTreeSet;
use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::Result;

use crate::io::{open_reader, open_writer};
use crate::vcf::{VariantKey, parse_record_line};

pub fn run(a: &Path, b: &Path, output: &Path) -> Result<()> {
    let a_keys = read_keys(a)?;
    let b_keys = read_keys(b)?;
    let shared = a_keys.intersection(&b_keys).count();
    let only_in_a = a_keys.difference(&b_keys).count();
    let only_in_b = b_keys.difference(&a_keys).count();

    let mut writer = open_writer(output)?;
    writer.write_all(b"status\tchrom\tpos\tref\talt\n")?;

    for key in a_keys.union(&b_keys) {
        let status = match (a_keys.contains(key), b_keys.contains(key)) {
            (true, true) => "shared",
            (true, false) => "only_in_a",
            (false, true) => "only_in_b",
            (false, false) => unreachable!("union only yields keys from one or both inputs"),
        };
        write_key(&mut writer, status, key)?;
    }

    writer.flush()?;
    eprintln!("shared={shared} only_in_a={only_in_a} only_in_b={only_in_b}");
    Ok(())
}

fn read_keys(path: &Path) -> Result<BTreeSet<VariantKey>> {
    let mut reader = open_reader(path)?;
    let mut line = String::new();
    let mut keys = BTreeSet::new();

    while reader.read_line(&mut line)? != 0 {
        if !line.starts_with('#') {
            keys.insert(parse_record_line(&line)?.key());
        }
        line.clear();
    }

    Ok(keys)
}

fn write_key(writer: &mut dyn Write, status: &str, key: &VariantKey) -> Result<()> {
    writeln!(
        writer,
        "{status}\t{}\t{}\t{}\t{}",
        key.chrom, key.pos, key.reference, key.alternate
    )?;
    Ok(())
}
