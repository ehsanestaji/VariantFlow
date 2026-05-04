use std::fs;
use std::path::Path;

#[test]
fn benchmark_harness_defines_report_and_correctness_contract() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = fs::read_to_string(root.join("benchmark/run_benchmarks.sh")).unwrap();

    assert!(script.contains("REPORT="));
    assert!(script.contains("## VCF-Fast Benchmark Report"));
    assert!(script.contains("VCF_FAST_BENCH_SIZES"));
    assert!(script.contains("bcftools filter"));
    assert!(script.contains("Output equivalence"));
    assert!(script.contains("hyperfine"));
}
