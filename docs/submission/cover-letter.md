# Bioinformatics Cover Letter Template

Dear Editors,

We are pleased to submit the Application Note manuscript,
"VariantFlow: selective execution for VCF filtering and analytical export," for
consideration in Bioinformatics.

VariantFlow is an open-source Rust command-line tool for evidence-tracked
post-calling variant workflows. It targets a common inefficiency in VCF/BGZF
analysis: many filters, summaries, and exports depend on only a small subset of
site, INFO, or FORMAT fields, yet conventional compatibility-first workflows may
parse substantially more of each record. VariantFlow uses selective native
execution, line-preserving filtering where possible, optional HTSlib
compatibility, `.vfi` index-assisted filtering, VCFtools-style population
summaries, and Parquet export for repeated analytical queries.

The manuscript reports only correctness-matched benchmark rows from tracked
repository reports. Current evidence includes public human FORMAT-rich
filtering, IGSR BGZF filtering, query-aware indexed filtering, VCFtools-style
population-genetics summaries, and DuckDB/Parquet repeated-query workflows. The
paper scopes VariantFlow as a measured accelerator and complement to bcftools,
HTSlib, GATK, and VCFtools under measured conditions.

The software is released under the MIT OR Apache-2.0 license. The submitted
version, source archive, and reproducibility materials are available at:

- Repository: https://github.com/ehsanestaji/VariantFlow
- Release tag: v1.5.0
- Archive DOI: 10.5281/zenodo.21198171
- Public benchmark table: `docs/public-benchmark-table.md`

AI usage disclosure: AI coding assistants (OpenAI Codex and Anthropic Claude
Code) were used for planning, code-review support, test scaffolding,
benchmark-report organization, and prose drafting support. The human authors
reviewed and remain responsible for the manuscript, code, benchmarks,
licensing, and scientific claims.

All authors have approved this submission and declare no competing interests.

Sincerely,

Ehsan Estaji and Jian-Feng Mao
Umeå Plant Science Centre, Umeå University
Correspondence: jianfeng.mao@umu.se
