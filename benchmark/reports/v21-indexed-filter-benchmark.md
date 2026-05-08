# v2.1 Indexed Filter Benchmark

This report template tracks v2.1 Indexed Filter performance for BGZF virtual offsets. It compares VariantFlow indexed filtering with the default native path and `bcftools`, and records skip rate evidence from `VCF_FAST_INDEX_REPORT`.

Rows are not yet measured until `make bench-v21-index` rewrites this file. Keep the caveat attached to any claim decision.

| tier records | chunks_total | chunks_skipped | skip rate | records_skipped_estimate | correctness result | runtime mean/stddev | speedup | variants/sec | peak RSS | claim decision | caveat |
| ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- | --- | --- |
| 10000 | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | caveat: run `make bench-v21-index` before making a performance claim |

## Exact Commands

Command placeholders are not yet measured. A completed report must include exact commands for:

- index: `variantflow index <input.vcf.gz> -o <input.vcf.gz.vfi>`
- default native filter: temporarily disable `<input.vcf.gz.vfi>`, then run `variantflow filter <input.vcf.gz> --where 'QUAL > 1000' -o /dev/null`
- indexed native filter: `VCF_FAST_INDEX_REPORT=<report.json> variantflow filter <input.vcf.gz> --where 'QUAL > 1000' -o /dev/null`
- bcftools: `bcftools filter -Ov -i 'QUAL>1000' <input.vcf.gz> -o /dev/null`
