.PHONY: build test test-htslib fmt clippy verify release-candidate-check bioconda-recipe-check paper-check benchmark-table vcftools-parity bench-vcftools-popgen bench-vcftools-true-popgen bench-smoke bench-stress bench-public bench-public-region bench-heavy bench-compat bench-v09 bench-v10-compressed bench-v10-parquet bench-v10-columnar bench-v11-parallel bench-v12 bench-v14 bench-v17 bench-v18 bench-v19 bench-v20 bench-v21-index bench-v21-public-index bench-v22-scheduler bench-v22-matrix bench-v06-smoke

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
	bash -n benchmark/run_v14_public_parallel_scale_benchmarks.sh
	bash -n benchmark/run_v17_public_format_baselines.sh
	bash -n benchmark/run_v18_public_format_expression_breadth.sh
	bash -n benchmark/run_v19_second_public_format_cohort.sh
	bash -n benchmark/run_v20_human_format_cohort.sh
	bash -n benchmark/run_v21_indexed_filter_benchmarks.sh
	bash -n benchmark/run_v22_scheduler_benchmarks.sh
	bash -n benchmark/run_v22_scheduler_matrix.sh
	bash -n benchmark/run_vcftools_population_benchmarks.sh
	bash -n benchmark/run_v17_true_population_evidence.sh
	bash -n benchmark/run_vcftools_parity.sh
	bash -n packaging/bioconda/variantflow/build.sh
	bash -n packaging/bioconda/variantflow/run_test.sh
	python3 -m py_compile benchmark/igsr_population_files.py
	python3 -m py_compile benchmark/*.py
	python3 -m py_compile packaging/*.py
	python3 packaging/check_bioconda_recipe.py
	make paper-check
	python3 benchmark/generate_public_benchmark_table.py --check
	VCF_FAST_BENCH_SIZES="100" VCF_FAST_BENCH_RUNS=1 VCF_FAST_BENCH_WARMUP=0 make bench-v06-smoke

release-candidate-check:
	make verify
	cargo test --features htslib-static
	cargo clippy --features htslib-static --all-targets -- -D warnings
	make bioconda-recipe-check
	make paper-check
	cd paper/bioinformatics-application-note && $(MAKE)

bioconda-recipe-check:
	bash -n packaging/bioconda/variantflow/build.sh
	bash -n packaging/bioconda/variantflow/run_test.sh
	python3 packaging/check_bioconda_recipe.py

paper-check:
	python3 -m py_compile paper/*.py
	python3 paper/check_paper.py
	@if [ "$${VCF_FAST_PAPER_INARA:-0}" = "1" ]; then \
		docker run --rm --volume "$$PWD/paper:/data" --user "$$(id -u):$$(id -g)" --env JOURNAL=joss openjournals/inara; \
	else \
		echo "Set VCF_FAST_PAPER_INARA=1 to compile paper/paper.md with Open Journals Inara Docker."; \
	fi

benchmark-table:
	python3 benchmark/generate_public_benchmark_table.py

vcftools-parity:
	./benchmark/run_vcftools_parity.sh

bench-vcftools-popgen:
	./benchmark/run_vcftools_population_benchmarks.sh

bench-vcftools-true-popgen:
	./benchmark/run_v17_true_population_evidence.sh

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

bench-v14:
	./benchmark/run_v14_public_parallel_scale_benchmarks.sh

bench-v17:
	./benchmark/run_v17_public_format_baselines.sh

bench-v18:
	./benchmark/run_v18_public_format_expression_breadth.sh

bench-v19:
	./benchmark/run_v19_second_public_format_cohort.sh

bench-v20:
	./benchmark/run_v20_human_format_cohort.sh

bench-v21-index:
	./benchmark/run_v21_indexed_filter_benchmarks.sh

bench-v21-public-index:
	VCF_FAST_V21_MODE=public-igsr VCF_FAST_V21_REPORT="$${VCF_FAST_V21_PUBLIC_REPORT:-benchmark/reports/v21-public-indexed-filter-benchmark.md}" VCF_FAST_V21_OUT_DIR="$${VCF_FAST_V21_PUBLIC_OUT_DIR:-tests/output/benchmark-results/v21-public-indexed-filter}" ./benchmark/run_v21_indexed_filter_benchmarks.sh

bench-v22-scheduler:
	./benchmark/run_v22_scheduler_benchmarks.sh

bench-v22-matrix:
	./benchmark/run_v22_scheduler_matrix.sh

bench-v06-smoke:
	VCF_FAST_BENCH_MODE=synthetic VCF_FAST_BENCH_SIZES="$${VCF_FAST_BENCH_SIZES:-100}" VCF_FAST_BENCH_RUNS="$${VCF_FAST_BENCH_RUNS:-1}" VCF_FAST_BENCH_WARMUP="$${VCF_FAST_BENCH_WARMUP:-0}" ./benchmark/run_benchmarks.sh
