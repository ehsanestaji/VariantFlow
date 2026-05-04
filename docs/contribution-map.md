# VCF-Fast Contribution Map

VCF-Fast is not intended to be only a faster VCF scanner. The project is building toward a benchmarked execution engine for post-calling variant operations.

## Current Implemented Contributions

### Selective Parsing

The filter engine parses only fields required by the expression. For example, `QUAL > 30` does not parse INFO values, while `INFO/DP > 40` extracts the INFO field needed for that predicate.

Evidence:

- Filter implementation uses required-field analysis from the expression AST.
- Tests cover borrowed record parsing that skips unneeded INFO parsing.
- Benchmarks show speedups for QUAL, INFO/DP, INFO/AF, and gzip-input QUAL cases.

### Original-Record Preservation

The filter command writes original VCF header and record lines when records pass. This avoids expensive record reconstruction and preserves unsupported fields, sample columns, and formatting.

Evidence:

- CLI tests assert exact passing record preservation.
- gzip input/output tests verify compressed streaming behavior.

### Typed Expression Evaluation

The expression engine parses filter strings into an AST with typed numeric and string comparisons, boolean precedence, and parenthesized grouping.

Supported examples:

```text
QUAL > 30
INFO/DP > 10 && INFO/AF > 0.01
(QUAL > 55 || INFO/DP > 45) && FILTER == "PASS"
```

### Canonical Variant Keys

The `diff` command compares variants by `CHROM + POS + REF + ALT`, producing shared and unique variant sets as TSV.

Evidence:

- Integration tests verify shared, only-in-A, and only-in-B outputs.

### Reproducible Benchmarking

The benchmark harness builds deterministic synthetic VCFs, compares VCF-Fast against bcftools, checks output equivalence on filtered core records, and records results in Markdown.

Evidence:

- Docker installs `bcftools`, `hyperfine`, and the Rust toolchain.
- Current benchmark report: `benchmark/reports/synthetic-filter-benchmark.md`.

## Claims Supported So Far

- Correctness: VCF-Fast matches bcftools filtered core records for supported synthetic QUAL, INFO/DP, INFO/AF, and gzip-input QUAL cases.
- Performance: On the tracked 100k synthetic benchmark run, VCF-Fast was `1.62x` to `1.88x` faster than bcftools across measured supported cases.

## Claims Not Yet Proven

- Performance on public real-world VCFs.
- Performance on million-record and ten-million-record datasets.
- BGZF/tabix-compatible compressed output.
- FORMAT/sample-specific filtering.
- Persistent columnar Arrow/Parquet execution.

## Next Contribution Targets

1. Public dataset benchmark scripts.
2. Larger synthetic stress datasets with many unused INFO/FORMAT fields.
3. FORMAT-aware selective parsing for sample-level predicates.
4. Arrow/Parquet export for repeated analytical workloads.
