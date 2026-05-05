# VariantFlow Rename Plan

VariantFlow is the accepted professional name for the project formerly called VCF-Fast. The rename removes the adjective-based `fast` claim from the product name while keeping the evidence-first performance story in benchmark reports.

## Public Names

- Project name: `VariantFlow`
- Bioconda package: `variantflow`
- Primary CLI binary: `variantflow`
- Compatibility CLI alias: `vcf-fast`
- Rust package during migration: `vcf-fast`

The Rust package name stays unchanged for this migration slice so the crate module path and historical benchmark scripts remain stable. A later package rename can happen after the external distribution path is ready.

## CLI Migration

`variantflow` is the primary binary for new documentation and package-manager publishing. `vcf-fast` remains a one-release compatibility alias so existing scripts continue to work while users migrate.

Examples:

```bash
variantflow --version
variantflow filter input.vcf.gz --where "QUAL > 30" -o output.vcf.gz
vcf-fast --version
```

The two binaries call the same implementation. The only intended difference is help/version branding.

## Bioconda Plan

The Bioconda recipe should use package name `variantflow` after the collision check is complete. The local scaffold lives at `packaging/bioconda/variantflow`. The recipe should install and test the primary `variantflow` command. If the compatibility alias is shipped in the same release, the recipe should also test `vcf-fast --version`.

The first Bioconda recipe should use a tagged GitHub source tarball, a recorded `sha256`, Rust compiler macro support, `cargo-bundle-licenses`, and deterministic Cargo install flags.

## Collision Check

Before publishing the package, repeat the exact-name collision check across:

- Bioconda
- conda-forge
- crates.io
- GitHub repositories and organizations
- Docker/container registries
- common bioinformatics tool indexes and publication search

The 2026-05-05 check found no exact `variantflow` package in Bioconda or conda-forge, but this must be repeated immediately before opening the recipe PR.

## Documentation Policy

New user-facing docs should lead with `VariantFlow, formerly VCF-Fast`. Historical benchmark reports can keep their original `VCF-Fast` names because they are evidence records from previous milestones.

README claims must stay evidence-bound. The rename is a product identity change, not a new performance claim.
