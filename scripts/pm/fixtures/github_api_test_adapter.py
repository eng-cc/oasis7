"""Isolated subprocess fixture for Project adapter tests.

Tests copy this file to a temporary directory as ``github_api.py`` so adapter
integration cases continue to use their existing fake ``gh`` fixtures without
loading the production transport or enabling a network endpoint override.
"""
from __future__ import annotations

import json
import os
import re
import subprocess
from typing import Any


class APIError(RuntimeError):
    def __init__(self, message: str, *, kind: str, status_code: int | None = None,
                 retry_after_seconds: int | None = None, uncertain: bool = False,
                 mutation_started: bool = False, details: dict[str, Any] | None = None):
        super().__init__(message)
        self.kind = kind
        self.status_code = status_code
        self.retry_after_seconds = retry_after_seconds
        self.uncertain = uncertain
        self.mutation_started = mutation_started or uncertain
        self.details = details or {}

    @property
    def workflow_status(self) -> str:
        if self.kind in {"primary_rate_limit", "secondary_rate_limit", "rate_limit_probe_pending"}:
            return "external_wait"
        if self.uncertain:
            return "uncertain"
        return "capability_blocked"

    @property
    def exit_code(self) -> int:
        return 75 if self.workflow_status == "external_wait" else 2

    def as_dict(self) -> dict[str, Any]:
        result = {
            "status": self.workflow_status,
            "reason": self.kind,
            "error": str(self),
            "ready_for_merge": False,
            "mutation_started": self.mutation_started,
            "uncertain": self.uncertain,
        }
        if self.status_code is not None:
            result["status_code"] = self.status_code
        if self.retry_after_seconds is not None:
            result["retry_after_seconds"] = self.retry_after_seconds
        result.update(self.details)
        return result


class GitHubAPIClient:
    """Fixture client that translates adapter calls into the suite's fake gh."""

    def __init__(self, token: str | None = None, *, transport=None, state_root=None,
                 clock=None, sleeper=None, random_value=None, timeout: float = 30.0):
        self.token = token
        self.transport = transport

    @classmethod
    def from_gh(cls, **kwargs):
        token = os.environ.get("GH_TOKEN", "").strip() or os.environ.get("GITHUB_TOKEN", "").strip()
        return cls(token or "isolated-test-credential", **kwargs)

    @staticmethod
    def _gh(args: list[str]) -> Any:
        try:
            result = subprocess.run(
                ["gh", *args], check=True, text=True, stdout=subprocess.PIPE,
                stderr=subprocess.PIPE, timeout=30,
            )
        except subprocess.CalledProcessError as exc:
            match = re.search(r"\(HTTP (\d{3})\)", str(exc.stderr or ""))
            if match is not None:
                status_code = int(match.group(1))
                kind = "permission_denied" if status_code in {401, 403} else "http_error"
                raise APIError(
                    f"fixture GitHub returned HTTP {status_code}",
                    kind=kind,
                    status_code=status_code,
                ) from exc
            raise APIError("fixture GitHub command failed", kind="transport_error") from exc
        except (OSError, subprocess.TimeoutExpired) as exc:
            raise APIError("fixture GitHub command failed", kind="transport_error") from exc
        stdout = result.stdout.strip()
        try:
            value = json.loads(stdout) if stdout else {}
        except json.JSONDecodeError as exc:
            raise APIError("fixture GitHub response is malformed", kind="malformed_response") from exc
        if not isinstance(value, (dict, list)):
            raise APIError("fixture GitHub response is not an object or array", kind="malformed_response")
        return value

    @staticmethod
    def _variable_args(variables: dict[str, Any]) -> list[str]:
        result: list[str] = []
        for key, value in variables.items():
            values = value if isinstance(value, list) else [value]
            for item in values:
                option = "-F" if isinstance(item, (bool, int, float)) else "-f"
                field = f"{key}[]={item}" if isinstance(value, list) else f"{key}={item}"
                result.extend([option, field])
        return result

    def graphql(self, query: str, variables: dict[str, Any] | None = None, *,
                operation: str, mutation: bool = False, context: dict[str, Any] | None = None):
        del operation, mutation, context
        if os.environ.get("GH_FIXTURE_EXTERNAL_WAIT") == "1":
            raise APIError("fixture rate limit pause", kind="primary_rate_limit",
                           retry_after_seconds=120)
        payload = self._gh(["api", "graphql", "-f", "query=" + query,
                            *self._variable_args(variables or {})])
        if not isinstance(payload, dict):
            raise APIError("fixture GitHub GraphQL response is not an object", kind="malformed_response")
        errors = payload.get("errors")
        data = payload.get("data")
        if errors:
            raise APIError("fixture GitHub GraphQL returned errors", kind="graphql_error")
        if not isinstance(data, dict):
            raise APIError("fixture GitHub GraphQL response has no object data", kind="malformed_response")
        return data

    def rest(self, method: str, path: str, payload: Any = None, *, operation: str,
             mutation: bool | None = None, context: dict[str, Any] | None = None):
        del operation, mutation, context
        args = ["api", path.lstrip("/"), "--method", method.upper()]
        if isinstance(payload, dict):
            for key, value in payload.items():
                args.extend(["-f", f"{key}={value}"])
        return self._gh(args)

    def rate_limit_snapshot(self, *, max_age_seconds: float | None = None):
        del max_age_seconds
        return None

    def rate_limit_guard(self, *, minimum_remaining: int = 100, max_age_seconds: float = 300,
                         operation: str = "rate_limit_guard", context: dict[str, Any] | None = None):
        del max_age_seconds
        try:
            data = self.graphql(
                "query RateLimitBudget { rateLimit { cost remaining used resetAt limit } }",
                operation=operation,
                context=context,
            )
        except APIError as exc:
            return {
                "status": "capability_blocked",
                "reason": "graphql_rate_limit_unavailable",
                "error": str(exc),
                "resumable": True,
                "resume": "restore GitHub rateLimit access and rerun",
            }
        rate = data.get("rateLimit")
        if not isinstance(rate, dict) or not isinstance(rate.get("remaining"), int):
            return {"status": "capability_blocked", "reason": "graphql_budget_unknown",
                    "resumable": True, "resume": "restore GitHub rateLimit visibility and rerun"}
        remaining = rate["remaining"]
        if remaining == 0:
            return {"status": "external_wait", "reason": "primary_rate_limit",
                    "remaining": remaining, "resetAt": rate.get("resetAt"),
                    "resumable": True, "resume": f"resume after {rate.get('resetAt')}"}
        if remaining < minimum_remaining:
            return {"status": "capability_blocked", "reason": "graphql_budget_insufficient",
                    "remaining": remaining, "resetAt": rate.get("resetAt"),
                    "resumable": True, "resume": f"resume after {rate.get('resetAt')}"}
        return {"status": "ok", "remaining": remaining, "resetAt": rate.get("resetAt")}
