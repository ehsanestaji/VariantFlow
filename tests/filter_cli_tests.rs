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
