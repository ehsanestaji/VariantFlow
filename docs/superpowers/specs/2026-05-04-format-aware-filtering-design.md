# VCF-Fast v0.4 FORMAT-Aware Filtering Design

## Summary

VCF-Fast v0.4 adds targeted, Rust-native FORMAT-aware filtering for a single selected sample. The goal is to make VCF-Fast useful for common sample-level filtering while preserving its core advantage: selective streaming that avoids parsing unused VCF fields, unused FORMAT keys, and non-target samples.

This milestone keeps the public command structure stable and extends only the `filter` command with `--sample <name>`. It supports `FORMAT/GT`, `FORMAT/DP`, and `FORMAT/GQ` predicates, validates results against `bcftools filter`, and reports correctness/performance evidence before making new claims.

## Public Interface

Add one option to `filter`:

```bash
vcf-fast filter input.vcf.gz --where 'FORMAT/DP > 20 && FORMAT/GQ >= 30' --sample HG002 -o output.vcf.gz
```

Rules:

- `--sample <name>` is required when the expression references `FORMAT/...`.
- `--sample` is allowed for site-only expressions so users can reuse command templates.
- Missing `--sample` with FORMAT predicates exits non-zero with `FORMAT predicates require --sample <name>`.
- Unknown sample exits non-zero with `sample '<name>' not found in VCF header`.
- Existing site-level filter behavior remains unchanged.

Supported new fields:

```text
FORMAT/GT
FORMAT/DP
FORMAT/GQ
```

Supported comparisons:

```text
FORMAT/GT == "0/1"
FORMAT/GT != "./."
FORMAT/DP > 20
FORMAT/GQ >= 30
```

There are no v0.4 aliases for FORMAT fields. Bare `DP` continues to mean `INFO/DP`; users must write `FORMAT/DP` for sample-level depth.

## Architecture

Keep the existing small-module structure:

- `src/cli.rs`: add optional `--sample` to the `filter` command.
- `src/expr/mod.rs`: add `FORMAT/GT`, `FORMAT/DP`, and `FORMAT/GQ`; extend required-field analysis so the filter engine knows when FORMAT/sample parsing is needed.
- `src/vcf.rs`: add borrowed helpers for parsing sample names from `#CHROM`, resolving a selected sample column, and extracting selected sample FORMAT values.
- `src/engine/filter.rs`: resolve the target sample once from the `#CHROM` header, parse only the required FORMAT keys from the selected sample column, and keep writing passing original records unchanged.

Data flow:

1. Stream headers unchanged.
2. When `#CHROM` is seen, resolve the target sample index if FORMAT predicates are required.
3. For each record, parse the first eight VCF columns as today.
4. If FORMAT predicates are required, parse only the `FORMAT` column, the selected sample column, and the required FORMAT keys.
5. Evaluate the expression.
6. Write the original record line unchanged when it passes.

This preserves the project’s selective parsing claim: v0.4 should not parse unused FORMAT keys or non-target samples.

## Behavior

Missing FORMAT/sample data makes the relevant predicate false:

- selected sample value is `.`
- FORMAT key is absent
- sample has fewer colon fields than the FORMAT column
- numeric value is `.`
- numeric value cannot parse as a number
- requested sample is absent from the header

`FORMAT/GT` compares the raw genotype string exactly. For v0.4, `0/1` and `0|1` are different values.

Multi-allelic records are evaluated at the record/sample level. v0.4 does not normalize, decompose, or split records by allele.

v0.4 supports exactly one selected sample per run:

```bash
--sample HG002
```

The first milestone does not include `ANY`, `ALL`, sample regexes, or multi-sample lists. Those can be added after the single-sample path is correct and benchmarked.

FORMAT metadata header lines are not required for evaluation. The record-level `FORMAT` column controls parsing.

## Testing And Evidence

Unit tests cover:

- parsing sample names from `#CHROM`
- resolving `--sample HG002` to the correct sample column
- extracting `FORMAT/GT`, `FORMAT/DP`, and `FORMAT/GQ` from only the selected sample
- missing FORMAT key
- sample value `.`
- sample with fewer `:` fields than FORMAT
- numeric parse failure
- exact `FORMAT/GT` string comparison

Integration tests use a two-sample fixture with `HG002` and `NA12878`:

```bash
vcf-fast filter tests/data/format_example.vcf \
  --where 'FORMAT/DP > 20 && FORMAT/GQ >= 30' \
  --sample HG002 \
  -o out.vcf
```

Expected behavior:

- headers are preserved
- passing records are preserved unchanged
- only the selected sample column affects the result
- changing `--sample` changes the result when sample values differ
- missing `--sample` with FORMAT predicates fails clearly
- unknown sample fails clearly
- site-level filters still work without `--sample`

Benchmark correctness compares against `bcftools filter`, for example:

```bash
vcf-fast filter input.vcf --where 'FORMAT/DP > 20' --sample HG002 -o fast.vcf
bcftools filter -s HG002 -i 'FMT/DP > 20' input.vcf -o bcftools.vcf
```

The benchmark harness compares filtered core records as it does for site-level filters.

Stress benchmark cases add:

- `FORMAT/DP > 20`
- `FORMAT/GQ >= 30`
- `FORMAT/GT == "0/1"`

Reports include runtime, speedup, variants/sec, peak RSS, sample count, FORMAT shape, correctness result, competitor version, exact commands, and caveats.

Documentation updates happen only after measured results exist:

- `README.md`
- `docs/contribution-map.md`
- benchmark report for FORMAT-aware filtering

Allowed claim after successful measurement:

> FORMAT-aware filtering matched `bcftools filter` for selected-sample predicates and measured a documented speedup on stress FORMAT cases.

## Out Of Scope

- multiple selected samples
- `ANY` or `ALL` sample predicates
- sample-name regexes
- arbitrary FORMAT keys beyond `GT`, `DP`, and `GQ`
- BCF input
- BGZF/tabix-compatible output
- indexed region reads
- genotype normalization such as treating `0/1` and `0|1` as equivalent
- allele-specific FORMAT semantics

## Assumptions

- Rust remains the primary implementation language.
- `bcftools filter` is the primary correctness and performance competitor.
- v0.4 prioritizes selective parsing and correctness over broad FORMAT expressiveness.
- The first FORMAT milestone should be narrow enough to prove cleanly before generalizing.
