# VariantFlow v1.7 True Public Population Evidence Design

Date: 2026-05-07
Status: approved design direction; implementation plan not started

## Strategic Goal

VariantFlow should not be released as a broad bioinformatics tool until its
population-genetics evidence is stronger than smoke-scale or fallback-metadata
benchmarks. The next milestone is therefore not packaging. It is true public
population evidence: larger human-first cohort runs with real population labels,
followed by a plant cohort and targeted LD memory surgery.

The short version:

> Prove VariantFlow against VCFtools on real population cohorts before claiming
> replacement-grade population-genomics workflows.

This milestone directly addresses the current VCFtools evidence caveat: the
public tiers requested `1k`, `10k`, and `50k`, but the staged CHM13 benchmark
resolved to only `682` actual biallelic records and used header-derived fallback
population groups. That was useful as a correctness smoke, but not enough for a
serious replacement claim.

## Chosen Direction

Use a human-first evidence upgrade, with a plant cohort as the next proof row.

Primary dataset family:

- 1000 Genomes / IGSR high-coverage chr22 or chromosome-scale VCF/BCF.
- Official sample population and superpopulation metadata.
- Target staged biallelic tiers: `10000`, `50000`, `100000`, with larger opt-in
  tiers where local resources allow.

Secondary dataset family:

- A plant population cohort such as Arabidopsis 1001 Genomes or 3K Rice.
- Real accession/population/group metadata.
- Added after the human row is stable, because plant metadata and VCF formatting
  may need extra normalization.

This gives VariantFlow a defensible path for both general human bioinformatics
and the plant-genomics audience around Umea Plant Science Center.

## Non-Goals

- No public release tag from this milestone.
- No Bioconda PR from this milestone.
- No broad "VCFtools replacement" headline until larger cohort parity and
  benchmark rows pass.
- No paper figure refresh until the evidence rows exist.

Release hardening remains important, but it is intentionally delayed until the
tool can join the ecosystem with stronger correctness, efficiency, portability,
and professionalism.

## Workstreams

### 1. Dataset Discovery And Staging

Goal: find and cache a real population cohort with enough biallelic records and
real sample metadata.

Requirements:

- Prefer official 1000 Genomes / IGSR sources for the first human run.
- Use official population labels, not auto-derived header fallback groups.
- Stage bounded VCF/BCF tiers without giant plain VCF intermediates.
- Keep all large artifacts under ignored `tests/output/...` paths.
- Record source URL, download timestamp, checksum where practical, contig naming,
  sample count, record count, and metadata provenance.

Acceptance:

- Staged tiers contain at least `10k`, `50k`, and `100k` actual diploid
  biallelic records unless the dataset itself blocks that target.
- Population files contain real biological groups such as 1000 Genomes
  populations or superpopulations.
- The benchmark report states whether the population labels are official,
  curated, or fallback.

### 2. VCFtools Replacement Evidence

Goal: rerun the supported VCFtools-style workflows on true larger public
cohorts.

Required workflows:

- Allele frequency.
- Site missingness.
- Individual missingness.
- HWE.
- Heterozygosity.
- Site pi.
- Windowed pi.
- Tajima's D.
- Genotype LD, matching the supported VCFtools `--geno-r2` subset.
- Weir-Cockerham Fst.

Required report fields:

- Dataset source URL.
- Population metadata source.
- Input size.
- Actual record count.
- Sample count.
- Workflow command.
- Exact VariantFlow command.
- Exact VCFtools command.
- VCFtools version.
- Correctness result.
- Runtime mean and standard deviation.
- Variants/sec where meaningful.
- Peak RSS on Linux where available.
- CPU-hour estimate.
- Caveat.

Acceptance:

- Correctness must pass before speed is treated as a win.
- Rows with failed correctness are engineering targets, not performance claims.
- Smoke tiers below `10k` remain validation only.

### 3. Real Population Files

Goal: replace generated benchmark populations with biologically meaningful
groups.

Human-first policy:

- Build population files from official 1000 Genomes sample metadata.
- Support superpopulation comparisons such as AFR vs EUR, EAS vs EUR, and other
  documented groups where sample sizes are sufficient.
- Preserve sample identifiers exactly as they appear in the VCF header.
- Report dropped or unmatched samples.

Plant-next policy:

- Use accession metadata from the chosen plant resource.
- Keep group derivation explicit and reproducible.
- Document any accession-name normalization.

Acceptance:

- Fst rows state the exact population groups used.
- Missing samples fail clearly or are reported explicitly.
- Benchmark reports no longer rely on `header-fallback` for serious public
  claims.

### 4. VCFtools Edge Semantics

Goal: make the replacement claim precise rather than approximate.

Edge cases to close or document:

- HWE exact p-value output and rounding policy.
- Missing genotype behavior.
- Partially missing genotype behavior.
- Monomorphic sites.
- Multiallelic behavior policy.
- Non-diploid support or rejection.
- `--keep`, `--remove`, and population-file combinations.
- Window boundary semantics for pi, Tajima's D, and Fst.
- Undefined statistic output such as `nan`.

Acceptance:

- Supported cases match VCFtools on fixtures.
- Unsupported cases either fail with actionable errors or are documented as out
  of scope.
- Numeric tolerance is used only where floating-point formatting makes exact
  text comparison inappropriate.

### 5. LD Memory Surgery

Goal: keep the LD speed win while reducing the current RSS gap.

Current observation:

- VariantFlow LD is faster on the staged public row, but uses much higher RSS
  than VCFtools.

Planned investigation:

- Profile LD storage for genotype vectors and pairwise comparisons.
- Avoid retaining full per-site material when streaming windows are sufficient.
- Use compact diploid genotype encodings for supported biallelic sites.
- Bound LD window state by distance or site count.
- Preserve correctness against VCFtools `--geno-r2`.

Acceptance:

- LD RSS trend is reported for larger tiers.
- If memory remains higher than VCFtools, the caveat stays visible in the claim
  matrix.
- No memory optimization is merged without parity tests.

### 6. Optional PLINK And GATK Baselines

Goal: broaden credibility after the VCFtools public evidence row is stable.

PLINK-adjacent evidence:

- Compare overlapping genotype QC workflows where semantics align.
- Do not claim full PLINK replacement.
- Use this to position VariantFlow as useful for VCF-native genotype workflows.

GATK evidence:

- Add optional SelectVariants / VariantFiltration rows for filtering workflows.
- Keep GATK behind explicit environment flags because it is heavier.
- Use GATK rows to show ecosystem awareness, not as the first optimization
  target.

Acceptance:

- Optional baselines do not block default verification.
- Reports distinguish installed, skipped, unsupported, matched, and failed
  baselines.

### 7. Paper And Figure Upgrade

Goal: turn strong measured evidence into a clean Bioinformatics-style story.

Paper figure/table candidates:

- Native selective filtering wins.
- FORMAT aggregate filtering wins.
- Parquet repeated-query workflow wins.
- VCFtools-style population summaries with correctness and speed caveats.
- LD speed and memory tradeoff once optimized or fully documented.

Acceptance:

- Every figure/table value comes from tracked reports.
- The paper says where VariantFlow wins, matches, complements, or remains
  unproven.
- No broad "best tool" wording appears before the claim matrix supports it.

## Recommended Milestone Order

1. Human true-population evidence with 1000 Genomes / IGSR.
2. LD memory investigation based on the larger human tier.
3. VCFtools edge semantics hardening.
4. Plant population evidence, preferably Arabidopsis or rice.
5. Optional PLINK/GATK baselines.
6. Paper figure refresh.
7. Release hardening and Bioconda only after the evidence table is strong.

## Subagent-Driven Execution Model

Use focused workers when implementing this milestone:

- Worker 1: dataset discovery and population metadata staging.
- Worker 2: VCFtools benchmark harness and report schema.
- Worker 3: edge-semantics fixtures and normalizers.
- Worker 4: LD memory profiling and implementation proposal.
- Worker 5: docs, claim matrix, and paper figure inputs.

The coordinator reviews after each worker, runs targeted tests, keeps the branch
clean, and updates claims only from correctness-matched measurements.

## Success Criteria

This milestone is complete when:

- A real human public cohort benchmark has `10k`, `50k`, and `100k` actual
  biallelic rows or documents the exact blocker.
- Population files come from real biological metadata.
- All supported VCFtools-style workflows pass correctness checks or are clearly
  marked as failing targets.
- Reports include RSS and CPU-hour fields where the host environment supports
  them.
- The claim matrix distinguishes staged-smoke evidence from serious public
  population evidence.
- Release hardening remains deferred until these rows are strong enough to carry
  the project externally.
