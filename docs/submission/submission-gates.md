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
- Reviewer Gate: **PASS**. Four conflict-checked reviewers were nominated by the
  corresponding author on 2026-08-31. Their identities are held privately by the
  authors and entered directly into the submission form; they are deliberately not
  recorded in this public repository. None shares an institution with the authors
  and no co-authorship with either author was found.
- Co-author approval: **PASS**. The corresponding author reviewed the full
  package on 2026-08-31, including the corrected benchmark numbers, and approved
  submission. The option to re-run the FORMAT-filter benchmark at
  whole-chromosome scale was offered and not taken, so the disclosed
  1,000–50,000-record scope stands as written.
- Submission: **CLEARED**. One housekeeping item remains outside the manuscript:
  the published Zenodo record declares the MIT licence alone while the repository
  is MIT OR Apache-2.0. `.zenodo.json` is corrected for future depositions; the
  existing record needs the same edit in the Zenodo interface.
