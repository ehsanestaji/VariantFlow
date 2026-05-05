use std::path::PathBuf;

use anyhow::Result;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

use crate::compat::{CompressionMode, Region};
use crate::engine::{convert, diff, filter, stats};

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
    Diff {
        a: PathBuf,
        b: PathBuf,
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
        Command::Diff { a, b, output } => diff::run(&a, &b, &output),
        Command::Convert {
            input,
            region,
            target,
            output,
        } => convert::run(&input, &target, &output, region.as_ref()),
    }
}
