use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn benchmark_harness_defines_report_and_correctness_contract() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = fs::read_to_string(root.join("benchmark/run_benchmarks.sh")).unwrap();

    assert!(script.contains("REPORT="));
    assert!(script.contains("## VCF-Fast Benchmark Report"));
    assert!(script.contains("VCF_FAST_BENCH_SIZES"));
    assert!(script.contains("bcftools filter"));
    assert!(script.contains("bcftools query -u"));
    assert!(script.contains("bcftools view -H -r"));
    assert!(script.contains("gzip_dataset=\"${plain_dataset}.gz\""));
    assert!(script.contains("benchmark/normalize_tsv.py"));
    assert!(script.contains("normalized bcftools query TSV rows"));
    assert!(script.contains("Output equivalence"));
    assert!(script.contains("vcf-fast variants/s"));
    assert!(script.contains("bcftools variants/s"));
    assert!(script.contains("vcf-fast peak RSS"));
    assert!(script.contains("bcftools peak RSS"));
    assert!(script.contains("measure_peak_rss_kb"));
    assert!(script.contains("VCF_FAST_BENCH_MODE=stress"));
    assert!(script.contains("VCF_FAST_STRESS_INFO_FIELDS"));
    assert!(script.contains("VCF_FAST_STRESS_SAMPLES"));
    assert!(script.contains("generate_stress_vcf.sh"));
    assert!(script.contains("vcf-fast stats"));
    assert!(script.contains("bcftools stats"));
    assert!(script.contains("Stats JSON"));
    assert!(script.contains("Convert TSV"));
    assert!(script.contains("QUAL plain"));
    assert!(script.contains("DP plain"));
    assert!(script.contains("AF plain"));
    assert!(script.contains("QUAL gzip input"));
    assert!(script.contains("FORMAT/DP > 20"));
    assert!(script.contains("FORMAT/GQ >= 30"));
    assert!(script.contains("FORMAT/GT == \\\"0/1\\\""));
    assert!(script.contains("FMT/DP[0]>20"));
    assert!(script.contains("FMT/GQ[0]>=30"));
    assert!(script.contains("FMT/GT[0]=\\\"0/1\\\""));
    assert!(script.contains("--sample SAMPLE_001"));
    assert!(script.contains("fast_sample_option="));
    assert!(!script.contains("fast_sample_args[@]"));
    assert!(script.contains("hyperfine"));
}

#[test]
fn stress_generator_emits_unused_info_format_and_sample_columns() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = fs::read_to_string(root.join("benchmark/generate_stress_vcf.sh")).unwrap();

    assert!(script.contains("VCF_FAST_STRESS_INFO_FIELDS"));
    assert!(script.contains("VCF_FAST_STRESS_SAMPLES"));
    assert!(script.contains("GT:DP:GQ:AD"));
    assert!(script.contains("UNUSED"));
    assert!(script.contains("#CHROM"));
}

#[test]
fn public_data_downloader_pins_giab_and_igsr_sources() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = fs::read_to_string(root.join("benchmark/download_public_data.sh")).unwrap();

    assert!(script.contains("HG002_GRCh38_1_22_v4.2.1_benchmark.vcf.gz"));
    assert!(
        script.contains(
            "1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz"
        )
    );
    assert!(script.contains("ftp-trace.ncbi.nlm.nih.gov"));
    assert!(script.contains("ftp.1000genomes.ebi.ac.uk"));
}

#[test]
fn compatibility_report_tracks_required_benchmark_fields() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report =
        fs::read_to_string(root.join("benchmark/reports/compatibility-benchmark.md")).unwrap();

    assert!(report.contains("BCF input"));
    assert!(report.contains("BGZF output"));
    assert!(report.contains("Indexed VCF region filter"));
    assert!(report.contains("Indexed BCF region stats"));
    assert!(report.contains("bcftools filter"));
    assert!(report.contains("bcftools query"));
    assert!(report.contains("tabix -p vcf"));
    assert!(report.contains("competitor version"));
    assert!(report.contains("runtime"));
    assert!(report.contains("speedup"));
    assert!(report.contains("variants/sec"));
    assert!(report.contains("peak RSS"));
    assert!(report.contains("correctness result"));
    assert!(report.contains("not a broad speed claim"));
}

#[test]
fn v06_benchmark_modes_and_make_targets_are_declared() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = fs::read_to_string(root.join("benchmark/run_benchmarks.sh")).unwrap();
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap();

    assert!(script.contains("public-whole"));
    assert!(script.contains("public-region-repeated"));
    assert!(script.contains("compatibility"));
    assert!(script.contains("VCF_FAST_PUBLIC_RECORD_TIERS"));
    assert!(script.contains("cargo build --release --features htslib-static"));
    assert!(script.contains("\"$MODE\" == \"public-region-repeated\""));
    assert!(script.contains("bcftools view -r"));
    assert!(script.contains("--compression bgzf"));
    assert!(script.contains("tabix -p vcf"));
    assert!(script.contains("sort_vcf_for_indexing"));
    assert!(script.contains("sort -t"));
    assert!(makefile.contains("bench-public:"));
    assert!(makefile.contains("bench-public-region:"));
    assert!(makefile.contains("bench-compat:"));
    assert!(makefile.contains("bench-v06-smoke:"));
}

#[test]
fn v07_public_heavy_mode_and_artifact_caps_are_declared() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let harness = std::fs::read_to_string(root.join("benchmark/run_benchmarks.sh")).unwrap();
    assert!(harness.contains("public-heavy"));
    assert!(harness.contains("VCF_FAST_HEAVY_MAX_PLAIN_BYTES"));
    assert!(harness.contains("build_public_heavy_dataset"));
    assert!(harness.contains("deferred: plain artifact cap exceeded"));
    assert!(!harness.contains("if ! build_public_heavy_dataset"));
    assert!(harness.contains("heavy_status=$?"));
    assert!(harness.contains("[[ \"$heavy_status\" -eq 77 ]]"));
    assert!(harness.contains("[[ \"$heavy_status\" -ne 0 ]]"));

    let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();
    assert!(makefile.contains("bench-heavy"));
    assert!(makefile.contains("VCF_FAST_BENCH_MODE=public-heavy"));
}

#[test]
fn v07_report_tracks_bottleneck_caveat_and_next_action() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let harness = std::fs::read_to_string(root.join("benchmark/run_benchmarks.sh")).unwrap();
    assert!(harness.contains("bottleneck"));
    assert!(harness.contains("next action"));
    assert!(harness.contains("native-filter"));
    assert!(harness.contains("native-stats"));
    assert!(harness.contains("htslib-region-tsv"));

    let report =
        std::fs::read_to_string(root.join("benchmark/reports/v07-heavy-run-benchmark.md"))
            .unwrap();
    for required in [
        "correctness result",
        "runtime mean",
        "speedup",
        "variants/sec",
        "peak RSS",
        "bottleneck",
        "next action",
        "caveat",
        "native-stats",
    ] {
        assert!(report.contains(required), "missing {required}");
    }
}

#[test]
fn v06_reports_track_claim_matrix_and_required_fields() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let public_report =
        fs::read_to_string(root.join("benchmark/reports/public-whole-cohort-benchmark.md"))
            .unwrap();
    let contribution_map = fs::read_to_string(root.join("docs/contribution-map.md")).unwrap();

    for required in [
        "dataset source URL",
        "dataset size bytes",
        "record count",
        "exact VCF-Fast command",
        "exact competitor command",
        "competitor version",
        "runtime mean/stddev",
        "speedup",
        "variants/sec",
        "peak RSS",
        "correctness result",
        "caveats",
    ] {
        assert!(public_report.contains(required), "{required}");
    }

    assert!(contribution_map.contains("Claim Matrix"));
    assert!(contribution_map.contains("beats"));
    assert!(contribution_map.contains("matches"));
    assert!(contribution_map.contains("complements"));
    assert!(contribution_map.contains("VCFtools"));
    assert!(contribution_map.contains("GATK"));
}

#[test]
fn hyperfine_summary_handles_null_stddev_from_single_run() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::tempdir().unwrap();
    let json_path = temp.path().join("single-run-hyperfine.json");
    fs::write(
        &json_path,
        r#"{
  "results": [
    { "command": "vcf-fast", "mean": 0.010, "stddev": null },
    { "command": "bcftools", "mean": 0.020, "stddev": null }
  ]
}"#,
    )
    .unwrap();

    let output = Command::new("python3")
        .arg(root.join("benchmark/summarize_hyperfine.py"))
        .arg(&json_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "0.010000s 0.000000s 0.020000s 0.000000s 2.00x"
    );
}
