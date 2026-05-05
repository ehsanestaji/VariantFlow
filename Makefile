.PHONY: build test test-htslib fmt clippy verify bench-smoke bench-stress bench-public bench-public-region bench-heavy bench-compat bench-v09 bench-v10-compressed bench-v10-parquet bench-v10-columnar bench-v11-parallel bench-v12 bench-v06-smoke

build:
	cargo build

test:
	cargo test

test-htslib:
	cargo test --features htslib-static

fmt:
	cargo fmt --check

clippy:
	cargo clippy --all-targets -- -D warnings

verify:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo test
	bash -n benchmark/download_public_data.sh
	bash -n benchmark/bcftools_columnar_baseline.sh
	bash -n benchmark/generate_synthetic_vcf.sh
	bash -n benchmark/generate_stress_vcf.sh
	bash -n benchmark/run_benchmarks.sh
	bash -n benchmark/run_v09_expression_benchmarks.sh
	bash -n benchmark/run_v10_compressed_benchmarks.sh
	bash -n benchmark/run_v10_parquet_benchmarks.sh
	bash -n benchmark/run_v10_columnar_workflow_benchmarks.sh
	bash -n benchmark/run_v11_parallel_filter_benchmarks.sh
	bash -n benchmark/run_v12_public_parallel_workflow_benchmarks.sh
	python3 -m py_compile benchmark/*.py
	VCF_FAST_BENCH_SIZES="100" VCF_FAST_BENCH_RUNS=1 VCF_FAST_BENCH_WARMUP=0 make bench-v06-smoke

bench-smoke:
	./benchmark/run_benchmarks.sh

bench-stress:
	VCF_FAST_BENCH_MODE=stress VCF_FAST_BENCH_SIZES="$${VCF_FAST_BENCH_SIZES:-10000 100000}" ./benchmark/run_benchmarks.sh

bench-public:
	VCF_FAST_BENCH_MODE=public-whole VCF_FAST_PUBLIC_SOURCE=giab-hg002 VCF_FAST_BENCH_REPORT="$${VCF_FAST_GIAB_REPORT:-tests/output/benchmark-results/public-whole-giab-benchmark.md}" VCF_FAST_BENCH_SIZES="$${VCF_FAST_PUBLIC_RECORD_TIERS:-10000 100000 1000000}" ./benchmark/run_benchmarks.sh
	VCF_FAST_BENCH_MODE=public-whole VCF_FAST_PUBLIC_SOURCE=igsr-chr22 VCF_FAST_BENCH_REPORT="$${VCF_FAST_IGSR_REPORT:-tests/output/benchmark-results/public-whole-igsr-benchmark.md}" VCF_FAST_BENCH_SIZES="$${VCF_FAST_PUBLIC_RECORD_TIERS:-10000 100000 1000000}" ./benchmark/run_benchmarks.sh

bench-public-region:
	VCF_FAST_BENCH_MODE=public-region-repeated VCF_FAST_BENCH_REPORT="$${VCF_FAST_PUBLIC_REGION_REPORT:-tests/output/benchmark-results/public-region-repeated-benchmark.md}" VCF_FAST_BENCH_SIZES="$${VCF_FAST_PUBLIC_RECORD_TIERS:-10000 100000 1000000}" ./benchmark/run_benchmarks.sh

bench-heavy:
	VCF_FAST_BENCH_MODE=public-heavy VCF_FAST_BENCH_SIZES="$${VCF_FAST_BENCH_SIZES:-100000 1000000}" ./benchmark/run_benchmarks.sh

bench-compat:
	VCF_FAST_BENCH_MODE=compatibility VCF_FAST_BENCH_REPORT="$${VCF_FAST_COMPAT_REPORT:-tests/output/benchmark-results/compatibility-benchmark.md}" VCF_FAST_BENCH_SIZES="$${VCF_FAST_BENCH_SIZES:-10000 100000}" ./benchmark/run_benchmarks.sh

bench-v09:
	VCF_FAST_V09_SIZES="$${VCF_FAST_V09_SIZES:-10000 100000}" ./benchmark/run_v09_expression_benchmarks.sh

bench-v10-compressed:
	VCF_FAST_V10_SIZES="$${VCF_FAST_V10_SIZES:-10000 100000}" ./benchmark/run_v10_compressed_benchmarks.sh

bench-v10-parquet:
	VCF_FAST_V10_PARQUET_SIZES="$${VCF_FAST_V10_PARQUET_SIZES:-10000 100000}" ./benchmark/run_v10_parquet_benchmarks.sh

bench-v10-columnar:
	VCF_FAST_V10_COLUMNAR_SIZES="$${VCF_FAST_V10_COLUMNAR_SIZES:-10000 100000}" ./benchmark/run_v10_columnar_workflow_benchmarks.sh

bench-v11-parallel:
	VCF_FAST_V11_PARALLEL_SIZES="$${VCF_FAST_V11_PARALLEL_SIZES:-10000 100000}" ./benchmark/run_v11_parallel_filter_benchmarks.sh

bench-v12:
	./benchmark/run_v12_public_parallel_workflow_benchmarks.sh

bench-v06-smoke:
	VCF_FAST_BENCH_MODE=synthetic VCF_FAST_BENCH_SIZES="$${VCF_FAST_BENCH_SIZES:-100}" VCF_FAST_BENCH_RUNS="$${VCF_FAST_BENCH_RUNS:-1}" VCF_FAST_BENCH_WARMUP="$${VCF_FAST_BENCH_WARMUP:-0}" ./benchmark/run_benchmarks.sh
