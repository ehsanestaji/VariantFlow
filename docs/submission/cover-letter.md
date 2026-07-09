# Molecular Ecology Resources Cover Letter

Dear Editors,

We are pleased to submit the manuscript, "VariantFlow: a selective-execution
engine for efficient population genomic computation on large variant
datasets," for consideration in Molecular Ecology Resources, as a Computer
Programs submission.

VariantFlow is an open-source Rust command-line tool that accelerates
post-calling variant analysis --- that is, the analysis of variant call sets
after the variants have already been identified. To see the problem it solves,
picture a task you have lived: you have a chr22 cohort with thousands of
samples, and your question is narrow --- "keep sites where QUAL > 30," or "how
much missing data per site?" A VCF file carries the full account of every
sample at every site --- every genotype, depth, and quality value. That
generality is exactly why VCF is the shared currency of variant data; it is a
feature, not a defect. But when your question is narrow, a conventional tool
still copies out the entire record, every genotype and depth value for every
one of thousands of samples, before it can answer --- and on a large cohort
that copying of values you never asked for, not the biology, is what you wait
for.

VariantFlow's guiding design principle, selective execution, is the cure for
that wait: it reads only the labelled sections of a record (the "fields") that
a given filter, population-genetics statistic, or export actually requires ---
it opens only the sections your question names --- rather than decoding
(reading out) every field of every VCF record. The manuscript carries this one
book analogy consistently from start to finish:

> Every variant record is like a still-closed book with many labelled sections
> --- QUAL, each INFO tag, and, for every one of thousands of samples, a FORMAT
> entry such as GT:DP:GQ:AD; a conventional tool copies out every section of
> every book before it can answer, whereas VariantFlow reads only the sections
> your question names and leaves the rest closed. The number you get is exactly
> the same as if it had copied out every page --- it just touched a fraction of
> the ink --- and because it never rewrote the books it skimmed past, those
> records come back through untouched.

This plain-language on-ramp is added above, not in place of, the
field-selective, streaming, byte-scanning detail reviewers need to trust the
method; VariantFlow reads fewer fields, never fewer records, and never
approximates or samples, so the answer is the same, verified byte-identical ---
the same answer down to the last character.

On that foundation, VariantFlow implements a suite of population-genetics
statistics that can be computed in a single read-through of the file, without
ever loading the whole file into memory --- allele frequency, missingness,
heterozygosity, Hardy-Weinberg equilibrium, nucleotide diversity, a
missing-data-aware pi/dxy estimator matching pixy, Tajima's D, the site
frequency spectrum, Fst, and linkage disequilibrium. Alongside these
biological summaries it offers three data-handling features: selective
VCF/BGZF filtering (BGZF being the compressed VCF format), `.vfi`
index-assisted filtering (an optional index that skips straight to the matching
sites), and Parquet export (a columnar table format that makes repeated
analytical queries fast).

**Writing for the Molecular Ecology Resources readership.** We have written
and revised this manuscript for the broad readership of the journal, many of
whom are biologists without a computational-biology or computer-science
background, as well as for editors and reviewers reading from that vantage.
Above the technical detail that reviewers need to trust the method, the paper
adds a plain-language layer that motivates the problem, explains the core idea,
and states the practical value, in line with what the Computer Programs
category expects: a clear statement of need, the design rationale, a plain
description of function, usage, and output, and a performance evaluation
against existing software. Throughout, each technical passage leads with the
plain book image and then gives the precise term, so the plain layer is always
an added on-ramp above the field-selective, streaming, byte-scanning detail,
never a replacement for it.

The manuscript reports only correctness-matched benchmark evidence from
tracked repository reports, each independently validated against VCFtools
and, where applicable, pixy and scikit-allel. On the 1000 Genomes
3,202-sample high-coverage dataset, VariantFlow accelerated missingness
computation 3.67-4.78x over VCFtools at constant 9 MB memory --- a memory
footprint that stays flat no matter how many samples are in the cohort, so
the work fits comfortably on a laptop --- and other supported statistics
1.2-273x, while producing output that is byte-identical (the same answer
down to the last character), or numerically identical within machine epsilon
(identical to the finest precision a computer can represent), to the
established tool it was validated against. The paper positions VariantFlow
as a measured accelerator and complement to bcftools, HTSlib, GATK,
VCFtools, DuckDB, and scikit-allel, not a universal replacement; Table S5
explicitly delineates the statistics it computes from those that require the
entire table of every sample's genotypes to be held in memory at once (a full
genotype/haplotype matrix in memory) and are better served by scikit-allel.

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
