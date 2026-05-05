# VCF-Fast v1.0 Parallel BGZF First Slice Design

## Summary

The first v1.0 performance slice keeps the native selective filter as the winning core and targets compressed public VCF input. VCF-Fast will add an opt-in native multithreaded BGZF reader for `.vcf.gz` files that are actually BGZF, while preserving the existing flate2 gzip path for ordinary gzip streams.

The goal is not to claim universal parallel gzip support. BGZF is the right first target because tabix-indexed public VCFs are BGZF, BGZF blocks can be decompressed independently, and the native filter can still preserve passing record bytes after decompression.

## Public Interface

No CLI changes are added in this slice.

The new tuning knob is an environment variable:

```bash
VCF_FAST_NATIVE_BGZF_THREADS=4 vcf-fast filter input.vcf.gz --where "QUAL > 30" -o output.vcf
```

Behavior:

- unset or `1`: use the existing native flate2 gzip reader.
- positive integer greater than `1`: use the native multithreaded BGZF reader only when the input header is BGZF.
- ordinary gzip input with the variable set still falls back to flate2.
- `0`, negative values, or non-integers fail with a clear error.

## Architecture

`src/io.rs` owns compressed input selection. It detects BGZF by reading the gzip extra field and looking for the `BC` subfield. If the input is BGZF and `VCF_FAST_NATIVE_BGZF_THREADS` is greater than one, the reader uses `noodles_bgzf::io::MultithreadedReader`; otherwise it keeps the current `flate2::read::MultiGzDecoder`.

The filter, stats, and convert engines continue using `open_reader`, so they inherit the faster BGZF input path without changing their core loops. Output is unchanged: native filter still writes plain or gzip output based on the existing compression behavior, and htslib remains responsible for explicit `--compression bgzf`.

## Evidence Contract

Add `make bench-v10-compressed` to compare:

- default native gzip/BGZF input path
- native BGZF input with `VCF_FAST_NATIVE_BGZF_THREADS`
- `bcftools filter`

The report must include correctness result, exact commands, runtime mean/stddev, speedup, variants/sec, peak RSS, dataset size, thread count, and caveats. README claims should be updated only after correctness-matched measured rows exist.

## Caveats

This slice does not parallelize ordinary single-stream gzip. It does not add parallel output compression. It does not change htslib region or BCF paths. Those remain later v1.0 work after the BGZF input path is measured.
