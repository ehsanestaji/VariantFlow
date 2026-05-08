# VariantFlow Blazing-Fast v2.3-v3.0 Design

## Purpose

VariantFlow is already strongest where it can stay Rust-native, selective, line-preserving, and evidence-tracked. The next program turns that strength into a deeper performance stack: compressed-input scheduling, safer index pushdown, compact genotype kernels, columnar reuse, parser surgery, and a final public evidence pass.

The goal is not to claim that VariantFlow replaces every existing variant tool. The goal is sharper: make VariantFlow the leading option for the workflows where its design is naturally better, and prove those wins against trusted competitors with correctness-matched public evidence.

## Current Baseline

The current `main` branch is clean and pushed. VariantFlow already has:

- A Rust-native selective filter path with byte-preserving output for passing records.
- Optional htslib compatibility for BCF, indexed regions, and BGZF output.
- FORMAT-aware expressions including selected-sample predicates and aggregate predicates such as `ANY`, `ALL`, and `N_PASS`.
- Public FORMAT-rich cohort evidence on human CHM13 bounded tiers and other public cohorts.
- VCFtools-compatible population-genetics commands for supported diploid biallelic data.
- A `.vfi` index prototype with guarded safe fallback behavior.
- Parquet export and DuckDB repeated-query evidence.
- v2.2 scheduler evidence showing that tuned BGZF/predicate threading can improve public and stress FORMAT-heavy runs, while queue depth and batch size affect RSS strongly.

The strongest measured story today is selective native filtering and FORMAT-heavy aggregate filtering. The weakest performance areas are still full native BGZF pipeline coordination, deeper `.vfi` skip planning, LD/popgen memory, and compatibility/columnar workflows that need broader evidence.

## Design Principles

1. **Correctness before speed.** Any optimized path must match the existing native output byte-for-byte where line preservation is promised, and must match `bcftools` or VCFtools core results where external parity is the claim.
2. **Safe fallback.** Index, SIMD, and specialized kernels may accelerate only when their preconditions are proven. Otherwise the command falls back to the existing native streaming path.
3. **Bounded memory.** New parallel designs must use bounded queues and measured RSS gates, not unbounded buffering.
4. **No CLI churn.** Existing commands remain stable. Tuning stays behind documented environment variables unless a new explicit command is essential, such as index creation.
5. **Claims are report-backed.** README, paper, and claim matrix updates happen only from tracked reports with exact commands, versions, correctness, runtime, throughput, RSS, CPU seconds, and caveats.
6. **Release later.** Public packaging and Bioconda remain paused until the performance and correctness story is strong enough to enter the field professionally.

## v2.3 True Native BGZF Pipeline

### Goal

Replace the current partially overlapped native path with one coordinated BGZF input pipeline:

```text
BGZF block reader -> decode workers -> record batch workers -> ordered writer
```

The output must preserve input order and byte-for-byte record content for native filters.

### Scope

- Use existing tuning controls:
  - `VCF_FAST_NATIVE_BGZF_THREADS`
  - `VCF_FAST_NATIVE_FILTER_THREADS`
  - `VCF_FAST_NATIVE_FILTER_BATCH_RECORDS`
  - `VCF_FAST_NATIVE_FILTER_QUEUE_BATCHES`
- Keep ordinary `.vcf` streaming and htslib compatibility paths unchanged.
- Add internal pipeline components with explicit ownership:
  - BGZF block source and virtual-offset-aware chunk reader.
  - Bounded decode queue.
  - Record-batch splitter that preserves complete lines across block boundaries.
  - Predicate workers that return accepted original bytes with sequence numbers.
  - Ordered writer that flushes only when the next sequence is available.
- Preserve auto defaults from the v2.2 matrix unless new evidence beats them.

### Success Criteria

- Default native, forced single-thread, BGZF-only, predicate-only, combined pipeline, and existing fallback outputs match byte-for-byte on supported native filter cases.
- Core records match `bcftools filter`.
- Public low-skip compressed VCF cases improve or remain neutral without increasing RSS beyond the report-defined budget.
- FORMAT-heavy stress and human FORMAT-rich tiers retain the existing wins.

## v2.4 Smarter `.vfi` Index Planning

### Goal

Turn `.vfi` from useful metadata into a query planner that can skip compressed work only when it can prove no record in a chunk can pass.

### Scope

- Expand chunk metadata:
  - `CHROM` and `POS` range.
  - `QUAL` min/max and missing count.
  - `FILTER` token presence and absence.
  - Indexed numeric `INFO/<KEY>` min/max for selected keys.
  - `INFO/AF` any-value min/max semantics.
  - FORMAT key presence metadata, used only for fallback decisions unless sample-level skip rules become provable.
- Add expression-to-index requirement discovery:
  - Identify predicates that can be safely answered by chunk metadata.
  - Mark unsupported, mixed, or uncertain predicates as fallback.
- Add planning report fields:
  - chunks scanned
  - chunks skipped
  - skip percentage
  - fallback reason
  - index build cost
  - amortized break-even query count
- Prefer coalesced scan ranges for adjacent chunks when the underlying reader can exploit them.

### Success Criteria

- Indexed output matches default native output byte-for-byte.
- Low-skip public predicates fall back or report weak benefit honestly.
- High-skip predicates show measured wins against default native and `bcftools` where correctness matches.
- `.vfi` remains optional. Commands work exactly as before without it.

## v2.5 Bit-Packed Genotype Core

### Goal

Make VariantFlow a stronger VCFtools replacement by reducing memory and improving throughput for genotype-heavy operations.

### Scope

- Introduce a shared compact genotype representation for supported diploid biallelic records:
  - two-bit dosage states for `0`, `1`, `2`, and missing
  - optional bitsets for missingness and allele presence
  - window-local packed storage for LD
  - reusable per-site summaries for frequency, missingness, HWE, heterozygosity, pi, Tajima's D, and Fst
- Keep output formats compatible with existing VariantFlow and VCFtools parity tests.
- Keep unsupported cases explicit:
  - multiallelic genotypes
  - non-diploid genotypes
  - phased haplotype LD
  - VCFtools edge behavior outside the tested fixture/public scope
- Optimize LD first because it has the clearest RSS gap.
- Reuse the packed core for the rest of the population-genetics commands only after LD correctness and RSS improve.

### Success Criteria

- `make vcftools-parity` still passes.
- True public VCFtools population evidence remains correctness-matched.
- LD RSS drops substantially versus the current compact-window implementation.
- Runtime does not regress for frequency, missingness, HWE, heterozygosity, pi, Tajima's D, or Fst.

## v2.6 Columnar Pushdown And Reuse

### Goal

Make Parquet the repeated-analysis bridge: export once, then answer common cohort queries dramatically faster than repeated VCF scans.

### Scope

- Improve Parquet row-group sizing for public and stress cohorts.
- Preserve metadata and null semantics for:
  - `CHROM`
  - `POS`
  - `FILTER`
  - `QUAL`
  - `INFO/DP`
  - `INFO/AF`
  - selected FORMAT-derived summaries where supported
- Extend repeated-query benchmarks:
  - `QUAL > 30`
  - `INFO/DP > 40`
  - `FILTER == "PASS"`
  - grouped counts by `CHROM, FILTER`
  - grouped QUAL summaries
  - sample/FORMAT summary queries only after the packed genotype core stabilizes
- Compare DuckDB by default and keep Polars/PyArrow optional behind environment flags.

### Success Criteria

- Query results match normalized `bcftools` or VariantFlow native baselines.
- Reports show export time, query-only time, amortized time, break-even query count, and RSS.
- Claims remain workflow-specific: Parquet wins repeated analysis, not one-off streaming filters.

## v2.7 SIMD Parser Surgery

### Goal

Use profiling evidence to optimize the byte parsing hot loops without weakening portability or readability.

### Scope

- Profile after v2.3-v2.6 so SIMD targets reflect the new bottlenecks.
- Candidate helpers:
  - tab scanning
  - INFO `;` and `=` scanning
  - FORMAT `:` scanning
  - genotype separator scanning
  - integer parsing
  - float parsing
- Keep safe scalar or `memchr` fallbacks.
- Gate new dependencies behind measured wins and small internal APIs.

### Success Criteria

- SIMD or specialized parser helpers beat the current path on repeated public and stress benchmarks.
- No behavior changes in malformed-line handling, missing values, CRLF handling, or numeric predicate semantics.
- If profiling shows I/O or decompression dominates, SIMD work is deferred rather than forced.

## v2.8 Big Evidence Pass

### Goal

Run one coherent public evidence campaign after the major speed-stack changes.

### Evidence Rows

- IGSR public BGZF filtering:
  - default native
  - true BGZF pipeline
  - `.vfi` indexed high-skip predicates
  - `bcftools filter`
- CHM13 human FORMAT-rich cohort:
  - aggregate FORMAT expressions
  - selected-sample predicates
  - mixed site plus FORMAT predicates
- VCFtools true public population-genetics cohort:
  - frequency
  - missingness
  - HWE
  - heterozygosity
  - pi
  - window-pi
  - Tajima's D
  - LD
  - Weir-Cockerham Fst
- Columnar repeated workflows:
  - DuckDB default
  - optional Polars/PyArrow when installed

### Required Report Fields

- dataset source URL
- input size
- record count
- sample count
- compression and input format
- exact VariantFlow command
- exact competitor command
- competitor version
- runtime mean and stddev
- variants/sec or samples/sec
- peak RSS
- CPU seconds
- CPU-hour estimate
- correctness result
- caveat
- next action

### Success Criteria

- Reports separate wins, matches, slow paths, and unproven rows.
- README and claim matrix update only from correctness-matched rows.
- The paper can use the evidence table without inventing stronger wording.

## v3.0 Strong Public Candidate

### Goal

Prepare VariantFlow to enter public bioinformatics use only after the evidence stack is strong.

### Scope

- No Bioconda/tagged-release push until v2.8 evidence is reviewed.
- Refresh docs:
  - common workflows
  - VCFtools replacement scope
  - bcftools complementarity
  - GATK baseline status
  - Parquet repeated-analysis story
  - limitations and unsupported edge cases
- Refresh paper:
  - stronger abstract
  - clearer figures
  - public benchmark table
  - CPU-hour savings estimate
  - honest caveats
- Prepare release only when:
  - `make verify` passes
  - `cargo test --features htslib-static` passes
  - htslib clippy passes
  - VCFtools parity passes
  - key public benchmark reports are current

## Architecture Boundaries

### Native Streaming Core

Owns line-preserving `.vcf` and `.vcf.gz` filtering, byte-slice records, expression evaluation, and record output preservation. This remains the fast default.

### BGZF Pipeline

Owns compressed block scheduling and ordered pipeline execution. It must not leak concurrency concerns into expression evaluation or VCF parsing.

### Index Planner

Owns `.vfi` metadata, expression requirement discovery, skip/fallback decisions, and planning reports. It must never change filter semantics.

### Genotype Core

Owns compact diploid biallelic genotype and dosage buffers used by population-genetics commands. It must expose simple iterators/summaries so commands do not duplicate genotype parsing.

### Columnar Export

Owns Parquet schema, metadata, row-group sizing, and repeated-query benchmarks. It complements the native filter path rather than replacing it.

### Benchmark Harness

Owns evidence generation, competitor commands, report fields, correctness diffs, and claim-table regeneration. It is the gatekeeper for public wording.

## Testing Strategy

Each milestone starts with tests that prove expected behavior before performance changes:

- Byte-for-byte output equivalence for native filter modes.
- Core-record equivalence against `bcftools` for filter rows.
- VCFtools parity for population-genetics rows.
- DuckDB/columnar query equivalence against normalized VCF or `bcftools` baselines.
- Shell and Python syntax checks for benchmark harnesses.
- RSS and CPU reporting validation on Linux/Docker evidence runs.

Every milestone ends with:

```bash
make verify
cargo test --features htslib-static
cargo clippy --features htslib-static --all-targets -- -D warnings
```

Evidence milestones additionally run the relevant public/stress benchmark target and update tracked reports only from measured data.

## Implementation Order

1. v2.3 true native BGZF pipeline.
2. v2.4 smarter `.vfi` planning and pushdown.
3. v2.5 bit-packed genotype core, starting with LD memory.
4. v2.6 columnar pushdown and repeated-query expansion.
5. v2.7 SIMD parser surgery after profiling.
6. v2.8 big evidence pass.
7. v3.0 release and paper hardening.

This order is deliberate: compressed public VCF is the common path, index skipping amplifies selective filters, packed genotypes address VCFtools replacement workflows, columnar reuse addresses repeated analysis, and SIMD comes only after higher-level bottlenecks are exposed.

## Non-Goals

- No broad "best VCF tool" claim.
- No default htslib rewrite.
- No mandatory public-data downloads in CI.
- No unbounded buffering for speed.
- No release/Bioconda push before stronger evidence.
- No SIMD dependency unless profiling and benchmarks justify it.

## Open Risks

- BGZF pipeline complexity can introduce ordering bugs; sequence numbers and byte-for-byte tests are mandatory.
- `.vfi` skip logic can become unsafe if metadata semantics are too broad; fallback is required whenever proof is incomplete.
- Packed genotype kernels can accidentally drift from VCFtools edge behavior; parity tests must stay first-class.
- Full public runs can be limited by remote I/O and disk quotas; harnesses should support bounded tiers and `/dev/null` timed output.
- SIMD may be a small win if decompression dominates; profiling decides whether it proceeds.
