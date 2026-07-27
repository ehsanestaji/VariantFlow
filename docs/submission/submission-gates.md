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

Updated 2026-07-27 after the full certification audit.

- Author Gate: **PASS**. Release v1.5.0 is tagged (`d3b9f6a`) and published on
  GitHub (2026-07-04); the Zenodo version DOI for v1.5.0 is
  10.5281/zenodo.21198172 (concept DOI 10.5281/zenodo.21198171), both now cited
  correctly in the manuscript; both author ORCIDs and CRediT roles are present in
  `main.tex`.
- Claim audit: **PASS**. Every quantitative claim traces to a git-tracked report
  (v20, v24, v26, v29, v30). Table 1 memory figures corrected against v20/v24,
  the FORMAT-filter record tiers disclosed, the multi-host benchmark provenance
  stated in Table S2, and the superseded v25 population-genetics values marked
  SUPERSEDED wherever they were still published. No broad "best VCF tool" claim
  is present.
- Reviewer Gate: **BLOCKED** — suggested reviewers are still placeholders in
  `reviewer-package.md` and require author conflict-of-interest judgement. This
  is the only remaining blocker.
- Submission: BLOCKED solely on the Reviewer Gate.
