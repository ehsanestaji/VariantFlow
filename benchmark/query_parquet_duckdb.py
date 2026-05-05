#!/usr/bin/env python3
"""Run small repeated DuckDB queries against VCF-Fast Parquet output."""

from __future__ import annotations

import argparse
import sys


DUCKDB_MISSING = (
    "duckdb python package is required for columnar workflow benchmarks; "
    "install it with `python3 -m pip install duckdb`"
)


def load_duckdb():
    try:
        import duckdb  # type: ignore
    except ImportError:
        print(DUCKDB_MISSING, file=sys.stderr)
        raise SystemExit(2)
    return duckdb


def sql_string(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


QUERIES = {
    "row_count": 'SELECT COUNT(*) FROM variants',
    "qual_gt_30": 'SELECT COUNT(*) FROM variants WHERE QUAL > 30',
    "filter_pass": 'SELECT COUNT(*) FROM variants WHERE FILTER = \'PASS\'',
}


def count_query(parquet_path: str, query: str, repeats: int) -> int:
    duckdb = load_duckdb()
    connection = duckdb.connect(database=":memory:")
    connection.execute(f"CREATE VIEW variants AS SELECT * FROM read_parquet({sql_string(parquet_path)})")
    result = 0
    for _ in range(repeats):
        result = int(connection.execute(QUERIES[query]).fetchone()[0])
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("parquet", nargs="?", help="Parquet file produced by vcf-fast convert --to parquet")
    parser.add_argument("--query", choices=sorted(QUERIES), default="qual_gt_30")
    parser.add_argument("--repeats", type=int, default=1)
    parser.add_argument("--check", action="store_true", help="Verify DuckDB can be imported and exit")
    args = parser.parse_args()

    if args.check:
        load_duckdb()
        return 0

    if not args.parquet:
        parser.error("parquet path is required unless --check is used")
    if args.repeats < 1:
        parser.error("--repeats must be a positive integer")

    print(count_query(args.parquet, args.query, args.repeats))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
