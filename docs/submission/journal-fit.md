# VariantFlow Journal Fit

## Confirmed target: Molecular Ecology Resources, Computer Programs

Molecular Ecology Resources (MER) is the confirmed submission target, in the
Computer Programs article type. VariantFlow is a software/algorithm paper
with a clear command-line tool, tracked public benchmarks, and reproducible
correctness checks against established population-genetics tools (VCFtools,
pixy, scikit-allel) — matching MER's Computer Programs expectations: a clear
statement of the need for the program and its design rationale, a summary of
functions/usage/output, and a performance evaluation against existing
software on real data.

The VariantFlow story is framed as:

- selective execution for common post-calling variant workflows;
- correctness-matched evidence against `bcftools`, VCFtools-style population
  summaries, pixy's missing-data-aware diversity/divergence estimators, and
  scikit-allel's site-frequency spectrum;
- a workflow-specific accelerator for the population and conservation
  genomics, breeding, and large-cohort human-genomics communities, not a
  universal replacement for every VCF, BCF, GATK, VCFtools, or scikit-allel
  use case.

MER requires: an unstructured abstract ending with the audience/impact of the
work, author-year (APA-style) references, and a Data Accessibility and
Benefit-Sharing Statement beneath the references. All three are implemented
in `paper/bioinformatics-application-note/main.tex`.

## Backup route: JOSS

JOSS remains the software-native backup path. It is lower risk for pure
research software review, but it is not the same population-genomics-journal
signal as MER. The existing `paper/paper.md` scaffold is a separate,
JOSS-oriented track and is not required to mirror the MER manuscript.

## Stretch routes: Bioinformatics, NAR Genomics and Bioinformatics, GigaScience

If MER review does not proceed, Bioinformatics (Application Note),
NAR Genomics and Bioinformatics, or GigaScience remain viable alternative
targets, each requiring their own formatting and citation-style pass.

## Required submission evidence

- Public repository with an OSI-approved license.
- Stable release tag and archived software DOI.
- Installation and tutorial documentation.
- Test data and reproducibility commands.
- Performance claims tied to tracked reports only.
- AI usage disclosure in the manuscript and cover letter.
- Data Accessibility and Benefit-Sharing Statement in the manuscript.
- Author, funding, conflict, and reviewer metadata approved before submission.
