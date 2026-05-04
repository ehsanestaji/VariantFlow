use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use flate2::Compression;
use flate2::write::GzEncoder;
use predicates::prelude::*;
use tempfile::tempdir;

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn gzip_fixture(input: &Path, output: &Path) {
    let bytes = fs::read(input).unwrap();
    let file = fs::File::create(output).unwrap();
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder.write_all(&bytes).unwrap();
    encoder.finish().unwrap();
}

fn read_gzip(path: &Path) -> String {
    let file = fs::File::open(path).unwrap();
    let mut decoder = flate2::read::GzDecoder::new(file);
    let mut text = String::new();
    std::io::Read::read_to_string(&mut decoder, &mut text).unwrap();
    text
}

#[test]
fn qual_filter_preserves_headers_and_original_passing_records() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("filtered.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/example.vcf").to_str().unwrap(),
            "--where",
            "QUAL > 30",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = fs::read_to_string(output).unwrap();
    assert!(text.contains("##fileformat=VCFv4.3\n"));
    assert!(text.contains("#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n"));
    assert!(!text.contains("rsLow"));
    assert!(text.contains("1\t200\trsPass\tC\tT\t35\tPASS\tDP=12;AF=0.03\n"));
    assert!(text.contains("2\t400\trsFiltered\tT\tC\t50\tq10\tDP=5;AF=0.50\n"));
    assert!(text.contains("2\t500\trsMulti\tA\tC,G\t60\tPASS\tDP=22;AF=0.005,0.02\n"));
}

#[test]
fn combined_filter_uses_info_dp() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("dp.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/example.vcf").to_str().unwrap(),
            "--where",
            "QUAL >= 30 && DP > 10",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = fs::read_to_string(output).unwrap();
    assert!(text.contains("rsPass"));
    assert!(text.contains("rsMulti"));
    assert!(!text.contains("rsFiltered"));
}

#[test]
fn af_filter_accepts_gzip_input_and_gzip_output() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("example.vcf.gz");
    let output = dir.path().join("af.vcf.gz");
    gzip_fixture(&fixture("tests/data/example.vcf"), &input);

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            input.to_str().unwrap(),
            "--where",
            "AF > 0.01 && FILTER == \"PASS\"",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = read_gzip(&output);
    assert!(text.contains("rsLow"));
    assert!(text.contains("rsPass"));
    assert!(text.contains("rsMulti"));
    assert!(!text.contains("rsFiltered"));
}

#[test]
fn filter_supports_parenthesized_or_expressions() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("or.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/example.vcf").to_str().unwrap(),
            "--where",
            "(QUAL > 55 || INFO/DP > 45) && FILTER == \"PASS\"",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = fs::read_to_string(output).unwrap();
    assert!(text.contains("rsMissing"));
    assert!(text.contains("rsMulti"));
    assert!(!text.contains("rsFiltered"));
    assert!(!text.contains("rsPass"));
}

#[test]
fn invalid_where_expression_exits_with_clear_error() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("bad.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/example.vcf").to_str().unwrap(),
            "--where",
            "QUAL >",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected literal"));
}

#[test]
fn format_predicates_require_sample_selection() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("format.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/stress_small.vcf").to_str().unwrap(),
            "--where",
            "FORMAT/DP > 20",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "FORMAT predicates require --sample <name>",
        ));
}

#[test]
fn stress_filter_preserves_unused_format_and_sample_columns() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("stress.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/stress_small.vcf").to_str().unwrap(),
            "--where",
            "QUAL > 30",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = fs::read_to_string(output).unwrap();
    assert!(
        text.contains(
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tSAMPLE_A\tSAMPLE_B\n"
        )
    );
    assert!(text.contains("1\t200\tstressPass\tC\tT\t35\tPASS\tUNUSED0=3;DP=45;UNUSED1=4;AF=0.03\tGT:DP:GQ:AD\t0/1:45:60:20,25\t0/1:41:55:19,22\n"));
    assert!(text.contains("2\t300\tstressAf\tG\tA\t50\tq10\tUNUSED0=5;DP=5;UNUSED1=6;AF=0.005,0.25\tGT:DP:GQ:AD\t1/1:5:20:0,5\t0/1:8:25:4,4\n"));
    assert!(!text.contains("stressLow"));
    assert!(!text.contains("stressMissing"));
}

#[test]
fn stress_filter_uses_info_dp_and_any_af_value() {
    let dir = tempdir().unwrap();
    let dp_output = dir.path().join("stress-dp.vcf");
    let af_output = dir.path().join("stress-af.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/stress_small.vcf").to_str().unwrap(),
            "--where",
            "DP > 40",
            "-o",
            dp_output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let dp_text = fs::read_to_string(dp_output).unwrap();
    assert!(dp_text.contains("stressPass"));
    assert!(!dp_text.contains("stressLow"));
    assert!(!dp_text.contains("stressAf"));

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/stress_small.vcf").to_str().unwrap(),
            "--where",
            "AF > 0.2",
            "-o",
            af_output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let af_text = fs::read_to_string(af_output).unwrap();
    assert!(af_text.contains("stressAf"));
    assert!(!af_text.contains("stressPass"));
}

#[test]
fn format_filter_uses_selected_sample_and_preserves_records() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("format.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/format_example.vcf").to_str().unwrap(),
            "--where",
            "FORMAT/DP > 20 && FORMAT/GQ >= 30",
            "--sample",
            "HG002",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = fs::read_to_string(output).unwrap();
    assert!(
        text.contains("#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tHG002\tNA12878\n")
    );
    assert!(text.contains(
        "1\t200\tfmtPass\tC\tT\t50\tPASS\tDP=20;AF=0.2\tGT:DP:GQ\t0/1:25:40\t0/0:5:10\n"
    ));
    assert!(!text.contains("fmtLow"));
    assert!(!text.contains("fmtOtherSample"));
    assert!(!text.contains("fmtMissing"));
    assert!(!text.contains("fmtShort"));
}

#[test]
fn format_filter_result_changes_with_sample() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("format-na12878.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/format_example.vcf").to_str().unwrap(),
            "--where",
            "FORMAT/DP > 20 && FORMAT/GQ >= 30",
            "--sample",
            "NA12878",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = fs::read_to_string(output).unwrap();
    assert!(text.contains("fmtLow"));
    assert!(text.contains("fmtOtherSample"));
    assert!(text.contains("fmtMissing"));
    assert!(text.contains("fmtShort"));
    assert!(!text.contains("fmtPass"));
}

#[test]
fn format_gt_filter_uses_exact_string_comparison() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("format-gt.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/format_example.vcf").to_str().unwrap(),
            "--where",
            "FORMAT/GT == \"0/1\"",
            "--sample",
            "HG002",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = fs::read_to_string(output).unwrap();
    assert!(text.contains("fmtLow"));
    assert!(text.contains("fmtPass"));
    assert!(text.contains("fmtShort"));
    assert!(!text.contains("fmtOtherSample"));
    assert!(!text.contains("fmtMissing"));
}

#[test]
fn format_filter_requires_sample() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("missing-sample.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/format_example.vcf").to_str().unwrap(),
            "--where",
            "FORMAT/DP > 20",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "FORMAT predicates require --sample <name>",
        ));
}

#[test]
fn format_filter_rejects_unknown_sample() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("unknown-sample.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/format_example.vcf").to_str().unwrap(),
            "--where",
            "FORMAT/DP > 20",
            "--sample",
            "MISSING",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "sample 'MISSING' not found in VCF header",
        ));
}

#[test]
fn sample_argument_is_allowed_for_site_only_filters() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("site-with-sample.vcf");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "filter",
            fixture("tests/data/example.vcf").to_str().unwrap(),
            "--where",
            "QUAL > 30",
            "--sample",
            "HG002",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();
}
