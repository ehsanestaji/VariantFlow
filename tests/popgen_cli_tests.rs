use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::tempdir;

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

#[test]
fn freq_writes_vcftools_style_site_allele_frequencies() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("out.frq");

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "freq",
            fixture("tests/data/popgen_example.vcf").to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(output).unwrap(),
        "CHROM\tPOS\tN_ALLELES\tN_CHR\t{ALLELE:FREQ}\n\
1\t100\t2\t4\tA:0.75\tG:0.25\n\
1\t200\t3\t6\tC:0.5\tT:0.166667\tG:0.333333\n\
1\t300\t2\t2\tG:0\tA:1\n"
    );
}

#[test]
fn freq_respects_keep_sample_file() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("kept.frq");

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "freq",
            fixture("tests/data/popgen_example.vcf").to_str().unwrap(),
            "--keep",
            fixture("tests/data/popgen_keep.txt").to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(output).unwrap(),
        "CHROM\tPOS\tN_ALLELES\tN_CHR\t{ALLELE:FREQ}\n\
1\t100\t2\t4\tA:0.75\tG:0.25\n\
1\t200\t3\t4\tC:0.25\tT:0.25\tG:0.5\n\
1\t300\t2\t0\tG:.\tA:.\n"
    );
}

#[test]
fn missingness_writes_vcftools_style_site_and_individual_reports() {
    let dir = tempdir().unwrap();
    let prefix = dir.path().join("missingness");

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "missingness",
            fixture("tests/data/popgen_example.vcf").to_str().unwrap(),
            "-o",
            prefix.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(prefix.with_extension("lmiss")).unwrap(),
        "CHR\tPOS\tN_DATA\tN_GENOTYPE_FILTERED\tN_MISS\tF_MISS\n\
1\t100\t3\t0\t1\t0.333333\n\
1\t200\t3\t0\t0\t0\n\
1\t300\t3\t0\t2\t0.666667\n"
    );
    assert_eq!(
        fs::read_to_string(prefix.with_extension("imiss")).unwrap(),
        "INDV\tN_DATA\tN_GENOTYPE_FILTERED\tN_MISS\tF_MISS\n\
S1\t3\t0\t1\t0.333333\n\
S2\t3\t0\t1\t0.333333\n\
S3\t3\t0\t1\t0.333333\n"
    );
}

#[test]
fn missingness_respects_remove_sample_file() {
    let dir = tempdir().unwrap();
    let remove = dir.path().join("remove.txt");
    let prefix = dir.path().join("kept-missingness");
    fs::write(&remove, "S3\n").unwrap();

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "missingness",
            fixture("tests/data/popgen_example.vcf").to_str().unwrap(),
            "--remove",
            remove.to_str().unwrap(),
            "-o",
            prefix.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(prefix.with_extension("lmiss")).unwrap(),
        "CHR\tPOS\tN_DATA\tN_GENOTYPE_FILTERED\tN_MISS\tF_MISS\n\
1\t100\t2\t0\t0\t0\n\
1\t200\t2\t0\t0\t0\n\
1\t300\t2\t0\t2\t1\n"
    );
    assert_eq!(
        fs::read_to_string(prefix.with_extension("imiss")).unwrap(),
        "INDV\tN_DATA\tN_GENOTYPE_FILTERED\tN_MISS\tF_MISS\n\
S1\t3\t0\t1\t0.333333\n\
S2\t3\t0\t1\t0.333333\n"
    );
}
