use std::env;
use std::fmt;
use std::path::Path;
use std::str::FromStr;

use anyhow::{Result, bail};
use clap::ValueEnum;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub contig: String,
    pub start: Option<u64>,
    pub end: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum CompressionMode {
    #[default]
    Auto,
    Plain,
    Gzip,
    Bgzf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Native,
    Htslib,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtslibReason {
    BcfInput,
    Region,
    BgzfOutput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedBackend {
    pub backend: Backend,
    pub reason: Option<HtslibReason>,
}

impl FromStr for Region {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        if raw.is_empty() {
            bail!("region must not be empty");
        }

        let Some((contig, rest)) = raw.split_once(':') else {
            return Ok(Self {
                contig: raw.to_string(),
                start: None,
                end: None,
            });
        };

        if contig.is_empty() || rest.is_empty() {
            bail!("invalid region '{raw}'");
        }

        let Some((start, end)) = rest.split_once('-') else {
            bail!("invalid region '{raw}'; expected contig:start-end");
        };

        let start = parse_region_coordinate(start, raw)?;
        let end = parse_region_coordinate(end, raw)?;
        if start == 0 || end < start {
            bail!("invalid region '{raw}'; expected 1-based start <= end");
        }

        Ok(Self {
            contig: contig.to_string(),
            start: Some(start),
            end: Some(end),
        })
    }
}

impl Region {
    pub fn htslib_interval(&self) -> (u64, Option<u64>) {
        let start = self.start.unwrap_or(1) - 1;
        let end = self.end.map(|value| value - 1);
        (start, end)
    }
}

impl fmt::Display for Region {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.start, self.end) {
            (Some(start), Some(end)) => write!(formatter, "{}:{start}-{end}", self.contig),
            _ => formatter.write_str(&self.contig),
        }
    }
}

impl HtslibReason {
    pub fn unavailable_message(self) -> &'static str {
        match self {
            Self::BcfInput => "BCF input requires the htslib feature",
            Self::Region => "--region requires the htslib feature",
            Self::BgzfOutput => "--compression bgzf requires the htslib feature",
        }
    }
}

pub fn select_backend(
    input: &Path,
    region: Option<&Region>,
    compression: CompressionMode,
) -> SelectedBackend {
    if has_bcf_extension(input) {
        return SelectedBackend {
            backend: Backend::Htslib,
            reason: Some(HtslibReason::BcfInput),
        };
    }

    if region.is_some() {
        return SelectedBackend {
            backend: Backend::Htslib,
            reason: Some(HtslibReason::Region),
        };
    }

    if compression == CompressionMode::Bgzf {
        return SelectedBackend {
            backend: Backend::Htslib,
            reason: Some(HtslibReason::BgzfOutput),
        };
    }

    SelectedBackend {
        backend: Backend::Native,
        reason: None,
    }
}

pub const HTSLIB_THREADS_ENV: &str = "VCF_FAST_HTSLIB_THREADS";

pub fn htslib_threads_from_env() -> Result<Option<usize>> {
    match env::var(HTSLIB_THREADS_ENV) {
        Ok(raw) => parse_htslib_threads(Some(raw.as_str())),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => {
            bail!("{HTSLIB_THREADS_ENV} must be valid UTF-8")
        }
    }
}

pub fn parse_htslib_threads(raw: Option<&str>) -> Result<Option<usize>> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    let value = raw
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("{HTSLIB_THREADS_ENV} must be a positive integer"))?;
    if value == 0 {
        bail!("{HTSLIB_THREADS_ENV} must be a positive integer");
    }

    Ok(Some(value))
}

fn parse_region_coordinate(raw: &str, full_region: &str) -> Result<u64> {
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("invalid region '{full_region}'");
    }
    Ok(raw.parse()?)
}

fn has_bcf_extension(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "bcf")
}
