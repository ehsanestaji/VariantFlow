use std::fs;
use std::path::Path;
use std::process::Command;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

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
fn public_heavy_mode_does_not_reuse_plain_public_whole_builder() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let harness = std::fs::read_to_string(root.join("benchmark/run_benchmarks.sh")).unwrap();
    let heavy_start = harness.find("build_public_heavy_dataset").unwrap();
    let heavy_end = heavy_start
        + harness[heavy_start..]
            .find("\nsort_vcf_for_indexing()")
            .unwrap();
    let heavy_builder = &harness[heavy_start..heavy_end];
    assert!(!heavy_builder.contains("build_public_small_dataset"));
    assert!(heavy_builder.contains("bcftools view -h \"$source\""));
    assert!(heavy_builder.contains("bcftools view -H -r \"$region\" \"$source\""));
    assert!(heavy_builder.contains("| bgzip -c >\"$output\""));
    assert!(heavy_builder.contains("tabix -f -p vcf \"$output\""));
    assert!(!heavy_builder.contains(".plain.tmp.vcf"));
    assert!(!heavy_builder.contains("temp_plain"));
}

#[test]
fn bench_heavy_defaults_to_balanced_large_tiers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let makefile = std::fs::read_to_string(root.join("Makefile")).unwrap();

    assert!(makefile.contains("bench-heavy:"));
    assert!(makefile.contains("VCF_FAST_BENCH_SIZES=\"$${VCF_FAST_BENCH_SIZES:-100000 1000000}\""));
}

#[test]
fn htslib_backend_avoids_full_vcf_reconstruction_in_tsv_and_stats_paths() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let backend = std::fs::read_to_string(root.join("src/htslib_backend.rs")).unwrap();

    assert!(!backend.contains("to_vcf_string"));
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
        std::fs::read_to_string(root.join("benchmark/reports/v07-heavy-run-benchmark.md")).unwrap();
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
fn v08_core_efficiency_report_tracks_required_fields() {
    let root = repo_root();
    let report =
        fs::read_to_string(root.join("benchmark/reports/v08-core-efficiency-benchmark.md"))
            .expect("v0.8 report should exist");

    for required in [
        "v0.8 Core Efficiency Benchmark",
        "byte-core surgery",
        "correctness result",
        "runtime mean",
        "runtime stddev",
        "speedup",
        "variants/sec",
        "peak RSS",
        "exact VCF-Fast command",
        "exact competitor command",
        "competitor version",
        "dataset source",
        "caveat",
        "claim decision",
    ] {
        assert!(
            report.contains(required),
            "missing report field: {required}"
        );
    }

    assert!(
        report.contains("| case | dataset source | dataset shape | record count |"),
        "measured results table should include dataset shape"
    );
    assert!(
        report.contains(
            "| peak RSS | exact VCF-Fast command | exact competitor command | competitor version |"
        ),
        "measured results table should include competitor version"
    );
}

#[test]
fn v09_expression_parity_report_tracks_required_fields() {
    let report = std::fs::read_to_string("benchmark/reports/v09-expression-parity-benchmark.md")
        .expect("read v0.9 report");

    for required in [
        "dataset source",
        "dataset size",
        "record count",
        "exact VCF-Fast command",
        "exact competitor command",
        "competitor version",
        "correctness result",
        "runtime mean",
        "speedup",
        "variants per second",
        "peak RSS",
        "caveat",
    ] {
        assert!(report.contains(required), "missing {required}");
    }
}

#[test]
fn v09_expression_benchmark_script_tracks_competitor_cases() {
    let script = std::fs::read_to_string("benchmark/run_v09_expression_benchmarks.sh")
        .expect("read v0.9 expression benchmark script");
    let makefile = std::fs::read_to_string("Makefile").expect("read Makefile");

    for required in [
        "INFO/UNUSED7 > 300",
        "FORMAT/AD > 30",
        "ANY(FORMAT/AD > 80)",
        "ALL(FORMAT/DP > 20)",
        "N_PASS(FMT/AD[*:*]>80)>0",
        "N_PASS(FMT/DP>20)==N_SAMPLES",
        "bcftools filter",
        "hyperfine",
        "matches bcftools filtered core records",
    ] {
        assert!(script.contains(required), "missing {required}");
    }

    assert!(makefile.contains("bench-v09:"));
    assert!(makefile.contains("run_v09_expression_benchmarks.sh"));
}

#[test]
fn v10_compressed_benchmark_tracks_threaded_bgzf_evidence() {
    let script = std::fs::read_to_string("benchmark/run_v10_compressed_benchmarks.sh")
        .expect("read v1.0 compressed benchmark script");
    let makefile = std::fs::read_to_string("Makefile").expect("read Makefile");
    let report = std::fs::read_to_string("benchmark/reports/v10-compressed-input-benchmark.md")
        .expect("read v1.0 compressed benchmark report");

    for required in [
        "VCF_FAST_NATIVE_BGZF_THREADS",
        "threaded vs default",
        "threaded vs bcftools",
        "bcftools filter",
        "hyperfine",
        "default and threaded VCF-Fast match bcftools filtered core records",
        "ordinary gzip is still single-thread fallback",
    ] {
        assert!(script.contains(required), "missing {required}");
    }

    assert!(makefile.contains("bench-v10-compressed:"));
    assert!(makefile.contains("run_v10_compressed_benchmarks.sh"));

    for required in [
        "dataset size bytes",
        "record count",
        "correctness result",
        "threaded vs default",
        "threaded vs bcftools",
        "variants/sec",
        "peak RSS",
        "ordinary gzip is still single-thread fallback",
    ] {
        assert!(report.contains(required), "missing report field {required}");
    }
}

#[test]
fn v10_parquet_benchmark_tracks_columnar_export_evidence() {
    let script = std::fs::read_to_string("benchmark/run_v10_parquet_benchmarks.sh")
        .expect("read v1.0 parquet benchmark script");
    let makefile = std::fs::read_to_string("Makefile").expect("read Makefile");
    let report = std::fs::read_to_string("benchmark/reports/v10-parquet-export-benchmark.md")
        .expect("read v1.0 parquet benchmark report");
    let normalized_report = report.to_lowercase();

    for required in [
        "--to parquet",
        "--to tsv",
        "bcftools query",
        "hyperfine",
        "Parquet schema/null semantics verified by integration tests",
        "synthetic stress only",
    ] {
        assert!(script.contains(required), "missing {required}");
    }

    assert!(makefile.contains("bench-v10-parquet:"));
    assert!(makefile.contains("run_v10_parquet_benchmarks.sh"));

    for required in [
        "dataset source",
        "record count",
        "exact Parquet command",
        "exact TSV command",
        "exact competitor command",
        "correctness result",
        "variants/sec",
        "peak RSS",
        "claim decision",
    ] {
        assert!(
            normalized_report.contains(&required.to_lowercase()),
            "missing report field {required}"
        );
    }
}

#[test]
fn v10_columnar_workflow_benchmark_tracks_repeated_query_evidence() {
    let script = std::fs::read_to_string("benchmark/run_v10_columnar_workflow_benchmarks.sh")
        .expect("read v1.0 columnar workflow benchmark script");
    let helper = std::fs::read_to_string("benchmark/query_parquet_duckdb.py")
        .expect("read DuckDB parquet query helper");
    let makefile = std::fs::read_to_string("Makefile").expect("read Makefile");
    let report = std::fs::read_to_string("benchmark/reports/v10-columnar-workflow-benchmark.md")
        .expect("read v1.0 columnar workflow report");
    let normalized_report = report.to_lowercase();

    for required in [
        "--to parquet",
        "duckdb",
        "repeated queries",
        "bcftools filter",
        "public-heavy",
        "export once",
        "amortized",
        "QUAL > 30",
    ] {
        assert!(script.contains(required), "missing script text {required}");
    }

    for required in [
        "read_parquet",
        "QUAL > 30",
        "repeats",
        "duckdb python package is required",
    ] {
        assert!(helper.contains(required), "missing helper text {required}");
    }

    assert!(makefile.contains("bench-v10-columnar:"));
    assert!(makefile.contains("run_v10_columnar_workflow_benchmarks.sh"));

    for required in [
        "dataset source",
        "record count",
        "exact export command",
        "exact duckdb command",
        "exact competitor command",
        "correctness result",
        "export mean/stddev",
        "duckdb repeated query mean/stddev",
        "bcftools repeated scan mean/stddev",
        "amortized speedup",
        "variants/sec",
        "peak RSS",
        "claim decision",
        "caveat",
    ] {
        assert!(
            normalized_report.contains(&required.to_lowercase()),
            "missing report field {required}"
        );
    }
}

#[test]
fn v11_parallel_native_filter_benchmark_tracks_ordered_parallel_evidence() {
    let script = std::fs::read_to_string("benchmark/run_v11_parallel_filter_benchmarks.sh")
        .expect("read v1.1 parallel native filter benchmark script");
    let makefile = std::fs::read_to_string("Makefile").expect("read Makefile");
    let report =
        std::fs::read_to_string("benchmark/reports/v11-parallel-native-filter-benchmark.md")
            .expect("read v1.1 parallel native filter report");
    let normalized_report = report.to_lowercase();

    for required in [
        "VCF_FAST_NATIVE_FILTER_THREADS",
        "VCF_FAST_NATIVE_FILTER_BATCH_RECORDS",
        "parallel native",
        "default native",
        "bcftools filter",
        "matches default native and bcftools filtered core records",
        "line-preserving",
        "hyperfine",
    ] {
        assert!(script.contains(required), "missing script text {required}");
    }

    assert!(makefile.contains("bench-v11-parallel:"));
    assert!(makefile.contains("run_v11_parallel_filter_benchmarks.sh"));

    for required in [
        "dataset source",
        "record count",
        "exact default command",
        "exact parallel command",
        "exact competitor command",
        "correctness result",
        "default mean/stddev",
        "parallel mean/stddev",
        "bcftools mean/stddev",
        "parallel vs default",
        "parallel vs bcftools",
        "variants/sec",
        "peak RSS",
        "claim decision",
        "caveat",
    ] {
        assert!(
            normalized_report.contains(&required.to_lowercase()),
            "missing report field {required}"
        );
    }
}

#[test]
fn v12_public_parallel_workflow_benchmark_tracks_public_and_columnar_expansion() {
    let script =
        std::fs::read_to_string("benchmark/run_v12_public_parallel_workflow_benchmarks.sh")
            .expect("read v1.2 public parallel workflow benchmark script");
    let helper = std::fs::read_to_string("benchmark/query_parquet_duckdb.py")
        .expect("read DuckDB parquet query helper");
    let makefile = std::fs::read_to_string("Makefile").expect("read Makefile");
    let report =
        std::fs::read_to_string("benchmark/reports/v12-public-parallel-workflow-benchmark.md")
            .expect("read v1.2 public parallel workflow report");
    let normalized_report = report.to_lowercase();

    for required in [
        "public-heavy",
        "VCF_FAST_V12_PUBLIC_TIERS",
        "VCF_FAST_V12_STRESS_TIERS",
        "10000 100000 1000000",
        "100000 1000000",
        "VCF_FAST_NATIVE_BGZF_THREADS",
        "VCF_FAST_NATIVE_FILTER_THREADS",
        "combined threaded BGZF plus parallel native",
        "duckdb-venv/bin/python",
        "count_vcf_records",
        "ANY(FORMAT/AD > 80)",
        "INFO/DP > 40",
        "FILTER == \"PASS\"",
        "grouped counts by CHROM,FILTER",
        "bcftools filter",
        "bcftools query",
        "bcftools view",
        "hyperfine",
    ] {
        assert!(script.contains(required), "missing script text {required}");
    }

    for required in [
        "dp_gt_40",
        "group_by_chrom_filter",
        "INFO/DP > 40",
        "GROUP BY CHROM, FILTER",
    ] {
        assert!(helper.contains(required), "missing helper text {required}");
    }

    assert!(makefile.contains("bench-v12:"));
    assert!(makefile.contains("run_v12_public_parallel_workflow_benchmarks.sh"));
    assert!(makefile.contains("bash -n benchmark/run_v12_public_parallel_workflow_benchmarks.sh"));

    for required in [
        "dataset source",
        "record count",
        "exact default native command",
        "exact parallel native command",
        "exact threaded bgzf command",
        "exact combined command",
        "exact competitor command",
        "correctness result",
        "runtime mean/stddev",
        "speedup",
        "variants/sec",
        "peak RSS",
        "claim decision",
        "caveat",
    ] {
        assert!(
            normalized_report.contains(&required.to_lowercase()),
            "missing report field {required}"
        );
    }
}

#[test]
fn v14_public_parallel_scale_tracks_auto_bgzf_policy_and_modes() {
    let root = repo_root();
    let io = fs::read_to_string(root.join("src/io.rs")).unwrap();
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap();
    let script =
        fs::read_to_string(root.join("benchmark/run_v14_public_parallel_scale_benchmarks.sh"))
            .unwrap();
    let report =
        fs::read_to_string(root.join("benchmark/reports/v14-public-parallel-scale-benchmark.md"))
            .unwrap();
    let readme = fs::read_to_string(root.join("README.md")).unwrap();

    for required in [
        "DEFAULT_AUTO_BGZF_THREAD_CAP",
        "auto_native_bgzf_threads",
        "must be auto or a positive integer",
    ] {
        assert!(io.contains(required), "missing io policy text {required}");
    }

    assert!(makefile.contains("bench-v14:"));
    assert!(makefile.contains("run_v14_public_parallel_scale_benchmarks.sh"));
    assert!(makefile.contains("bash -n benchmark/run_v14_public_parallel_scale_benchmarks.sh"));

    for required in [
        "single-thread BGZF",
        "default auto BGZF",
        "auto BGZF plus predicate parallelism",
        "exact single-thread command",
        "exact auto BGZF command",
        "exact auto+predicate-parallel command",
        "exact explicit BGZF command",
        "VCF_FAST_NATIVE_BGZF_THREADS=1",
        "env -u VCF_FAST_NATIVE_BGZF_THREADS",
        "ANY(FORMAT/AD > 80)",
        "smoke validation only; no speed claim from sub-10k tier",
    ] {
        assert!(
            script.contains(required),
            "missing v1.4 script text {required}"
        );
    }

    for required in [
        "VCF-Fast v1.4 Public Parallel Scale Benchmark",
        "Auto BGZF policy",
        "single-thread, auto BGZF, auto+predicate-parallel",
        "smoke validation only; no speed claim from sub-10k tier",
        "Equivalence diffs",
    ] {
        assert!(
            report.contains(required),
            "missing v1.4 report text {required}"
        );
    }

    assert!(readme.contains("VCF_FAST_NATIVE_BGZF_THREADS=auto"));
    assert!(readme.contains("VCF_FAST_NATIVE_BGZF_THREADS=1"));
    assert!(readme.contains("make bench-v14"));
}

#[test]
fn v13_release_hardening_tracks_install_docs_and_generated_benchmark_table() {
    let root = repo_root();
    let cargo_toml = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let cli = fs::read_to_string(root.join("src/cli.rs")).unwrap();
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap();
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    let release_docs = fs::read_to_string(root.join("docs/release.md")).unwrap();
    let changelog = fs::read_to_string(root.join("CHANGELOG.md")).unwrap();
    let benchmark_table = fs::read_to_string(root.join("docs/public-benchmark-table.md")).unwrap();
    let generator =
        fs::read_to_string(root.join("benchmark/generate_public_benchmark_table.py")).unwrap();
    let release_workflow = fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();

    assert!(cargo_toml.contains("version = \"1.4.0\""));
    assert!(cli.contains("#[command(version)]"));

    assert!(makefile.contains("benchmark-table:"));
    assert!(makefile.contains("generate_public_benchmark_table.py --check"));
    assert!(readme.contains("vcf-fast --version"));
    assert!(readme.contains("docs/release.md"));
    assert!(readme.contains("docs/public-benchmark-table.md"));
    assert!(readme.contains("CHANGELOG.md"));

    for required in [
        "Install From Source",
        "Docker",
        "Compatibility Build",
        "Benchmark Prerequisites",
        "Public Data",
        "Release Checklist",
        "cargo build --release --features htslib-static",
    ] {
        assert!(
            release_docs.contains(required),
            "missing release doc text {required}"
        );
    }

    for required in ["v1.4.0", "v1.3.0", "v1.2", "v1.0", "v0.1"] {
        assert!(
            changelog.contains(required),
            "missing changelog version {required}"
        );
    }

    for required in [
        "generated by `benchmark/generate_public_benchmark_table.py`",
        "Threaded BGZF input",
        "DuckDB repeated-query workflow",
        "15.13x to 15.63x",
        "3.18x to 25.67x",
    ] {
        assert!(
            benchmark_table.contains(required),
            "missing benchmark table text {required}"
        );
    }

    for required in [
        "required_tokens",
        "--check",
        "public-benchmark-table.md",
        "benchmark/reports/v12-public-parallel-workflow-benchmark.md",
    ] {
        assert!(
            generator.contains(required),
            "missing generator text {required}"
        );
    }

    for required in [
        "actions/checkout@v6",
        "cargo build --release",
        "./target/release/vcf-fast --version",
        "actions/upload-artifact@v4",
        "softprops/action-gh-release@v2",
        "v*",
    ] {
        assert!(
            release_workflow.contains(required),
            "missing release workflow text {required}"
        );
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
