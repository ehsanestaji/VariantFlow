use std::path::Path;

use vcf_fast::compat::{
    Backend, CompressionMode, HtslibReason, Region, SelectedBackend, select_backend,
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
