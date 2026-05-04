#!/usr/bin/env bash
set -euo pipefail

OUTPUT="${1:-tests/output/benchmark-data/stress.vcf}"
RECORDS="${2:-10000}"
INFO_FIELDS="${VCF_FAST_STRESS_INFO_FIELDS:-40}"
SAMPLES="${VCF_FAST_STRESS_SAMPLES:-16}"

mkdir -p "$(dirname "$OUTPUT")"

awk \
  -v records="$RECORDS" \
  -v info_fields="$INFO_FIELDS" \
  -v samples="$SAMPLES" \
  'BEGIN {
    print "##fileformat=VCFv4.3"
    for (chrom = 1; chrom <= 22; chrom++) {
      printf "##contig=<ID=%d>\n", chrom
    }
    print "##FILTER=<ID=PASS,Description=\"All filters passed\">"
    print "##FILTER=<ID=q10,Description=\"Synthetic low-quality marker\">"
    print "##INFO=<ID=DP,Number=1,Type=Integer,Description=\"Total Depth\">"
    print "##INFO=<ID=AF,Number=A,Type=Float,Description=\"Allele Frequency\">"
    for (field = 0; field < info_fields; field++) {
      printf "##INFO=<ID=UNUSED%d,Number=1,Type=Integer,Description=\"Unused stress field\">\n", field
    }
    print "##FORMAT=<ID=GT,Number=1,Type=String,Description=\"Genotype\">"
    print "##FORMAT=<ID=DP,Number=1,Type=Integer,Description=\"Sample depth\">"
    print "##FORMAT=<ID=GQ,Number=1,Type=Integer,Description=\"Genotype quality\">"
    print "##FORMAT=<ID=AD,Number=R,Type=Integer,Description=\"Allelic depths\">"

    printf "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT"
    for (sample = 1; sample <= samples; sample++) {
      printf "\tSAMPLE_%03d", sample
    }
    printf "\n"

    refs[0] = "A"; alts[0] = "G"
    refs[1] = "C"; alts[1] = "T"
    refs[2] = "G"; alts[2] = "C"
    refs[3] = "T"; alts[3] = "A"

    for (i = 1; i <= records; i++) {
      chrom = i % 22 + 1
      pos = 100000 + i
      qual = i % 100
      dp = i % 80 + 1
      af = sprintf("0.%03d", i % 1000)
      allele = i % 4
      filter = i % 17 == 0 ? "q10" : "PASS"

      info = ""
      midpoint = int(info_fields / 2)
      for (field = 0; field < info_fields; field++) {
        if (field > 0) {
          info = info ";"
        }
        info = info sprintf("UNUSED%d=%d", field, (i + field) % 997)
        if (field == midpoint) {
          info = info sprintf(";DP=%d", dp)
        }
      }
      if (info_fields == 0) {
        info = sprintf("DP=%d", dp)
      }
      info = info sprintf(";AF=%s", af)

      printf "%d\t%d\tstress%d\t%s\t%s\t%d\t%s\t%s\tGT:DP:GQ:AD", chrom, pos, i, refs[allele], alts[allele], qual, filter, info
      for (sample = 1; sample <= samples; sample++) {
        sample_dp = (i + sample) % 90 + 1
        gq = (i + sample) % 99
        if ((i + sample) % 3 == 0) {
          gt = "1/1"
          ref_depth = 0
          alt_depth = sample_dp
        } else if ((i + sample) % 2 == 0) {
          gt = "0/1"
          ref_depth = int(sample_dp / 2)
          alt_depth = sample_dp - ref_depth
        } else {
          gt = "0/0"
          ref_depth = sample_dp
          alt_depth = 0
        }
        printf "\t%s:%d:%d:%d,%d", gt, sample_dp, gq, ref_depth, alt_depth
      }
      printf "\n"
    }
  }' >"$OUTPUT"
