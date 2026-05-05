use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn version_flag_reports_package_version() {
    Command::cargo_bin("vcf-fast")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "vcf-fast {}",
            env!("CARGO_PKG_VERSION")
        )));
}
