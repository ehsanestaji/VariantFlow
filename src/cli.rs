use std::path::PathBuf;

use anyhow::Result;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

use crate::compat::{CompressionMode, Region};
use crate::engine::{convert, diff, filter, popgen, stats};

#[derive(Debug, Parser)]
#[command(about = "Selective operations for VCF/BCF variant data")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Filter {
        input: PathBuf,
        #[arg(long)]
        region: Option<Region>,
        #[arg(long = "where")]
        where_expr: String,
        #[arg(long)]
        sample: Option<String>,
        #[arg(long, value_enum, default_value_t = CompressionMode::Auto)]
        compression: CompressionMode,
        #[arg(short, long)]
        output: PathBuf,
    },
    Stats {
        input: PathBuf,
        #[arg(long)]
        region: Option<Region>,
    },
    Freq {
        input: PathBuf,
        #[arg(long)]
        keep: Option<PathBuf>,
        #[arg(long)]
        remove: Option<PathBuf>,
        #[arg(short, long)]
        output: PathBuf,
    },
    Missingness {
        input: PathBuf,
        #[arg(long)]
        keep: Option<PathBuf>,
        #[arg(long)]
        remove: Option<PathBuf>,
        #[arg(short, long)]
        output: PathBuf,
    },
    Diff {
        a: PathBuf,
        b: PathBuf,
        #[arg(long, value_enum, default_value_t = diff::DiffMode::All)]
        mode: diff::DiffMode,
        #[arg(long = "key", value_enum, default_value_t = diff::DiffKeyMode::ChromPosRefAlt)]
        key_mode: diff::DiffKeyMode,
        #[arg(short, long)]
        output: PathBuf,
    },
    Convert {
        input: PathBuf,
        #[arg(long)]
        region: Option<Region>,
        #[arg(long = "to")]
        target: String,
        #[arg(short, long)]
        output: PathBuf,
    },
}

pub fn run() -> Result<()> {
    run_with_name("variantflow")
}

pub fn run_with_name(name: &'static str) -> Result<()> {
    let matches = Cli::command().name(name).get_matches();
    let cli = Cli::from_arg_matches(&matches)?;

    match cli.command {
        Command::Filter {
            input,
            region,
            where_expr,
            sample,
            compression,
            output,
        } => filter::run(
            &input,
            &where_expr,
            sample.as_deref(),
            &output,
            region.as_ref(),
            compression,
        ),
        Command::Stats { input, region } => stats::run(&input, region.as_ref()),
        Command::Freq {
            input,
            keep,
            remove,
            output,
        } => popgen::run_freq(&input, keep.as_deref(), remove.as_deref(), &output),
        Command::Missingness {
            input,
            keep,
            remove,
            output,
        } => popgen::run_missingness(&input, keep.as_deref(), remove.as_deref(), &output),
        Command::Diff {
            a,
            b,
            mode,
            key_mode,
            output,
        } => diff::run(&a, &b, &output, mode, key_mode),
        Command::Convert {
            input,
            region,
            target,
            output,
        } => convert::run(&input, &target, &output, region.as_ref()),
    }
}
