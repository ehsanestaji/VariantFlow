#!/usr/bin/env bash
set -euo pipefail

INPUT="$1"
QUERY="$2"
REPEATS="${3:-1}"

run_once() {
  case "$QUERY" in
    row_count)
      bcftools view -H "$INPUT" | wc -l | tr -d ' '
      ;;
    qual_gt_30)
      bcftools filter -i 'QUAL>30' "$INPUT" -Ou | bcftools view -H | wc -l | tr -d ' '
      ;;
    dp_gt_40)
      bcftools query -f '%INFO\n' "$INPUT" |
        awk -F ';' '
          {
            passes = 0
            for (i = 1; i <= NF; i++) {
              if ($i ~ /^DP=/) {
                split($i, field, "=")
                split(field[2], values, ",")
                for (j in values) {
                  if (values[j] != "." && values[j] + 0 > 40) {
                    passes = 1
                  }
                }
              }
            }
            if (passes) {
              n++
            }
          }
          END { print n + 0 }
        '
      ;;
    filter_pass)
      bcftools filter -i 'FILTER="PASS"' "$INPUT" -Ou | bcftools view -H | wc -l | tr -d ' '
      ;;
    group_by_chrom_filter)
      bcftools query -f '%CHROM\t%FILTER\n' "$INPUT" |
        awk -F '\t' 'BEGIN { OFS = "\t" } { key = $1 OFS ($2 == "" ? "." : $2); counts[key]++ } END { for (key in counts) print key, counts[key] }' |
        sort -k1,1 -k2,2
      ;;
    *)
      echo "unsupported columnar query: $QUERY" >&2
      exit 2
      ;;
  esac
}

result=""
for _ in $(seq 1 "$REPEATS"); do
  result="$(run_once)"
done

printf '%s\n' "$result"
