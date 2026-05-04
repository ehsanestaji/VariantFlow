.PHONY: build test test-htslib fmt clippy verify bench-smoke bench-stress

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
	bash -n benchmark/generate_synthetic_vcf.sh
	bash -n benchmark/generate_stress_vcf.sh
	bash -n benchmark/run_benchmarks.sh
	python3 -m py_compile benchmark/*.py
	VCF_FAST_BENCH_SIZES="100" VCF_FAST_BENCH_RUNS=1 VCF_FAST_BENCH_WARMUP=0 ./benchmark/run_benchmarks.sh

bench-smoke:
	./benchmark/run_benchmarks.sh

bench-stress:
	VCF_FAST_BENCH_MODE=stress VCF_FAST_BENCH_SIZES="$${VCF_FAST_BENCH_SIZES:-10000 100000}" ./benchmark/run_benchmarks.sh
