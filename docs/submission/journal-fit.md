# VariantFlow Journal Fit

## Primary target: Bioinformatics Application Note

Bioinformatics Application Note is the first target because VariantFlow is a
software and algorithm-implementation paper with a clear command-line tool,
tracked public benchmarks, and reproducible correctness checks. The journal
explicitly accepts short software descriptions, requires software availability,
expects test data and installation documentation, and asks authors to archive
the submitted software version.

The VariantFlow story should be framed as:

- selective execution for common post-calling variant workflows;
- correctness-matched evidence against `bcftools`, HTSlib-backed workflows, and
  VCFtools-style population summaries;
- a workflow-specific accelerator, not a universal replacement for every VCF,
  BCF, GATK, or VCFtools use case.

## Backup route: JOSS

JOSS remains the software-native backup path. It is lower risk for pure
research software review, but it is not the same bioinformatics-journal signal.
The existing `paper/paper.md` scaffold should stay synchronized with the
Bioinformatics manuscript, but Bioinformatics is the first submission target.

## Stretch routes: NAR Genomics and Bioinformatics and GigaScience

NAR Genomics and Bioinformatics may become suitable if VariantFlow grows into a
broader genomics-methods story with larger biological use cases. GigaScience may
fit if the project packages a larger open-science benchmark/data/reproducibility
object with persistent identifiers.

Neither stretch route should be submitted before the release archive, benchmark
data policy, and claim matrix are complete.

## Required submission evidence

- Public repository with an OSI-approved license.
- Stable release tag and archived software DOI.
- Installation and tutorial documentation.
- Test data and reproducibility commands.
- Performance claims tied to tracked reports only.
- AI usage disclosure in the manuscript and cover letter.
- Author, funding, conflict, and reviewer metadata approved before submission.
