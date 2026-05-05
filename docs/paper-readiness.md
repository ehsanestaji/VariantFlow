# VariantFlow JOSS Submission Readiness

This document tracks the synchronized paper, release, and Bioconda launch. The
JOSS manuscript lives in `paper/paper.md`; references live in
`paper/paper.bib`.

## Current blockers

- Choose and add an OSI-approved license at the repository root.
- Confirm the final author list, affiliations, funding statement, conflicts of
  interest, and corresponding author.
- Confirm public repository history is sufficient for JOSS screening.
- Create a tagged release for the software version described in the paper.
- Archive the release through Zenodo or another accepted archive and record a
  Zenodo DOI if submitting to JOSS after review acceptance.
- Replace Bioconda recipe placeholders after the tagged release exists:
  source URL, `sha256`, SPDX license, owner, and maintainer handle.

## Submission checklist

- `make verify` passes.
- `cargo test --features htslib-static` passes.
- `cargo clippy --features htslib-static --all-targets -- -D warnings` passes.
- `make bioconda-recipe-check` passes while still honestly reporting release
  placeholders.
- `make paper-check` passes and reports a JOSS-length manuscript.
- Paper claims match `docs/public-benchmark-table.md` and source reports.
- The manuscript avoids broad "best tool" language and frames VariantFlow as an
  evidence-tracked complement to `bcftools`, HTSlib, VCFtools, and GATK.

## Benchmark rows used by the manuscript

- Public BGZF QUAL filter:
  `benchmark/reports/v14-public-parallel-scale-benchmark.md`; correctness
  matched core records and measured `13.44x to 13.47x` faster than
  `bcftools filter`.
- Stress FORMAT aggregate filter:
  `benchmark/reports/v14-public-parallel-scale-benchmark.md`; correctness
  matched core records and measured `1.77x to 2.01x` faster than default native
  execution and `4.33x to 5.27x` faster than `bcftools filter`.
- Columnar repeated-query workflow:
  `benchmark/reports/v12-public-parallel-workflow-benchmark.md`; DuckDB query
  results matched normalized `bcftools` baselines and measured `3.18x to
  25.67x` amortized speedups.

## Author metadata needed

- Full author names.
- ORCID identifiers, if available.
- Institutional affiliations.
- Funding and grant numbers, if any.
- Conflict-of-interest statement.
- Contributor acknowledgements.

## Bioconda Launch Coordination

The Bioconda recipe scaffold is already tracked under
`packaging/bioconda/variantflow`, but the recipe must not be submitted until the
license, tagged source archive, `sha256`, and maintainer metadata are final.
The paper can mention planned Bioconda distribution only after the recipe PR is
opened or merged.

## Release notes for paper submission

JOSS submission should use a public tagged release and a repository state that
reviewers can clone, build, test, and inspect. If public-history screening is
not yet satisfied, keep this paper as a ready draft and submit after sufficient
open development history exists.

