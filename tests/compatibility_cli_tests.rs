use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::tempdir;

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

#[cfg(feature = "htslib")]
fn core_records(path: &Path) -> String {
    let text = std::fs::read_to_string(path).unwrap();
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| line.split('\t').take(7).collect::<Vec<_>>().join("\t"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(feature = "htslib")]
fn create_bgzf_vcf(input_vcf: &Path, output_vcf_gz: &Path) {
    use std::io::Write;

    let mut writer = rust_htslib::bgzf::Writer::from_path(output_vcf_gz).unwrap();
    writer
        .write_all(&std::fs::read(input_vcf).unwrap())
        .unwrap();
    drop(writer);
    rust_htslib::bcf::index::build(
        output_vcf_gz,
        None::<&Path>,
        1,
        rust_htslib::bcf::index::Type::Tbx,
    )
    .unwrap();
}

#[cfg(feature = "htslib")]
fn create_bcf(input_vcf: &Path, output_bcf: &Path) {
    use rust_htslib::bcf::header::Header;
    use rust_htslib::bcf::{Format, Read, Reader, Writer};

    let mut reader = Reader::from_path(input_vcf).unwrap();
    let header = Header::from_template(reader.header());
    let mut writer = Writer::from_path(output_bcf, &header, false, Format::Bcf).unwrap();
    for record in reader.records() {
        writer.write(&record.unwrap()).unwrap();
    }
}

#[cfg(not(feature = "htslib"))]
#[test]
fn region_filter_requires_htslib_feature_in_default_build() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("region.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/example.vcf").to_str().unwrap(),
            "--region",
            "1:1-1000",
            "--where",
            "QUAL > 30",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "--region requires the htslib feature",
        ));
}

#[cfg(not(feature = "htslib"))]
#[test]
fn bgzf_output_requires_htslib_feature_in_default_build() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("filtered.vcf.gz");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/example.vcf").to_str().unwrap(),
            "--where",
            "QUAL > 30",
            "--compression",
            "bgzf",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "--compression bgzf requires the htslib feature",
        ));
}

#[cfg(not(feature = "htslib"))]
#[test]
fn bcf_input_requires_htslib_feature_in_default_build() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.bcf");
    let output = dir.path().join("filtered.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            input.to_str().unwrap(),
            "--where",
            "QUAL > 30",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "BCF input requires the htslib feature",
        ));
}

#[cfg(feature = "htslib")]
#[test]
fn bcf_input_filter_matches_bcftools_core_records() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.bcf");
    let fast = dir.path().join("fast.vcf");
    create_bcf(&fixture("tests/data/compat_example.vcf"), &input);

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            input.to_str().unwrap(),
            "--where",
            "QUAL > 30",
            "-o",
            fast.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        core_records(&fast),
        "1\t200\trsPass\tC\tT\t35\tPASS\n\
2\t400\trsFiltered\tT\tC\t50\tq10\n\
2\t500\trsMulti\tA\tC,G\t60\tPASS"
    );
}

#[cfg(feature = "htslib")]
#[test]
fn htslib_commands_accept_valid_thread_env() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input.bcf");
    let output = temp.path().join("threaded.vcf");
    create_bcf(&fixture("tests/data/compat_example.vcf"), &input);

    assert_cmd::Command::cargo_bin("vcf-fast")
        .unwrap()
        .env("VCF_FAST_HTSLIB_THREADS", "2")
        .args([
            "filter",
            input.to_str().unwrap(),
            "--where",
            "QUAL > 30",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[cfg(feature = "htslib")]
#[test]
fn htslib_commands_reject_invalid_thread_env() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input.bcf");
    let output = temp.path().join("invalid-thread.vcf");
    create_bcf(&fixture("tests/data/compat_example.vcf"), &input);

    assert_cmd::Command::cargo_bin("vcf-fast")
        .unwrap()
        .env("VCF_FAST_HTSLIB_THREADS", "0")
        .args([
            "filter",
            input.to_str().unwrap(),
            "--where",
            "QUAL > 30",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "VCF_FAST_HTSLIB_THREADS must be a positive integer",
        ));
}

#[cfg(feature = "htslib")]
#[test]
fn htslib_path_rejects_arbitrary_format_predicates_in_v09() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("expression_parity.vcf.gz");
    let output = dir.path().join("arbitrary-format.vcf");
    create_bgzf_vcf(&fixture("tests/data/expression_parity.vcf"), &input);

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            input.to_str().unwrap(),
            "--region",
            "chr1:1-1000",
            "--sample",
            "HG002",
            "--where",
            "FORMAT/AD > 8",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "arbitrary FORMAT predicates are not implemented for htslib-backed input in v0.9",
        ));
}

#[cfg(feature = "htslib")]
#[test]
fn htslib_path_rejects_any_all_format_predicates_in_v09() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("expression_parity.vcf.gz");
    let output = dir.path().join("htslib-any-reject.vcf");
    create_bgzf_vcf(&fixture("tests/data/expression_parity.vcf"), &input);

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            input.to_str().unwrap(),
            "--region",
            "chr1:1-1000",
            "--where",
            "ANY(FORMAT/DP > 20)",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "ANY/ALL FORMAT predicates are not implemented for htslib-backed input in v0.9",
        ));
}

#[cfg(feature = "htslib")]
#[test]
fn bcf_input_filter_uses_info_af_and_filter_fields() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.bcf");
    let fast = dir.path().join("fast.vcf");
    create_bcf(&fixture("tests/data/compat_example.vcf"), &input);

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            input.to_str().unwrap(),
            "--where",
            "AF > 0.01 && FILTER == \"PASS\"",
            "-o",
            fast.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        core_records(&fast),
        "1\t100\trsLow\tA\tG\t10\tPASS\n\
1\t200\trsPass\tC\tT\t35\tPASS\n\
2\t500\trsMulti\tA\tC,G\t60\tPASS"
    );
}

#[cfg(feature = "htslib")]
#[test]
fn bcf_input_treats_undefined_info_key_as_missing() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.bcf");
    let output = dir.path().join("variants.tsv");
    let filtered = dir.path().join("filtered.vcf");
    create_bcf(
        &fixture("tests/data/compat_missing_info_filter.vcf"),
        &input,
    );

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "convert",
            input.to_str().unwrap(),
            "--to",
            "tsv",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert_eq!(
        std::fs::read_to_string(&output).unwrap(),
        "CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO/DP\tINFO/AF\n\
1\t100\trsMissingFilter\tA\tG\t40\t.\t11\t.\n\
1\t200\trsPass\tC\tT\t40\tPASS\t12\t.\n"
    );

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            input.to_str().unwrap(),
            "--where",
            "AF > 0.01",
            "-o",
            filtered.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert_eq!(core_records(&filtered), "");
}

#[cfg(feature = "htslib")]
#[test]
fn bcf_stats_observes_qual_and_af_without_vcf_text_reconstruction() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.bcf");
    create_bcf(&fixture("tests/data/compat_example.vcf"), &input);

    let output = assert_cmd::Command::cargo_bin("vcf-fast")
        .unwrap()
        .args(["stats", input.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["variants"], 5);
    assert_eq!(json["snps"], 6);
    assert_eq!(json["indels"], 0);
    assert_eq!(json["qual"]["count"], 4);
    assert_eq!(json["qual"]["min"], 10.0);
    assert_eq!(json["qual"]["max"], 60.0);
    assert_eq!(json["af"]["count"], 5);
    assert!((json["af"]["min"].as_f64().unwrap() - 0.005).abs() < 0.000001);
    assert_eq!(json["af"]["max"], 0.5);
    assert_eq!(json["missing_filter_values"], 0);
}

#[cfg(feature = "htslib")]
#[test]
fn bcf_input_preserves_filter_missing_semantics_for_stats_and_predicates() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.bcf");
    let filtered = dir.path().join("filtered.vcf");
    create_bcf(
        &fixture("tests/data/compat_missing_info_filter.vcf"),
        &input,
    );

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            input.to_str().unwrap(),
            "--where",
            "FILTER == \".\"",
            "-o",
            filtered.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert_eq!(
        core_records(&filtered),
        "1\t100\trsMissingFilter\tA\tG\t40\t."
    );

    let output = Command::cargo_bin("vcf-fast")
        .unwrap()
        .args(["stats", input.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["missing_filter_values"], 1);
}

#[cfg(feature = "htslib")]
#[test]
fn bcf_input_convert_to_tsv_matches_expected_rows() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.bcf");
    let output = dir.path().join("variants.tsv");
    create_bcf(&fixture("tests/data/compat_example.vcf"), &input);

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "convert",
            input.to_str().unwrap(),
            "--to",
            "tsv",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        std::fs::read_to_string(output).unwrap(),
        "CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO/DP\tINFO/AF\n\
1\t100\trsLow\tA\tG\t10\tPASS\t20\t0.02\n\
1\t200\trsPass\tC\tT\t35\tPASS\t12\t0.03\n\
1\t300\trsMissing\tG\tA\t.\tPASS\t50\t.\n\
2\t400\trsFiltered\tT\tC\t50\tq10\t5\t0.5\n\
2\t500\trsMulti\tA\tC,G\t60\tPASS\t22\t0.005,0.02\n"
    );
}

#[cfg(feature = "htslib")]
#[test]
fn indexed_region_convert_preserves_info_af_precision() {
    let dir = tempdir().unwrap();
    let plain = dir.path().join("input.vcf");
    let input = dir.path().join("input.vcf.gz");
    let output = dir.path().join("variants.tsv");
    std::fs::write(
        &plain,
        "##fileformat=VCFv4.3\n\
##contig=<ID=chr22>\n\
##FILTER=<ID=PASS,Description=\"All filters passed\">\n\
##INFO=<ID=DP,Number=1,Type=Integer,Description=\"Total Depth\">\n\
##INFO=<ID=AF,Number=A,Type=Float,Description=\"Allele Frequency\">\n\
#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n\
chr22\t10519265\tprecise\tCA\tC\t.\tPASS\tDP=7;AF=0.000312305\n",
    )
    .unwrap();
    create_bgzf_vcf(&plain, &input);

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "convert",
            input.to_str().unwrap(),
            "--region",
            "chr22:1-20000000",
            "--to",
            "tsv",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        std::fs::read_to_string(output).unwrap(),
        "CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO/DP\tINFO/AF\n\
chr22\t10519265\tprecise\tCA\tC\t.\tPASS\t7\t0.000312305\n"
    );
}

#[cfg(feature = "htslib")]
#[test]
fn indexed_bcf_region_stats_reports_subset_counts() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.bcf");
    create_bcf(&fixture("tests/data/compat_example.vcf"), &input);
    rust_htslib::bcf::index::build(
        input.as_path(),
        None::<&Path>,
        1,
        rust_htslib::bcf::index::Type::Csi(14),
    )
    .unwrap();

    let output = Command::cargo_bin("vcf-fast")
        .unwrap()
        .args(["stats", input.to_str().unwrap(), "--region", "1:1-250"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(json["variants"], 2);
    assert_eq!(json["snps"], 2);
    assert_eq!(json["variants_per_chromosome"]["1"], 2);
}

#[cfg(feature = "htslib")]
#[test]
fn indexed_region_filter_matches_bcftools_core_records() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.vcf.gz");
    let fast = dir.path().join("fast.vcf");
    create_bgzf_vcf(&fixture("tests/data/compat_example.vcf"), &input);

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            input.to_str().unwrap(),
            "--region",
            "1:1-250",
            "--where",
            "QUAL > 30",
            "-o",
            fast.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(core_records(&fast), "1\t200\trsPass\tC\tT\t35\tPASS");
}

#[cfg(feature = "htslib")]
#[test]
fn bgzf_output_is_gzip_readable_and_tabix_indexable() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("filtered.vcf.gz");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/compat_example.vcf").to_str().unwrap(),
            "--where",
            "QUAL > 30",
            "--compression",
            "bgzf",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let mut text = String::new();
    let file = std::fs::File::open(&output).unwrap();
    std::io::Read::read_to_string(&mut flate2::read::MultiGzDecoder::new(file), &mut text).unwrap();
    assert!(text.contains("rsPass"));
    rust_htslib::bcf::index::build(
        &output,
        Some(&output.with_extension("vcf.gz.tbi")),
        1,
        rust_htslib::bcf::index::Type::Tbx,
    )
    .unwrap();
}
