# VCF-Fast v1.4 Public Parallel Scale Benchmark

This report separates the public I/O-bound BGZF path from CPU-heavy predicate parallelism. It compares single-thread BGZF fallback, default auto BGZF, auto BGZF plus predicate parallelism, explicit BGZF threads, and `bcftools` only where correctness matches.

## Run Configuration

- Generated: 2026-05-05T21:34:52Z
- Public region: `chr22:1-20000000`
- Public tiers: `100`
- Stress tiers: `100`
- Auto BGZF policy: unset `VCF_FAST_NATIVE_BGZF_THREADS` resolves to auto-capped native BGZF workers
- Explicit BGZF input threads: `4`
- Native filter threads: `4`
- Native filter batch records: `4096`
- Repeated runs: `1`
- Warmup runs: `0`
- hyperfine: hyperfine 1.20.0
- bcftools: bcftools 1.23.1

## Public BGZF QUAL Filter Scale

| case | dataset source | dataset size bytes | record count | exact single-thread command | exact auto BGZF command | exact auto+predicate-parallel command | exact explicit BGZF command | exact competitor command | correctness result | runtime mean/stddev | speedup | variants/sec | peak RSS | caveat | claim decision |
| --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Public-heavy IGSR QUAL filter | public-heavy bounded IGSR chr22 BGZF | 41968 | 100 | `VCF_FAST_NATIVE_BGZF_THREADS=1 env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter tests/output/benchmark-results/v14-public-parallel-scale/data/public-heavy-100.vcf.gz --where 'QUAL > 30' -o tests/output/benchmark-results/v14-public-parallel-scale/public-single-bgzf-100.vcf` | `env -u VCF_FAST_NATIVE_BGZF_THREADS -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter tests/output/benchmark-results/v14-public-parallel-scale/data/public-heavy-100.vcf.gz --where 'QUAL > 30' -o tests/output/benchmark-results/v14-public-parallel-scale/public-auto-bgzf-100.vcf` | `env -u VCF_FAST_NATIVE_BGZF_THREADS VCF_FAST_NATIVE_FILTER_THREADS=4 VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=4096 ./target/release/vcf-fast filter tests/output/benchmark-results/v14-public-parallel-scale/data/public-heavy-100.vcf.gz --where 'QUAL > 30' -o tests/output/benchmark-results/v14-public-parallel-scale/public-auto-bgzf-parallel-100.vcf` | `VCF_FAST_NATIVE_BGZF_THREADS=4 env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter tests/output/benchmark-results/v14-public-parallel-scale/data/public-heavy-100.vcf.gz --where 'QUAL > 30' -o tests/output/benchmark-results/v14-public-parallel-scale/public-explicit-bgzf-100.vcf` | `bcftools filter -i 'QUAL>30' tests/output/benchmark-results/v14-public-parallel-scale/data/public-heavy-100.vcf.gz -o tests/output/benchmark-results/v14-public-parallel-scale/public-bcftools-100.vcf` | single-thread, auto BGZF, auto+predicate-parallel, and explicit BGZF native outputs match byte-for-byte; auto core records match bcftools filter | single 0.009407s +/- 0.000000s; auto 0.006460s +/- 0.000000s; auto+parallel 0.006677s +/- 0.000000s; explicit 0.006457s +/- 0.000000s; bcftools 0.009917s +/- 0.000000s | auto/single 1.46x; auto+parallel/single 1.41x; explicit/single 1.46x; auto/bcftools 1.54x | single 10630 / auto 15480 / auto+parallel 14977 / explicit 15487 / bcftools 10084 | single n/a / auto n/a / auto+parallel n/a / explicit n/a / bcftools n/a KB | bounded chr22 region; requested tier may exceed available records in chr22:1-20000000 | smoke validation only; no speed claim from sub-10k tier |

## Stress FORMAT Aggregate Scale

| case | dataset source | dataset size bytes | record count | exact default command | exact parallel command | exact competitor command | correctness result | runtime mean/stddev | speedup | variants/sec | peak RSS | caveat | claim decision |
| --- | --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Stress ANY FORMAT/AD filter | deterministic stress VCF | 81484 | 100 | `env -u VCF_FAST_NATIVE_FILTER_THREADS -u VCF_FAST_NATIVE_FILTER_BATCH_RECORDS ./target/release/vcf-fast filter tests/output/benchmark-results/v14-public-parallel-scale/data/stress-100.vcf --where 'ANY(FORMAT/AD > 80)' -o tests/output/benchmark-results/v14-public-parallel-scale/stress-default-100.vcf` | `VCF_FAST_NATIVE_FILTER_THREADS=4 VCF_FAST_NATIVE_FILTER_BATCH_RECORDS=4096 ./target/release/vcf-fast filter tests/output/benchmark-results/v14-public-parallel-scale/data/stress-100.vcf --where 'ANY(FORMAT/AD > 80)' -o tests/output/benchmark-results/v14-public-parallel-scale/stress-parallel-100.vcf` | `bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' tests/output/benchmark-results/v14-public-parallel-scale/data/stress-100.vcf -o tests/output/benchmark-results/v14-public-parallel-scale/stress-bcftools-100.vcf` | parallel native matches default native byte-for-byte and matches bcftools filtered core records | default 0.005635s +/- 0.000000s; parallel 0.004373s +/- 0.000000s; bcftools 0.006521s +/- 0.000000s | parallel/default 1.29x; parallel/bcftools 1.49x | default 17746 / parallel 22868 / bcftools 15335 | default n/a / parallel n/a / bcftools n/a KB | synthetic stress CPU-heavy expression; public FORMAT-heavy evidence still pending | smoke validation only; no speed claim from sub-10k tier |

## Raw Artifacts

- Working datasets: `tests/output/benchmark-results/v14-public-parallel-scale/data`
- Hyperfine JSON files: `tests/output/benchmark-results/v14-public-parallel-scale/hyperfine-*.json`
- Equivalence diffs: `tests/output/benchmark-results/v14-public-parallel-scale/equivalence-*.diff`
- Peak RSS files: `tests/output/benchmark-results/v14-public-parallel-scale/rss-*.txt`
