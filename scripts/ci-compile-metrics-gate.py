#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def load_comparison(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Enforce compile-metrics regression thresholds from comparison.json."
    )
    parser.add_argument("--comparison", required=True, help="Path to comparison.json")
    parser.add_argument(
        "--max-package-count-regression-pct",
        type=float,
        default=None,
        help="Maximum allowed percentage increase in package closure count.",
    )
    parser.add_argument(
        "--max-cargo-check-regression-pct",
        type=float,
        default=None,
        help="Maximum allowed percentage increase in cold cargo check wall-clock time.",
    )
    parser.add_argument(
        "--max-cargo-build-release-regression-pct",
        type=float,
        default=None,
        help="Maximum allowed percentage increase in cold cargo build --release wall-clock time.",
    )
    parser.add_argument(
        "--max-release-binary-bytes-regression-pct",
        type=float,
        default=None,
        help="Maximum allowed percentage increase in release binary size.",
    )
    parser.add_argument(
        "--require-wasmtime-absent",
        action="store_true",
        help="Fail if current metrics still report wasmtime in the package closure.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    comparison = load_comparison(Path(args.comparison))
    current = comparison["current"]
    baseline = comparison.get("baseline")
    failures: list[str] = []

    if args.require_wasmtime_absent and current.get("wasmtime_present"):
        failures.append("current package closure still includes wasmtime")

    if baseline is None:
        if failures:
            for failure in failures:
                print(f"gate: FAIL: {failure}")
            return 1
        print("gate: SKIP: no baseline metrics available for regression thresholds")
        return 0

    thresholds = {
        "package_count": args.max_package_count_regression_pct,
        "cargo_check_seconds": args.max_cargo_check_regression_pct,
        "cargo_build_release_seconds": args.max_cargo_build_release_regression_pct,
        "release_binary_bytes": args.max_release_binary_bytes_regression_pct,
    }

    metric_rows = {row["metric"]: row for row in comparison.get("metric_rows", [])}
    metric_labels = {
        "package_count": "package closure count",
        "cargo_check_seconds": "cold cargo check time",
        "cargo_build_release_seconds": "cold cargo build --release time",
        "release_binary_bytes": "release binary size",
    }

    for metric, threshold in thresholds.items():
        if threshold is None:
            continue
        row = metric_rows.get(metric)
        if row is None:
            failures.append(f"missing comparison row for {metric}")
            continue
        percent = row.get("percent")
        if percent is None:
            failures.append(f"cannot evaluate {metric} regression percentage")
            continue
        if percent > threshold:
            failures.append(
                f"{metric_labels[metric]} regressed by {percent:+.2f}% (threshold {threshold:+.2f}%)"
            )

    if failures:
        for failure in failures:
            print(f"gate: FAIL: {failure}")
        return 1

    print("gate: PASS: compile metrics are within configured thresholds")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
