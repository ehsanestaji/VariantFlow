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

#[test]
fn convert_to_tsv_writes_stable_columns_and_missing_values() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("variants.tsv");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "convert",
            fixture("tests/data/convert_edge.vcf").to_str().unwrap(),
            "--to",
            "tsv",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = fs::read_to_string(output).unwrap();
    assert_eq!(
        text,
        "CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO/DP\tINFO/AF\n\
1\t100\trsComplete\tA\tG\t42\tPASS\t18\t0.125\n\
1\t200\t.\tC\tT\t.\tq10\t.\t0.25\n\
2\t300\trsNoAf\tG\tA\t12\tPASS\t7\t.\n\
2\t400\trsMulti\tT\tC,G\t99\tPASS\t20\t0.005,0.02\n"
    );
}

#[test]
fn convert_to_tsv_accepts_gzip_input() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("convert_edge.vcf.gz");
    let output = dir.path().join("variants.tsv");
    gzip_fixture(&fixture("tests/data/convert_edge.vcf"), &input);

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

    let text = fs::read_to_string(output).unwrap();
    assert_eq!(text.lines().count(), 5);
    assert!(text.contains("2\t400\trsMulti\tT\tC,G\t99\tPASS\t20\t0.005,0.02"));
}

#[test]
fn convert_rejects_unsupported_target() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("variants.parquet");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "convert",
            fixture("tests/data/convert_edge.vcf").to_str().unwrap(),
            "--to",
            "parquet",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported convert target"));
}
