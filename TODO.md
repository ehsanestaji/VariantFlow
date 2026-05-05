# VariantFlow Release TODO

This list tracks release work that should happen before the project is presented as a broadly installable public tool.

## Bioconda Release

Goal: publish VariantFlow through Bioconda after the source release is tagged.

- Accepted direction: VariantFlow.
- Use Bioconda package name `variantflow` unless the final pre-PR collision check finds a blocker.
- Verify `variantflow` is not already used in Bioconda, conda-forge, crates.io, GitHub, and common bioinformatics tool indexes immediately before opening the recipe PR.
- Create a tagged GitHub source release and record the stable source tarball URL.
- Compute and record the source `sha256`.
- Add a Bioconda recipe under `bioconda-recipes/recipes/variantflow/meta.yaml`.
- Use the Bioconda Rust recipe pattern: `{{ compiler('rust') }}`, `cargo-bundle-licenses`, and `cargo install -v --locked --no-track --root $PREFIX --path .`.
- Include command tests such as `variantflow --version`, a tiny `filter` fixture, and a tiny `convert --to tsv` fixture.
- Include `vcf-fast --version` only if the compatibility alias is shipped in the Bioconda package.
- Keep test commands dependent only on runtime dependencies so Bioconda's clean mulled test can pass.
- Include SPDX license metadata and generated third-party license metadata.
- Document optional htslib behavior carefully. If the Bioconda package builds the default native engine only, say so; if it builds `htslib-static`, test `.bcf`, `--region`, and BGZF output in the recipe or release notes.
- Test locally with `bioconda-utils` and a clean container before opening the PR.
- Open a PR to `bioconda/bioconda-recipes` only after `make verify`, `cargo test --features htslib-static`, release artifacts, and name migration docs are green.

Sources checked on 2026-05-05:

- Bioconda contributor guide: https://bioconda.github.io/contributor/index.html
- Bioconda recipe guidelines: https://bioconda.github.io/contributor/guidelines.html
- Bioconda local testing guide: https://bioconda.github.io/contributor/building-locally.html

## Professional Rename

Goal: move away from an adjective-based name before package distribution. `Fast` is a benchmark claim, not a durable product identity.

Recommended approach:

- Use `VariantFlow` as the professional public project name.
- Use `variantflow` as the primary binary for new documentation and package distribution.
- Keep `vcf-fast` as a compatibility alias for at least one release after the rename.
- Update package metadata, README, benchmark reports, release workflow, Docker image tag, and Bioconda recipe together.
- Add migration notes in `CHANGELOG.md` and `docs/release.md`.

Name criteria:

- Neutral and evidence-friendly, with no performance adjective.
- Connected to variants, selective execution, or analytical workflow.
- Short enough for command-line use.
- Available across Bioconda, conda-forge, crates.io, GitHub, and container registries.
- Not confusingly similar to `bcftools`, `vcftools`, `vcflib`, `vembrane`, `slivar`, `cyvcf2`, or GATK tools.

Rejected alternatives for now:

- `VariantForge`: strong engineering identity, but more branded than necessary.
- `VarStream`: concise and streaming-oriented, but narrower than the workflow roadmap.
- `VarQuery`: good for query/export, too narrow for filter/stats/diff.

Current recommendation: continue with `VariantFlow`, repeat collision checks before public publishing, and keep `vcf-fast` as a compatibility alias while users migrate.

## Release Claim Discipline

- Every README performance sentence must point to a tracked report row.
- Smoke tiers below 10k records are validation only.
- RSS claims require GNU time or another reproducible memory measurement path.
- Slower compatibility paths stay visible as caveats instead of being hidden.
- The phrase "best VCF tool" remains a roadmap ambition until `docs/claim-matrix.md` supports it workflow by workflow.
