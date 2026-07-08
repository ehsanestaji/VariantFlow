# Molecular Ecology Resources Cover Letter Template

Dear Editors,

We are pleased to submit the manuscript, "VariantFlow: a selective-execution
engine for efficient population genomic computation on large variant
datasets," for consideration in Molecular Ecology Resources, as a Computer
Programs submission.

VariantFlow is an open-source Rust command-line tool that accelerates
post-calling variant analysis. Its guiding design principle is selective
execution: it parses only the fields a given filter, population-genetics
statistic, or export actually requires, rather than decoding every field of
every VCF record. VariantFlow implements a streaming-computable suite of
population-genetics statistics --- allele frequency, missingness,
heterozygosity, Hardy-Weinberg equilibrium, nucleotide diversity, a
missing-data-aware pi/dxy estimator matching pixy, Tajima's D, the site
frequency spectrum, Fst, and linkage disequilibrium --- alongside selective
VCF/BGZF filtering, `.vfi` index-assisted filtering, and Parquet export for
repeated analytical queries.

The manuscript reports only correctness-matched benchmark evidence from
tracked repository reports, each independently validated against VCFtools
and, where applicable, pixy and scikit-allel. On the 1000 Genomes
3,202-sample high-coverage dataset, VariantFlow accelerated missingness
computation 3.67-4.78x over VCFtools at constant 9 MB memory, and other
supported statistics 1.2-273x, while producing output that is
byte-identical, or numerically identical within machine epsilon, to the
established tool it was validated against. The paper positions VariantFlow
as a measured accelerator and complement to bcftools, HTSlib, GATK,
VCFtools, DuckDB, and scikit-allel, not a universal replacement; Table S5
explicitly delineates the statistics it computes from those that require a
full genotype/haplotype matrix in memory and are better served by
scikit-allel.

We expect VariantFlow to be of practical value to the readership of
Molecular Ecology Resources: population and conservation/ecological
genomics (nucleotide diversity, differentiation, and linkage
disequilibrium on large cohorts), plant and animal breeding programmes
(large-panel screening), large-cohort human-genomics quality control
(missingness and allele-frequency summaries), and bioinformatics pipeline
development (a fast, scriptable filtering and export layer). Its
distinctive contribution is bringing whole-cohort population genomic
summaries from batch to interactive timescales on commodity hardware,
while preserving exact correctness against established tools; a companion
online user guide and statistics reference lower the barrier to adoption
for students and non-specialists.

The software is released under the MIT OR Apache-2.0 license. The
submitted version, source archive, and reproducibility materials are
available at:

- Repository: https://github.com/ehsanestaji/VariantFlow
- Release tag: v1.5.0
- Archive DOI: 10.5281/zenodo.21198171
- Public benchmark table: `docs/public-benchmark-table.md`

No new biological specimens or sequence data were generated for this
study; all benchmarks reuse publicly available, previously deposited 1000
Genomes Project variant call sets under their existing data-access terms.
A Data Accessibility and Benefit-Sharing Statement is included in the
manuscript, beneath the references, as required by journal policy.

AI usage disclosure: AI coding assistants (OpenAI Codex and Anthropic
Claude Code) were used for planning, code-review support, test
scaffolding, benchmark-report organization, and prose drafting support.
The human authors reviewed and remain responsible for the manuscript,
code, benchmarks, licensing, and scientific claims.

All authors have approved this submission and declare no competing
interests. This manuscript is not currently under consideration elsewhere.

Sincerely,

Ehsan Estaji and Jian-Feng Mao
Umeå Plant Science Centre, Umeå University
Correspondence: jianfeng.mao@umu.se
