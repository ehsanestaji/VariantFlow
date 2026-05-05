#!/usr/bin/env python3
import json
import sys


def seconds(value: float) -> str:
    return f"{value:.6f}s"


with open(sys.argv[1], "r", encoding="utf-8") as handle:
    data = json.load(handle)

results = data["results"]
fast_mean = results[0]["mean"]
fast_stddev = results[0].get("stddev") or 0.0
bcftools_mean = results[1]["mean"]
bcftools_stddev = results[1].get("stddev") or 0.0
speedup = bcftools_mean / fast_mean if fast_mean > 0 else 0.0

print(
    seconds(fast_mean),
    seconds(fast_stddev),
    seconds(bcftools_mean),
    seconds(bcftools_stddev),
    f"{speedup:.2f}x",
)
