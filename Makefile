.PHONY: build test fmt clippy bench-smoke

build:
	cargo build

test:
	cargo test

fmt:
	cargo fmt --check

clippy:
	cargo clippy --all-targets -- -D warnings

bench-smoke:
	./benchmark/run_benchmarks.sh
