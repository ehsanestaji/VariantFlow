# VCF-Fast v0.7 Heavy-Run And Htslib Optimization Design

## Summary

VCF-Fast v0.7 keeps the Rust-native selective filter path as the winning core, then improves the next weak spots shown by v0.6 evidence: htslib-backed TSV/stats/BCF/BGZF paths and public heavy-run benchmarking. The goal is not to claim broad superiority. The goal is to make larger public evidence runs complete without huge plain IGSR intermediates, reduce obvious compatibility-path overhead, and publish a claim matrix that says exactly where VCF-Fast wins, matches, lags, or defers.

This is a balanced milestone: evidence first, safe hot-path work second, and no large platform jump into Arrow, Parquet, or DataFusion yet. Small focused dependencies are allowed only if they clearly improve streaming, measurement, or delimiter scanning without making the core harder to maintain.

## Research Grounding

The design follows three lessons from current tools and data systems:

- `bcftools` is the broad correctness/performance baseline. Its documented expression system supports INFO, FORMAT, sample-aware logic, vector indexing, arithmetic, regexes, and missing-value behavior that VCF-Fast should not try to duplicate all at once.
- HTSlib is the ecosystem compatibility substrate for BCF, BGZF, tabix/CSI indexes, and indexed region reads. `rust-htslib` exposes reader/writer threading and reusable records, which are direct opportunities for v0.7 compatibility-path optimization.
- Big-data systems win by avoiding unnecessary work: projection pushdown, predicate pushdown, partition/region pruning, bounded materialization, memory-local columnar layout, and honest measurement. Arrow and Parquet are relevant to the long-term tool vision, but v0.7 should apply their lessons before importing their full stack.

Primary references:

- bcftools manual and expression/query documentation: https://samtools.github.io/bcftools/
- HTSlib documentation and format ecosystem: https://www.htslib.org/doc/
- rust-htslib `set_threads` and writer/record APIs: https://docs.rs/rust-htslib/latest/rust_htslib/bcf/
- Apache Arrow columnar format: https://arrow.apache.org/docs/format/Columnar.html
- Apache Parquet concepts and row groups: https://parquet.apache.org/docs/concepts/

## Public Interface

No user-facing CLI behavior changes are required in v0.7:

```bash
vcf-fast filter input.vcf.gz --where "QUAL > 30" -o output.vcf.gz
vcf-fast stats input.vcf.gz
vcf-fast diff a.vcf.gz b.vcf.gz -o diff.tsv
vcf-fast convert input.vcf.gz --to tsv -o variants.tsv
vcf-fast filter input.bcf --where "QUAL > 30" -o output.vcf
vcf-fast filter input.vcf.gz --region chr22:1-20000000 --where "QUAL > 30" -o output.vcf
vcf-fast filter input.vcf --where "QUAL > 30" --compression bgzf -o output.vcf.gz
```

New benchmark and tuning interfaces:

```bash
VCF_FAST_BENCH_MODE=public-heavy make bench-smoke
make bench-heavy
VCF_FAST_HTSLIB_THREADS=4 cargo test --features htslib-static
```

`VCF_FAST_HTSLIB_THREADS` is an environment control first. A public `--threads` CLI option is deferred unless v0.7 measurements show users need explicit per-command control.

## Architecture

VCF-Fast remains a two-engine tool with a clear planner in front.

The native selective engine remains the default for `.vcf` and `.vcf.gz` streaming filters when no compatibility-only feature is requested. It preserves original records, evaluates required fields only, and remains the main speed claim.

The htslib compatibility engine remains optional and handles `.bcf`, indexed `--region`, and `--compression bgzf`. v0.7 makes this path leaner without changing its role: valid ecosystem-compatible output, not byte-preserving native output.

The benchmark harness becomes a stronger evidence layer. It must support heavy public runs while keeping intermediate files bounded. It should report native wins separately from htslib correctness matches and htslib speed gaps.

Main units:

- `src/compat.rs`: backend selection and htslib tuning configuration.
- `src/htslib_backend.rs`: htslib record reading, writer setup, typed field extraction, and compatibility execution.
- `benchmark/run_benchmarks.sh`: benchmark modes, heavy-run orchestration, correctness checks, artifact limits, and report fields.
- `benchmark/summarize_hyperfine.py`: repeated-run summaries, null-safe statistics, throughput, speedup, and RSS parsing.
- report files under `benchmark/reports/` and `docs/contribution-map.md`: evidence and claim matrix updates.

## Component Design

### Heavy-Run Benchmark Mode

Add a `public-heavy` benchmark mode and `make bench-heavy` target. The mode should benchmark larger GIAB/IGSR tiers without converting entire sample-rich public inputs into giant plain VCF files.

Acceptable strategies:

- keep public inputs compressed/indexed for as long as possible
- use bounded compressed subsets instead of unbounded plain VCF subsets
- use indexed region windows for sample-rich IGSR evidence
- use pipes where the benchmark case does not require a reusable materialized fixture
- hard-cap temporary artifact size and mark tiers deferred if the cap would be exceeded

Acceptance is not "always faster." Acceptance is "1M+ public evidence can run or defer honestly without exploding local disk."

### Htslib Thread Configuration

Add an htslib tuning helper that reads `VCF_FAST_HTSLIB_THREADS`.

Rules:

- unset means current behavior
- positive integer enables htslib reader/writer threading where the rust-htslib API supports it
- `0`, negative values, and non-integers fail clearly in tests or commands that use the setting
- unsupported threading surfaces should not panic; they should continue single-threaded with an explicit caveat when benchmark reporting needs it

### Reusable Htslib Records

Replace allocation-heavy `for result in reader.records()` loops in htslib paths where practical with reusable record loops using `empty_record()` and `read()`, if the rust-htslib API supports the needed reader type cleanly.

Targets:

- BCF input filtering
- indexed region filtering
- BCF and region TSV conversion
- BCF and region stats

The change must preserve current htslib integration tests.

### Fast Compatibility Field Extraction

Reduce `record.to_vcf_string()` usage in htslib paths.

Rules:

- Use typed record access for `CHROM`, `POS`, `ID`, `REF`, `ALT`, `QUAL`, and common filter values.
- Use direct INFO getters for numeric stats and predicates where exact textual representation is not required.
- Preserve exact `INFO/AF` text in TSV when available; if exact text requires raw VCF reconstruction, isolate that fallback to TSV only.
- Do not reconstruct a full VCF line in stats unless no typed alternative exists.
- Keep correctness against normalized `bcftools query -u` and overlapping `bcftools stats` counts as the deciding rule.

### Evidence And Claim Matrix

Add or update a v0.7 report with separate rows for:

- native filter
- native TSV
- htslib indexed-region filter
- htslib indexed-region TSV
- htslib indexed-region stats
- BCF filter
- BCF TSV
- BGZF output
- public-heavy tiers

Each row must state:

- correctness result
- runtime mean/stddev
- speedup
- variants/sec
- peak RSS
- dataset source and shape
- input format and output compression
- exact VCF-Fast command
- exact competitor command
- competitor version
- bottleneck
- caveat
- next action

## Data Flow

Native streaming path:

1. Read `.vcf` or `.vcf.gz`.
2. Preserve headers unchanged.
3. Analyze expression-required fields.
4. Parse only required VCF columns, INFO keys, FORMAT keys, and selected sample column.
5. Evaluate typed expression.
6. Write passing original record lines unchanged.

Htslib compatibility path:

1. Route `.bcf`, `--region`, or `--compression bgzf` to htslib.
2. Apply htslib thread config if set.
3. Reuse records where practical.
4. Extract typed fields directly for filter/stats/TSV.
5. Write valid VCF/BGZF output through htslib.
6. Report that output is valid and core-record equivalent, not byte-preserving.

Heavy-run path:

1. Start from cached public `.vcf.gz` or `.bcf` plus index.
2. Choose compressed subset, indexed-region, or streaming pipeline based on benchmark case.
3. Enforce artifact size caps before materializing output.
4. Run VCF-Fast and competitor commands with repeated measurements.
5. Run correctness diffs before reporting speed as a win.
6. Write report rows with measured results or explicit deferral.

## Error Handling

The benchmark harness must fail clearly for:

- missing public data
- missing `bcftools`, `tabix`, `hyperfine`, or GNU time when required
- empty public region
- invalid `VCF_FAST_HTSLIB_THREADS`
- unsupported htslib thread configuration
- artifact size cap exceeded
- correctness mismatch

If correctness fails, the report must not present speed as a win. If a tier is deferred due to artifact limits or runtime limits, the report must say deferred and include the reason.

## Testing And Evidence

Unit tests:

- backend selection still chooses native vs htslib deterministically
- htslib thread config parses valid values and rejects invalid values
- htslib-only behavior remains feature-gated in default builds
- heavy-run planner chooses compressed/streaming/region paths over unbounded plain intermediates
- report validation requires correctness, runtime, RSS, throughput, dataset source, commands, bottleneck, caveat, and next action fields

Integration tests:

- native `.vcf` and `.vcf.gz` filter tests remain line-preserving
- BCF input filtering still matches expected core records
- indexed region filtering still matches expected core records
- BGZF output remains gzip-readable, `bcftools view` readable, and `tabix -p vcf` indexable
- htslib TSV preserves `INFO/AF` precision
- htslib stats matches overlapping record counts
- heavy-run smoke uses a tiny compressed/indexed fixture and proves no large plain file is created

Benchmark acceptance:

- correctness diffs are empty before speedups are reported
- repeated runs complete
- peak RSS and variants/sec are present
- exact commands and tool versions are recorded
- temporary artifact sizes are bounded and documented

## Out Of Scope

- full bcftools expression parity
- multi-sample `ANY`/`ALL`
- arbitrary FORMAT keys beyond current support
- native BCF parser implementation
- replacing HTSlib
- Arrow export
- Parquet export
- DataFusion or DuckDB integration
- distributed execution
- GPU acceleration
- broad "best VCF tool" marketing claims

These remain roadmap topics. v0.7 should make the current engine more credible and measurable before expanding the platform.

## Success Criteria

v0.7 succeeds when:

- native selective filtering remains unchanged and verified
- public-heavy mode avoids giant plain IGSR intermediates
- htslib TSV/stats/BCF/BGZF paths have lower measured overhead or clearly documented remaining bottlenecks
- all compatibility correctness tests still pass
- benchmark reports separate wins from matches, lags, failures, and deferrals
- README and contribution-map claims are updated only from measured evidence

## Recommendation

Use the balanced design. It keeps VCF-Fast sharp where it already wins, attacks the measured weak paths without a risky rewrite, and imports the useful ideas from data engineering without prematurely pulling in a columnar execution stack.
