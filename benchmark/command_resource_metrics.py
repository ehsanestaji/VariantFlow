#!/usr/bin/env python3
"""Run one command and write wall/RSS/CPU metrics as JSON."""

from __future__ import annotations

import argparse
import json
import os
import resource
import subprocess
import sys
import time
from pathlib import Path


def peak_rss_kb(raw_maxrss: int) -> int:
    if sys.platform == "darwin":
        return raw_maxrss // 1024
    return raw_maxrss


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json-out", required=True, type=Path)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if not args.command:
        raise SystemExit("command_resource_metrics.py requires a command after --")
    command = args.command[1:] if args.command[0] == "--" else args.command

    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    started = time.perf_counter()
    completed = subprocess.run(command, check=False)
    elapsed = time.perf_counter() - started
    after = resource.getrusage(resource.RUSAGE_CHILDREN)

    user_seconds = max(0.0, after.ru_utime - before.ru_utime)
    system_seconds = max(0.0, after.ru_stime - before.ru_stime)
    cpu_seconds = user_seconds + system_seconds
    metrics = {
        "command": command,
        "exit_code": completed.returncode,
        "wall_seconds": elapsed,
        "user_seconds": user_seconds,
        "system_seconds": system_seconds,
        "cpu_seconds": cpu_seconds,
        "cpu_hours": cpu_seconds / 3600.0,
        "peak_rss_kb": peak_rss_kb(after.ru_maxrss),
        "platform": sys.platform,
        "pid": os.getpid(),
    }
    args.json_out.parent.mkdir(parents=True, exist_ok=True)
    args.json_out.write_text(json.dumps(metrics, indent=2, sort_keys=True) + "\n")
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
