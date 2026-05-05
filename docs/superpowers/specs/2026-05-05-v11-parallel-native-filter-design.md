# VCF-Fast v1.1 Parallel Native Filter Design

## Goal

Add the first native parallel filter execution path without changing the public CLI or weakening line-preserving output. The existing threaded BGZF input path already parallelizes decompression; this slice parallelizes record predicate evaluation and keeps output ordered.

## Chosen Approach

Use an opt-in environment variable, `VCF_FAST_NATIVE_FILTER_THREADS`, for native `.vcf` and `.vcf.gz` filters. When set to a value greater than `1`, the filter reads records into bounded batches, evaluates records in parallel with a fixed worker pool, then writes passing original record bytes back in the same input order. Header handling, sample resolution, expression parsing, and htslib routing stay unchanged.

This is deliberately safer than splitting files into independent output shards. It works with plain VCF, ordinary gzip, and threaded BGZF input, preserves the native line-preserving contract, and limits memory with a configurable batch size.

## Public Contract

- Existing CLI remains unchanged.
- `VCF_FAST_NATIVE_FILTER_THREADS=<N>` enables native parallel predicate evaluation.
- `VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=<N>` controls bounded batch size.
- Invalid values exit non-zero with a clear message.
- htslib-backed `.bcf`, `--region`, or `--compression bgzf` paths are not changed in this slice.

## Correctness

Parallel native output must match default native output byte-for-byte for supported filters, including stress fixtures with FORMAT/sample columns and aggregate predicates. Passing records remain the original input lines.

## Evidence

Add a v1.1 benchmark harness comparing:

- default native filter,
- parallel native filter,
- `bcftools filter`.

The first report focuses on deterministic stress VCFs where FORMAT aggregate predicates create enough CPU work for parallel evaluation to matter. Claims are updated only from measured rows with empty correctness diffs.

## v1.2 Handoff

If v1.1 shows wins, the next step is deeper scheduling: BGZF block-aware pipelines and possibly parallel native output compression. If v1.1 is neutral on I/O-bound cases, keep the feature as an opt-in CPU-heavy expression path and focus v1.2 on decompression/output bottlenecks.
