# v2.1 Indexed Filter Benchmark

This report template tracks v2.1 Indexed Filter performance for BGZF virtual offsets. It compares VariantFlow indexed filtering with the default native path and `bcftools`, and records skip rate evidence from `VCF_FAST_INDEX_REPORT`.

Rows are not yet measured until `make bench-v21-index` rewrites this file. Keep the caveat attached to any claim decision.

| tier records | chunks_total | chunks_skipped | skip rate | records_skipped_estimate | correctness result | runtime mean/stddev | speedup | variants/sec | peak RSS | claim decision | caveat |
| ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- | --- | --- |
| 10000 | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | caveat: run `make bench-v21-index` before making a performance claim |
