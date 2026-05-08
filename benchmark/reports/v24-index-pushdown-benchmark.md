# VariantFlow v2.4 Index Pushdown Benchmark

This report tracks guarded `.vfi` pushdown evidence for `CHROM`, `POS`, `QUAL`, `FILTER`, `INFO/DP`, `INFO/AF`, and indexed numeric `INFO/<KEY>` predicates. The planner may skip chunks only when metadata proves no record can pass; uncertain cases must fall back to normal streaming.

## Required Fields

- chunks scanned
- chunks skipped
- skip rate
- fallback reason
- index build cost
- break-even query count
- exact VariantFlow command
- exact competitor command
- correctness result
- caveat

## Measured Rows

Source generated reports: `tests/output/benchmark-results/v24-index-pushdown/*/report.md` from Docker/Linux on 2026-05-08. Exact per-tier commands are retained in those generated reports.

| case | dataset | predicate | chunks scanned | chunks skipped | skip rate | fallback reason | index build cost | break-even query count | runtime mean/stddev | speedup | peak RSS KB | correctness result | claim decision |
| --- | --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| synthetic stress | 100k | `QUAL > 1000` | 0 | 13 | 100.0% | none | recorded in generated report | not yet computed | 0.015848s +/- 0.000123s | 22.64x vs default; 11.11x vs `bcftools` | indexed 4312; `bcftools` 3444 | default and indexed byte-for-byte match; indexed and `bcftools` core records match | `.vfi` proves a high-skip win for this synthetic QUAL row |
| synthetic stress | 1M | `QUAL > 1000` | 0 | 122 | 100.0% | none | recorded in generated report | not yet computed | 0.114456s +/- 0.002872s | 30.75x vs default; 15.64x vs `bcftools` | indexed 5380; `bcftools` 3444 | default and indexed byte-for-byte match; indexed and `bcftools` core records match | `.vfi` proves a high-skip win for this synthetic QUAL row |
| synthetic stress | 100k | `FILTER == "PASS"` | 13 | 0 | 0.0% | skip estimate below threshold | recorded in generated report | not applicable | 0.382177s +/- 0.005346s | 0.96x vs default; 0.68x vs `bcftools` | indexed 13572; `bcftools` 3488 | default and indexed byte-for-byte match; indexed and `bcftools` core records match | no indexed speed claim; safe fallback behavior |
| synthetic stress | 1M | `FILTER == "PASS"` | 122 | 0 | 0.0% | skip estimate below threshold | recorded in generated report | not applicable | 3.516366s +/- 0.012408s | 0.97x vs default; 0.69x vs `bcftools` | indexed 14500; `bcftools` 3448 | default and indexed byte-for-byte match; indexed and `bcftools` core records match | no indexed speed claim; safe fallback behavior |
| public IGSR chr22 | 100k attempted | `AF > 0.99` | 13 | 0 | 0.0% | skip estimate below threshold after regenerating a valid `.vfi` | recorded in generated report | not applicable | interrupted before accepted timing row | no claim | not accepted | correctness precheck matched for staged 100k; full timing interrupted because planner could not skip | public AF at current chunk size is not a high-skip `.vfi` win |

## Caveat

Do not generalize `.vfi` speedups from high-skip synthetic QUAL to public AF. The public IGSR AF predicate did not skip chunks at the current chunk size, so the next engineering target is better public high-skip predicate selection and/or finer chunking with explicit index-build amortization.
