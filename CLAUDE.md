# VariantFlow Claude Code Handoff

## Project State

VariantFlow, formerly VCF-Fast, is a Rust-native selective execution engine for
post-calling variant workflows. The project is intentionally evidence-bound:
only claim speed where tracked reports show correctness-matched measurements.

Current strongest evidence areas:

- native selective VCF/BGZF filtering;
- human FORMAT-rich cohort predicates;
- query-aware `.vfi` indexed high-skip filters;
- VCFtools-style population summaries on supported diploid biallelic rows;
- Parquet/DuckDB export-once, query-many workflows.

Do not add broad "best VCF tool" or universal replacement claims. Use
`docs/public-benchmark-table.md`, `docs/claim-matrix.md`, and source reports
under `benchmark/reports/` as the claim source of truth.

## Current Paper/Submission Status

The Bioinformatics Application Note is the primary target. The submission
mechanism is in place under `docs/submission/` and `paper/submission.yml`.

Strict submission is intentionally blocked until human/release fields are
filled:

- contact email;
- ORCID or explicit none;
- funding statement;
- conflict statement;
- suggested/opposed reviewers;
- release tag;
- source archive URL;
- archive DOI.

Use structural validation while blocked:

```bash
VCF_FAST_SUBMISSION_ALLOW_BLOCKED=1 make submission-check
```

## Core Verification Commands

Use these before claiming completion:

```bash
make verify
cargo test --features htslib-static
cargo clippy --features htslib-static --all-targets -- -D warnings
make paper-check
VCF_FAST_SUBMISSION_ALLOW_BLOCKED=1 make submission-check
```

Compile the Bioinformatics PDF with:

```bash
cd paper/bioinformatics-application-note
make
```

## Development Rules

- Keep `variantflow` as the primary binary; `vcf-fast` is a compatibility alias.
- Keep htslib optional and compatibility-focused.
- Preserve line/order correctness for native filtering paths.
- Update README/claim docs only from correctness-matched reports.
- Keep large data, benchmark outputs, logs, and Claude worktrees out of Git.
- Prefer small, evidence-backed changes over broad unverified claims.
