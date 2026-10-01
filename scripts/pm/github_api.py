#!/usr/bin/env python3
"""Shared, metered GitHub JSON transport for bounded PM workflow paths."""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import pathlib
import random
import re
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from typing import Any


API_ROOT = "https://api.github.com"
GRAPHQL_URL = f"{API_ROOT}/graphql"
MAX_QUERY_ATTEMPTS = 3
TELEMETRY_RETENTION_DAYS = 7
TELEMETRY_MAX_BYTES = 50 * 1024 * 1024
_PROCESS_PAUSES: dict[str, dict[str, Any]] = {}


def _load_file_lock():
    path = pathlib.Path(__file__).with_name("portable_file_lock.py")
    spec = importlib.util.spec_from_file_location("oasis7_portable_file_lock", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load shared file-lock helper: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


_FILE_LOCK = _load_file_lock()


class APIError(RuntimeError):
    """Typed GitHub API failure safe for workflow JSON and process routing."""

    def __init__(self, message: str, *, kind: str, status_code: int | None = None,
                 retry_after_seconds: int | None = None, uncertain: bool = False,
                 mutation_started: bool = False, details: dict[str, Any] | None = None):
        super().__init__(message)
        self.kind = kind
        self.status_code = status_code
        self.retry_after_seconds = retry_after_seconds
        self.uncertain = uncertain
        self.mutation_started = mutation_started
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
        result: dict[str, Any] = {
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
            result["retry_after_seconds"] = max(0, int(self.retry_after_seconds))
        result.update(self.details)
        return result


class HTTPResponse:
    def __init__(self, status: int, headers: Any, body: bytes | str):
        self.status = int(status)
        self.headers = {str(k).lower(): str(v) for k, v in dict(headers or {}).items()}
        self.body = body.encode("utf-8") if isinstance(body, str) else bytes(body)


def _default_transport(method: str, url: str, headers: dict[str, str],
                       body: bytes | None, timeout: float) -> HTTPResponse:
    class _SameHostRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, response_headers, new_url):
            parsed = urllib.parse.urlsplit(new_url)
            if parsed.scheme != "https" or parsed.hostname != "api.github.com":
                return None
            return super().redirect_request(req, fp, code, msg, response_headers, new_url)

    request = urllib.request.Request(url, data=body, headers=headers, method=method)
    opener = urllib.request.build_opener(_SameHostRedirect())
    try:
        with opener.open(request, timeout=timeout) as response:
            return HTTPResponse(response.status, response.headers, response.read())
    except urllib.error.HTTPError as exc:
        return HTTPResponse(exc.code, exc.headers, exc.read())


def _response(value: Any) -> HTTPResponse:
    if isinstance(value, HTTPResponse):
        return value
    if isinstance(value, dict):
        status = value.get("status", value.get("status_code", 200))
        body = value.get("body", b"")
        if isinstance(body, (dict, list)):
            body = json.dumps(body, separators=(",", ":"), ensure_ascii=False)
        return HTTPResponse(status, value.get("headers"), body)
    return HTTPResponse(value.status, value.headers, value.read())


def _now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def _epoch(clock) -> float:
    return float(clock())


def _int_header(headers: dict[str, str], key: str) -> int | None:
    value = headers.get(key.lower(), "").strip()
    try:
        return int(value) if re.fullmatch(r"-?[0-9]+", value) else None
    except ValueError:
        return None


def _retry_after(headers: dict[str, str], now: float) -> int | None:
    raw = headers.get("retry-after", "").strip()
    if raw:
        if re.fullmatch(r"[0-9]+", raw):
            return max(0, int(raw))
        try:
            parsed = datetime.strptime(raw, "%a, %d %b %Y %H:%M:%S GMT").replace(tzinfo=timezone.utc)
            return max(0, int(parsed.timestamp() - now))
        except ValueError:
            pass
    return None


def _credential_digest(token: str | None) -> str:
    return hashlib.sha256((token or "<missing-token>").encode("utf-8")).hexdigest()


def _git_common_dir() -> pathlib.Path | None:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
            check=True, text=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=5,
        )
    except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired):
        return None
    value = result.stdout.strip()
    return pathlib.Path(value).resolve() if value else None


def _safe_operation(value: str) -> str:
    normalized = re.sub(r"[^A-Za-z0-9_.-]", "_", str(value or "unknown"))[:100]
    return normalized or "unknown"


def _safe_context(context: dict[str, Any] | None) -> dict[str, str]:
    context = context or {}
    safe: dict[str, str] = {}
    for key in ("script", "operation", "operation_id", "parent_operation_id", "phase",
                "api_family", "operation_name", "task_uid", "pr_number"):
        value = context.get(key)
        if value is None:
            continue
        text = str(value)
        if key == "task_uid" and not re.fullmatch(r"task_[0-9a-f]{32}", text):
            continue
        if key == "pr_number" and not re.fullmatch(r"[1-9][0-9]*", text):
            continue
        safe[key] = _safe_operation(text) if key in {
            "script", "operation", "operation_id", "parent_operation_id", "phase",
            "api_family", "operation_name",
        } else text
    return safe


class GitHubAPIClient:
    """Injectable transport with shared backoff and local request accounting."""

    def __init__(self, token: str | None = None, *, transport=None, state_root=None,
                 clock=None, sleeper=None, random_value=None, timeout: float = 30.0):
        self.token = token.strip() if isinstance(token, str) and token.strip() else None
        self.transport = transport or _default_transport
        self.clock = clock or time.time
        self.sleeper = sleeper or time.sleep
        self.random_value = random_value or random.random
        self.timeout = float(timeout)
        if state_root is None:
            common = _git_common_dir()
            state_root = common / "oasis7" / "github-api-v1" if common else None
        self.state_root = pathlib.Path(state_root).expanduser().resolve() if state_root is not None else None
        self.credential_scope_digest = _credential_digest(self.token)
        self._rate_state_key = hashlib.sha256(
            f"{self.state_root or '<no-shared-state>'}\n{self.credential_scope_digest}".encode("utf-8")
        ).hexdigest()
        self._account_scope_state_key: str | None = None
        self._restore_account_scope_identity()

    @classmethod
    def from_gh(cls, *, transport=None, state_root=None, clock=None, sleeper=None,
                random_value=None, timeout: float = 30.0):
        token = os.environ.get("GH_TOKEN", "").strip() or os.environ.get("GITHUB_TOKEN", "").strip()
        if not token:
            try:
                result = subprocess.run(
                    ["gh", "auth", "token"], check=True, text=True,
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=10,
                )
            except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired) as exc:
                raise APIError("GitHub credentials are unavailable through environment or gh auth",
                               kind="authentication_unavailable") from exc
            token = result.stdout.strip()
        if not token:
            raise APIError("GitHub credential provider returned an empty token", kind="authentication_unavailable")
        return cls(token, transport=transport, state_root=state_root, clock=clock,
                   sleeper=sleeper, random_value=random_value, timeout=timeout)

    @property
    def observations_dir(self) -> pathlib.Path | None:
        return self.state_root / "observations" if self.state_root else None

    @property
    def telemetry_dir(self) -> pathlib.Path | None:
        return self.state_root / "telemetry" if self.state_root else None

    @property
    def locks_dir(self) -> pathlib.Path | None:
        return self.state_root / "locks" if self.state_root else None

    def state_path(self, area: str, key: str) -> pathlib.Path:
        if self.state_root is None:
            raise OSError("GitHub API shared state is unavailable outside a registered Git worktree")
        if area not in {"budget", "observations", "locks"}:
            raise ValueError(f"unsupported shared state area: {area}")
        safe_key = hashlib.sha256(str(key).encode("utf-8")).hexdigest()
        suffix = ".lock" if area == "locks" else ".json"
        return self.state_root / area / f"{safe_key}{suffix}"

    def file_lock(self, key: str, *, timeout: float = 2.0):
        return _FILE_LOCK.locked_file(self.state_path("locks", key), timeout=timeout)

    def read_state(self, area: str, key: str, *, default=None, strict: bool = False):
        path = self.state_path(area, key)
        try:
            return json.loads(path.read_text(encoding="utf-8"))
        except FileNotFoundError:
            return default
        except (OSError, json.JSONDecodeError) as exc:
            if strict:
                raise APIError(f"shared GitHub API {area} state is unreadable",
                               kind="shared_state_unavailable") from exc
            return default

    def write_state(self, area: str, key: str, value: dict[str, Any], *, critical: bool = True) -> bool:
        try:
            _atomic_json(self.state_path(area, key), value)
            return True
        except OSError as exc:
            if critical:
                raise APIError(f"shared GitHub API {area} state could not be persisted",
                               kind="shared_state_unavailable") from exc
            self._warn_log_failure()
            return False

    def rate_limit_snapshot(self, *, max_age_seconds: float | None = None) -> dict[str, Any] | None:
        state = self._load_budget_state(strict=True)
        if state is None:
            return None
        if max_age_seconds is not None:
            observed = float(state.get("observed_at_epoch") or 0)
            if observed <= 0 or _epoch(self.clock) - observed > max_age_seconds:
                return None
        now = _epoch(self.clock)
        result = {
            "source": "shared_response_observation",
            "remaining": state.get("remaining"),
            "limit": state.get("limit"),
            "used": state.get("used"),
            "cost": state.get("cost"),
            "resetAt": state.get("resetAt"),
            "observed_at": state.get("observed_at"),
            "pause_until": state.get("pause_until"),
        }
        pause_until = float(state.get("pause_until_epoch") or 0)
        if pause_until > now:
            result["status"] = "external_wait"
            result["retry_after_seconds"] = max(0, int(pause_until - now + 0.999))
            result["reason"] = state.get("pause_reason") or "github_rate_limited"
        else:
            result["status"] = "observed"
        return result

    def rate_limit_guard(self, *, minimum_remaining: int = 100, max_age_seconds: float = 300,
                         operation: str = "rate_limit_guard",
                         context: dict[str, Any] | None = None) -> dict[str, Any]:
        """Reuse fresh response metadata or perform one serialized budget probe."""
        try:
            with self.file_lock(f"rate-limit-guard:{self._account_scope_state_key or self._rate_state_key}", timeout=2.0):
                state = self._load_budget_state(strict=True)
                now = _epoch(self.clock)
                if state is not None:
                    pause_until = float(state.get("pause_until_epoch") or 0)
                    if pause_until > now:
                        return self._wait_result(state, now)
                    observed = float(state.get("observed_at_epoch") or 0)
                    remaining = state.get("remaining")
                    fresh = observed > 0 and now - observed <= max_age_seconds
                    if fresh and isinstance(remaining, int):
                        return self._budget_result(state, minimum_remaining)
                payload = self.graphql(
                    "query RateLimitBudget { rateLimit { cost remaining used resetAt limit } }",
                    operation=operation,
                    context=context or {"script": pathlib.Path(sys.argv[0]).name, "operation": operation},
                )
                rate = payload.get("rateLimit") if isinstance(payload, dict) else None
                if not isinstance(rate, dict) or not isinstance(rate.get("remaining"), int):
                    return {"status": "capability_blocked", "reason": "graphql_budget_unknown",
                            "resumable": True, "resume": "restore GraphQL rateLimit visibility and rerun"}
                state = self._load_budget_state(strict=True) or {}
                return self._budget_result(state, minimum_remaining)
        except TimeoutError:
            return {"status": "external_wait", "reason": "graphql_budget_probe_in_progress",
                    "retry_after_seconds": 2, "resumable": True}
        except APIError as exc:
            return exc.as_dict()

    def graphql(self, query: str, variables: dict[str, Any] | None = None, *,
                operation: str, mutation: bool = False,
                context: dict[str, Any] | None = None) -> dict[str, Any]:
        if not isinstance(query, str) or not query.strip():
            raise APIError("GraphQL query is empty", kind="invalid_request")
        detected_mutation = bool(re.match(r"\s*mutation\b", query, re.IGNORECASE))
        is_mutation = mutation or detected_mutation
        payload = {"query": query, "variables": variables or {}}
        result = self._request("POST", GRAPHQL_URL, payload=payload, operation=operation,
                               is_query=not is_mutation, context=context, graphql=True)
        data = result.get("data")
        if not isinstance(data, dict):
            raise APIError("GraphQL response has no object data", kind="malformed_response")
        return data

    def rest(self, method: str, path: str, payload: Any = None, *, operation: str,
             mutation: bool | None = None, context: dict[str, Any] | None = None):
        method = method.upper()
        if method not in {"GET", "POST", "PATCH", "PUT", "DELETE"}:
            raise APIError("unsupported GitHub REST method", kind="invalid_request")
        parsed = urllib.parse.urlsplit(path)
        if parsed.scheme or parsed.netloc or ".." in pathlib.PurePosixPath(parsed.path).parts:
            raise APIError("REST path must be relative to api.github.com", kind="invalid_request")
        url = f"{API_ROOT}/{path.lstrip('/')}"
        is_mutation = mutation if mutation is not None else method != "GET"
        return self._request(method, url, payload=payload, operation=operation,
                             is_query=not is_mutation, context=context, graphql=False)

    def _request(self, method: str, url: str, *, payload: Any, operation: str,
                 is_query: bool, context: dict[str, Any] | None, graphql: bool):
        if not self.token:
            raise APIError("GitHub API token is unavailable", kind="authentication_unavailable")
        if not url.startswith(API_ROOT + "/"):
            raise APIError("GitHub API URL is outside the allowed host", kind="invalid_request")
        recovery_probe = self._before_request(is_query=is_query, graphql=graphql)
        body = None if payload is None else json.dumps(payload, separators=(",", ":")).encode("utf-8")
        headers = {
            "Authorization": f"Bearer {self.token}",
            "Accept": "application/vnd.github+json",
            "Content-Type": "application/json",
            "X-GitHub-Api-Version": "2022-11-28",
        }
        attempts = MAX_QUERY_ATTEMPTS if is_query else 1
        safe_context = _safe_context(context)
        op = _safe_operation(operation)
        request_fingerprint = hashlib.sha256(
            method.encode("ascii") + b"\n" + url.encode("utf-8") + b"\n" + (body or b"")
        ).hexdigest()
        try:
            for attempt in range(1, attempts + 1):
                started = _epoch(self.clock)
                try:
                    response = _response(self.transport(method, url, headers, body, self.timeout))
                    elapsed_ms = max(0, int((_epoch(self.clock) - started) * 1000))
                except Exception as exc:
                    elapsed_ms = max(0, int((_epoch(self.clock) - started) * 1000))
                    retryable = is_query and attempt < attempts
                    self._log_attempt(op, safe_context, attempt, None, elapsed_ms, None, None,
                                      None, "network_error", None, graphql=graphql,
                                      request_fingerprint=request_fingerprint)
                    if retryable:
                        self.sleeper(min(2.0, 0.2 * (2 ** (attempt - 1))) + self.random_value() * 0.05)
                        continue
                    error = APIError("GitHub API transport failed", kind="transport_error",
                                     uncertain=not is_query, mutation_started=not is_query)
                    self._finish_probe(recovery_probe, success=False)
                    raise error from exc

                raw = response.body
                decoded: Any = None
                if raw:
                    try:
                        decoded = json.loads(raw.decode("utf-8"))
                    except (UnicodeDecodeError, json.JSONDecodeError):
                        decoded = None
                now = _epoch(self.clock)
                cost, remaining, used, reset_at, limit = _rate_values(response.headers, decoded)
                rate_kind, retry_seconds = _rate_limited(response.status, response.headers, decoded, now)
                if rate_kind:
                    self._record_rate_limit(rate_kind, retry_seconds, reset_at, remaining,
                                            used, limit, cost, now, graphql=graphql)
                    self._log_attempt(op, safe_context, attempt, response.status, elapsed_ms, cost,
                                      remaining, reset_at, rate_kind, retry_seconds, graphql=graphql,
                                      request_fingerprint=request_fingerprint)
                    raise APIError("GitHub API rate limit is active", kind=rate_kind,
                                   status_code=response.status, retry_after_seconds=retry_seconds,
                                   mutation_started=False)

                if 200 <= response.status < 300 and raw and decoded is None:
                    self._log_attempt(op, safe_context, attempt, response.status, elapsed_ms, cost,
                                      remaining, reset_at, "malformed_response", None, graphql=graphql,
                                      request_fingerprint=request_fingerprint)
                    self._finish_probe(recovery_probe, success=False)
                    raise APIError("GitHub API returned malformed JSON", kind="malformed_response",
                                   status_code=response.status)

                if 200 <= response.status < 300 and (isinstance(decoded, (dict, list)) or not raw):
                    if graphql and not isinstance(decoded, dict):
                        raise APIError("GraphQL response is not a JSON object", kind="malformed_response",
                                       status_code=response.status)
                    errors = decoded.get("errors") if graphql else None
                    if graphql and "errors" in decoded and not isinstance(errors, list):
                        self._log_attempt(op, safe_context, attempt, response.status, elapsed_ms, cost,
                                          remaining, reset_at, "malformed_response", None, graphql=True,
                                          request_fingerprint=request_fingerprint)
                        self._finish_probe(recovery_probe, success=False)
                        raise APIError("GitHub GraphQL errors field is not an array",
                                       kind="malformed_response", status_code=response.status,
                                       uncertain=not is_query, mutation_started=not is_query)
                    if isinstance(errors, list) and errors:
                        codes = []
                        for item in errors:
                            if not isinstance(item, dict):
                                continue
                            extensions = item.get("extensions")
                            extensions = extensions if isinstance(extensions, dict) else {}
                            codes.append(str(extensions.get("code") or item.get("type") or ""))
                        self._log_attempt(op, safe_context, attempt, response.status, elapsed_ms, cost,
                                          remaining, reset_at, "graphql_error", None, graphql=True,
                                          request_fingerprint=request_fingerprint)
                        self._finish_probe(recovery_probe, success=remaining != 0)
                        partial_mutation = not is_query
                        raise APIError("GitHub GraphQL returned errors: " + ",".join(codes[:3]),
                                       kind="graphql_error", status_code=response.status,
                                       uncertain=partial_mutation,
                                       mutation_started=partial_mutation,
                                       details={"error_codes": codes[:3],
                                                "partial_data": isinstance(decoded.get("data"), dict)})
                    if graphql:
                        self._confirm_account_identity(decoded.get("data"))
                    self._record_success(remaining, used, limit, reset_at, cost, now,
                                         recovery_probe=recovery_probe, graphql=graphql)
                    self._log_attempt(op, safe_context, attempt, response.status, elapsed_ms, cost,
                                      remaining, reset_at, None, None, graphql=graphql,
                                      request_fingerprint=request_fingerprint)
                    return decoded

                kind = "permission_denied" if response.status in {401, 403} else "http_error"
                retryable = is_query and response.status in {500, 502, 503, 504} and attempt < attempts
                uncertain = not is_query and response.status >= 500
                self._log_attempt(op, safe_context, attempt, response.status, elapsed_ms, cost,
                                  remaining, reset_at, kind, None, graphql=graphql,
                                  request_fingerprint=request_fingerprint)
                if retryable:
                    self.sleeper(min(2.0, 0.2 * (2 ** (attempt - 1))) + self.random_value() * 0.05)
                    continue
                self._record_success(remaining, used, limit, reset_at, cost, now,
                                     recovery_probe=recovery_probe, response_ok=False, graphql=graphql)
                raise APIError(f"GitHub API returned HTTP {response.status}", kind=kind,
                               status_code=response.status, uncertain=uncertain,
                               mutation_started=not is_query)
            raise APIError("GitHub API query attempts exhausted", kind="transport_error")
        except APIError:
            if recovery_probe:
                self._finish_probe(recovery_probe, success=False)
            raise

    def _budget_path(self) -> pathlib.Path:
        return self.state_path("budget", self._rate_state_key)

    def _budget_lock_key(self) -> str:
        return f"budget:{self._account_scope_state_key or self._rate_state_key}"

    def _account_identity_map_key(self) -> str:
        return f"credential-account:{self.credential_scope_digest}"

    def _restore_account_scope_identity(self) -> None:
        if self.state_root is None:
            return
        mapping = self.read_state("budget", self._account_identity_map_key(), default=None, strict=False)
        key = mapping.get("account_scope_state_key") if isinstance(mapping, dict) else None
        if isinstance(key, str) and re.fullmatch(r"account:[0-9a-f]{64}", key):
            self._account_scope_state_key = key

    def _confirm_account_identity(self, data: Any) -> None:
        viewer = data.get("viewer") if isinstance(data, dict) else None
        account_id = viewer.get("id") if isinstance(viewer, dict) else None
        if not isinstance(account_id, str) or not account_id or len(account_id) > 512:
            return
        if any(ord(character) < 0x20 for character in account_id):
            return
        key = "account:" + hashlib.sha256(account_id.encode("utf-8")).hexdigest()
        self._account_scope_state_key = key
        self.write_state("budget", self._account_identity_map_key(),
                         {"account_scope_state_key": key}, critical=False)

    def _load_budget_state(self, *, strict: bool = False) -> dict[str, Any] | None:
        state = _PROCESS_PAUSES.get(self._rate_state_key)
        if state is not None:
            current = self.read_state("budget", self._rate_state_key, default=None, strict=strict)
            if current is not None:
                state = current
                _PROCESS_PAUSES[self._rate_state_key] = current
        else:
            state = self.read_state("budget", self._rate_state_key, default=None, strict=strict)
        if isinstance(state, dict):
            _PROCESS_PAUSES[self._rate_state_key] = state
        if self._account_scope_state_key is None:
            return dict(state) if isinstance(state, dict) else None
        account_cache_key = self._account_scope_state_key
        account_state = _PROCESS_PAUSES.get(account_cache_key)
        try:
            current_account = self.read_state("budget", self._account_scope_state_key,
                                              default=None, strict=strict)
        except APIError:
            raise
        if current_account is not None:
            account_state = current_account
            _PROCESS_PAUSES[account_cache_key] = current_account
        own = state if isinstance(state, dict) else None
        shared = account_state if isinstance(account_state, dict) else None
        if own is None:
            chosen = shared
        elif shared is None:
            chosen = own
        else:
            now = _epoch(self.clock)
            own_pause = max(float(own.get("pause_until_epoch") or 0), float(own.get("probe_until_epoch") or 0))
            shared_pause = max(float(shared.get("pause_until_epoch") or 0),
                               float(shared.get("probe_until_epoch") or 0))
            if shared_pause > max(now, own_pause):
                chosen = shared
            elif own_pause > max(now, shared_pause):
                chosen = own
            else:
                chosen = shared if float(shared.get("observed_at_epoch") or 0) >= float(own.get("observed_at_epoch") or 0) else own
        return dict(chosen) if isinstance(chosen, dict) else None

    def _before_request(self, *, is_query: bool, graphql: bool) -> bool:
        state = self._load_budget_state(strict=True)
        if not state:
            return False
        now = _epoch(self.clock)
        pause_until = float(state.get("pause_until_epoch") or 0)
        if pause_until > now:
            raise self._wait_error(state, now)
        remaining = state.get("remaining")
        if isinstance(remaining, int) and remaining <= 0:
            derived_until = _zero_budget_pause_until(state, now)
            if derived_until > now:
                waiting_state = dict(state)
                waiting_state.update({"pause_until_epoch": derived_until,
                                      "pause_reason": "primary_rate_limit",
                                      "pause_resetAt": state.get("resetAt")})
                raise self._wait_error(waiting_state, now)
        if not pause_until and not (isinstance(remaining, int) and remaining <= 0):
            return False
        if not is_query or not graphql:
            raise APIError("a GraphQL read-only recovery probe is required before resuming API traffic",
                           kind="rate_limit_probe_pending", retry_after_seconds=0,
                           mutation_started=False)
        try:
            with self.file_lock(self._budget_lock_key(), timeout=2.0):
                state = self._load_budget_state(strict=True) or state
                now = _epoch(self.clock)
                pause_until = float(state.get("pause_until_epoch") or 0)
                if pause_until > now:
                    raise self._wait_error(state, now)
                probe_until = float(state.get("probe_until_epoch") or 0)
                if probe_until > now:
                    raise APIError("another process owns the bounded rate-limit probe",
                                   kind="rate_limit_probe_pending",
                                   retry_after_seconds=max(1, int(probe_until - now + 0.999)))
                state["probe_until_epoch"] = now + 30
                state["probe_until"] = int(now + 30)
                self._write_budget_locked(state)
                return True
        except TimeoutError as exc:
            raise APIError("another process is checking the shared rate-limit window",
                           kind="rate_limit_probe_pending", retry_after_seconds=2) from exc

    def _record_rate_limit(self, kind: str, retry_seconds: int | None, reset_at: str | None,
                           remaining: int | None, used: int | None, limit: int | None,
                           cost: int | None, now: float, *, graphql: bool) -> None:
        retry = max(60, int(retry_seconds or 0)) if kind == "secondary_rate_limit" else int(retry_seconds or 0)
        if kind == "primary_rate_limit" and not retry:
            retry = 60
        state = self._load_budget_state(strict=False) or {}
        state.update({
            "schema": "oasis7.github-api-budget/v1",
            "observed_at": _now_utc(),
            "observed_at_epoch": now,
            "pause_reason": kind,
            "pause_until_epoch": now + retry,
            "pause_until": int(now + retry),
            "pause_resetAt": reset_at,
            "probe_until_epoch": 0,
            "probe_until": 0,
        })
        if graphql:
            state.update({"remaining": remaining, "used": used, "limit": limit,
                          "cost": cost, "resetAt": reset_at})
        else:
            state.update({"rest_remaining": remaining, "rest_resetAt": reset_at})
        _PROCESS_PAUSES[self._rate_state_key] = state
        try:
            with self.file_lock(self._budget_lock_key(), timeout=2.0):
                self._write_budget_locked(state)
        except (OSError, TimeoutError, APIError):
            # Keep the current process safe, but never claim cross-process state.
            state["shared_persistence"] = False
            _PROCESS_PAUSES[self._rate_state_key] = state

    def _record_success(self, remaining: int | None, used: int | None, limit: int | None,
                        reset_at: str | None, cost: int | None, now: float, *,
                        recovery_probe: bool, response_ok: bool = True, graphql: bool) -> None:
        state = self._load_budget_state(strict=False) or {}
        if graphql and remaining is not None:
            state.update({"remaining": remaining, "used": used, "limit": limit,
                          "cost": cost, "resetAt": reset_at,
                          "observed_at": _now_utc(), "observed_at_epoch": now})
            if remaining <= 0:
                pause_until = _zero_budget_pause_until(state, now)
                state.update({"pause_reason": "primary_rate_limit",
                              "pause_until_epoch": pause_until,
                              "pause_until": int(pause_until),
                              "pause_resetAt": reset_at,
                              "probe_until_epoch": 0, "probe_until": 0})
        elif recovery_probe and response_ok:
            state.update({"remaining": None, "used": None, "cost": cost,
                          "observed_at": _now_utc(), "observed_at_epoch": now})
        exhausted = graphql and isinstance(remaining, int) and remaining <= 0
        if recovery_probe and response_ok and not exhausted:
            state.update({"pause_until_epoch": 0, "pause_until": 0, "pause_reason": None,
                          "probe_until_epoch": 0, "probe_until": 0})
        elif recovery_probe:
            state["probe_until_epoch"] = now + 10
            state["probe_until"] = int(now + 10)
        if not state:
            return
        _PROCESS_PAUSES[self._rate_state_key] = state
        try:
            with self.file_lock(self._budget_lock_key(), timeout=2.0):
                self._write_budget_locked(state)
        except (OSError, TimeoutError, APIError):
            state["shared_persistence"] = False
            _PROCESS_PAUSES[self._rate_state_key] = state

    def _write_budget_locked(self, state: dict[str, Any]) -> None:
        _atomic_json(self._budget_path(), state)
        _PROCESS_PAUSES[self._rate_state_key] = state
        if self._account_scope_state_key is not None:
            _atomic_json(self.state_path("budget", self._account_scope_state_key), state)
            _PROCESS_PAUSES[self._account_scope_state_key] = state

    def _finish_probe(self, recovery_probe: bool, *, success: bool) -> None:
        if not recovery_probe:
            return
        state = self._load_budget_state(strict=False) or {}
        now = _epoch(self.clock)
        state["probe_until_epoch"] = 0 if success else now + 10
        state["probe_until"] = int(state["probe_until_epoch"])
        _PROCESS_PAUSES[self._rate_state_key] = state
        try:
            with self.file_lock(self._budget_lock_key(), timeout=2.0):
                self._write_budget_locked(state)
        except (OSError, TimeoutError, APIError):
            state["shared_persistence"] = False
            _PROCESS_PAUSES[self._rate_state_key] = state

    def _wait_error(self, state: dict[str, Any], now: float) -> APIError:
        pause_until = float(state.get("pause_until_epoch") or now)
        return APIError("GitHub API is paused by the shared rate-limit window",
                        kind=str(state.get("pause_reason") or "primary_rate_limit"),
                        retry_after_seconds=max(0, int(pause_until - now + 0.999)),
                        mutation_started=False,
                        details={"resetAt": state.get("pause_resetAt") or state.get("resetAt"),
                                 "shared_persistence": state.get("shared_persistence", True)})

    def _wait_result(self, state: dict[str, Any], now: float) -> dict[str, Any]:
        return self._wait_error(state, now).as_dict()

    def _budget_result(self, state: dict[str, Any], minimum_remaining: int) -> dict[str, Any]:
        remaining = state.get("remaining")
        reset_at = state.get("resetAt")
        if not isinstance(remaining, int):
            return {"status": "capability_blocked", "reason": "graphql_budget_unknown",
                    "resumable": True, "resume": "restore GraphQL rateLimit visibility and rerun"}
        if remaining <= 0:
            now = _epoch(self.clock)
            pause_until = max(float(state.get("pause_until_epoch") or 0),
                              _zero_budget_pause_until(state, now))
            return {"status": "external_wait", "reason": "graphql_budget_exhausted",
                    "retry_after_seconds": max(0, int(pause_until - now + 0.999)),
                    "remaining": remaining, "resetAt": reset_at,
                    "ready_for_merge": False}
        if remaining < minimum_remaining:
            return {"status": "capability_blocked", "reason": "graphql_budget_insufficient",
                    "remaining": remaining, "resetAt": reset_at, "resumable": True,
                    "resume": f"resume after {reset_at}" if reset_at else "refresh rateLimit visibility and rerun"}
        return {"status": "ok", "remaining": remaining, "resetAt": reset_at,
                "source": "shared_response_observation"}

    def _log_attempt(self, operation: str, context: dict[str, str], attempt: int,
                     status_code: int | None, elapsed_ms: int, cost: int | None,
                     remaining: int | None, reset_at: str | None, error_kind: str | None,
                     retry_after: int | None, *, request_sent: bool = True,
                     cache_hit: bool = False, graphql: bool = True,
                     request_fingerprint: str | None = None) -> None:
        event = {
            "timestamp": _now_utc(), "timestamp_epoch": _epoch(self.clock),
            "pid": os.getpid(), "script": context.get("script") or pathlib.Path(sys.argv[0]).name,
            "operation": context.get("operation") or operation,
            "task_uid": context.get("task_uid"), "pr_number": context.get("pr_number"),
            "operation_id": context.get("operation_id"),
            "parent_operation_id": context.get("parent_operation_id"),
            "phase": context.get("phase"),
            "api_family": context.get("api_family") or ("graphql" if graphql else "rest"),
            "operation_name": context.get("operation_name") or operation,
            "credential_scope_digest": self.credential_scope_digest,
            "request_fingerprint": request_fingerprint, "attempt": attempt, "status_code": status_code,
            "elapsed_ms": elapsed_ms, "graphql": graphql, "graphql_cost": cost if graphql else None,
            "rate_limit_remaining": remaining, "rate_limit_reset": reset_at,
            "cache_hit": cache_hit, "request_sent": request_sent, "error_kind": error_kind,
            "retry_after_seconds": retry_after,
        }
        self._append_telemetry(event)

    def _append_telemetry(self, event: dict[str, Any]) -> None:
        try:
            directory = self.telemetry_dir
            if directory is None:
                raise OSError("shared telemetry path is unavailable")
            _private_dir(directory)
            path = directory / f"{datetime.now(timezone.utc):%Y-%m-%d}-{os.getpid()}.jsonl"
            encoded = (json.dumps(event, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")
            descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
            try:
                os.write(descriptor, encoded)
            finally:
                os.close(descriptor)
            self._prune_telemetry(directory)
        except OSError:
            self._warn_log_failure()

    def log_cache_event(self, operation: str, *, context: dict[str, Any] | None = None,
                        cache_hit: bool, error_kind: str | None = None) -> None:
        safe = _safe_context(context)
        self._log_attempt(_safe_operation(operation), safe, 0, None, 0, None,
                          None, None, error_kind, None, request_sent=False, cache_hit=cache_hit)

    def _last_telemetry_path(self) -> pathlib.Path | None:
        directory = self.telemetry_dir
        if directory is None:
            return None
        return directory / f"{datetime.now(timezone.utc):%Y-%m-%d}-{os.getpid()}.jsonl"

    def _prune_telemetry(self, directory: pathlib.Path) -> None:
        now = _epoch(self.clock)
        files = sorted(directory.glob("*.jsonl"), key=lambda p: p.stat().st_mtime)
        total = sum(path.stat().st_size for path in files)
        for path in files:
            age = now - path.stat().st_mtime
            if age > TELEMETRY_RETENTION_DAYS * 86400 or total > TELEMETRY_MAX_BYTES:
                size = path.stat().st_size
                try:
                    path.unlink()
                    total -= size
                except OSError:
                    pass

    @staticmethod
    def _warn_log_failure() -> None:
        print("github-api: telemetry write failed; request remains counted only in process", file=sys.stderr)


def _rate_values(headers: dict[str, str], payload: Any):
    remaining = _int_header(headers, "x-ratelimit-remaining")
    used = _int_header(headers, "x-ratelimit-used")
    limit = _int_header(headers, "x-ratelimit-limit")
    cost = None
    reset_epoch = _int_header(headers, "x-ratelimit-reset")
    reset_at = datetime.fromtimestamp(reset_epoch, timezone.utc).isoformat().replace("+00:00", "Z") if reset_epoch else None
    if isinstance(payload, dict):
        rate = ((payload.get("data") or {}).get("rateLimit") or {})
        if not isinstance(rate, dict):
            rate = {}
        for name, target in (("remaining", "remaining"), ("used", "used"), ("limit", "limit"), ("cost", "cost")):
            value = rate.get(name)
            if isinstance(value, int) and value >= 0:
                if target == "remaining": remaining = value
                elif target == "used": used = value
                elif target == "limit": limit = value
                else: cost = value
        if isinstance(rate.get("resetAt"), str) and rate["resetAt"]:
            reset_at = rate["resetAt"]
        ext_cost = ((payload.get("extensions") or {}).get("cost") or {})
        if cost is None and isinstance(ext_cost, dict):
            value = ext_cost.get("actualQueryCost", ext_cost.get("requestedQueryCost"))
            if isinstance(value, int) and value >= 0:
                cost = value
    return cost, remaining, used, reset_at, limit


def _rate_limited(status: int, headers: dict[str, str], payload: Any, now: float):
    message = ""
    codes: list[str] = []
    if isinstance(payload, dict):
        message += " " + str(payload.get("message") or "")
        errors = payload.get("errors") or []
        for item in errors:
            if isinstance(item, dict):
                message += " " + str(item.get("message") or "")
                extension = item.get("extensions") or {}
                if isinstance(extension, dict):
                    codes.append(str(extension.get("code") or extension.get("type") or "").upper())
                codes.append(str(item.get("type") or "").upper())
    message = message.lower()
    remaining = _int_header(headers, "x-ratelimit-remaining")
    if isinstance(payload, dict):
        rate = ((payload.get("data") or {}).get("rateLimit") or {})
        if isinstance(rate, dict) and isinstance(rate.get("remaining"), int):
            remaining = rate["remaining"]
    retry = _retry_after(headers, now)
    reset = _int_header(headers, "x-ratelimit-reset")
    reset_text = None
    if reset is None and isinstance(payload, dict):
        rate = ((payload.get("data") or {}).get("rateLimit") or {})
        reset_text = rate.get("resetAt") if isinstance(rate, dict) else None
        if isinstance(reset_text, str):
            try:
                reset = int(datetime.fromisoformat(reset_text.replace("Z", "+00:00")).timestamp())
            except ValueError:
                reset = None
    if reset is not None:
        reset_delay = max(0, reset - int(now))
        retry = max(retry or 0, reset_delay + 2)
    elif isinstance(reset_text, str):
        try:
            reset_epoch = datetime.fromisoformat(reset_text.replace("Z", "+00:00")).timestamp()
            retry = max(retry or 0, max(0, int(reset_epoch - now)) + 2)
        except ValueError:
            pass
    has_errors = isinstance(payload, dict) and bool(payload.get("errors"))
    primary = ((remaining == 0 and (status in {403, 429} or has_errors))
               or any("RATE_LIMITED" in code or "PRIMARY_RATE_LIMIT" in code for code in codes)
               or "primary rate limit" in message or "api rate limit exceeded" in message)
    secondary = (status == 429 or "secondary rate limit" in message or "abuse detection" in message
                 or ("rate limit" in message and status == 403))
    if primary:
        return "primary_rate_limit", retry
    if secondary and (status in {403, 429} or "rate limit" in message or "abuse detection" in message):
        return "secondary_rate_limit", max(60, retry or 60)
    return None, None


def _zero_budget_pause_until(state: dict[str, Any], now: float) -> float:
    """Return the known reset deadline or a short bounded recovery cooldown."""
    explicit_pause = float(state.get("pause_until_epoch") or 0)
    if explicit_pause > now:
        return explicit_pause
    reset_at = state.get("resetAt")
    if isinstance(reset_at, str):
        try:
            reset_epoch = datetime.fromisoformat(reset_at.replace("Z", "+00:00")).timestamp()
            if reset_epoch > now:
                return reset_epoch + 2
        except ValueError:
            pass
    observed_at = float(state.get("observed_at_epoch") or 0)
    return max(explicit_pause, observed_at + 60)


def _atomic_json(path: pathlib.Path, value: dict[str, Any]) -> None:
    _private_dir(path.parent)
    data = (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode("utf-8")
    descriptor, temp_name = tempfile.mkstemp(prefix=f".{path.name}.", suffix=".tmp", dir=path.parent)
    try:
        if os.name != "nt":
            os.fchmod(descriptor, 0o600)
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp_name, path)
        if os.name != "nt":
            os.chmod(path, 0o600)
    except BaseException:
        try:
            os.unlink(temp_name)
        except OSError:
            pass
        raise


def _private_dir(path: pathlib.Path) -> None:
    path.mkdir(parents=True, exist_ok=True, mode=0o700)
    if os.name != "nt":
        os.chmod(path, 0o700)


def _budget_from_payload(payload: dict[str, Any]) -> dict[str, Any] | None:
    data = payload.get("data") if isinstance(payload, dict) else None
    rate = data.get("rateLimit") if isinstance(data, dict) else None
    return rate if isinstance(rate, dict) else None


def _budget_guard_cli(client: GitHubAPIClient, threshold: int, max_age: int) -> dict[str, Any]:
    return client.rate_limit_guard(minimum_remaining=threshold, max_age_seconds=max_age,
                                   operation="local_budget_guard",
                                   context={"script": "github_api.py", "operation": "local_budget_guard"})


def _parse_since(value: str) -> float:
    match = re.fullmatch(r"([1-9][0-9]*)([smhd])", value)
    if not match:
        raise argparse.ArgumentTypeError("--since must be a positive duration such as 30m, 1h, or 7d")
    number, unit = int(match.group(1)), match.group(2)
    return number * {"s": 1, "m": 60, "h": 3600, "d": 86400}[unit]


def _stats(client: GitHubAPIClient, since_seconds: float) -> dict[str, Any]:
    cutoff = _epoch(client.clock) - since_seconds
    groups: dict[tuple[str, str, str, str], dict[str, Any]] = {}
    directory = client.telemetry_dir
    files = sorted(directory.glob("*.jsonl")) if directory is not None else []
    for path in files:
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except OSError:
            continue
        for line in lines:
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if float(event.get("timestamp_epoch") or 0) < cutoff:
                continue
            key = (str(event.get("script") or "unknown"), str(event.get("operation") or "unknown"),
                   str(event.get("task_uid") or ""), str(event.get("pr_number") or ""))
            row = groups.setdefault(key, {"script": key[0], "operation": key[1],
                                          "task_uid": key[2] or None, "pr_number": key[3] or None,
                                          "requests": 0, "known_cost": 0, "unknown_cost": 0,
                                          "cache_hits": 0, "rate_limits": 0, "retries": 0})
            if event.get("cache_hit") is True:
                row["cache_hits"] += 1
            if event.get("request_sent") is False:
                continue
            row["requests"] += 1
            if event.get("graphql") is True:
                if isinstance(event.get("graphql_cost"), int): row["known_cost"] += event["graphql_cost"]
                else: row["unknown_cost"] += 1
            if event.get("error_kind") in {"primary_rate_limit", "secondary_rate_limit"}: row["rate_limits"] += 1
            if int(event.get("attempt") or 1) > 1: row["retries"] += 1
    return {"coverage": "instrumented_paths_only", "window_seconds": int(since_seconds),
            "groups": sorted(groups.values(), key=lambda row: (row["script"], row["operation"],
                                                                  row["task_uid"] or "", row["pr_number"] or ""))}


def _status(client: GitHubAPIClient) -> dict[str, Any]:
    state = client._load_budget_state(strict=False)
    if not state:
        return {"status": "unknown", "shared_state": client.state_root is not None,
                "coverage": "instrumented_paths_only"}
    now = _epoch(client.clock)
    pause_until = float(state.get("pause_until_epoch") or 0)
    return {"status": "external_wait" if pause_until > now else "observed",
            "reason": state.get("pause_reason"), "retry_after_seconds": max(0, int(pause_until - now + 0.999)),
            "remaining": state.get("remaining"), "resetAt": state.get("resetAt"),
            "observed_at": state.get("observed_at"),
            "shared_persistence": state.get("shared_persistence", True),
            "coverage": "instrumented_paths_only"}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    stats = sub.add_parser("stats")
    stats.add_argument("--since", type=_parse_since, default=_parse_since("1h"))
    stats.add_argument("--json", action="store_true")
    status = sub.add_parser("status")
    status.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    client = GitHubAPIClient()
    result = _stats(client, args.since) if args.command == "stats" else _status(client)
    print(json.dumps(result, sort_keys=True, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
