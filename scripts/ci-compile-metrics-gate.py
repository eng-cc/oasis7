#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
import re
import subprocess
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

REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
GIT_OID_LENGTHS = {"sha1": 40, "sha256": 64}


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


def repository_oid_length() -> int:
    result = subprocess.run(
        ["git", "-C", str(REPOSITORY_ROOT), "rev-parse", "--show-object-format"],
        capture_output=True,
        text=True,
        check=False,
    )
    object_format = result.stdout.strip()
    if result.returncode != 0 or object_format not in GIT_OID_LENGTHS:
        raise RuntimeError("unable to resolve repository Git object format")
    return GIT_OID_LENGTHS[object_format]


def resolves_to_commit(commit_oid: str) -> bool:
    result = subprocess.run(
        [
            "git",
            "-C",
            str(REPOSITORY_ROOT),
            "cat-file",
            "-e",
            f"{commit_oid}^{{commit}}",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    return result.returncode == 0


def repository_head_oid(oid_length: int) -> str:
    result = subprocess.run(
        [
            "git",
            "-C",
            str(REPOSITORY_ROOT),
            "rev-parse",
            "--verify",
            "HEAD^{commit}",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    head_oid = result.stdout.strip()
    if (
        result.returncode != 0
        or re.fullmatch(rf"[0-9a-f]{{{oid_length}}}", head_oid) is None
        or not resolves_to_commit(head_oid)
    ):
        raise RuntimeError("unable to resolve repository HEAD commit OID")
    return head_oid


def main() -> int:
    args = parse_args()
    comparison = load_comparison(Path(args.comparison))
    if not isinstance(comparison, dict):
        print("gate: FAIL: comparison must be a JSON object")
        return 1

    current = comparison.get("current")
    baseline = comparison.get("baseline")
    failures: list[str] = []
    try:
        oid_length = repository_oid_length()
        repository_head = repository_head_oid(oid_length)
    except RuntimeError as exc:
        print(f"gate: FAIL: {exc}")
        return 1

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

    # The harness records commit OIDs in both the per-checkout metrics and the
    # comparison envelope.  Require those duplicated fields to agree before
    # evaluating a regression row, so a stale or cross-checkout artifact cannot
    # be presented as evidence for the requested baseline.
    def commit_oid_for(metrics: object, label: str) -> str | None:
        if not isinstance(metrics, dict):
            return None
        commit_oid = metrics.get("commit_oid")
        if type(commit_oid) is not str or not commit_oid:
            failures.append(f"{label} metrics commit_oid must be a non-empty string")
            return None
        if re.fullmatch(rf"[0-9a-f]{{{oid_length}}}", commit_oid) is None:
            failures.append(
                f"{label} metrics commit_oid must be a canonical full Git OID"
            )
            return None
        if not resolves_to_commit(commit_oid):
            failures.append(
                f"{label} metrics commit_oid does not resolve to a commit object"
            )
            return None
        return commit_oid

    current_commit_oid = commit_oid_for(current, "current")
    reported_current_commit_oid = comparison.get("current_commit_oid")
    if type(reported_current_commit_oid) is not str or not reported_current_commit_oid:
        failures.append(
            "comparison current_commit_oid must be a non-empty string"
        )
    elif re.fullmatch(rf"[0-9a-f]{{{oid_length}}}", reported_current_commit_oid) is None:
        failures.append(
            "comparison current_commit_oid must be a canonical full Git OID"
        )
    elif not resolves_to_commit(reported_current_commit_oid):
        failures.append(
            "comparison current_commit_oid does not resolve to a commit object"
        )
    elif (
        current_commit_oid is not None
        and reported_current_commit_oid != current_commit_oid
    ):
        failures.append(
            "comparison current_commit_oid does not match current metrics commit_oid"
        )
    if current_commit_oid is not None and current_commit_oid != repository_head:
        failures.append("current metrics commit_oid does not match repository HEAD")
    if (
        type(reported_current_commit_oid) is str
        and reported_current_commit_oid != repository_head
        and re.fullmatch(rf"[0-9a-f]{{{oid_length}}}", reported_current_commit_oid)
        is not None
        and resolves_to_commit(reported_current_commit_oid)
    ):
        failures.append("comparison current_commit_oid does not match repository HEAD")

    if baseline is None:
        if "baseline_ref" not in comparison:
            failures.append("comparison is missing baseline_ref")
        elif comparison["baseline_ref"] is not None:
            failures.append("comparison baseline_ref must be null without baseline metrics")
        if "baseline_commit_oid" not in comparison:
            failures.append("comparison is missing baseline_commit_oid")
        elif comparison["baseline_commit_oid"] is not None:
            failures.append(
                "comparison baseline_commit_oid must be null without baseline metrics"
            )
    else:
        baseline_commit_oid = commit_oid_for(baseline, "baseline")
        reported_baseline_commit_oid = comparison.get("baseline_commit_oid")
        if (
            type(reported_baseline_commit_oid) is not str
            or not reported_baseline_commit_oid
        ):
            failures.append(
                "comparison baseline_commit_oid must be a non-empty string with baseline metrics"
            )
        elif re.fullmatch(rf"[0-9a-f]{{{oid_length}}}", reported_baseline_commit_oid) is None:
            failures.append(
                "comparison baseline_commit_oid must be a canonical full Git OID"
            )
        elif not resolves_to_commit(reported_baseline_commit_oid):
            failures.append(
                "comparison baseline_commit_oid does not resolve to a commit object"
            )
        elif (
            baseline_commit_oid is not None
            and reported_baseline_commit_oid != baseline_commit_oid
        ):
            failures.append(
                "comparison baseline_commit_oid does not match baseline metrics commit_oid"
            )

        reported_baseline_ref = comparison.get("baseline_ref")
        if type(reported_baseline_ref) is not str or not reported_baseline_ref:
            failures.append(
                "comparison baseline_ref must be a non-empty string with baseline metrics"
            )
        elif re.fullmatch(rf"[0-9a-f]{{{oid_length}}}", reported_baseline_ref) is None:
            failures.append("comparison baseline_ref must be a canonical full Git OID")
        elif not resolves_to_commit(reported_baseline_ref):
            failures.append(
                "comparison baseline_ref does not resolve to a commit object"
            )
        elif (
            reported_baseline_commit_oid is not None
            and reported_baseline_ref != reported_baseline_commit_oid
        ):
            failures.append(
                "comparison baseline_ref does not match baseline_commit_oid"
            )

    if (
        args.require_wasmtime_absent
        and isinstance(current, dict)
        and current.get("wasmtime_present")
    ):
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
        if (
            not isinstance(percent, (int, float))
            or isinstance(percent, bool)
            or not math.isfinite(percent)
        ):
            failures.append(
                f"comparison row {metric} percent must be a finite number"
            )
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
