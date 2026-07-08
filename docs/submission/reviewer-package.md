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

| Name | Institution | Email/profile | Expertise rationale | Conflict checked |
|---|---|---|---|---|
| TODO_REVIEWER_1 | TODO | TODO | TODO | TODO |
| TODO_REVIEWER_2 | TODO | TODO | TODO | TODO |
| TODO_REVIEWER_3 | TODO | TODO | TODO | TODO |

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
