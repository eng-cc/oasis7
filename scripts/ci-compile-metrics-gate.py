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
    "warm_check_enabled",
)
LEGACY_IDENTITY_FIELDS = IDENTITY_FIELDS[:-1]
KNOWN_METRICS = (
    "package_count",
    "cargo_check_seconds",
    "cargo_build_release_seconds",
    "release_binary_bytes",
    "cargo_check_warm_seconds",
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


def is_finite_number(value: object) -> bool:
    if not isinstance(value, (int, float)) or isinstance(value, bool):
        return False
    try:
        return math.isfinite(value)
    except (OverflowError, ValueError):
        return False


def is_finite_non_negative_number(value: object) -> bool:
    return is_finite_number(value) and value >= 0


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
        "--max-cargo-check-warm-regression-pct",
        type=parse_regression_threshold,
        default=None,
        help="Maximum allowed percentage increase in warm/no-op cargo check time.",
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
    missing_schema = object()
    comparison_schema = comparison.get("schema_version", missing_schema)
    if comparison_schema is missing_schema:
        # Unversioned artifacts are the legacy V1 cold-only format.  They are
        # accepted only when neither side advertises V2 fields.
        v2 = False
    elif type(comparison_schema) is int and comparison_schema == 2:
        v2 = True
    else:
        failures.append("comparison schema_version must be 2")
        v2 = True

    def validate_payload_schema(metrics: object, label: str) -> None:
        if not isinstance(metrics, dict):
            return
        metric_schema = metrics.get("schema_version", missing_schema)
        if v2:
            if metric_schema is missing_schema:
                failures.append(f"{label} metrics is missing schema_version 2")
            elif type(metric_schema) is not int or metric_schema != 2:
                failures.append(f"{label} metrics schema_version must be 2")
            if "warm_check_enabled" not in metrics:
                failures.append(f"{label} metrics is missing warm_check_enabled")
            elif type(metrics["warm_check_enabled"]) is not bool:
                failures.append(f"{label} warm_check_enabled must be a boolean")
            if "cargo_check_warm_seconds" not in metrics:
                failures.append(f"{label} metrics is missing cargo_check_warm_seconds")
            elif type(metrics.get("warm_check_enabled")) is bool:
                warm_value = metrics["cargo_check_warm_seconds"]
                if metrics["warm_check_enabled"]:
                    if not is_finite_non_negative_number(warm_value):
                        failures.append(
                            f"{label} cargo_check_warm_seconds must be a finite non-negative number when enabled"
                        )
                elif warm_value is not None:
                    failures.append(
                        f"{label} cargo_check_warm_seconds must be null when warm_check_enabled is false"
                    )
        elif any(
            field in metrics
            for field in ("schema_version", "warm_check_enabled", "cargo_check_warm_seconds")
        ):
            failures.append(f"legacy {label} metrics must be cold-only")

    def validate_v2_payload_metrics(metrics: object, label: str) -> None:
        if not v2 or not isinstance(metrics, dict):
            return

        def require_non_negative(field: str) -> None:
            if field not in metrics:
                failures.append(f"{label} metrics is missing {field}")
            elif not is_finite_non_negative_number(metrics[field]):
                failures.append(
                    f"{label} metrics {field} must be a finite non-negative number"
                )

        for field in ("package_count", "cargo_check_seconds"):
            require_non_negative(field)

        check_only = metrics.get("check_only")
        for field in ("cargo_build_release_seconds", "release_binary_bytes"):
            if field not in metrics:
                failures.append(f"{label} metrics is missing {field}")
            elif check_only is True:
                if metrics[field] is not None:
                    failures.append(
                        f"{label} metrics {field} must be null for check-only measurements"
                    )
            elif check_only is False:
                require_non_negative(field)

        for field in ("wasmtime_present", "wasm_executor_present"):
            if field not in metrics:
                failures.append(f"{label} metrics is missing {field}")
            elif type(metrics[field]) is not bool:
                failures.append(f"{label} metrics {field} must be a boolean")

    validate_payload_schema(current, "current")
    validate_payload_schema(baseline, "baseline")
    validate_v2_payload_metrics(current, "current")
    validate_v2_payload_metrics(baseline, "baseline")
    if not v2 and isinstance(comparison.get("measurement_identity"), dict):
        if "warm_check_enabled" in comparison["measurement_identity"]:
            failures.append("legacy comparison must not include warm identity")
    if args.max_cargo_check_warm_regression_pct is not None and not v2:
        failures.append("warm threshold requires V2 metrics")
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
        identity_fields = IDENTITY_FIELDS if v2 else LEGACY_IDENTITY_FIELDS
        missing = [field for field in identity_fields if field not in metrics]
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
        return {field: metrics[field] for field in identity_fields}

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
            for field in (IDENTITY_FIELDS if v2 else LEGACY_IDENTITY_FIELDS)
            if current_identity[field] != baseline_identity[field]
        ]
        failures.append(
            "current/baseline measurement identity mismatch: "
            + ", ".join(mismatches)
        )
    if (
        args.max_cargo_check_warm_regression_pct is not None
        and current_identity is not None
        and not current_identity.get("warm_check_enabled", False)
    ):
        failures.append(
            "warm threshold requires warm_check_enabled to be true"
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

    # V2 payloads are dereferenced below for field and row binding.  Stop with
    # the accumulated deterministic diagnostics before any mapping access when
    # either side is not a JSON object.
    if v2 and (
        not isinstance(current, dict)
        or (baseline is not None and not isinstance(baseline, dict))
    ):
        for failure in failures:
            print(f"gate: FAIL: {failure}")
        return 1

    if (
        args.require_wasmtime_absent
        and isinstance(current, dict)
        and current.get("wasmtime_present")
    ):
        failures.append("current package closure still includes wasmtime")

    # Current-only runs validate the current payload and provenance but do not
    # require paired rows.  A selected threshold is explicitly not evaluated;
    # it must never disappear behind the baseline-conditional gate step.
    if baseline is None and v2:
        if failures:
            for failure in failures:
                print(f"gate: FAIL: {failure}")
            return 1
        if args.max_cargo_check_warm_regression_pct is not None:
            print(
                "gate: SKIP: no baseline metrics available; warm cargo check "
                f"threshold {args.max_cargo_check_warm_regression_pct:+.2f}% not evaluated"
            )
        else:
            print("gate: SKIP: no baseline metrics available for regression thresholds")
        return 0

    thresholds = {
        "package_count": args.max_package_count_regression_pct,
        "cargo_check_seconds": args.max_cargo_check_regression_pct,
        "cargo_build_release_seconds": args.max_cargo_build_release_regression_pct,
        "release_binary_bytes": args.max_release_binary_bytes_regression_pct,
        "cargo_check_warm_seconds": args.max_cargo_check_warm_regression_pct,
    }

    # Treat metric_rows as a schema-bearing list rather than a map projection:
    # duplicate or malformed rows must fail closed instead of silently
    # replacing evidence before threshold evaluation.  KNOWN_METRICS is static;
    # a selected threshold cannot make an unknown row valid.
    metric_rows: dict[str, dict] = {}
    metric_values: dict[str, dict[str, object]] = {}
    missing_metric_rows = object()
    raw_metric_rows = comparison.get("metric_rows", missing_metric_rows)
    if raw_metric_rows is missing_metric_rows:
        failures.append("comparison is missing metric_rows")
    elif not isinstance(raw_metric_rows, list):
        failures.append("comparison metric_rows must be a JSON array")
    else:
        for index, row in enumerate(raw_metric_rows):
            if not isinstance(row, dict):
                failures.append(
                    f"comparison metric_rows entry {index} must be a JSON object"
                )
                continue
            metric = row.get("metric")
            if type(metric) is not str or not metric:
                failures.append(
                    f"comparison metric_rows entry {index} metric must be a non-empty string"
                )
                continue
            if metric not in KNOWN_METRICS:
                failures.append(
                    f"comparison metric_rows entry {index} metric is unsupported: {metric}"
                )
                continue
            if not v2 and metric == "cargo_check_warm_seconds":
                failures.append("legacy comparison cannot contain warm metric rows")
                continue
            if metric in metric_rows:
                failures.append(
                    f"comparison metric_rows contains duplicate metric row for {metric}"
                )
                continue
            metric_rows[metric] = row

            baseline_value = row.get("baseline")
            current_value = row.get("current")
            delta_value = row.get("delta")
            percent = row.get("percent")

            if v2:
                expected_baseline = baseline.get(metric)
                expected_current = current.get(metric)
                if expected_baseline is None or baseline_value != expected_baseline:
                    failures.append(
                        f"comparison row {metric} baseline does not match baseline metrics {metric}"
                    )
                if expected_current is None or current_value != expected_current:
                    failures.append(
                        f"comparison row {metric} current does not match current metrics {metric}"
                    )

            baseline_valid = is_finite_non_negative_number(baseline_value)
            current_valid = is_finite_non_negative_number(current_value)
            delta_valid = is_finite_number(delta_value)
            percent_valid = is_finite_number(percent)
            if not baseline_valid:
                failures.append(
                    f"comparison row {metric} baseline must be a finite non-negative number"
                )
            if not current_valid:
                failures.append(
                    f"comparison row {metric} current must be a finite non-negative number"
                )
            if not delta_valid:
                failures.append(
                    f"comparison row {metric} delta must be a finite number"
                )
            if delta_valid and baseline_valid and current_valid and not math.isclose(
                delta_value,
                current_value - baseline_value,
                rel_tol=1e-9,
                abs_tol=1e-9,
            ):
                failures.append(
                    f"comparison row {metric} delta must equal current - baseline"
                )
            warm_zero_baseline_report_only = (
                metric == "cargo_check_warm_seconds"
                and args.max_cargo_check_warm_regression_pct is None
                and baseline_valid
                and baseline_value == 0
                and percent is None
            )
            if warm_zero_baseline_report_only:
                pass
            elif percent is None:
                failures.append(f"cannot evaluate {metric} regression percentage")
            elif baseline_valid and baseline_value == 0:
                failures.append(
                    f"cannot evaluate {metric} regression percentage with zero baseline"
                )
            elif not percent_valid:
                failures.append(
                    f"comparison row {metric} percent must be a finite number"
                )
            elif baseline_valid and baseline_value > 0 and current_valid:
                expected_percent = (
                    (current_value - baseline_value) / baseline_value
                ) * 100.0
                if not math.isclose(
                    percent,
                    expected_percent,
                    rel_tol=1e-9,
                    abs_tol=1e-9,
                ):
                    failures.append(
                        "comparison row "
                        f"{metric} percent must equal ((current - baseline) / baseline) * 100"
                    )
            metric_values[metric] = {
                "baseline": baseline_value,
                "current": current_value,
                "delta": delta_value,
                "percent": percent,
                "baseline_valid": baseline_valid,
                "current_valid": current_valid,
                "delta_valid": delta_valid,
                "percent_valid": percent_valid,
                "warm_zero_baseline_report_only": warm_zero_baseline_report_only,
            }

    if (
        v2
        and isinstance(current, dict)
        and isinstance(baseline, dict)
        and current.get("warm_check_enabled") is True
        and baseline.get("warm_check_enabled") is True
        and "cargo_check_warm_seconds" not in metric_rows
    ):
        failures.append(
            "comparison is missing cargo_check_warm_seconds row for warm measurements"
        )

    if baseline is None:
        if failures:
            for failure in failures:
                print(f"gate: FAIL: {failure}")
            return 1
        if args.max_cargo_check_warm_regression_pct is not None:
            print(
                "gate: SKIP: no baseline metrics available; warm cargo check "
                f"threshold {args.max_cargo_check_warm_regression_pct:+.2f}% not evaluated"
            )
        else:
            print("gate: SKIP: no baseline metrics available for regression thresholds")
        return 0

    metric_labels = {
        "package_count": "package closure count",
        "cargo_check_seconds": "cold cargo check time",
        "cargo_build_release_seconds": "cold cargo build --release time",
        "release_binary_bytes": "release binary size",
        "cargo_check_warm_seconds": "warm cargo check time",
    }

    for metric, threshold in thresholds.items():
        if threshold is None:
            continue
        row = metric_rows.get(metric)
        if row is None:
            failures.append(f"missing comparison row for {metric}")
            continue
        values = metric_values[metric]
        percent = values["percent"]
        percent_valid = values["percent_valid"]
        if percent is None or not percent_valid:
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
