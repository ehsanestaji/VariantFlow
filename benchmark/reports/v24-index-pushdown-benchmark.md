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

Source generated reports: `tests/output/benchmark-results/v24-index-pushdown/*/report.md` from Docker/Linux on 2026-05-09. Exact per-tier commands are retained in those generated reports.

| case | dataset | predicate | chunks scanned | chunks skipped | skip rate | fallback reason | index build cost | break-even query count | runtime mean/stddev | speedup | peak RSS KB | correctness result | claim decision |
| --- | --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| synthetic stress | 100k | `QUAL > 1000` | 0 | 13 | 100.0% | none | recorded in generated report | not yet computed | 0.015848s +/- 0.000123s | 22.64x vs default; 11.11x vs `bcftools` | indexed 4312; `bcftools` 3444 | default and indexed byte-for-byte match; indexed and `bcftools` core records match | `.vfi` proves a high-skip win for this synthetic QUAL row |
| synthetic stress | 1M | `QUAL > 1000` | 0 | 122 | 100.0% | none | recorded in generated report | not yet computed | 0.114456s +/- 0.002872s | 30.75x vs default; 15.64x vs `bcftools` | indexed 5380; `bcftools` 3444 | default and indexed byte-for-byte match; indexed and `bcftools` core records match | `.vfi` proves a high-skip win for this synthetic QUAL row |
| synthetic stress | 100k | `FILTER == "PASS"` | 13 | 0 | 0.0% | skip estimate below threshold | recorded in generated report | not applicable | 0.382177s +/- 0.005346s | 0.96x vs default; 0.68x vs `bcftools` | indexed 13572; `bcftools` 3488 | default and indexed byte-for-byte match; indexed and `bcftools` core records match | no indexed speed claim; safe fallback behavior |
| synthetic stress | 1M | `FILTER == "PASS"` | 122 | 0 | 0.0% | skip estimate below threshold | recorded in generated report | not applicable | 3.516366s +/- 0.012408s | 0.97x vs default; 0.69x vs `bcftools` | indexed 14500; `bcftools` 3448 | default and indexed byte-for-byte match; indexed and `bcftools` core records match | no indexed speed claim; safe fallback behavior |
| public IGSR chr22 | 100k | `FILTER == "PASS"` | 0 | 13 | 100.0% | none | recorded in generated report | not computed | 0.028606s +/- 0.000133s | 3362.58x vs default; 122.95x vs `bcftools` | indexed 4528; `bcftools` 4396 | default and indexed byte-for-byte match; indexed and `bcftools` core records match | `.vfi` public high-skip win for a predicate where metadata proves no passing chunks |
| public IGSR chr22 | 1M | `FILTER == "PASS"` | 0 | 123 | 100.0% | none | recorded in generated report | not computed | 0.212767s +/- 0.000881s | 4158.36x vs default; 272.73x vs `bcftools` | indexed 6064; `bcftools` 4216 | default and indexed byte-for-byte match; indexed and `bcftools` core records match | `.vfi` public high-skip win for a predicate where metadata proves no passing chunks |
| public IGSR chr22 | 100k | `AF > 0.99`, chunk target 256 | 40 | 351 | 89.8% | none | recorded in generated report | not computed | 0.722366s +/- 0.003858s | 120.61x vs default; 4.93x vs `bcftools` | indexed 9896; `bcftools` 4412 | default and indexed byte-for-byte match; indexed and `bcftools` core records match | `.vfi` public high-skip win after finer chunking |
| public IGSR chr22 | 1M | `AF > 0.99`, chunk target 256 | 510 | 3397 | 86.9% | none | recorded in generated report | not computed | 7.242389s +/- 0.017060s | 116.73x vs default; 4.70x vs `bcftools` | indexed 66096; `bcftools` 4368 | default and indexed byte-for-byte match; indexed and `bcftools` core records match | `.vfi` public high-skip win after finer chunking |

## Caveat

Do not generalize `.vfi` speedups from high-skip synthetic QUAL to all public predicates. Public IGSR `FILTER == "PASS"` and `AF > 0.99` with chunk target `256` are correctness-matched high-skip wins through the 1M requested tier, but coarser AF chunks (`8192` and `1024`) fell back safely or did not beat `bcftools`. Public `QUAL > 1000` also fell back because useful QUAL bounds were absent in the staged IGSR records. The earlier focused 100k single-run row measured `4.87x` faster than `bcftools`; the completed matrix supersedes it with repeated 100k/1M rows. The next engineering step is better public predicate planning plus explicit index-build amortization and break-even query count reporting.

## v3.0 Public Matrix Attempt

The v3.0 pass added `VCF_FAST_INDEX_CHUNK_RECORDS` and fixed `.vfi` checksum normalization for large public metadata. The full public matrix completed. Accepted public wins are limited to correctness-matched rows where metadata proved enough chunks irrelevant: `FILTER == "PASS"` at 100k/1M and `AF > 0.99` with chunk target `256` at 100k/1M. Public `QUAL > 1000` and coarser AF chunk targets are retained as caveats because the guarded planner fell back or could not beat `bcftools`.
