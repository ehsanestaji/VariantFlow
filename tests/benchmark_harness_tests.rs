use std::fs;
use std::path::Path;

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
