use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use arrow_array::{Array, Float64Array, Int64Array, StringArray};
use arrow_schema::DataType;
use assert_cmd::Command;
use flate2::Compression;
use flate2::write::GzEncoder;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
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
    let output = dir.path().join("variants.json");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "convert",
            fixture("tests/data/convert_edge.vcf").to_str().unwrap(),
            "--to",
            "json",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported convert target"));
}

#[test]
fn convert_to_parquet_writes_typed_columns_and_nulls() {
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
        .success();

    let file = fs::File::open(output).unwrap();
    let mut reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap()
        .build()
        .unwrap();
    let batch = reader.next().unwrap().unwrap();
    assert_eq!(batch.num_rows(), 4);

    let schema = batch.schema();
    assert_eq!(schema.field(0).name(), "CHROM");
    assert_eq!(schema.field(1).data_type(), &DataType::Int64);
    assert_eq!(schema.field(5).data_type(), &DataType::Float64);
    assert_eq!(schema.field(7).name(), "INFO/DP");
    assert_eq!(schema.field(8).name(), "INFO/AF");

    let chrom = batch
        .column(0)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let pos = batch
        .column(1)
        .as_any()
        .downcast_ref::<Int64Array>()
        .unwrap();
    let qual = batch
        .column(5)
        .as_any()
        .downcast_ref::<Float64Array>()
        .unwrap();
    let dp = batch
        .column(7)
        .as_any()
        .downcast_ref::<Int64Array>()
        .unwrap();
    let af = batch
        .column(8)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();

    assert_eq!(chrom.value(0), "1");
    assert_eq!(pos.value(3), 400);
    assert_eq!(qual.value(0), 42.0);
    assert!(qual.is_null(1));
    assert_eq!(dp.value(0), 18);
    assert!(dp.is_null(1));
    assert_eq!(af.value(3), "0.005,0.02");
    assert!(af.is_null(2));
    assert!(reader.next().is_none());
}

#[test]
fn convert_to_parquet_accepts_gzip_input() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("convert_edge.vcf.gz");
    let output = dir.path().join("variants.parquet");
    gzip_fixture(&fixture("tests/data/convert_edge.vcf"), &input);

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "convert",
            input.to_str().unwrap(),
            "--to",
            "parquet",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let file = fs::File::open(output).unwrap();
    let rows: usize = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap()
        .build()
        .unwrap()
        .map(|batch| batch.unwrap().num_rows())
        .sum();
    assert_eq!(rows, 4);
}

#[test]
fn convert_to_tsv_ignores_stress_format_and_sample_columns() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("stress.tsv");

    Command::cargo_bin("vcf-fast")
        .unwrap()
        .args([
            "convert",
            fixture("tests/data/stress_small.vcf").to_str().unwrap(),
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
1\t100\tstressLow\tA\tG\t10\tPASS\t20\t0.02\n\
1\t200\tstressPass\tC\tT\t35\tPASS\t45\t0.03\n\
2\t300\tstressAf\tG\tA\t50\tq10\t5\t0.005,0.25\n\
2\t400\tstressMissing\tT\tC\t.\tPASS\t.\t.\n"
    );
}
