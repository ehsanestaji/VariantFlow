# VariantFlow Submission Gates

VariantFlow uses a two-party paper mechanism. The author party prepares the
submission package. The reviewer party independently checks claims,
reproducibility, and submission readiness.

## Author Gate

- [ ] Final author list and author order approved.
- [ ] All author affiliations, ORCID values, and CRediT roles recorded.
- [ ] Corresponding author email approved.
- [ ] Funding statement finalized.
- [ ] Conflict-of-interest statement finalized.
- [ ] AI usage disclosure included in manuscript and cover letter.
- [ ] Release tag recorded.
- [ ] Source archive URL recorded.
- [ ] Software archive DOI recorded.
- [ ] Data Accessibility and Benefit-Sharing Statement matches release/archive state.
- [x] Manuscript PDF compiles.
- [x] Manuscript is within Molecular Ecology Resources' 8,000-word Computer
      Programs length target.
- [ ] Figures and tables approved by the author party.

## Reviewer Gate

- [ ] Every performance claim maps to a tracked benchmark report or
      `docs/public-benchmark-table.md`.
- [ ] Correctness wording is scoped to the checked workflow.
- [ ] Caveats are visible for slower, bounded, or unsupported paths.
- [ ] No broad "best VCF tool" claim is present.
- [ ] Reviewer suggestions are conflict checked.
- [ ] Opposed reviewers are recorded or explicitly marked as none.
- [ ] Reproducibility commands are present for core evidence rows.
- [ ] Installation/test documentation is adequate for reviewers.
- [ ] `make submission-check` passes in strict mode.

## Current status

- Author Gate: BLOCKED by release/archive DOI and human metadata placeholders.
- Reviewer Gate: BLOCKED by reviewer metadata placeholders and final claim audit.
- Submission: BLOCKED until both gates pass.
