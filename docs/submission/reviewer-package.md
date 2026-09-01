# VariantFlow Reviewer Package

This file is the reviewer-party source of truth for independent submission
review. It is not a list of guaranteed journal reviewers; it is a conflict-aware
package for author discussion and journal submission forms.

## Suggested reviewer criteria

Suggested reviewers should have expertise in at least one of:

- VCF/BCF, HTSlib, `bcftools`, or variant-processing infrastructure;
- population and conservation/ecological genomics, especially VCFtools- or
  pixy-style diversity, differentiation, and missing-data-aware statistics;
- high-performance bioinformatics software, Rust/C/C++, compression, or
  columnar analytics;
- reproducible benchmarking for genomics tools.

Suggested reviewers must not have financial, supervisory, recent collaboration,
family, or institutional conflicts with any author.

## Suggested reviewers

The corresponding author nominated four reviewers on 2026-08-31. Their names,
institutions, contact addresses, expertise rationales, and per-person conflict
assessments are held privately by the authors and entered directly into the
journal's submission form.

They are deliberately **not** recorded in this public repository. Suggested
reviewers are confidential to the authors and the handling editor, and publishing
the nominees' institutional email addresses would expose third parties' contact
details without their consent and could compromise the integrity of the review.

Summary for the record, without identifying details: four nominees across four
countries, spanning forest-tree population genomics, plant adaptive evolution, and
conservation genomics. None shares an institution with either author, and an
automated Crossref search found no co-authorship with either author. All four are
users of population-genetic statistics rather than developers of variant-processing
infrastructure; if the handling editor prefers a tool-development perspective, the
authors of bcftools/HTSlib, pixy, scikit-allel or vcfexpress would be unconflicted
alternatives.

## Opposed reviewers

| Name | Institution | Reason | Approved for submission form |
|---|---|---|---|
| None | — | — | — |

## Independent reviewer gate

The reviewer party should check:

- manuscript claims against `docs/public-benchmark-table.md`;
- source reports for exact commands, correctness, caveats, and versions;
- release/archive metadata;
- installation and reproducibility documentation;
- AI usage disclosure;
- conflict and authorship metadata;
- no broad "best VCF tool" or universal replacement claim.

The Reviewer Gate cannot pass while reviewer suggestions, opposed-reviewer
status, or conflict checks contain TODO placeholders.
