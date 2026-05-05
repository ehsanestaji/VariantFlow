use std::collections::BTreeSet;
use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::Result;
use clap::ValueEnum;

use crate::io::{open_reader, open_writer};
use crate::vcf::{VariantKey, parse_record_fields};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DiffMode {
    All,
    Shared,
    OnlyA,
    OnlyB,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DiffKeyMode {
    ChromPosRefAlt,
    Position,
    IdAware,
}

pub fn run(a: &Path, b: &Path, output: &Path, mode: DiffMode, key_mode: DiffKeyMode) -> Result<()> {
    let a_keys = read_keys(a, key_mode)?;
    let b_keys = read_keys(b, key_mode)?;
    let shared = a_keys.intersection(&b_keys).count();
    let only_in_a = a_keys.difference(&b_keys).count();
    let only_in_b = b_keys.difference(&a_keys).count();

    let mut writer = open_writer(output)?;
    write_header(&mut writer, key_mode)?;

    for key in a_keys.union(&b_keys) {
        let status = match (a_keys.contains(key), b_keys.contains(key)) {
            (true, true) => "shared",
            (true, false) => "only_in_a",
            (false, true) => "only_in_b",
            (false, false) => unreachable!("union only yields keys from one or both inputs"),
        };
        if mode.includes(status) {
            write_key(&mut writer, status, key)?;
        }
    }

    writer.flush()?;
    eprintln!("shared={shared} only_in_a={only_in_a} only_in_b={only_in_b}");
    Ok(())
}

impl DiffMode {
    fn includes(self, status: &str) -> bool {
        match self {
            Self::All => true,
            Self::Shared => status == "shared",
            Self::OnlyA => status == "only_in_a",
            Self::OnlyB => status == "only_in_b",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum DiffKey {
    ChromPosRefAlt(VariantKey),
    Position {
        chrom: String,
        pos: u64,
    },
    IdAware {
        chrom: String,
        pos: u64,
        id: String,
        reference: String,
        alternate: String,
    },
}

fn read_keys(path: &Path, key_mode: DiffKeyMode) -> Result<BTreeSet<DiffKey>> {
    let mut reader = open_reader(path)?;
    let mut line = String::new();
    let mut keys = BTreeSet::new();

    while reader.read_line(&mut line)? != 0 {
        if !line.starts_with('#') {
            keys.insert(parse_diff_key(&line, key_mode)?);
        }
        line.clear();
    }

    Ok(keys)
}

fn parse_diff_key(line: &str, key_mode: DiffKeyMode) -> Result<DiffKey> {
    let fields = parse_record_fields(line)?;
    let pos = fields.pos_u64()?;
    Ok(match key_mode {
        DiffKeyMode::ChromPosRefAlt => DiffKey::ChromPosRefAlt(VariantKey {
            chrom: fields.chrom.to_owned(),
            pos,
            reference: fields.reference.to_owned(),
            alternate: fields.alternate.to_owned(),
        }),
        DiffKeyMode::Position => DiffKey::Position {
            chrom: fields.chrom.to_owned(),
            pos,
        },
        DiffKeyMode::IdAware => DiffKey::IdAware {
            chrom: fields.chrom.to_owned(),
            pos,
            id: fields.id.to_owned(),
            reference: fields.reference.to_owned(),
            alternate: fields.alternate.to_owned(),
        },
    })
}

fn write_header(writer: &mut dyn Write, key_mode: DiffKeyMode) -> Result<()> {
    match key_mode {
        DiffKeyMode::ChromPosRefAlt | DiffKeyMode::Position => {
            writer.write_all(b"status\tchrom\tpos\tref\talt\n")?;
        }
        DiffKeyMode::IdAware => {
            writer.write_all(b"status\tchrom\tpos\tid\tref\talt\n")?;
        }
    }
    Ok(())
}

fn write_key(writer: &mut dyn Write, status: &str, key: &DiffKey) -> Result<()> {
    match key {
        DiffKey::ChromPosRefAlt(key) => {
            writeln!(
                writer,
                "{status}\t{}\t{}\t{}\t{}",
                key.chrom, key.pos, key.reference, key.alternate
            )?;
        }
        DiffKey::Position { chrom, pos } => {
            writeln!(writer, "{status}\t{chrom}\t{pos}\t.\t.")?;
        }
        DiffKey::IdAware {
            chrom,
            pos,
            id,
            reference,
            alternate,
        } => {
            writeln!(
                writer,
                "{status}\t{chrom}\t{pos}\t{id}\t{reference}\t{alternate}"
            )?;
        }
    }
    Ok(())
}
