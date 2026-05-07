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
    assert!(script.contains("NA12878.trio.hg19_multianno.vcf.gz"));
    assert!(script.contains("trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz"));
    assert!(script.contains("19.filtered_intersect.vcf.gz"));
    assert!(script.contains("ERZ324584"));
    assert!(script.contains("453 sheep"));
    assert!(script.contains("Dutch_Genebank_Cattle_Y_merged.vcf.gz"));
    assert!(script.contains("ERZ18456468"));
    assert!(script.contains("29 cattle"));
    assert!(script.contains("format-cattle29"));
    assert!(script.contains("format-ovis453"));
    assert!(script.contains("format-wgs-trio"));
    assert!(script.contains("format-trio"));
    assert!(script.contains("ftp-trace.ncbi.nlm.nih.gov"));
    assert!(script.contains("ftp.1000genomes.ebi.ac.uk"));
    assert!(script.contains("sourceforge.net/projects/project123vcf"));
    assert!(script.contains("ftp.sra.ebi.ac.uk/vol1/analysis/ERZ324/ERZ324584"));
    assert!(script.contains("ftp.sra.ebi.ac.uk/vol1/analysis/ERZ184/ERZ18456468"));
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
        "auto BGZF is the preferred default for this public BGZF QUAL filter",
        "predicate parallelism remains the preferred opt-in path",
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
fn release_todo_tracks_bioconda_and_professional_rename() {
    let root = repo_root();
    let todo = fs::read_to_string(root.join("TODO.md")).unwrap();
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    let release_docs = fs::read_to_string(root.join("docs/release.md")).unwrap();
    let rename_plan = fs::read_to_string(root.join("docs/rename-plan.md")).unwrap();

    for required in [
        "Bioconda Release",
        "bioconda-recipes",
        "cargo install -v --locked --no-track --root $PREFIX --path .",
        "Professional Rename",
        "Accepted direction: VariantFlow",
        "compatibility alias",
        "crates.io",
        "docs/claim-matrix.md",
    ] {
        assert!(todo.contains(required), "missing TODO text {required}");
    }

    for required in [
        "VariantFlow Rename Plan",
        "Primary CLI binary: `variantflow`",
        "Compatibility CLI alias: `vcf-fast`",
        "Bioconda package: `variantflow`",
        "one-release compatibility alias",
        "collision check",
    ] {
        assert!(
            rename_plan.contains(required),
            "missing rename plan text {required}"
        );
    }

    assert!(readme.contains("VariantFlow, formerly VCF-Fast"));
    assert!(readme.contains("variantflow --version"));
    assert!(readme.contains("Bioconda release planning"));
    assert!(readme.contains("TODO.md"));
    assert!(release_docs.contains("Distribution And Naming TODO"));
}

#[test]
fn bioconda_packaging_prep_tracks_recipe_template_and_release_blockers() {
    let root = repo_root();
    let cargo_toml = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let license = fs::read_to_string(root.join("LICENSE")).unwrap();
    let license_mit = fs::read_to_string(root.join("LICENSE-MIT")).unwrap();
    let license_apache = fs::read_to_string(root.join("LICENSE-APACHE")).unwrap();
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap();
    let meta = fs::read_to_string(root.join("packaging/bioconda/variantflow/meta.yaml")).unwrap();
    let build = fs::read_to_string(root.join("packaging/bioconda/variantflow/build.sh")).unwrap();
    let run_test =
        fs::read_to_string(root.join("packaging/bioconda/variantflow/run_test.sh")).unwrap();
    let docs = fs::read_to_string(root.join("docs/bioconda-packaging.md")).unwrap();
    let checker = fs::read_to_string(root.join("packaging/check_bioconda_recipe.py")).unwrap();

    assert!(makefile.contains("bioconda-recipe-check:"));
    assert!(makefile.contains("packaging/check_bioconda_recipe.py"));
    assert!(cargo_toml.contains("license = \"MIT OR Apache-2.0\""));
    assert!(license.contains("MIT OR Apache-2.0"));
    assert!(license_mit.contains("MIT License"));
    assert!(license_apache.contains("Apache License"));

    for required in [
        "{% set name = \"variantflow\" %}",
        "{% set version = \"1.5.0\" %}",
        "https://github.com/ehsanestaji/VCF-FAST/archive/v{{ version }}.tar.gz",
        "sha256: TODO_RELEASE_SHA256",
        "cargo-bundle-licenses",
        "{{ compiler('rust') }}",
        "license: MIT OR Apache-2.0",
        "license_file:",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "THIRDPARTY.yml",
        "variantflow --version",
        "vcf-fast --version",
        "recipe-maintainers:",
        "- ehsanestaji",
    ] {
        assert!(
            meta.contains(required),
            "missing Bioconda recipe text {required}"
        );
    }

    for required in [
        "cargo-bundle-licenses --format yaml --output THIRDPARTY.yml",
        "cargo install -v --locked --no-track --root \"$PREFIX\" --path .",
    ] {
        assert!(build.contains(required), "missing build text {required}");
    }

    for required in [
        "variantflow --version",
        "vcf-fast --version",
        "variantflow filter",
        "variantflow convert",
    ] {
        assert!(
            run_test.contains(required),
            "missing run_test text {required}"
        );
    }

    for required in [
        "Current Blockers",
        "tagged GitHub source release",
        "sha256",
        "MIT OR Apache-2.0",
        "ehsanestaji",
        "Exact-name check on 2026-05-06",
        "bioconda/variantflow: 404",
        "crates/variantflow: 404",
    ] {
        assert!(
            docs.contains(required),
            "missing Bioconda docs text {required}"
        );
    }

    for required in [
        "TODO_RELEASE_SHA256",
        "license: MIT OR Apache-2.0",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "ehsanestaji",
        "variantflow --version",
        "cargo install -v --locked --no-track",
    ] {
        assert!(
            checker.contains(required),
            "missing checker text {required}"
        );
    }
}

#[test]
fn joss_paper_scaffold_tracks_evidence_and_submission_blockers() {
    let root = repo_root();
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap();
    let paper = fs::read_to_string(root.join("paper/paper.md")).unwrap();
    let bib = fs::read_to_string(root.join("paper/paper.bib")).unwrap();
    let readiness = fs::read_to_string(root.join("docs/paper-readiness.md")).unwrap();
    let checker = fs::read_to_string(root.join("paper/check_paper.py")).unwrap();

    assert!(makefile.contains("paper-check:"));
    assert!(makefile.contains("paper/check_paper.py"));
    assert!(makefile.contains("make paper-check"));

    for required in [
        "VariantFlow: a selective Rust execution engine for evidence-tracked VCF filtering and analytical export",
        "# Summary",
        "# Statement of need",
        "# State of the field",
        "# Software design",
        "# Research impact statement",
        "# AI usage disclosure",
        "# Acknowledgements",
        "# References",
        "benchmark/reports/v14-public-parallel-scale-benchmark.md",
        "benchmark/reports/v12-public-parallel-workflow-benchmark.md",
        "13.44x to 13.47x",
        "1.77x to 2.01x",
        "3.18x to 25.67x",
        "not a claim that VariantFlow replaces bcftools or GATK",
    ] {
        assert!(paper.contains(required), "missing paper text {required}");
    }

    for required in [
        "@article{bcftools",
        "@article{htslib",
        "@article{bioconda",
        "@misc{joss",
        "@misc{apache_arrow",
        "@misc{parquet",
        "@misc{duckdb",
        "@misc{vcf_spec",
    ] {
        assert!(bib.contains(required), "missing paper.bib entry {required}");
    }

    for required in [
        "JOSS Submission Readiness",
        "Current blockers",
        "MIT OR Apache-2.0",
        "Umeå Plant Science Center",
        "ehsanestaji",
        "public repository history",
        "tagged release",
        "Zenodo DOI",
        "Bioconda Launch Coordination",
        "Benchmark rows used by the manuscript",
        "Author metadata needed",
    ] {
        assert!(
            readiness.contains(required),
            "missing readiness text {required}"
        );
    }

    for required in [
        "required_sections",
        "750",
        "1750",
        "paper.bib",
        "benchmark/reports/v14-public-parallel-scale-benchmark.md",
        "benchmark/reports/v12-public-parallel-workflow-benchmark.md",
    ] {
        assert!(
            checker.contains(required),
            "missing checker text {required}"
        );
    }
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

    assert!(cargo_toml.contains("version = \"1.5.0\""));
    assert!(cargo_toml.contains("name = \"variantflow\""));
    assert!(cargo_toml.contains("name = \"vcf-fast\""));
    assert!(cli.contains("#[command(version)]"));

    assert!(makefile.contains("benchmark-table:"));
    assert!(makefile.contains("generate_public_benchmark_table.py --check"));
    assert!(readme.contains("variantflow --version"));
    assert!(readme.contains("vcf-fast"));
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
        "variantflow --version",
        "vcf-fast --version",
    ] {
        assert!(
            release_docs.contains(required),
            "missing release doc text {required}"
        );
    }

    for required in ["v1.5.0", "v1.4.0", "v1.3.0", "v1.2", "v1.0", "v0.1"] {
        assert!(
            changelog.contains(required),
            "missing changelog version {required}"
        );
    }

    for required in [
        "generated by `benchmark/generate_public_benchmark_table.py`",
        "Default auto BGZF input",
        "DuckDB repeated-query workflow",
        "Public FORMAT expression breadth",
        "13.44x to 13.47x",
        "3.18x to 25.67x",
        "3.22x to 8.77x",
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
        "benchmark/reports/v14-public-parallel-scale-benchmark.md",
        "benchmark/reports/v18-public-format-expression-breadth.md",
    ] {
        assert!(
            generator.contains(required),
            "missing generator text {required}"
        );
    }

    for required in [
        "actions/checkout@v6",
        "cargo build --release",
        "./target/release/variantflow --version",
        "./target/release/vcf-fast --version",
        "cp target/release/variantflow dist/",
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

#[test]
fn bioinformatics_readiness_tracks_release_docs_workflows_and_claim_matrix() {
    let root = repo_root();
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap();
    let workflow_docs = fs::read_to_string(root.join("docs/bioinformatics-workflows.md")).unwrap();
    let claim_matrix = fs::read_to_string(root.join("docs/claim-matrix.md")).unwrap();
    let snakemake = fs::read_to_string(root.join("examples/Snakefile")).unwrap();
    let nextflow = fs::read_to_string(root.join("examples/variantflow.nf")).unwrap();
    let public_recipe =
        fs::read_to_string(root.join("examples/public_benchmark_recipe.sh")).unwrap();

    assert!(makefile.contains("release-candidate-check:"));
    assert!(makefile.contains("make verify"));
    assert!(makefile.contains("cargo test --features htslib-static"));

    for required in [
        "Install",
        "Common Filters",
        "VariantFlow",
        "bcftools equivalent",
        "Parquet + DuckDB",
        "Public benchmark reproduction",
        "Limitations",
    ] {
        assert!(
            workflow_docs.contains(required),
            "missing workflow text {required}"
        );
    }

    for required in [
        "beats",
        "matches",
        "complements",
        "not yet proven",
        "benchmark/reports/v14-public-parallel-scale-benchmark.md",
        "benchmark/reports/v12-public-parallel-workflow-benchmark.md",
        "BCF TSV",
        "GATK",
    ] {
        assert!(
            claim_matrix.contains(required),
            "missing claim text {required}"
        );
    }

    assert!(snakemake.contains("variantflow filter"));
    assert!(snakemake.contains("variantflow convert"));
    assert!(nextflow.contains("process VARIANTFLOW_FILTER"));
    assert!(nextflow.contains("process VARIANTFLOW_EXPORT_PARQUET"));
    assert!(public_recipe.contains("benchmark/download_public_data.sh"));
    assert!(public_recipe.contains("make bench-v14"));
}

#[test]
fn v17_public_format_and_optional_baseline_harness_is_declared() {
    let root = repo_root();
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap();
    let script =
        fs::read_to_string(root.join("benchmark/run_v17_public_format_baselines.sh")).unwrap();
    let report =
        fs::read_to_string(root.join("benchmark/reports/v17-public-format-baselines.md")).unwrap();

    assert!(makefile.contains("bench-v17:"));
    assert!(makefile.contains("run_v17_public_format_baselines.sh"));

    for required in [
        "VCF_FAST_ENABLE_VCFTOOLS",
        "VCF_FAST_ENABLE_GATK",
        "VCF_FAST_ENABLE_POLARS",
        "VCF_FAST_ENABLE_PYARROW",
        "VCF_FAST_FORMAT_VCF",
        "VCF_FAST_FORMAT_COHORT_VCF",
        "VCF_FAST_FORMAT_WGS_TRIO_VCF",
        "VCF_FAST_V17_RUNS",
        "VCF_FAST_V17_WARMUP",
        "VCF_FAST_V17_HEAVY_OUTPUT_RECORDS",
        "heavy_output_mode",
        "/dev/stdout",
        "/dev/null",
        "core records only",
        "full-chromosome",
        "NA12878.trio.hg19_multianno.vcf.gz",
        "trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz",
        "19.filtered_intersect.vcf.gz",
        "453-sample",
        "ERZ324584",
        "1097167",
        "9dabe9929a8923e62c8808d6fbf15314",
        "Mayo VCF-Miner",
        "629 samples",
        "N_PASS(FORMAT/AD[1] > 10)",
        "bcftools filter",
        "hyperfine",
        "measure_peak_rss_kb",
        "real_seconds_from_time",
        "speedup_ratio",
        "runtime_mean_stddev",
        "correctness result",
        "/^#CHROM/ { exit(ad && dp ? 0 : 1) }",
    ] {
        assert!(script.contains(required), "missing harness text {required}");
    }

    for required in [
        "v1.7 Public FORMAT And Optional Baselines",
        "public FORMAT-heavy",
        "FORMAT-rich public trio",
        "FORMAT-rich public cohort",
        "larger FORMAT-rich WGS trio",
        "Repeated local timing",
        "VCFtools",
        "GATK",
        "Polars",
        "PyArrow",
        "not yet proven",
    ] {
        assert!(report.contains(required), "missing report text {required}");
    }
}

#[test]
fn v18_public_format_expression_breadth_harness_is_declared() {
    let root = repo_root();
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap();
    let script =
        fs::read_to_string(root.join("benchmark/run_v18_public_format_expression_breadth.sh"))
            .unwrap();
    let report =
        fs::read_to_string(root.join("benchmark/reports/v18-public-format-expression-breadth.md"))
            .unwrap();

    assert!(makefile.contains("bench-v18:"));
    assert!(makefile.contains("run_v18_public_format_expression_breadth.sh"));
    assert!(makefile.contains("bash -n benchmark/run_v18_public_format_expression_breadth.sh"));

    for required in [
        "VCF_FAST_V18_TIERS",
        "VCF_FAST_V18_RUNS",
        "VCF_FAST_V18_WARMUP",
        "VCF_FAST_V18_HEAVY_OUTPUT_RECORDS",
        "VCF_FAST_V18_SAMPLE",
        "heavy_output_mode",
        "/dev/stdout",
        "/dev/null",
        "core records only",
        "19.filtered_intersect.vcf.gz",
        "ERZ324584",
        "453-sample",
        "1097167",
        "ANY(FORMAT/DP > 20)",
        "ALL(FORMAT/GQ >= 30)",
        "N_PASS(FORMAT/AD[1] > 10) >= 10",
        "FORMAT/DP > 20",
        "QUAL > 30 && ANY(FORMAT/DP > 20)",
        "N_PASS(FMT/DP[*]>20)>0",
        "N_PASS(FMT/GQ[*]>=30)==",
        "N_PASS(FMT/AD[*:1]>10)>=10",
        "bcftools view -s",
        "bcftools filter",
        "hyperfine",
        "measure_peak_rss_kb",
        "runtime_mean_stddev",
        "correctness result",
    ] {
        assert!(script.contains(required), "missing harness text {required}");
    }

    for required in [
        "v1.8 Public FORMAT Expression Breadth",
        "ANY(FORMAT/DP > 20)",
        "ALL(FORMAT/GQ >= 30)",
        "N_PASS(FORMAT/AD[1] > 10) >= 10",
        "selected-sample FORMAT/DP > 20",
        "QUAL > 30 && ANY(FORMAT/DP > 20)",
        "matched core records",
        "3.22x",
        "8.77x",
    ] {
        assert!(report.contains(required), "missing report text {required}");
    }
}

#[test]
fn second_public_format_cohort_harness_is_declared() {
    let root = repo_root();
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap();
    let script =
        fs::read_to_string(root.join("benchmark/run_v19_second_public_format_cohort.sh")).unwrap();
    let report =
        fs::read_to_string(root.join("benchmark/reports/v19-second-public-format-cohort.md"))
            .unwrap();

    assert!(makefile.contains("bench-v19:"));
    assert!(makefile.contains("run_v19_second_public_format_cohort.sh"));
    assert!(makefile.contains("bash -n benchmark/run_v19_second_public_format_cohort.sh"));

    for required in [
        "VCF_FAST_V19_TIERS",
        "VCF_FAST_V19_RUNS",
        "VCF_FAST_V19_WARMUP",
        "VCF_FAST_V19_HEAVY_OUTPUT_RECORDS",
        "Dutch_Genebank_Cattle_Y_merged.vcf.gz",
        "ERZ18456468",
        "PRJEB60909",
        "Bos taurus",
        "29-sample",
        "131795380",
        "FORMAT/AD",
        "FORMAT/DP",
        "FORMAT/GQ",
        "ANY(FORMAT/DP > 20)",
        "ALL(FORMAT/GQ >= 30)",
        "N_PASS(FORMAT/AD[1] > 10) >= 2",
        "QUAL > 30 && ANY(FORMAT/DP > 20)",
        "N_PASS(FMT/DP[*]>20)>0",
        "N_PASS(FMT/GQ[*]>=30)==",
        "N_PASS(FMT/AD[*:1]>10)>=2",
        "bcftools filter",
        "hyperfine",
        "measure_peak_rss_kb",
        "correctness result",
        "/dev/stdout",
        "/dev/null",
        "Mayo VCF-Miner",
        "403",
    ] {
        assert!(script.contains(required), "missing harness text {required}");
    }

    for required in [
        "v1.9 Second Public FORMAT-Rich Cohort",
        "Dutch Genebank Cattle",
        "ERZ18456468",
        "5488549 actual",
        "1.46x",
        "26.66x",
        "second public FORMAT-rich cohort",
        "matched core records",
    ] {
        assert!(report.contains(required), "missing report text {required}");
    }
}

#[test]
fn human_public_format_cohort_harness_is_declared() {
    let root = repo_root();
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap();
    let downloader = fs::read_to_string(root.join("benchmark/download_public_data.sh")).unwrap();
    let script = fs::read_to_string(root.join("benchmark/run_v20_human_format_cohort.sh")).unwrap();
    let report =
        fs::read_to_string(root.join("benchmark/reports/v20-human-format-cohort.md")).unwrap();

    assert!(makefile.contains("bench-v20:"));
    assert!(makefile.contains("run_v20_human_format_cohort.sh"));
    assert!(makefile.contains("bash -n benchmark/run_v20_human_format_cohort.sh"));
    assert!(downloader.contains("format-human-chm13-chr22"));
    assert!(downloader.contains("CHM13_autosome_PAR.chr22.vcf.gz.csi"));

    for required in [
        "VCF_FAST_V20_TIERS",
        "VCF_FAST_V20_RUNS",
        "VCF_FAST_V20_WARMUP",
        "VCF_FAST_HUMAN_FORMAT_VCF",
        "VCF_FAST_ALLOW_REMOTE_FULL",
        "CHM13_autosome_PAR.chr22.vcf.gz",
        "CHM13_autosome_PAR.chr22.vcf.gz.csi",
        "DDBJ",
        "public-human-genomes",
        "3715-sample",
        "27232829080",
        "FORMAT/AD",
        "FORMAT/DP",
        "FORMAT/GQ",
        "ANY(FORMAT/DP > 20)",
        "ALL(FORMAT/GQ >= 30)",
        "N_PASS(FORMAT/AD[1] > 10) >= 10",
        "QUAL > 30 && ANY(FORMAT/DP > 20)",
        "N_PASS(FMT/DP[*]>20)>0",
        "N_PASS(FMT/GQ[*]>=30)==",
        "N_PASS(FMT/AD[*:1]>10)>=10",
        "bcftools filter",
        "hyperfine",
        "measure_peak_rss_kb",
        "correctness result",
        "bounded streaming",
        "does not cache the 27 GB VCF",
    ] {
        assert!(script.contains(required), "missing harness text {required}");
    }

    for required in [
        "v2.0 Human FORMAT-Rich Cohort",
        "DDBJ CHM13",
        "3715-sample",
        "1000 requested / 1000 actual",
        "matched core records",
        "public human",
    ] {
        assert!(report.contains(required), "missing report text {required}");
    }
}

#[test]
fn vcftools_popgen_benchmark_scaffold_tracks_required_fields() {
    let root = repo_root();
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap();
    let script = fs::read_to_string(root.join("benchmark/run_vcftools_population_benchmarks.sh"))
        .expect("read VCFtools population benchmark script");
    let report =
        fs::read_to_string(root.join("benchmark/reports/vcftools-popgen-parity-benchmark.md"))
            .expect("read VCFtools population benchmark report");

    assert!(makefile.contains("bench-vcftools-popgen:"));
    assert!(makefile.contains("run_vcftools_population_benchmarks.sh"));
    assert!(makefile.contains("bash -n benchmark/run_vcftools_population_benchmarks.sh"));

    for required in [
        "VCF_FAST_VCFTOOLS_POPGEN_INPUT",
        "VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_INPUT",
        "VCF_FAST_VCFTOOLS_POPGEN_REPORT",
        "VCF_FAST_VCFTOOLS_POPGEN_RUNS",
        "VCF_FAST_VCFTOOLS_POPGEN_WARMUP",
        "VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_TIERS",
        "1000 10000 50000",
        "vcftools_input_flag",
        "--gzvcf",
        "public_population_files",
        "POPULATION_METADATA_HELPER",
        "prepare_public_biallelic_dataset",
        "bcftools view -m2 -M2",
        "mktemp",
        "mv -f",
        "human-format-cohort-1000.vcf.gz",
        "tests/data/popgen_stats.vcf",
        "make vcftools-parity",
        "benchmark/check_vcftools_parity.py",
        "frequency",
        "missingness",
        "HWE",
        "heterozygosity",
        "site pi",
        "window pi",
        "Tajima's D",
        "LD",
        "Weir-Cockerham Fst",
        "vcftools --version",
        "json_field",
        "$RESOURCE_RUNNER --json-out",
        "-- bash -lc",
        "peak_rss_kb",
        "cpu_seconds",
        "cpu_hours",
        "record count",
        "sample count",
        "runtime",
        "speedup",
        "peak RSS KB",
        "CPU seconds",
        "CPU-hour estimate",
        "exact VariantFlow command",
        "exact VCFtools command",
        "correctness result",
        "caveats",
        "pending",
    ] {
        assert!(script.contains(required), "missing script text {required}");
    }

    for required in [
        "VCFtools Population-Genetics Parity Benchmark",
        "runtime",
        "speedup",
        "input size",
        "record count",
        "sample count",
        "peak RSS KB",
        "CPU seconds",
        "CPU-hour estimate",
        "exact VariantFlow command",
        "exact VCFtools command",
        "VCFtools version",
        "correctness result",
        "caveats",
        "frequency",
        "missingness",
        "HWE",
        "heterozygosity",
        "site pi",
        "window pi",
        "Tajima's D",
        "LD",
        "Weir-Cockerham Fst",
        "staged bounded biallelic cohort",
        "public biallelic staged cohort",
        "This report does not support a broad VCFtools replacement claim",
    ] {
        assert!(report.contains(required), "missing report text {required}");
    }
}

#[test]
fn v16_vcftools_public_evidence_tracks_tiers_real_populations_and_resources() {
    let root = repo_root();
    let makefile = fs::read_to_string(root.join("Makefile")).expect("read Makefile");
    let script = fs::read_to_string(root.join("benchmark/run_vcftools_population_benchmarks.sh"))
        .expect("read VCFtools population benchmark script");
    let report =
        fs::read_to_string(root.join("benchmark/reports/vcftools-popgen-parity-benchmark.md"))
            .expect("read VCFtools population benchmark report");

    assert!(makefile.contains("bench-vcftools-popgen:"));
    assert!(script.contains("VCF_FAST_VCFTOOLS_POPGEN_PUBLIC_TIERS"));
    assert!(script.contains("1000 10000 50000"));
    assert!(script.contains("vcftools_population_metadata.py"));
    assert!(script.contains("command_resource_metrics.py"));
    assert!(script.contains("peak RSS"));
    assert!(script.contains("CPU seconds"));
    assert!(script.contains("CPU-hour estimate"));
    assert!(script.contains("$RESOURCE_RUNNER --json-out"));
    assert!(script.contains("-- bash -lc"));
    assert!(script.contains("json_field"));
    assert!(script.contains("peak_rss_kb"));
    assert!(script.contains("cpu_seconds"));
    assert!(script.contains("cpu_hours"));
    assert!(script.contains("real population files"));
    assert!(script.contains("public cohort 1000"));
    assert!(script.contains("public cohort 10000"));
    assert!(script.contains("public cohort 50000"));
    assert!(script.contains("blocked: public cohort tier staging requires bcftools and bgzip"));
    assert!(!script.contains("public cohort %s pending"));
    assert!(!script.contains(".plain.tmp.vcf"));

    for required in [
        "runtime mean",
        "peak RSS",
        "peak RSS KB",
        "CPU seconds",
        "CPU-hour estimate",
        "command_resource_metrics.py",
        "population source",
        "tier",
        "correctness result",
        "This report does not support a broad VCFtools replacement claim",
    ] {
        assert!(report.contains(required), "missing report text {required}");
    }
    assert!(
        report.contains("| public cohort 1000 |") || report.contains("| public cohort pending |")
    );
    assert!(!report.contains("| public cohort |"));
}

#[test]
fn v17_true_public_population_evidence_harness_is_declared() {
    let root = repo_root();
    let makefile = fs::read_to_string(root.join("Makefile")).expect("read Makefile");
    let script = fs::read_to_string(root.join("benchmark/run_v17_true_population_evidence.sh"))
        .expect("read v1.7 true public population benchmark script");
    let helper = fs::read_to_string(root.join("benchmark/igsr_population_files.py"))
        .expect("read IGSR population metadata helper");
    let report = fs::read_to_string(
        root.join("benchmark/reports/v17-true-public-population-evidence.md"),
    )
    .expect("read v1.7 true public population report");

    assert!(makefile.contains("bench-vcftools-true-popgen:"));
    assert!(makefile.contains("run_v17_true_population_evidence.sh"));
    assert!(makefile.contains("bash -n benchmark/run_v17_true_population_evidence.sh"));
    assert!(makefile.contains("python3 -m py_compile benchmark/igsr_population_files.py"));

    for required in [
        "VCF_FAST_V17_TRUE_POP_INPUT",
        "VCF_FAST_V17_TRUE_POP_METADATA",
        "VCF_FAST_V17_TRUE_POP_TIERS",
        "10000 50000 100000",
        "VCF_FAST_V17_TRUE_POP_GROUPS",
        "AFR:EUR",
        "prepare_true_public_biallelic_dataset",
        "bcftools view -m2 -M2 -v snps",
        "actual_records",
        "igsr_population_files.py",
        "population metadata source",
        "official",
        "no header-fallback",
        "frequency",
        "missingness",
        "HWE",
        "heterozygosity",
        "site pi",
        "window pi",
        "Tajima's D",
        "LD",
        "Weir-Cockerham Fst",
        "peak RSS KB",
        "CPU seconds",
        "CPU-hour estimate",
        "This report does not support a broad VCFtools replacement claim",
    ] {
        assert!(script.contains(required), "missing script text {required}");
    }

    for required in [
        "sample",
        "population",
        "superpopulation",
        "write_population_files",
        "unmatched samples",
        "AFR",
        "EUR",
        "EAS",
        "SAS",
        "AMR",
    ] {
        assert!(helper.contains(required), "missing helper text {required}");
    }

    for required in [
        "VariantFlow v1.7 True Public Population Evidence",
        "1000 Genomes / IGSR",
        "actual record count",
        "official population metadata",
        "population metadata source",
        "peak RSS KB",
        "CPU seconds",
        "CPU-hour estimate",
        "no broad VCFtools replacement claim",
    ] {
        assert!(report.contains(required), "missing report text {required}");
    }
}
