#!/usr/bin/env python3
"""Lightweight checks for the VariantFlow Bioconda recipe scaffold.

This is not a replacement for bioconda-utils lint/build. It keeps the in-repo
template honest until a tagged source release and sha256 are available.
"""

from __future__ import annotations

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RECIPE = ROOT / "packaging" / "bioconda" / "variantflow"


def read(path: Path) -> str:
    if not path.exists():
        raise SystemExit(f"missing required packaging file: {path.relative_to(ROOT)}")
    return path.read_text(encoding="utf-8")


def require_tokens(label: str, text: str, tokens: tuple[str, ...]) -> None:
    missing = [token for token in tokens if token not in text]
    if missing:
        formatted = ", ".join(repr(token) for token in missing)
        raise SystemExit(f"{label} is missing required token(s): {formatted}")


def main() -> int:
    meta = read(RECIPE / "meta.yaml")
    build = read(RECIPE / "build.sh")
    run_test = read(RECIPE / "run_test.sh")
    docs = read(ROOT / "docs" / "bioconda-packaging.md")

    require_tokens(
        "meta.yaml",
        meta,
        (
            '{% set name = "variantflow" %}',
            '{% set version = "1.5.0" %}',
            "https://github.com/ehsanestaji/VCF-FAST/archive/v{{ version }}.tar.gz",
            "sha256: TODO_RELEASE_SHA256",
            "{{ compiler('rust') }}",
            "cargo-bundle-licenses",
            "license: MIT OR Apache-2.0",
            "license_file:",
            "LICENSE-MIT",
            "LICENSE-APACHE",
            "THIRDPARTY.yml",
            "variantflow --version",
            "vcf-fast --version",
            "recipe-maintainers:",
            "- ehsanestaji",
        ),
    )
    require_tokens(
        "build.sh",
        build,
        (
            "cargo-bundle-licenses --format yaml --output THIRDPARTY.yml",
            'cargo install -v --locked --no-track --root "$PREFIX" --path .',
        ),
    )
    require_tokens(
        "run_test.sh",
        run_test,
        (
            "variantflow --version",
            "vcf-fast --version",
            "variantflow filter",
            "variantflow convert",
        ),
    )
    require_tokens(
        "docs/bioconda-packaging.md",
        docs,
        (
            "Current Blockers",
            "tagged GitHub source release",
            "sha256",
            "MIT OR Apache-2.0",
            "ehsanestaji",
            "bioconda/variantflow: 404",
            "crates/variantflow: 404",
        ),
    )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
