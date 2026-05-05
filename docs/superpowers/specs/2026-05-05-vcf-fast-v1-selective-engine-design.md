# VCF-Fast v1.0 Selective Engine Design

## Summary

VCF-Fast v1.0 should become the fastest trustworthy selective execution engine for supported VCF workflows: filtering, stats, diff, and export. The project should not claim to replace every VCF tool. It should claim exact measured strengths: native selective execution beats `bcftools` on correctness-matched public and stress cases where the evidence proves it, while HTSlib-backed paths preserve ecosystem compatibility for BCF, BGZF, and indexed regions.

The v1.0 route is evidence-led. First finish post-v0.8 evidence after the byte-core surgery, then expand expression coverage for real filters, then add parallel BGZF and columnar export as the major v1.0 differentiators.

## Positioning

The public v1.0 narrative is:

> VCF-Fast is the fastest trustworthy selective execution engine for supported VCF workflows: filtering, stats, diff, and export.

This means:

- Fastest only where repeated public or stress benchmarks prove it.
- Trustworthy means correctness matched against `bcftools`/HTSlib, with clear caveats.
- Selective execution remains the core advantage: parse only required fields, preserve original VCF records where possible, and avoid unused FORMAT/sample work.
- Export grows from TSV into Parquet, with Arrow either as an internal bridge or optional output if it stays simple.

VCF-Fast should be described as complementing `bcftools` today, beating it on measured native selective paths, and becoming better for repeated analytical workflows through columnar export.

## Milestones

### v0.8 Evidence Completion

Finish proof after the merged byte-core surgery:

- Run 1M, 3-repeat stress and public-heavy benchmarks.
- Compare native filter, native stats, native TSV, and public-heavy gzip paths against `bcftools`.
- Update README and the claim matrix only from measured rows.
- Outcome: exact evidence for what v0.8 improved and where it did not.

### v0.9 Expression Parity

Make real-user filters much more useful:

- Expand beyond fixed `INFO/DP`, `INFO/AF`, `FORMAT/GT`, `FORMAT/DP`, and `FORMAT/GQ`.
- Support selected arbitrary `INFO/*` and `FORMAT/*` numeric/string fields.
- Add sample semantics in this order: selected sample first, then `ANY`/`ALL` over sample groups.
- Match `bcftools` behavior where practical, and document intentional differences.
- Outcome: users can replace or complement common `bcftools filter` commands without losing the native selective speed path.

### v1.0 Parallel And Columnar

Add the major differentiators:

- Native parallel BGZF execution for large compressed VCF filtering and export, with deterministic output order.
- `convert --to parquet`; Arrow is allowed as an internal representation or optional output if it does not complicate the public contract.
- DuckDB, Polars, and PyArrow validation examples for repeated analysis workflows.
- Release-grade claim matrix, installation docs, reproducible benchmark reports, and exact caveats.
- Outcome: VCF-Fast is not only faster at selective filtering, but also more useful for repeated analysis.

## Architecture

### Native Selective Engine

The native engine remains the winning core. It handles `.vcf` and `.vcf.gz` streaming without `--region`, uses byte-slice parsing (`RecordView`, `InfoView`, and expression requirement analysis), preserves passing records for filters, and should become the home for parallel BGZF and native columnar export where practical.

### HTSlib Compatibility Engine

The HTSlib-backed engine remains the compatibility bridge. It handles `.bcf`, `--region`, and `--compression bgzf`. Its contract is valid ecosystem-compatible output for supported operations, not byte-for-byte original record preservation. Optimize it where measurements show real workflow pain, especially BCF TSV, but do not let it replace the native engine as the project identity.

### Benchmark And Evidence Layer

Benchmarking is a first-class subsystem. Every report row must include correctness result, runtime mean/stddev, speedup, variants/sec, peak RSS, exact VCF-Fast command, exact competitor command, competitor version, dataset source, dataset shape, and caveat. README claims and contribution-map claims must be copied only from measured reports.

### Export Layer

The export layer starts from the current TSV projection and expands to Parquet. Column definitions must be explicit and testable. Missing values, multiallelic values, INFO arrays, and FORMAT/sample projections must have documented output semantics before being claimed stable.

## Success Criteria

### Correctness

- Supported native filter outputs match `bcftools filter` core records.
- TSV and Parquet selected columns match normalized `bcftools query` where applicable.
- BCF, BGZF, and region paths remain HTSlib-compatible.
- Every mismatch becomes either a fixed bug or an explicit documented semantic difference.

### Performance

- Native selective filter remains faster than `bcftools filter` on repeated 1M stress and public-heavy cases.
- Native stats and export are at least competitive, with exact measured caveats when they are not.
- Parallel BGZF shows real wins over single-thread native execution and relevant `bcftools` baselines.
- Memory and RSS are reported rather than inferred.

### Workflow Coverage

- Users can express common filters involving `QUAL`, `FILTER`, arbitrary selected `INFO/*`, arbitrary selected `FORMAT/*`, selected samples, and `ANY`/`ALL` sample-group predicates.
- Users can export selected fields to TSV and Parquet.
- Users can keep using BCF, BGZF, and indexed-region workflows through HTSlib compatibility paths.

### Release Trust

- `make verify`, `cargo test --features htslib-static`, clippy, Docker tests, and benchmark smoke checks pass.
- README includes a claim matrix with `beats`, `matches`, `complements`, and `not yet proven`.
- v1.0 docs avoid broad "best VCF tool" language and say exactly where VCF-Fast wins.

## Scope Boundaries

In scope for v1.0:

- Full v0.8 post-surgery evidence run.
- Expression parity for common real filters.
- Native parallel BGZF execution.
- TSV plus Parquet export.
- Claim matrix against `bcftools`/HTSlib as the primary baseline.
- VCFtools and GATK tracked as later or optional heavier comparisons.

Out of scope for v1.0:

- Full `bcftools` language clone.
- Full GATK replacement.
- Complete normalization/decomposition engine.
- Association or statistical genetics workflows.
- GUI or web app.
- Broad "best VCF tool" claims.

The ambition is sharper because of these boundaries: VCF-Fast wins by becoming the best selective execution and export engine first.

## Implementation Defaults

- Rust remains the primary language.
- Native selective execution takes priority over compatibility-path speed unless evidence says otherwise.
- HTSlib interop stays optional and feature-gated.
- Benchmarks stay reproducible local commands; expensive public runs are not mandatory CI jobs.
- Public claims are updated only after correctness-matched repeated measurements.
