#!/usr/bin/env python3
"""RED contract for the C2 Cargo package/profile CI driver.

The GREEN implementation must provide
``scripts/pm/cargo_package_profile_driver.py`` with
``validate_planned_items(plan, results, integration_base, source_head,
tested_tree)``.  The driver is the fail-closed boundary consumed by
``ci-tests.sh``: every planned item must run exactly once, with the frozen
source/integration/tested-tree identity and a zero exit status.
"""

from __future__ import annotations

import importlib.util
import hashlib
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
IMPLEMENTATION = ROOT / "scripts" / "pm" / "cargo_package_profile_driver.py"


class CargoPackageProfileDriverContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not IMPLEMENTATION.is_file():
            raise AssertionError(
                "RED: missing implementation: expected driver at "
                f"{IMPLEMENTATION}; add production behavior without editing this test"
            )
        spec = importlib.util.spec_from_file_location("cargo_package_profile_driver", IMPLEMENTATION)
        if spec is None or spec.loader is None:
            raise AssertionError(f"RED: unable to load driver module {IMPLEMENTATION}")
        cls.api = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.api)
        if not hasattr(cls.api, "validate_planned_items"):
            raise AssertionError("RED: driver must export validate_planned_items")

    def setUp(self) -> None:
        self.integration_base = "b" * 40
        self.source_head = "h" * 40
        self.tested_tree = "t" * 40
        self.plan = {
            "schema": "oasis7-cargo-package-profile-plan/v1",
            "plan_id": "plan-c2-fixture",
            "source_scope_base": "s" * 40,
            "integration_base": self.integration_base,
            "source_head": self.source_head,
            "tested_tree": self.tested_tree,
            "selected_items": ["alpha-native", "alpha-wasm"],
            "items": [
                {
                    "id": "alpha-native",
                    "package": "alpha",
                    "profile": "native",
                    "target": "native",
                    "features": [],
                },
                {
                    "id": "alpha-wasm",
                    "package": "alpha",
                    "profile": "wasm",
                    "target": "wasm32-unknown-unknown",
                    "features": ["wasm"],
                },
            ],
        }
        unsigned = dict(self.plan)
        unsigned.pop("plan_id", None)
        self.plan["plan_id"] = "sha256:" + hashlib.sha256(
            json.dumps(unsigned, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()

    def _result(self, item_id: str, **overrides: object) -> dict[str, object]:
        result: dict[str, object] = {
            "item_id": item_id,
            "status": "passed",
            "exit_code": 0,
            "source_head": self.source_head,
            "integration_base": self.integration_base,
            "tested_tree": self.tested_tree,
        }
        result.update(overrides)
        return result

    def _validate(self, results: list[dict[str, object]], **kwargs: object) -> dict[str, object]:
        return self.api.validate_planned_items(
            self.plan,
            results,
            integration_base=kwargs.pop("integration_base", self.integration_base),
            source_head=kwargs.pop("source_head", self.source_head),
            tested_tree=kwargs.pop("tested_tree", self.tested_tree),
            **kwargs,
        )

    def test_complete_matching_plan_passes_and_preserves_item_order(self) -> None:
        result = self._validate(
            [self._result("alpha-native"), self._result("alpha-wasm")]
        )
        self.assertEqual("passed", result["status"])
        self.assertEqual(["alpha-native", "alpha-wasm"], result["completed_items"])

    def test_duplicate_planned_item_result_fails_closed(self) -> None:
        with self.assertRaisesRegex(Exception, "duplicate"):
            self._validate(
                [
                    self._result("alpha-native"),
                    self._result("alpha-native"),
                    self._result("alpha-wasm"),
                ]
            )

    def test_unknown_planned_item_result_fails_closed(self) -> None:
        with self.assertRaisesRegex(Exception, "unknown"):
            self._validate(
                [self._result("alpha-native"), self._result("not-in-plan")]
            )

    def test_missing_planned_item_result_fails_closed(self) -> None:
        with self.assertRaisesRegex(Exception, "missing"):
            self._validate([self._result("alpha-native")])

    def test_skipped_planned_item_is_not_success(self) -> None:
        with self.assertRaisesRegex(Exception, "skip"):
            self._validate(
                [
                    self._result("alpha-native"),
                    self._result("alpha-wasm", status="skipped"),
                ]
            )

    def test_identity_mismatch_fails_closed(self) -> None:
        for field, value, reason in (
            ("source_head", "x" * 40, "source"),
            ("integration_base", "y" * 40, "integration"),
            ("tested_tree", "z" * 40, "tested.tree"),
        ):
            with self.subTest(field=field):
                results = [self._result("alpha-native"), self._result("alpha-wasm")]
                results[0][field] = value
                with self.assertRaisesRegex(Exception, reason):
                    self._validate(results)

    def test_nonzero_planned_item_result_fails_closed(self) -> None:
        with self.assertRaisesRegex(Exception, "nonzero|exit"):
            self._validate(
                [
                    self._result("alpha-native", status="failed", exit_code=17),
                    self._result("alpha-wasm"),
                ]
            )

    def test_stale_plan_after_integration_base_advance_fails_closed(self) -> None:
        with self.assertRaisesRegex(Exception, "stale|integration"):
            self._validate(
                [self._result("alpha-native"), self._result("alpha-wasm")],
                integration_base="c" * 40,
            )

    def test_forged_plan_id_fails_closed_even_when_items_and_identity_match(self) -> None:
        self.plan["plan_id"] = "sha256:forged-plan"
        with self.assertRaisesRegex(Exception, "plan.?id|digest|forged"):
            self._validate(
                [self._result("alpha-native"), self._result("alpha-wasm")]
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
