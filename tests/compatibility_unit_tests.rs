use std::path::Path;

use vcf_fast::compat::{
    Backend, CompressionMode, HtslibReason, Region, SelectedBackend, parse_htslib_threads,
    select_backend,
};

#[test]
fn region_parser_accepts_contig_only() {
    assert_eq!(
        "chr22".parse::<Region>().unwrap(),
        Region {
            contig: "chr22".to_string(),
            start: None,
            end: None,
        }
    );
}

#[test]
fn region_parser_accepts_one_based_closed_range() {
    assert_eq!(
        "chr22:1-20000000".parse::<Region>().unwrap(),
        Region {
            contig: "chr22".to_string(),
            start: Some(1),
            end: Some(20_000_000),
        }
    );
}

#[test]
fn region_parser_rejects_malformed_ranges() {
    for raw in ["", "chr22:", "chr22:0-10", "chr22:10-1", "chr22:abc-10"] {
        assert!(raw.parse::<Region>().is_err(), "{raw} should be rejected");
    }
}

#[test]
fn backend_selector_keeps_simple_vcf_on_native_path() {
    assert_eq!(
        select_backend(Path::new("input.vcf"), None, CompressionMode::Auto),
        SelectedBackend {
            backend: Backend::Native,
            reason: None,
        }
    );
    assert_eq!(
        select_backend(Path::new("input.vcf.gz"), None, CompressionMode::Auto).backend,
        Backend::Native
    );
}

#[test]
fn backend_selector_uses_htslib_for_compatibility_features() {
    assert_eq!(
        select_backend(Path::new("input.bcf"), None, CompressionMode::Auto),
        SelectedBackend {
            backend: Backend::Htslib,
            reason: Some(HtslibReason::BcfInput),
        }
    );
    assert_eq!(
        select_backend(
            Path::new("input.vcf.gz"),
            Some(&"chr22".parse::<Region>().unwrap()),
            CompressionMode::Auto
        )
        .reason,
        Some(HtslibReason::Region)
    );
    assert_eq!(
        select_backend(Path::new("input.vcf"), None, CompressionMode::Bgzf).reason,
        Some(HtslibReason::BgzfOutput)
    );
}

#[test]
fn htslib_thread_config_accepts_unset_and_positive_integer() {
    assert_eq!(parse_htslib_threads(None).unwrap(), None);
    assert_eq!(parse_htslib_threads(Some("1")).unwrap(), Some(1));
    assert_eq!(parse_htslib_threads(Some("4")).unwrap(), Some(4));
}

#[test]
fn htslib_thread_config_rejects_zero_negative_and_non_integer() {
    let zero = parse_htslib_threads(Some("0")).unwrap_err().to_string();
    assert!(zero.contains("VCF_FAST_HTSLIB_THREADS must be a positive integer"));

    let negative = parse_htslib_threads(Some("-2")).unwrap_err().to_string();
    assert!(negative.contains("VCF_FAST_HTSLIB_THREADS must be a positive integer"));

    let text = parse_htslib_threads(Some("fast")).unwrap_err().to_string();
    assert!(text.contains("VCF_FAST_HTSLIB_THREADS must be a positive integer"));
}
