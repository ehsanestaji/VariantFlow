use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

use crate::engine::filter;

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
        output: Option<PathBuf>,
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
        Command::Stats { input: _ } => bail!("stats not implemented in v0.1"),
        Command::Diff {
            a: _,
            b: _,
            output: _,
        } => bail!("diff not implemented in v0.1"),
    }
}
