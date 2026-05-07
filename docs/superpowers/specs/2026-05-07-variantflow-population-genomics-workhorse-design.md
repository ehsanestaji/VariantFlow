# VariantFlow Population-Genomics Workhorse Mega Design

Date: 2026-05-07
Status: approved design direction; implementation plan not started

## Strategic Position

VariantFlow should not rush into a public release. The next phase is a private
replacement-grade push: the tool should enter the ecosystem only when it is
strong enough to be taken seriously by bioinformaticians who currently rely on
VCFtools, bcftools, PLINK, GATK, and ad hoc VCF-to-table workflows.

The target identity is:

> VariantFlow is a Rust-native, evidence-tracked population-genomics engine for
> selective VCF/BCF operations, VCFtools-style summaries, cohort-scale genotype
> analysis, and analytical export.

The guiding product principle is replacement-grade trust before launch. That
means correctness, efficiency, portability, and professional engineering
quality all need explicit evidence before claims are broadened.

## Recommended Roadmap Shape

The chosen approach is **Population-Genomics Workhorse**.

This sits between a narrow VCFtools clone and an overly broad attempt to replace
every variant tool at once. It keeps VariantFlow's fast selective Rust engine as
the core advantage, but expands the tool into the workflows bioinformaticians
actually run around VCF files: population summaries, sample subsets, genotype
QC, windowed statistics, LD, Fst, and analytical export.

Public release, Bioconda, and the paper remain important, but they move later.
They should happen when the evidence table can carry the story.

## Versioned Pre-Release Train

### v1.6: VCFtools Replacement Evidence Expansion

Goal: make the VCFtools replacement claim much harder to dismiss.

Scope:

- Run repeated VCFtools public-cohort benchmarks with real population files.
- Add larger staged tiers where feasible: `1k`, `10k`, `50k`, `100k`, and
  larger opt-in tiers.
- Keep fixture parity for every supported command.
- Expand public-cohort parity for:
  - `--freq`
  - `--missing-site`
  - `--missing-indv`
  - `--hardy`
  - `--het`
  - `--site-pi`
  - `--window-pi`
  - `--TajimaD`
  - `--geno-r2`
  - `--weir-fst-pop`
- Add Linux RSS and CPU-hour estimates to serious public rows.
- Document unsupported VCFtools edge behavior explicitly.

Acceptance:

- Every supported command has fixture tests and VCFtools parity checks.
- Every public benchmark row reports correctness, runtime, peak RSS when
  available, variants/sec, CPU-hour estimate, competitor version, and caveat.
- Claims say `beats`, `matches`, `complements`, or `unsupported`; nothing else.
- No public release is made from this milestone alone.

### v1.7: Population-Genomics Workflow Layer

Goal: make VariantFlow useful as a daily analysis tool, not only a collection of
command clones.

Scope:

- Add a documented cohort summary workflow.
- Support richer sample and population handling:
  - multiple population files
  - sample metadata tables
  - population labels
  - include/exclude rules
  - clear errors for missing samples or malformed files
- Add windowed summary workflows:
  - allele-frequency windows
  - missingness windows
  - pi windows
  - Tajima's D windows
  - Fst windows
- Add machine-readable outputs where useful:
  - TSV
  - JSON
  - Parquet for repeated downstream analysis
- Add reproducible report bundles containing commands, versions, dataset
  metadata, benchmark results, and caveats.

Acceptance:

- A bioinformatician can run a documented workflow from VCF to population
  summary outputs.
- Outputs are checked against VCFtools or another trusted baseline where
  applicable.
- Reports are reproducible from ignored local benchmark artifacts and tracked
  scripts.

### v1.8: PLINK-Adjacent Genotype Workflows

Goal: cover genotype QC workflows users often run around VCFtools and PLINK,
without claiming full PLINK replacement before semantics are proven.

Scope:

- Add or harden PLINK-adjacent operations starting from VCF:
  - variant missingness
  - sample missingness
  - minor allele frequency filters
  - HWE filters
  - LD summaries
  - LD pruning candidates
  - genotype call-rate filters
- Add clean export paths:
  - TSV summaries
  - Parquet summaries
  - PLINK-compatible or PLINK-intermediate export only if semantics and testing
    are clear enough
- Add baselines:
  - PLINK where input/output semantics align
  - VCFtools for overlapping statistics
  - bcftools where filtering overlaps

Acceptance:

- VariantFlow replaces a meaningful subset of genotype QC workflows from VCF.
- Claims remain `PLINK-adjacent` unless exact PLINK parity is proven.
- Unsupported PLINK semantics are documented rather than approximated silently.

### v1.9: Ecosystem Baselines And Edge Cases

Goal: broaden trust against real tools and messy real data.

Scope:

- Add optional GATK baselines:
  - SelectVariants
  - VariantFiltration
- Expand VCFtools edge-case coverage:
  - missing genotypes
  - partially missing genotypes
  - monomorphic sites
  - multiallelic handling policy
  - non-diploid handling or rejection
  - chromosome naming differences
  - unsorted records
  - sparse population files
  - `nan` and undefined floating statistic behavior
- Add larger public human and plant cohorts.
- Make Linux RSS, CPU-hour estimates, input size, output size, and disk
  footprint first-class report fields.

Acceptance:

- The claim matrix covers VCFtools, bcftools/HTSlib, PLINK, and GATK with
  explicit statuses.
- Edge cases either match a baseline, are intentionally supported with
  documented semantics, or fail with actionable errors.

### v2.0: Replacement-Grade Public Candidate

Goal: reach the point where a public launch feels justified.

Scope:

- Freeze the supported workflow matrix.
- Harden CLI consistency and output stability.
- Regenerate all benchmark reports from tracked harnesses.
- Build paper-quality figures and tables from tracked reports.
- Prepare release artifacts, Bioconda recipe, and manuscript.
- Decide final launch claim from the evidence table.

Acceptance:

- No major claim exists without correctness and performance evidence.
- Install docs work from a clean machine.
- Bioconda recipe has a real tag, hash, license metadata, and package tests.
- The paper and README summarize the claim matrix instead of inventing claims.
- The project can honestly say:

  > VariantFlow is a replacement-grade Rust engine for supported
  > VCFtools-style population-genomics workflows and a faster selective
  > VCF/Parquet workflow engine where measured.

## Workstreams

### Engine Workstream

Purpose: keep the Rust-native core fast and memory-efficient.

Responsibilities:

- Keep byte-slice VCF record views as the foundation.
- Avoid per-sample rescans in all sample-heavy commands.
- Reuse FORMAT schema indexes per record.
- Keep streaming output for large result sets.
- Add shared genotype iterators for popgen, filters, LD, missingness, and
  PLINK-adjacent QC.
- Add reusable window accumulators for pi, Tajima's D, Fst, missingness, and
  allele frequency.

Risk:

- Popgen commands can duplicate parsing logic. Shared genotype, sample
  selection, and window modules should be extracted when duplication appears.

### Compatibility And Baseline Workstream

Purpose: prove correctness against trusted tools.

Responsibilities:

- Treat the VCFtools parity harness as a first-class gate.
- Add PLINK baselines only where semantics are clear.
- Keep GATK baselines optional and heavier.
- Ensure every benchmark row records:
  - exact VariantFlow command
  - exact competitor command
  - competitor version
  - correctness result
  - caveat
- Make normalizers explicit:
  - exact match where possible
  - numeric tolerance for floating statistics
  - documented `nan`, missing, and undefined-value behavior

Risk:

- Normalized output comparison can hide semantic drift. Every normalizer must
  document what it ignores and why.

### Evidence And Data Engineering Workstream

Purpose: make public evidence credible and reproducible.

Responsibilities:

- Use cached public datasets but avoid giant plain VCF intermediates.
- Prefer BGZF streaming, bounded tiers, and ignored output directories.
- Add real population files for human and plant cohorts.
- Standardize Linux measurements:
  - runtime mean/stddev
  - peak RSS
  - variants/sec
  - CPU-hour estimate
  - input size
  - output size
  - record count
  - sample count
- Use tiered runs:
  - smoke: `100`
  - development: `1k` and `10k`
  - evidence: `50k`, `100k`, and `1M` where feasible
  - full cohort: opt-in

Risk:

- Public-data downloads can break. Harnesses should cache, record source URLs,
  and record exact failure/caveat states instead of inventing evidence.

### Product And Workflow Workstream

Purpose: make VariantFlow coherent for bioinformaticians.

Responsibilities:

- Keep command names and output conventions consistent.
- Add workflow documentation:
  - VCFtools replacement recipes
  - population-genomics cohort reports
  - PLINK-adjacent QC from VCF
  - Parquet/DuckDB analysis
- Keep Snakemake and Nextflow examples current.
- Provide clear unsupported-case errors.

Risk:

- Too many commands can make the CLI feel scattered. Commands should be grouped
  by workflow and documented with equivalent baseline commands.

### Publication And Launch Workstream

Purpose: prepare the external story, but only after evidence is strong.

Responsibilities:

- Treat the Bioinformatics Application Note as the main paper route.
- Generate figures from tracked reports.
- Generate the public benchmark table from tracked reports.
- Keep the Bioconda recipe ready but do not submit it until the v2.0 candidate.
- Keep release notes exact about claim boundaries.

Risk:

- The paper can get ahead of evidence. Manuscript claims must point to the
  claim matrix and source reports.

## Evidence Rules

1. No claim without a baseline.
   - VCFtools for population summaries.
   - bcftools/HTSlib for filtering, BCF/BGZF/regions, stats, and query.
   - PLINK for genotype QC where semantics match.
   - GATK for heavier Java workflow comparisons.

2. Correctness comes before speed.
   - If correctness fails, speed is not reported as a win.
   - The row becomes an engineering target.

3. Exact where possible, normalized where necessary.
   - Exact file comparisons for deterministic count and TSV outputs.
   - Numeric tolerance for floating statistics.
   - Explicit `nan` and missing-value policies.
   - Documented unsupported cases.

4. Smoke runs are not evidence.
   - Smoke runs prove harness behavior.
   - Evidence rows require repeated runs, dataset metadata, competitor versions,
     and caveats.

5. Linux RSS and CPU-hours are first-class metrics.
   - Serious rows report memory and compute savings, not only wall time.

6. The claim matrix is the source of truth.
   - README, paper, and release notes summarize the claim matrix.
   - They do not introduce new performance claims.

## Replacement-Grade Criteria

A workflow becomes replacement-grade only when:

- The command exists and is documented.
- It has fixture tests.
- It has competitor parity tests.
- It has at least one public-cohort evidence row.
- It handles or clearly rejects edge cases.
- Its output format is stable.
- Its caveats are documented.
- The claim matrix states `beats`, `matches`, `complements`, or `unsupported`.

## v2.0 Launch Readiness

No broad public launch should happen until:

- VCFtools-style workflows have broader public evidence.
- PLINK-adjacent workflows have at least a first useful slice.
- GATK baseline status is documented, even if it is outside replacement scope.
- Bioinformatics paper figures are generated from tracked reports.
- Bioconda packaging is ready with real tag, hash, license metadata, and tests.
- Install docs work from a clean environment.
- Portability is tested across Linux and macOS release builds.
- The claim matrix is defensible line by line.

## Implementation Approach

Implementation should be subagent-driven when work begins. Suggested ownership:

- Worker 1: VCFtools public evidence, real population files, and larger tiers.
- Worker 2: shared genotype/window/sample-selection internals.
- Worker 3: PLINK-adjacent workflow design and first implementation slice.
- Worker 4: evidence reporting, Linux RSS, CPU-hour metrics, and claim matrix.
- Worker 5: paper figures, workflow docs, and launch readiness docs.

The coordinator should review each slice before integration, run the relevant
verification gate, and keep claims evidence-bound.

## Immediate Next Plan To Write

After this spec is approved, the next implementation plan should start with
v1.6:

1. Find or derive real population files for the current public human cohort.
2. Add repeated public VCFtools benchmark tiers with Linux RSS and CPU-hour
   metrics.
3. Extend VCFtools normalizers only where public rows reveal real differences.
4. Update reports and claim matrix only for correctness-matched rows.
5. Keep public release, Bioconda, and paper submission blocked until later
   replacement-grade gates pass.
