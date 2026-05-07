# BGZF Virtual-Offset Index And Index-Aware Filtering Design

## Summary

VariantFlow should upgrade the current `.vfi` sidecar from record-chunk
metadata into a BGZF-aware index with virtual offsets, then use that index to
skip compressed chunks that cannot satisfy a supported filter predicate. The
index must never decide that a record passes. It can only prove that a chunk is
impossible and let the existing native evaluator remain the source of truth for
records that are read.

This is the recommended first sub-project in the broader blazing-fast roadmap
because it can create a new class of speedup: avoiding work entirely rather than
only parsing or scheduling the same records faster.

## Current Context

VariantFlow already has:

- native threaded BGZF input through `VCF_FAST_NATIVE_BGZF_THREADS`
- optional ordered predicate parallelism through `VCF_FAST_NATIVE_FILTER_THREADS`
- a first `.vfi` JSON sidecar with record-chunk metadata
- byte-slice native record evaluation
- Parquet export and repeated-query evidence
- rolling-window compact LD evidence against VCFtools

The current `.vfi` explicitly uses `offset_model = "record-chunk"` and
`virtual_offsets_available = false`. The next step is to add true BGZF virtual
offsets for `.vcf.gz` inputs and a conservative skip planner for supported
expressions.

## Goals

- Add BGZF virtual offsets to `.vfi` for BGZF `.vcf.gz` inputs.
- Keep plain VCF indexing useful as metadata-only with no virtual offsets.
- Add safe index-aware filtering that produces byte-for-byte identical output to
  default native filtering.
- Skip only chunks that metadata proves cannot contain a passing record.
- Fall back to normal streaming for missing, stale, unsupported, or incomplete
  indexes.
- Report skipped chunks/records in benchmark evidence before making speed
  claims.

## Non-Goals

- No automatic index creation during ordinary filtering.
- No broad claim that every filter is faster with an index.
- No use of index metadata to mark records as passing.
- No htslib rewrite for this milestone.
- No SIMD/parser surgery in this milestone.
- No bit-packed genotype core in this milestone.

## Approach

Use a conservative safe-skip index.

The index writer records virtual offsets and chunk summaries. The filter reads
the index only when safe. If a chunk might contain a matching record, VariantFlow
scans that chunk normally and evaluates each record with the existing expression
engine. If a chunk is proven impossible, VariantFlow seeks past it.

This approach has a lower speed ceiling than aggressive predicate indexing, but
it is much safer and easier to prove. A wrong skip would silently drop variants,
so the first implementation must choose correctness over maximal skip coverage.

## Architecture

### 1. BGZF Offset Reader

Add an internal BGZF block walker separate from the normal `open_reader` path.
It should:

- read BGZF blocks from `.vcf.gz`
- expose each block's compressed virtual start offset
- decompress block payloads for metadata scanning
- preserve enough block boundary information to create seekable index chunks

The normal streaming reader remains unchanged for ordinary filtering.

### 2. `.vfi` v2 Index Writer

Upgrade `variantflow index` so BGZF inputs produce virtual-offset chunks.

Each chunk should include:

- `schema_version`
- `index_kind = "variantflow-vfi"`
- `offset_model = "bgzf-virtual"`
- `virtual_offsets_available = true`
- source identity metadata: path, size, mtime, and optionally digest
- `virtual_start`
- `virtual_end`
- record count
- `CHROM`/`POS` range
- `QUAL` min/max
- `FILTER` value set
- `INFO/DP` min/max
- `INFO/AF` min/max when all values are numeric and parseable
- `INFO/AF` availability flag
- `FORMAT` key set

For plain VCF input, keep the existing record-chunk metadata and mark
`virtual_offsets_available = false`.

### 3. Safe Skip Planner

Add a small planner that inspects the parsed expression AST and one index chunk.
It returns one of:

- `CanSkip`
- `MustScan`
- `UnsupportedForIndex`

Unsupported expressions never fail because of the index. They simply use the
normal streaming path.

Initial supported skip predicates:

- `QUAL >`, `>=`, `<`, `<=`
- `FILTER ==`, `!=`
- `INFO/DP >`, `>=`, `<`, `<=`
- `INFO/AF >`, `>=`, `<`, `<=` when min/max metadata is complete
- boolean `&&`
- boolean `||`

Skip examples:

- `QUAL > 30`: skip if `qual_max <= 30`.
- `QUAL < 10`: skip if `qual_min >= 10`.
- `FILTER == "PASS"`: skip if `PASS` is absent from the chunk filter set.
- `INFO/DP > 40`: skip if `info_dp_max <= 40`.
- `INFO/AF > 0.2`: skip if `info_af_max <= 0.2`.
- `A && B`: skip if either side proves impossible.
- `A || B`: skip only if both sides prove impossible.

`!=` should be conservative. It may skip only when metadata proves all values in
the chunk equal the rejected value. Otherwise, scan.

### 4. Indexed Native Filter Path

Add an indexed native path for `.vcf.gz` only when:

- the input is BGZF
- a matching `.vfi` exists or is explicitly selected later
- the `.vfi` source identity matches the input
- virtual offsets are available
- the expression planner can make safe skip decisions

The indexed path should:

- write headers exactly once and unchanged
- read only non-skipped chunks
- evaluate records with the existing native evaluator
- write passing original record bytes in input order
- produce byte-for-byte identical output to default native filtering

If any requirement fails, use normal streaming.

## Data Flow

```text
variantflow index input.vcf.gz
  -> BGZF block walker
  -> record metadata scanner
  -> .vfi v2 JSON with virtual offsets and chunk summaries

variantflow filter input.vcf.gz --where EXPR
  -> load matching .vfi if present
  -> build skip planner from EXPR
  -> for each indexed chunk:
       CanSkip -> seek past chunk
       MustScan -> read chunk, evaluate records normally
       UnsupportedForIndex -> fall back to normal streaming
  -> ordered, line-preserving output
```

## Correctness And Fallbacks

Indexed filtering must match default filtering byte-for-byte for native
line-preserving cases.

Fallback behavior:

- missing `.vfi`: stream normally
- unsupported `.vfi` schema: stream normally with a clear note in verbose or
  benchmark output
- stale source identity: stream normally
- unsupported expression: stream normally
- missing metadata for a field: scan that chunk
- missing BGZF virtual offsets: stream normally
- BGZF seek/decode error: fail clearly

The safe rule is: uncertainty means scan.

## Testing

Unit tests:

- BGZF virtual-offset index writer emits `offset_model = "bgzf-virtual"`.
- Plain VCF index keeps `offset_model = "record-chunk"`.
- Source identity mismatch disables index use.
- Skip planner handles `QUAL`, `FILTER`, `INFO/DP`, `INFO/AF`, `&&`, and `||`.
- Skip planner treats unsupported fields as `UnsupportedForIndex`.
- Missing metadata returns `MustScan`.

Integration tests:

- `variantflow index bgzf.vcf.gz -o bgzf.vcf.gz.vfi`.
- Indexed `QUAL > 30` filter matches default output byte-for-byte.
- Indexed `FILTER == "PASS"` filter matches default output byte-for-byte.
- Indexed `INFO/DP > 40` filter matches default output byte-for-byte.
- Unsupported expression falls back and matches default output.
- Stale/corrupt index falls back or fails according to the failure policy.
- Combined BGZF-threaded and predicate-threaded filtering still matches output
  order.

## Benchmark Evidence

Add skip-heavy synthetic BGZF benchmark cases:

- mostly low `QUAL`, rare high `QUAL`
- mostly non-`PASS`, rare `PASS`
- mostly low `INFO/DP`, rare high `INFO/DP`

Compare:

- default native filter
- indexed native filter
- `bcftools filter`

Report:

- chunks total
- chunks skipped
- skipped record estimate
- runtime mean/stddev
- speedup
- variants/sec
- peak RSS
- exact commands
- correctness result
- caveats

Then run public evidence on IGSR chr22 BGZF tiers using predicates where skip
opportunities are plausible:

- `QUAL > 30`
- `FILTER == "PASS"`
- `INFO/DP > 40`

Update README and claim docs only for correctness-matched measured wins.

## Risks

- BGZF virtual-offset handling is easy to get subtly wrong. Tests must validate
  offsets with real BGZF data.
- Some predicates may skip little on public data. Reports should expose skip
  rate instead of hiding it.
- OR logic can be unsafe if treated too aggressively. The first planner should
  scan unless both sides prove impossible.
- Index source identity must prevent accidental use of stale metadata.

## Acceptance Criteria

- `.vfi` v2 is generated for BGZF `.vcf.gz` with virtual offsets.
- Plain VCF `.vfi` behavior remains supported and clearly metadata-only.
- Indexed native filtering falls back safely when unsupported.
- Supported indexed filters match default native output byte-for-byte.
- Benchmarks report skip rate, correctness, runtime, RSS, and exact commands.
- No broad performance claim is added without measured report rows.
