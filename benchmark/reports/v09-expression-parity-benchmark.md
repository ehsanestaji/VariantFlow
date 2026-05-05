# VCF-Fast v0.9 Expression Parity Benchmark

## Status

This report tracks correctness and performance for v0.9 expression parity cases. Rows are added only after the command output matches the stated `bcftools` baseline. No runtime win is claimed for v0.9 expression parity until measured public benchmark rows replace the pending fixture rows below. The native scope is arbitrary `INFO/<KEY>`, selected-sample `FORMAT/<KEY>` with `--sample`, and `ANY(FORMAT/<KEY>)` / `ALL(FORMAT/<KEY>)` sample aggregate predicates.

## Native Expression Cases

| Case | Dataset | VCF-Fast command | Competitor command | Correctness result | Runtime | Speedup | Caveat |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Arbitrary INFO numeric/string | `tests/data/expression_parity.vcf` | `vcf-fast filter tests/data/expression_parity.vcf --where 'INFO/MQ >= 50 && INFO/CSQ == "synonymous_variant"' -o out.vcf` | `bcftools filter -i 'INFO/MQ >= 50 && INFO/CSQ == "synonymous_variant"' tests/data/expression_parity.vcf -o bcftools.vcf` | Fixture expectation covered by integration tests; public benchmark measurement pending | `n/a` | `n/a` | Small fixture proves semantics, not performance |
| Selected arbitrary FORMAT | `tests/data/expression_parity.vcf` | `vcf-fast filter tests/data/expression_parity.vcf --sample HG002 --where 'FORMAT/AD > 8 && FORMAT/FT == "PASS"' -o out.vcf` | `bcftools view -s HG002 tests/data/expression_parity.vcf | bcftools filter -i 'FMT/AD[*] > 8 && FMT/FT == "PASS"' -o bcftools.vcf` | Fixture expectation covered by integration tests; bcftools vector syntax requires explicit normalization | `n/a` | `n/a` | bcftools FORMAT vector semantics differ for multi-value fields |
| ANY sample aggregate | `tests/data/expression_parity.vcf` | `vcf-fast filter tests/data/expression_parity.vcf --where 'ANY(FORMAT/AD > 15)' -o out.vcf` | `bcftools filter -i 'N_PASS(FMT/AD[*] > 15) > 0' tests/data/expression_parity.vcf -o bcftools.vcf` | Fixture expectation covered by integration tests; normalized bcftools comparison pending | `n/a` | `n/a` | Aggregate semantics are intentionally documented before public performance claims |

## Required Report Fields

- dataset source
- dataset size
- record count
- exact VCF-Fast command
- exact competitor command
- competitor version
- correctness result
- runtime mean and standard deviation
- speedup
- variants per second
- peak RSS
- caveat
