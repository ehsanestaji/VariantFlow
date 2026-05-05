# VCF-Fast Release TODO

This list tracks release work that should happen before the project is presented as a broadly installable public tool.

## Bioconda Release

Goal: publish the tool through Bioconda after the professional name is chosen and the source release is tagged.

- Choose the final package and binary name before opening the Bioconda recipe PR.
- Verify the final name is not already used in Bioconda, conda-forge, crates.io, GitHub, and common bioinformatics tool indexes.
- Create a tagged GitHub source release and record the stable source tarball URL.
- Compute and record the source `sha256`.
- Add a Bioconda recipe under `bioconda-recipes/recipes/<final-name>/meta.yaml`.
- Use the Bioconda Rust recipe pattern: `{{ compiler('rust') }}`, `cargo-bundle-licenses`, and `cargo install -v --locked --no-track --root $PREFIX --path .`.
- Include command tests such as `<final-binary> --version`, a tiny `filter` fixture, and a tiny `convert --to tsv` fixture.
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

- Do not rename the binary abruptly in the current release train.
- Pick a professional, descriptive name that can outlive individual speed claims.
- Keep `vcf-fast` as a compatibility alias for at least one release after the rename.
- Update package metadata, README, benchmark reports, release workflow, Docker image tag, and Bioconda recipe together.
- Add migration notes in `CHANGELOG.md` and `docs/release.md`.

Name criteria:

- Neutral and evidence-friendly, with no performance adjective.
- Connected to variants, selective execution, or analytical workflow.
- Short enough for command-line use.
- Available across Bioconda, conda-forge, crates.io, GitHub, and container registries.
- Not confusingly similar to `bcftools`, `vcftools`, `vcflib`, `vembrane`, `slivar`, `cyvcf2`, or GATK tools.

Candidate direction:

- `VariantFlow`: clear workflow-oriented name, good fit for streaming filter plus columnar export.
- `VariantForge`: strong engineering identity, but check for collisions and possible over-branding.
- `VarStream`: concise and streaming-oriented, but check for generic package collisions.

Current recommendation: evaluate `VariantFlow` first, then collision-check it before changing code. If it is unavailable, prefer another neutral `Variant*` name over another speed adjective.

## Release Claim Discipline

- Every README performance sentence must point to a tracked report row.
- Smoke tiers below 10k records are validation only.
- RSS claims require GNU time or another reproducible memory measurement path.
- Slower compatibility paths stay visible as caveats instead of being hidden.
- The phrase "best VCF tool" remains a roadmap ambition until `docs/claim-matrix.md` supports it workflow by workflow.
