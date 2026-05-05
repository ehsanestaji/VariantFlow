# VCF-Fast v1.0 Parquet Export First Slice Design

## Summary

The second v1.0 slice adds a native columnar export path:

```bash
vcf-fast convert input.vcf.gz --to parquet -o variants.parquet
```

This is not a full VCF-to-warehouse engine yet. It is the smallest trustworthy bridge from selective VCF streaming to repeated analytical workflows.

## Scope

The first Parquet schema mirrors the existing TSV projection:

- `CHROM`: UTF-8
- `POS`: int64
- `ID`: UTF-8
- `REF`: UTF-8
- `ALT`: UTF-8
- `QUAL`: nullable float64
- `FILTER`: UTF-8
- `INFO/DP`: nullable int64
- `INFO/AF`: nullable UTF-8

`AF` stays a string because comma-separated values must remain lossless in this slice. Missing numeric values such as `.`, empty values, and absent INFO keys become nulls. String columns preserve the original field text.

## Architecture

The native `convert` engine keeps its byte-streaming model. It reads only the first eight VCF columns, scans INFO only for `DP` and `AF`, appends values into Arrow builders, and flushes bounded record batches into a Parquet `ArrowWriter`. FORMAT and sample tails are skipped exactly as in TSV conversion.

HTSlib-backed `.bcf` and `--region` Parquet conversion are out of scope for this slice and return a clear error. TSV behavior remains unchanged.

## Evidence Contract

Tests must prove schema, row count, typed null handling, multiallelic AF preservation, gzip input support, and unchanged TSV behavior. The benchmark report starts as a scaffold until measured rows compare Parquet export against `bcftools query` plus a lightweight TSV-to-Parquet baseline.

README claims stay conservative: Parquet export exists as an analysis bridge, not yet a broad speed claim.
