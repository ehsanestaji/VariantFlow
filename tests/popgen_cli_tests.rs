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
1\t100\t6\t0\t2\t0.333333\n\
1\t200\t6\t0\t0\t0\n\
1\t300\t5\t0\t3\t0.6\n"
    );
    assert_eq!(
        fs::read_to_string(prefix.with_extension("imiss")).unwrap(),
        "INDV\tN_DATA\tN_GENOTYPES_FILTERED\tN_MISS\tF_MISS\n\
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
1\t100\t4\t0\t0\t0\n\
1\t200\t4\t0\t0\t0\n\
1\t300\t3\t0\t3\t1\n"
    );
    assert_eq!(
        fs::read_to_string(prefix.with_extension("imiss")).unwrap(),
        "INDV\tN_DATA\tN_GENOTYPES_FILTERED\tN_MISS\tF_MISS\n\
S1\t3\t0\t1\t0.333333\n\
S2\t3\t0\t1\t0.333333\n"
    );
}

#[test]
fn hardy_reports_biallelic_hwe_counts_and_chisq() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("out.hwe");

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "hardy",
            fixture("tests/data/popgen_stats.vcf").to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(output).unwrap(),
        "CHROM\tPOS\tOBS_HOM_REF\tOBS_HET\tOBS_HOM_ALT\tE_HOM_REF\tE_HET\tE_HOM_ALT\tCHISQ_HWE\n\
1\t100\t1\t2\t1\t1\t2\t1\t0\n\
1\t200\t2\t1\t0\t2.083333\t0.833333\t0.083333\t0.12\n\
1\t300\t1\t1\t2\t0.5625\t1.875\t1.5625\t0.871111\n"
    );
}

#[test]
fn het_reports_individual_observed_expected_homozygosity() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("out.het");

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "het",
            fixture("tests/data/popgen_stats.vcf").to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(output).unwrap(),
        "INDV\tO_HOM\tE_HOM\tN_SITES\tF\n\
S1\t2\t1.6\t3\t0.30579\n\
S2\t2\t1.6\t3\t0.30579\n\
S3\t2\t1.6\t3\t0.30579\n\
S4\t1\t0.9\t2\t0.09677\n"
    );
}

#[test]
fn fst_reports_pairwise_population_estimates() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("out.fst");
    let explicit_output = dir.path().join("out.explicit-hudson.fst");

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "fst",
            fixture("tests/data/popgen_stats.vcf").to_str().unwrap(),
            "--pop",
            fixture("tests/data/popgen_pop1.txt").to_str().unwrap(),
            "--pop",
            fixture("tests/data/popgen_pop2.txt").to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();
    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "fst",
            fixture("tests/data/popgen_stats.vcf").to_str().unwrap(),
            "--pop",
            fixture("tests/data/popgen_pop1.txt").to_str().unwrap(),
            "--pop",
            fixture("tests/data/popgen_pop2.txt").to_str().unwrap(),
            "--estimator",
            "hudson",
            "-o",
            explicit_output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let expected = "CHROM\tPOS\tHUDSON_FST\n\
1\t100\t0.2\n\
1\t200\t0\n\
1\t300\t-0.166667\n";
    assert_eq!(fs::read_to_string(&output).unwrap(), expected);
    assert_eq!(fs::read_to_string(explicit_output).unwrap(), expected);
}

#[test]
fn fst_reports_weir_cockerham_estimates_when_requested() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("out.weir.fst");

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "fst",
            fixture("tests/data/popgen_stats.vcf").to_str().unwrap(),
            "--pop",
            fixture("tests/data/popgen_pop1.txt").to_str().unwrap(),
            "--pop",
            fixture("tests/data/popgen_pop2.txt").to_str().unwrap(),
            "--estimator",
            "weir-cockerham",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(output).unwrap(),
        "CHROM\tPOS\tWEIR_AND_COCKERHAM_FST\n\
1\t100\t0.2\n\
1\t200\t0.6\n\
1\t300\t-0.5\n"
    );
}

#[test]
fn weir_cockerham_fst_reports_nan_for_undefined_sites() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("undefined_fst.vcf");
    let pop1 = dir.path().join("pop1.txt");
    let pop2 = dir.path().join("pop2.txt");
    let output = dir.path().join("out.weir.fst");
    fs::write(
        &input,
        "##fileformat=VCFv4.2\n\
##FORMAT=<ID=GT,Number=1,Type=String,Description=\"Genotype\">\n\
#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1\tS2\tS3\tS4\n\
1\t100\t.\tA\tG\t50\tPASS\t.\tGT\t0/0\t0/1\t1/1\t0/1\n\
1\t400\t.\tC\tT\t50\tPASS\t.\tGT\t0/0\t0/0\t0/0\t0/0\n",
    )
    .unwrap();
    fs::write(&pop1, "S1\nS2\n").unwrap();
    fs::write(&pop2, "S3\nS4\n").unwrap();

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "fst",
            input.to_str().unwrap(),
            "--pop",
            pop1.to_str().unwrap(),
            "--pop",
            pop2.to_str().unwrap(),
            "--estimator",
            "weir-cockerham",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        fs::read_to_string(output)
            .unwrap()
            .contains("1\t400\tnan\n")
    );
}

#[test]
fn weir_cockerham_fst_rejects_multiallelic_sites() {
    let dir = tempdir().unwrap();
    let pop1 = dir.path().join("pop1.txt");
    let pop2 = dir.path().join("pop2.txt");
    let output = dir.path().join("out.weir.fst");
    fs::write(&pop1, "S1\nS2\n").unwrap();
    fs::write(&pop2, "S3\n").unwrap();

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "fst",
            fixture("tests/data/popgen_example.vcf").to_str().unwrap(),
            "--pop",
            pop1.to_str().unwrap(),
            "--pop",
            pop2.to_str().unwrap(),
            "--estimator",
            "weir-cockerham",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "weir-cockerham fst supports only biallelic sites",
        ));
}

#[test]
fn pi_and_window_pi_report_site_diversity() {
    let dir = tempdir().unwrap();
    let site_output = dir.path().join("out.sites.pi");
    let window_output = dir.path().join("out.windowed.pi");

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "pi",
            fixture("tests/data/popgen_stats.vcf").to_str().unwrap(),
            "-o",
            site_output.to_str().unwrap(),
        ])
        .assert()
        .success();
    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "pi",
            fixture("tests/data/popgen_stats.vcf").to_str().unwrap(),
            "--window-size",
            "200",
            "-o",
            window_output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(site_output).unwrap(),
        "CHROM\tPOS\tPI\n\
1\t100\t0.571429\n\
1\t200\t0.333333\n\
1\t300\t0.535714\n"
    );
    assert_eq!(
        fs::read_to_string(window_output).unwrap(),
        "CHROM\tBIN_START\tBIN_END\tN_VARIANTS\tPI\n\
1\t1\t200\t2\t0.003759\n\
1\t201\t400\t1\t0.002679\n"
    );
}

#[test]
fn pi_rejects_zero_window_size() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("out.windowed.pi");

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "pi",
            fixture("tests/data/popgen_stats.vcf").to_str().unwrap(),
            "--window-size",
            "0",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("--window-size must be positive"));
}

#[test]
fn tajima_d_reports_windowed_summary() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("out.Tajima.D");

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "tajima-d",
            fixture("tests/data/popgen_stats.vcf").to_str().unwrap(),
            "--window-size",
            "200",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(output).unwrap(),
        "CHROM\tBIN_START\tN_SNPS\tTajimaD\n\
1\t0\t1\t1.444161\n\
1\t200\t2\t0.395054\n"
    );
}

#[test]
fn tajima_d_reports_nan_for_empty_windows_between_observed_sites() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("gapped_tajima.vcf");
    let output = dir.path().join("out.Tajima.D");
    fs::write(
        &input,
        "##fileformat=VCFv4.2\n\
##FORMAT=<ID=GT,Number=1,Type=String,Description=\"Genotype\">\n\
#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1\tS2\tS3\tS4\n\
1\t100\t.\tA\tG\t50\tPASS\t.\tGT\t0/0\t0/1\t1/1\t0/1\n\
1\t500\t.\tC\tT\t50\tPASS\t.\tGT\t0/0\t0/1\t1/1\t0/1\n",
    )
    .unwrap();

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "tajima-d",
            input.to_str().unwrap(),
            "--window-size",
            "200",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        fs::read_to_string(output)
            .unwrap()
            .contains("1\t200\t0\tnan\n")
    );
}

#[test]
fn tajima_d_rejects_multiallelic_sites() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("out.Tajima.D");

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "tajima-d",
            fixture("tests/data/popgen_example.vcf").to_str().unwrap(),
            "--window-size",
            "200",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "tajima-d supports only biallelic sites",
        ));
}

#[test]
fn ld_reports_genotype_dosage_r2_between_biallelic_sites() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("out.geno.ld");

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "ld",
            fixture("tests/data/popgen_stats.vcf").to_str().unwrap(),
            "--max-distance",
            "250",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(output).unwrap(),
        "CHR\tPOS1\tPOS2\tN_INDV\tR^2\n\
1\t100\t200\t3\t0.75\n\
1\t100\t300\t4\t0.181818\n\
1\t200\t300\t3\t0.25\n"
    );
}
