# Competitor Notes

VCF-Fast should become the best option by being precise about where it wins, where it complements mature tools, and where compatibility matters more than raw speed.

## bcftools

`bcftools` is the primary correctness and performance baseline. Its filtering engine is broad and mature: it unpacks BCF/VCF records through HTSlib, evaluates compiled expression stacks, supports typed INFO/FORMAT getters, handles sample masks, and covers much richer expression semantics than VCF-Fast v0.5.

Design lesson for VCF-Fast:

- Keep the Rust-native selective path for cases where avoiding unused parsing is the advantage.
- Compare supported predicates against `bcftools filter` before claiming correctness.
- Do not chase full expression parity blindly; add parity where users need it and measure the cost.

Reference: [samtools/bcftools filter.c](https://github.com/samtools/bcftools/blob/develop/filter.c)

## HTSlib

HTSlib is the ecosystem compatibility baseline for BCF, BGZF, tabix/CSI/TBI indexes, and region reads. v0.5 uses optional `rust-htslib` interop for these compatibility surfaces while keeping the default build dependency-light.

Design lesson for VCF-Fast:

- Use HTSlib where format compatibility is the product requirement.
- Keep Rust-native line-preserving filtering as the default for plain `.vcf` and `.vcf.gz` streams.
- Make backend selection explicit and testable so users know when original record preservation no longer applies.

Reference: [samtools/htslib](https://github.com/samtools/htslib)

## VCFtools

VCFtools remains useful historical context for VCF filtering and statistics workflows, but it is not the main performance target for VCF-Fast. Its broad command surface is a reminder that researchers value practical operations more than engine purity.

Design lesson for VCF-Fast:

- Keep CLI workflows simple and composable.
- Prioritize correctness for common post-calling operations before expanding into niche options.
- Compare selected stats/filter outputs later where VCFtools remains commonly used.

Reference: [vcftools/vcftools](https://github.com/vcftools/vcftools)

## Roadmap Implication

The path to “best VCF tool” is not one giant speed claim. It is a claim matrix:

- fastest for supported selective streaming filters where evidence proves it.
- compatible with HTSlib-backed BCF/BGZF/indexed workflows where compatibility is required.
- increasingly expressive where users need FORMAT/sample semantics.
- export-friendly for TSV now and Arrow/Parquet later when repeated analytical workloads matter.
