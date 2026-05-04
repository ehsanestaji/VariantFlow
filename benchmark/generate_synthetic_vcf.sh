#!/usr/bin/env bash
set -euo pipefail

OUTPUT="${1:-tests/output/benchmark-data/synthetic.vcf}"
RECORDS="${2:-10000}"

mkdir -p "$(dirname "$OUTPUT")"

{
  echo '##fileformat=VCFv4.3'
  echo '##INFO=<ID=DP,Number=1,Type=Integer,Description="Total Depth">'
  echo '##INFO=<ID=AF,Number=A,Type=Float,Description="Allele Frequency">'
  echo -e '#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO'

  for ((i = 1; i <= RECORDS; i++)); do
    chrom=$((i % 22 + 1))
    pos=$((100000 + i))
    qual=$((i % 100))
    dp=$((i % 80 + 1))
    af=$(printf '0.%03d' $((i % 1000)))

    case $((i % 4)) in
      0)
        ref='A'
        alt='G'
        ;;
      1)
        ref='C'
        alt='T'
        ;;
      2)
        ref='G'
        alt='C'
        ;;
      *)
        ref='T'
        alt='A'
        ;;
    esac

    if ((i % 17 == 0)); then
      filter='q10'
    else
      filter='PASS'
    fi

    printf '%s\t%s\tsyn%s\t%s\t%s\t%s\t%s\tDP=%s;AF=%s\n' \
      "$chrom" "$pos" "$i" "$ref" "$alt" "$qual" "$filter" "$dp" "$af"
  done
} >"$OUTPUT"
