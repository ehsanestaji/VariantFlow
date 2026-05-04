use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::engine::{convert, diff, filter, stats};

#[derive(Debug, Parser)]
#[command(name = "vcf-fast")]
#[command(about = "Fast, selective operations for VCF data")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Filter {
        input: PathBuf,
        #[arg(long = "where")]
        where_expr: String,
        #[arg(short, long)]
        output: PathBuf,
    },
    Stats {
        input: PathBuf,
    },
    Diff {
        a: PathBuf,
        b: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    Convert {
        input: PathBuf,
        #[arg(long = "to")]
        target: String,
        #[arg(short, long)]
        output: PathBuf,
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Filter {
            input,
            where_expr,
            output,
        } => filter::run(&input, &where_expr, &output),
        Command::Stats { input } => stats::run(&input),
        Command::Diff { a, b, output } => diff::run(&a, &b, &output),
        Command::Convert {
            input,
            target,
            output,
        } => convert::run(&input, &target, &output),
    }
}
