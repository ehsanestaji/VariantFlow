use std::fs;
use std::io::Write;
use std::path::Path;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

const FORMAT_RICH_VCF: &str = "\
##fileformat=VCFv4.3
#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1\tS2
chr1\t10\t.\tA\tG\t50\tPASS\tDP=12;AF=0.2\tGT:DP:AD\t0/1:12:6,6\t0/0:10:10,0
chr1\t20\t.\tC\tT\t.\tq10\tDP=5\tGT:DP\t0/1:5\t./.:.
";

fn write_bgzf(path: &Path, text: &str) {
    let file = fs::File::create(path).unwrap();
    let mut writer = noodles_bgzf::io::Writer::new(file);
    writer.write_all(text.as_bytes()).unwrap();
    writer.finish().unwrap();
}

#[test]
fn plain_vcf_index_records_source_identity_and_record_chunk_offset_model() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("format-rich.vcf");
    let output = dir.path().join("format-rich.vcf.vfi");
    fs::write(&input, FORMAT_RICH_VCF).unwrap();

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "index",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let json: Value = serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    assert_eq!(json["schema_version"], 2);
    assert_eq!(json["index_kind"], "variantflow-vfi");
    assert_eq!(json["offset_model"], "record-chunk");
    assert_eq!(json["virtual_offsets_available"], false);
    assert_eq!(
        json["source"]["size_bytes"],
        fs::metadata(&input).unwrap().len()
    );
    assert!(
        json["source"]["modified_unix_seconds"]
            .as_u64()
            .is_some_and(|seconds| seconds > 0)
    );
    assert_eq!(json["record_count"], 2);
}

#[test]
fn index_writes_query_aware_metadata_sidecar() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("format-rich.vcf");
    let output = dir.path().join("format-rich.vcf.vfi");
    fs::write(&input, FORMAT_RICH_VCF).unwrap();

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "index",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let json: Value = serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    assert_eq!(json["schema_version"], 2);
    assert_eq!(json["index_kind"], "variantflow-vfi");
    assert_eq!(json["offset_model"], "record-chunk");
    assert_eq!(json["virtual_offsets_available"], false);
    assert_eq!(
        json["source"]["size_bytes"],
        fs::metadata(&input).unwrap().len()
    );
    assert_eq!(json["record_count"], 2);
    assert_eq!(json["chunks"].as_array().unwrap().len(), 1);

    let chunk = &json["chunks"][0];
    assert_eq!(chunk["first_record"], 0);
    assert_eq!(chunk["record_count"], 2);
    assert_eq!(chunk["chrom_start"], "chr1");
    assert_eq!(chunk["chrom_end"], "chr1");
    assert_eq!(chunk["pos_min"], 10);
    assert_eq!(chunk["pos_max"], 20);
    assert_eq!(chunk["qual_min"], 50.0);
    assert_eq!(chunk["qual_max"], 50.0);
    assert_eq!(chunk["info_dp_min"], 5);
    assert_eq!(chunk["info_dp_max"], 12);
    assert_eq!(chunk["has_info_af"], true);

    let filters = chunk["filters"].as_array().unwrap();
    assert!(filters.iter().any(|value| value == "PASS"));
    assert!(filters.iter().any(|value| value == "q10"));

    let format_keys = chunk["format_keys"].as_array().unwrap();
    assert!(format_keys.iter().any(|value| value == "GT"));
    assert!(format_keys.iter().any(|value| value == "DP"));
    assert!(format_keys.iter().any(|value| value == "AD"));
}

#[test]
fn index_accepts_bgzf_vcf_input() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("format-rich.vcf.gz");
    let output = dir.path().join("format-rich.vcf.gz.vfi");
    write_bgzf(&input, FORMAT_RICH_VCF);

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "index",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .env("VCF_FAST_NATIVE_BGZF_THREADS", "2")
        .assert()
        .success();

    let json: Value = serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    assert_eq!(json["schema_version"], 2);
    assert_eq!(json["offset_model"], "bgzf-virtual");
    assert_eq!(json["virtual_offsets_available"], true);
    assert_eq!(json["record_count"], 2);
    assert!(json["chunks"][0]["virtual_start"].as_u64().is_some());
    assert!(
        json["chunks"][0]["virtual_end"].as_u64().unwrap()
            > json["chunks"][0]["virtual_start"].as_u64().unwrap()
    );
    assert_eq!(json["chunks"][0]["format_keys"][0], "AD");
}
