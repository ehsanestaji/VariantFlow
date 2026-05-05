use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

#[test]
fn stats_outputs_json_summary_for_site_level_metrics() {
    let output = Command::cargo_bin("vcf-fast")
        .unwrap()
        .args(["stats", fixture("tests/data/example.vcf").to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(json["variants"], 5);
    assert_eq!(json["snps"], 6);
    assert_eq!(json["indels"], 0);
    assert_eq!(json["variants_per_chromosome"]["1"], 3);
    assert_eq!(json["variants_per_chromosome"]["2"], 2);
    assert_eq!(json["missing_filter_values"], 0);
    assert_eq!(json["filter_counts"]["PASS"], 4);
    assert_eq!(json["filter_counts"]["q10"], 1);
    assert_eq!(json["qual"]["count"], 4);
    assert_eq!(json["qual"]["min"], 10.0);
    assert_eq!(json["qual"]["max"], 60.0);
    assert_eq!(json["af"]["count"], 5);
    assert_eq!(json["transition_transversion_ratio"], 5.0);
}

#[test]
fn stats_counts_stress_fixture_without_parsing_samples() {
    let output = Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "stats",
            fixture("tests/data/stress_small.vcf").to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(json["variants"], 4);
    assert_eq!(json["snps"], 4);
    assert_eq!(json["indels"], 0);
    assert_eq!(json["variants_per_chromosome"]["1"], 2);
    assert_eq!(json["variants_per_chromosome"]["2"], 2);
    assert_eq!(json["qual"]["count"], 3);
    assert_eq!(json["af"]["count"], 4);
}

#[test]
fn diff_writes_shared_and_unique_variant_keys_to_tsv() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("diff.tsv");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "diff",
            fixture("tests/data/diff_a.vcf").to_str().unwrap(),
            fixture("tests/data/diff_b.vcf").to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicates::str::contains(
            "shared=1 only_in_a=2 only_in_b=1",
        ));

    let text = fs::read_to_string(output).unwrap();
    assert_eq!(
        text,
        "status\tchrom\tpos\tref\talt\n\
only_in_a\t1\t100\tA\tG\n\
shared\t1\t200\tC\tT\n\
only_in_a\t2\t300\tAT\tA\n\
only_in_b\t2\t400\tG\tA\n"
    );
}

#[test]
fn diff_supports_mode_and_position_key_for_practical_comparisons() {
    let dir = tempdir().unwrap();
    let shared_output = dir.path().join("shared.tsv");
    let position_output = dir.path().join("position.tsv");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "diff",
            fixture("tests/data/diff_a.vcf").to_str().unwrap(),
            fixture("tests/data/diff_b.vcf").to_str().unwrap(),
            "--mode",
            "shared",
            "-o",
            shared_output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(shared_output).unwrap(),
        "status\tchrom\tpos\tref\talt\nshared\t1\t200\tC\tT\n"
    );

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "diff",
            fixture("tests/data/diff_a.vcf").to_str().unwrap(),
            fixture("tests/data/diff_b.vcf").to_str().unwrap(),
            "--key",
            "position",
            "-o",
            position_output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(position_output).unwrap(),
        "status\tchrom\tpos\tref\talt\n\
only_in_a\t1\t100\t.\t.\n\
shared\t1\t200\t.\t.\n\
only_in_a\t2\t300\t.\t.\n\
only_in_b\t2\t400\t.\t.\n"
    );
}

#[test]
fn diff_id_aware_key_includes_id_column() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("id-aware.tsv");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "diff",
            fixture("tests/data/diff_a.vcf").to_str().unwrap(),
            fixture("tests/data/diff_b.vcf").to_str().unwrap(),
            "--key",
            "id-aware",
            "--mode",
            "only-a",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(output).unwrap(),
        "status\tchrom\tpos\tid\tref\talt\n\
only_in_a\t1\t100\ta1\tA\tG\n\
only_in_a\t1\t200\tshared\tC\tT\n\
only_in_a\t2\t300\ta2\tAT\tA\n"
    );
}
