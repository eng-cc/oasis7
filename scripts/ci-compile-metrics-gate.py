#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path


# These fields define whether current and baseline compile measurements are
# comparable, independent of their commit provenance.
IDENTITY_FIELDS = (
    "package",
    "binary",
    "check_only",
    "no_default_features",
)


def load_comparison(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def parse_regression_threshold(raw_value: str) -> float:
    try:
        value = float(raw_value)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("must be a number") from exc
    if not math.isfinite(value) or value < 0:
        raise argparse.ArgumentTypeError(
            "must be finite and non-negative"
        )
    return value


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Enforce compile-metrics regression thresholds from comparison.json."
    )
    parser.add_argument("--comparison", required=True, help="Path to comparison.json")
    parser.add_argument(
        "--max-package-count-regression-pct",
        type=parse_regression_threshold,
        default=None,
        help="Maximum allowed percentage increase in package closure count.",
    )
    parser.add_argument(
        "--max-cargo-check-regression-pct",
        type=parse_regression_threshold,
        default=None,
        help="Maximum allowed percentage increase in cold cargo check wall-clock time.",
    )
    parser.add_argument(
        "--max-cargo-build-release-regression-pct",
        type=parse_regression_threshold,
        default=None,
        help="Maximum allowed percentage increase in cold cargo build --release wall-clock time.",
    )
    parser.add_argument(
        "--max-release-binary-bytes-regression-pct",
        type=parse_regression_threshold,
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

    def identity_for(metrics: object, label: str) -> dict | None:
        if not isinstance(metrics, dict):
            failures.append(f"{label} metrics must be a JSON object")
            return None
        missing = [field for field in IDENTITY_FIELDS if field not in metrics]
        if missing:
            failures.append(
                f"{label} metrics missing measurement identity fields: {', '.join(missing)}"
            )
            return None

        valid = True
        package = metrics["package"]
        if type(package) is not str or not package:
            valid = False
            failures.append(
                f"{label} measurement identity package must be a non-empty string"
            )

        binary = metrics["binary"]
        binary_valid = binary is None or (type(binary) is str and bool(binary))
        if not binary_valid:
            valid = False
            failures.append(
                f"{label} measurement identity binary must be null or a non-empty string"
            )

        for field in ("check_only", "no_default_features"):
            if type(metrics[field]) is not bool:
                valid = False
                failures.append(
                    f"{label} measurement identity {field} must be a boolean"
                )

        check_only = metrics["check_only"]
        if type(check_only) is bool and binary_valid:
            if check_only and binary is not None:
                valid = False
                failures.append(
                    f"{label} measurement identity check_only requires binary to be null"
                )
            if not check_only and binary is None:
                valid = False
                failures.append(
                    f"{label} measurement identity release-build requires binary to be a non-empty string"
                )

        if not valid:
            return None
        return {field: metrics[field] for field in IDENTITY_FIELDS}

    current_identity = identity_for(current, "current")
    comparison_identity = comparison.get("measurement_identity")
    if comparison_identity is None:
        failures.append("comparison is missing measurement_identity")
    elif current_identity is not None and comparison_identity != current_identity:
        failures.append(
            "comparison measurement identity does not match current metrics"
        )

    baseline_identity = (
        identity_for(baseline, "baseline") if baseline is not None else None
    )
    if (
        current_identity is not None
        and baseline_identity is not None
        and current_identity != baseline_identity
    ):
        mismatches = [
            field
            for field in IDENTITY_FIELDS
            if current_identity[field] != baseline_identity[field]
        ]
        failures.append(
            "current/baseline measurement identity mismatch: "
            + ", ".join(mismatches)
        )

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
