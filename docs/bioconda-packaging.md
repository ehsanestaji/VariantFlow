# VariantFlow Bioconda Packaging

This document tracks the local Bioconda recipe scaffold for `variantflow`. It is intentionally not presented as a submitted recipe yet: Bioconda needs a stable tagged source URL, a real source hash, and maintainer metadata before the recipe is PR-ready.

## Current Status

- Recipe scaffold: `packaging/bioconda/variantflow/meta.yaml`
- Build script: `packaging/bioconda/variantflow/build.sh`
- Install smoke test: `packaging/bioconda/variantflow/run_test.sh`
- Local scaffold check: `make bioconda-recipe-check`

## Current Blockers

- Create a tagged GitHub source release for the first packaged version.
- Replace `OWNER` and `TODO_GITHUB_HANDLE` in the recipe.
- Replace `TODO_RELEASE_SHA256` with the release tarball `sha256`.
- Run `bioconda-utils` lint/build in a clean environment.
- Confirm whether the Bioconda package should build only the default native engine or include `htslib-static`.

The project license is now recorded as `MIT OR Apache-2.0` with root license
notice `LICENSE` and full texts in `LICENSE-MIT` and `LICENSE-APACHE`.

## Exact-name check on 2026-05-06

The following checks returned HTTP `404`, meaning no exact package was found through the queried package APIs at the time of the check:

- `bioconda/variantflow: 404`
- `bioconda/vcf-fast: 404`
- `conda-forge/variantflow: 404`
- `conda-forge/vcf-fast: 404`
- `crates/variantflow: 404`
- `crates/vcf-fast: 404`

Repeat these checks immediately before opening a Bioconda PR.

## Recipe Shape

The recipe follows current Bioconda Rust guidance:

- use `{{ compiler('rust') }}`;
- generate `THIRDPARTY.yml` with `cargo-bundle-licenses`;
- install with `cargo install -v --locked --no-track --root "$PREFIX" --path .`;
- test both `variantflow --version` and the `vcf-fast` compatibility alias;
- run tiny `filter` and `convert --to tsv` smoke tests in `run_test.sh`.

## Release PR Steps

1. Confirm the recorded project license remains `MIT OR Apache-2.0`.
2. Tag the release and verify the GitHub source tarball URL.
3. Compute the tarball hash:

   ```bash
   curl -L -o variantflow-v1.5.0.tar.gz https://github.com/OWNER/VariantFlow/archive/v1.5.0.tar.gz
   shasum -a 256 variantflow-v1.5.0.tar.gz
   ```

4. Replace the recipe placeholders.
5. Copy `packaging/bioconda/variantflow` into `bioconda-recipes/recipes/variantflow`.
6. Run local Bioconda lint/build/mulled tests.
7. Open the Bioconda PR only after upstream `make verify`, `cargo test --features htslib-static`, and release artifact checks pass.
