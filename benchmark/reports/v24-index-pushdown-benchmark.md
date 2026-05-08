# VariantFlow v2.4 Index Pushdown Benchmark

This scaffold tracks guarded `.vfi` pushdown evidence for `CHROM`, `POS`, `QUAL`, `FILTER`, `INFO/DP`, `INFO/AF`, and indexed numeric `INFO/<KEY>` predicates. The planner may skip chunks only when metadata proves no record can pass; uncertain cases must fall back to normal streaming.

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

| case | dataset | predicate | chunks scanned | chunks skipped | skip rate | fallback reason | index build cost | break-even query count | runtime mean/stddev | speedup | peak RSS KB | correctness result | claim decision |
| --- | --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| pending | pending | pending | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured | not yet measured |

## Caveat

No README or claim-matrix update should be made from this scaffold. Claims require generated local rows where VariantFlow output matches default streaming and `bcftools` core records.
