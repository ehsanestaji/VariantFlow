# Molecular Ecology Resources Cover Letter

Dear Editors,

We are pleased to submit the manuscript, "VariantFlow: a selective-execution
engine for efficient population genomic computation on large variant datasets,"
for consideration in Molecular Ecology Resources as a Computer Programs
submission.

VariantFlow is an open-source Rust command-line tool that accelerates
post-calling analysis of variant call sets. Population-scale projects now
produce call sets with thousands of samples and millions of sites, and because
the Variant Call Format stores every field of every record together, a tool
answering a field-limited question still parses the unused annotations, FORMAT
blocks, and per-sample values. That cost is incurred without changing the
result, and on large cohorts it dominates the wait.

The manuscript's central contribution is selective execution. Each query,
whether a filter expression, a population-genetic statistic, or a column
projection, is compiled into a typed predicate tree identifying the fields on
which the result depends; VariantFlow then streams once through the file and
decodes only those fields, treating each record as a borrowed byte-slice view
rather than materialising it as strings. Because the skipped fields cannot by
construction affect the result, the output is identical to that of a tool that
decodes every field. VariantFlow reads fewer fields, never fewer records, and
neither approximates nor samples; this equivalence is verified empirically
against established tools rather than asserted.

On that foundation, VariantFlow implements a suite of population-genetics
statistics that can be computed in a single read-through of the file, without
ever loading the whole file into memory — allele frequency, missingness,
heterozygosity, Hardy–Weinberg equilibrium, nucleotide diversity, a
missing-data-aware pi/dxy estimator matching pixy, Tajima's D, the site
frequency spectrum, Fst, and linkage disequilibrium. Alongside these biological
summaries it offers three data-handling features: selective VCF/BGZF filtering
(BGZF being the compressed VCF format), `.vfi` index-assisted filtering (an
optional index that skips straight to the matching sites), and Parquet export (a
columnar table format that makes repeated analytical queries fast).

The manuscript follows the Computer Programs category: a statement of need, the
design rationale, a description of function, usage, and output, and a
performance evaluation against existing software. It is written for the
journal's readership of practising biologists, in direct declarative prose that
defines technical terms at first use without assuming a computer-science
background.

The manuscript reports only correctness-matched benchmark evidence from tracked
repository reports, each independently validated against VCFtools and, where
applicable, pixy and scikit-allel. On the 1000 Genomes 3,202-sample
high-coverage dataset, VariantFlow accelerated missingness computation
8.0–8.5x over VCFtools at a constant 8.6 MB peak resident set that is
independent of chromosome size, and other supported statistics by 1.1–273x
depending on how much of each record the operation must decode. Population-genetic
outputs are byte-identical to VCFtools; per-individual missingness and the
site-frequency spectrum match scikit-allel exactly; and the missing-data-aware
pi and dxy estimators reproduce pixy's pairwise counts in every window. The paper
positions VariantFlow as a measured accelerator and complement to bcftools,
HTSlib, GATK, VCFtools, DuckDB, and scikit-allel, not a universal replacement;
Table S5 delineates the statistics it computes from those requiring a full
genotype or haplotype matrix resident in memory, which are better served by
scikit-allel.

We expect VariantFlow to be of practical value to the readership of Molecular
Ecology Resources: population and conservation/ecological genomics (nucleotide
diversity, differentiation, and linkage disequilibrium on large cohorts), plant
and animal breeding programmes (large-panel screening), large-cohort
human-genomics quality control (missingness and allele-frequency summaries), and
bioinformatics pipeline development (a fast, scriptable filtering and export
layer). Its distinctive contribution is bringing whole-cohort population genomic
summaries from batch to interactive timescales on a single workstation, while
preserving exact correctness against established tools; a companion online user
guide and statistics reference lower the barrier to adoption for students and
non-specialists.

The software is released under the MIT OR Apache-2.0 license. The submitted
version, source archive, and reproducibility materials are available at:

- Repository: https://github.com/ehsanestaji/VariantFlow
- Release tag: v1.5.0
- Archive DOI (v1.5.0 version DOI): 10.5281/zenodo.21198172
- Concept DOI (all versions): 10.5281/zenodo.21198171
- Public benchmark table: `docs/public-benchmark-table.md`

No new biological specimens or sequence data were generated for this study. The
benchmarks reuse publicly available data: the 1000 Genomes Project high-coverage
release for the population-genetic and indexed-filter results, and a 3,715-sample
chromosome 22 joint call set aligned to the T2T-CHM13 reference, distributed by
DDBJ, for the FORMAT-filter results; the Parquet/DuckDB results use a
deterministic synthetic call set generated by a script in the repository. All are
used under their existing data-access terms. A Data Accessibility
and Benefit-Sharing Statement is included in the manuscript, beneath the
references, as required by journal policy.

AI usage disclosure: AI coding assistants (OpenAI Codex and Anthropic Claude
Code) were used for planning, code-review support, test scaffolding,
benchmark-report organization, and prose drafting support. The human authors
reviewed and remain responsible for the manuscript, code, benchmarks, licensing,
and scientific claims.

All authors have approved this submission and declare no competing interests.
This manuscript is not currently under consideration elsewhere.

Sincerely,

Ehsan Estaji and Jian-Feng Mao
Umeå Plant Science Centre, Umeå University
Correspondence: ehsan.estaji@umu.se
