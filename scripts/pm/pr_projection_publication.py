#!/usr/bin/env python3
"""Ordered, recoverable CI projection publication; reuse remains disabled."""
from __future__ import annotations

import hashlib
import inspect
import json
import re
import time
from typing import Any, Callable

from pr_projection_journal import JournalError, PublicationJournal
from projection_publication_contract import (
    CI_PUBLICATION_IDENTITY_FIELDS, ContractError, build_contract, digest,
    decode_marker, encode_marker, validate_ci_publication, validate_contract,
)

PUBLICATION_BINDING_SCHEMA = "oasis7-ci-publication-binding/v1"
_BODY_MARKER = "<!-- oasis7-ci-impact-publication:v2 -->"
_BINDING_MARKER = "<!-- oasis7-ci-publication-binding/v1 -->"
_SHA1_OID_RE = re.compile(r"[0-9a-f]{40}\Z")


class PublicationError(RuntimeError):
    def __init__(self, code: str, detail: str):
        super().__init__(f"{code}: {detail}")
        self.code, self.detail = code, detail


def _is_full_sha1_oid(value: Any) -> bool:
    """Whether value is a complete lowercase Git SHA-1 object ID."""
    return isinstance(value, str) and _SHA1_OID_RE.fullmatch(value) is not None


def prepare(*, task_uid: str, source_head_oid: str, scope_base_oid: str,
            projection_digest: str, clauses: list[str] | None = None,
            revision: int = 3) -> tuple[dict[str, Any], str]:
    value = build_contract(task_uid=task_uid, source_head_oid=source_head_oid,
                           scope_base_oid=scope_base_oid, projection_digest=projection_digest,
                           clauses=clauses, revision=revision)
    return value, encode_marker(value)


def build_task_publication(**identity: Any) -> dict[str, Any]:
    fields = (
        "repository", "repository_id", "task_uid", "bootstrap_epoch",
        "source_repository_id", "source_ref", "target_ref", "source_head_oid",
        "source_scope_oid", "planner_authority_oid", "planner_config_sha256",
        "policy_digest", "projection_digest",
    )
    if set(identity) != set(fields):
        raise ContractError("Task publication identity fields do not match contract")
    value = {"schema": "oasis7-ci-publication/v1", **identity, "projection_required": True}
    value["publication_id"] = digest({key: value[key] for key in CI_PUBLICATION_IDENTITY_FIELDS})
    return validate_ci_publication(value)


def publication_comment(publication: dict[str, Any]) -> str:
    value = validate_ci_publication(publication)
    return "<!-- oasis7-ci-publication/v1 -->\n" + json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
    )


def parse_publication_comment(body: str) -> dict[str, Any]:
    marker = "<!-- oasis7-ci-publication/v1 -->"
    if not isinstance(body, str) or body.count(marker) != 1 or not body.startswith(marker + "\n"):
        raise ContractError("CI publication marker must occur exactly once")
    payload = body.split(marker, 1)[1].strip()
    decoder = json.JSONDecoder(object_pairs_hook=_unique_object)
    try:
        value, end = decoder.raw_decode(payload)
    except (json.JSONDecodeError, ContractError) as exc:
        raise ContractError("CI publication comment is invalid") from exc
    if payload[end:].strip():
        raise ContractError("CI publication comment has trailing content")
    return validate_ci_publication(value)


def reject_competing_current_publications(
    current: dict[str, Any], publications: list[dict[str, Any]],
) -> None:
    """Require a single publication identity for the current task/head/scope.

    Callers supply publications parsed from their fresh authoritative comment
    read, after binding the selected current intent. Historical other-head
    publications remain eligible for the separate dual-lineage route.
    """
    fields = ("repository", "task_uid", "source_head_oid", "source_scope_oid")
    if any(
        publication["publication_id"] != current["publication_id"]
        and all(publication[field] == current[field] for field in fields)
        for publication in publications
    ):
        raise ValueError("competing same-task/head/scope publication blocks recovery")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ContractError("duplicate CI publication field")
        result[key] = value
    return result


def build_publication_binding(publication: dict[str, Any], pr_number: int,
                              pr_url: str) -> dict[str, Any]:
    value = validate_ci_publication(publication)
    if type(pr_number) is not int or pr_number < 1:
        raise ContractError("reciprocal PR number must be positive")
    if pr_url != f"https://github.com/{value['repository']}/pull/{pr_number}":
        raise ContractError("reciprocal PR URL does not match repository and number")
    binding = {
        "schema": PUBLICATION_BINDING_SCHEMA, "repository": value["repository"],
        "task_uid": value["task_uid"], "publication_id": value["publication_id"],
        "pr_number": pr_number, "pr_url": pr_url,
    }
    binding["binding_digest"] = digest(binding)
    return binding


def pr_number_from_url(pr_url: Any, repository: str) -> int:
    """Parse one canonical GitHub PR URL bound to the expected repository."""
    if (not isinstance(repository, str)
            or re.fullmatch(r"[^/\s]+/[^/\s]+", repository) is None):
        raise ContractError("expected PR repository identity is malformed")
    if not isinstance(pr_url, str):
        raise ContractError("PR URL is malformed")
    match = re.fullmatch(
        r"https://github\.com/([^/\s]+)/([^/\s]+)/pull/([1-9][0-9]*)",
        pr_url,
    )
    if match is None:
        raise ContractError("PR URL is malformed")
    if f"{match.group(1)}/{match.group(2)}" != repository:
        raise ContractError("PR URL repository mismatch")
    return int(match.group(3))


def validate_publication_binding(value: Any, publication: dict[str, Any] | None = None) -> dict[str, Any]:
    required = {"schema", "repository", "task_uid", "publication_id", "pr_number", "pr_url", "binding_digest"}
    if not isinstance(value, dict) or set(value) != required or value.get("schema") != PUBLICATION_BINDING_SCHEMA:
        raise ContractError("publication binding fields or version are unsupported")
    if (type(value["pr_number"]) is not int or value["pr_number"] < 1
            or not isinstance(value["repository"], str) or not isinstance(value["task_uid"], str)
            or not isinstance(value["publication_id"], str) or not isinstance(value["pr_url"], str)):
        raise ContractError("publication binding identity is invalid")
    body = {key: value[key] for key in required if key != "binding_digest"}
    if value["binding_digest"] != digest(body):
        raise ContractError("publication binding digest mismatch")
    if value["pr_url"] != f"https://github.com/{value['repository']}/pull/{value['pr_number']}":
        raise ContractError("publication binding URL identity mismatch")
    if publication is not None:
        publication = validate_ci_publication(publication)
        if (value["repository"], value["task_uid"], value["publication_id"]) != (
                publication["repository"], publication["task_uid"], publication["publication_id"]):
            raise ContractError("publication binding does not match immutable publication")
    return value


def publication_binding_comment(binding: dict[str, Any]) -> str:
    value = validate_publication_binding(binding)
    return _BINDING_MARKER + "\n" + json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
    )


def parse_publication_binding_comment(body: str) -> dict[str, Any]:
    if (not isinstance(body, str) or body.count(_BINDING_MARKER) != 1
            or not body.startswith(_BINDING_MARKER + "\n")):
        raise ContractError("CI publication binding marker must occur exactly once")
    payload = body.split(_BINDING_MARKER, 1)[1].strip()
    decoder = json.JSONDecoder(object_pairs_hook=_unique_object)
    try:
        value, end = decoder.raw_decode(payload)
    except (json.JSONDecodeError, ContractError) as exc:
        raise ContractError("CI publication binding comment is invalid") from exc
    if payload[end:].strip():
        raise ContractError("CI publication binding comment has trailing content")
    return validate_publication_binding(value)


def replace_projection_marker(body: str, marker: str, *,
                              legacy_projection_b64: str | None = None) -> str:
    """Replace one trailing v2 machine block and preserve the manual body prefix."""
    from projection_publication_contract import MAX_BODY_BYTES
    if not isinstance(body, str) or not isinstance(marker, str):
        raise PublicationError("EVENT_PROJECTION_INVALID", "body and marker must be text")
    if marker.count(_BODY_MARKER) != 1 or not marker.startswith(_BODY_MARKER + "\n"):
        raise PublicationError("EVENT_PROJECTION_INVALID", "new projection marker is malformed")
    count = body.count(_BODY_MARKER)
    if count > 1:
        raise PublicationError("EVENT_PROJECTION_INVALID", "PR body has duplicate projection markers")
    if "<!-- oasis7-ci-impact-publication:v1 -->" in body:
        raise PublicationError("UNSUPPORTED_RUN_PROTOCOL", "legacy v1 marker cannot be upgraded in place")
    if count == 1:
        try:
            decode_marker(body)
        except ContractError as exc:
            raise PublicationError("EVENT_PROJECTION_INVALID", "existing PR projection marker is malformed") from exc
    prefix = body.split(_BODY_MARKER, 1)[0] if count else body
    legacy_token = "<!-- oasis7-impact-projection-b64:"
    legacy_count = prefix.count(legacy_token)
    if legacy_count > 1:
        raise PublicationError("EVENT_PROJECTION_INVALID", "PR body has duplicate legacy projection markers")
    if legacy_count == 1:
        match = re.search(r"(?m)^<!-- oasis7-impact-projection-b64:\s*([A-Za-z0-9+/=_-]+)\s*-->[ \t]*$", prefix)
        if not match or legacy_projection_b64 is None or match.group(1) != legacy_projection_b64:
            raise PublicationError("EVENT_PROJECTION_INVALID", "legacy projection marker does not match the verified input")
        start, end = match.span()
        prefix = prefix[:start] + prefix[end:]
    prefix = prefix.rstrip()
    result = (prefix + "\n\n" if prefix else "") + marker
    if len(result.encode("utf-8")) > MAX_BODY_BYTES:
        raise PublicationError("EVENT_PROJECTION_INVALID", "PR body exceeds 60KiB")
    return result


def _candidate(publication: dict[str, Any], projection: dict[str, Any]) -> tuple[dict[str, Any], str]:
    try:
        publication = validate_ci_publication(publication)
        if publication.get("pr_number") is not None:
            raise ContractError("immutable publication must not contain a PR number")
        if not isinstance(projection, dict):
            raise ContractError("impact projection must be an object")
        for key, expected in (
            ("task_uid", publication["task_uid"]),
            ("source_head_oid", publication["source_head_oid"]),
            ("scope_base_oid", publication["source_scope_oid"]),
            ("planner_config_sha256", publication["planner_config_sha256"]),
            ("projection_digest", publication["projection_digest"]),
        ):
            if projection.get(key) != expected:
                raise ContractError(f"impact projection {key} differs from Task publication")
        consumed = projection.get("consumed_contracts", [])
        if not isinstance(consumed, list):
            raise ContractError("impact projection consumed_contracts must be a list")
        clauses = [
            item if isinstance(item, str) else json.dumps(
                item, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
            )
            for item in consumed
        ]
        _, marker = prepare(
            task_uid=publication["task_uid"], source_head_oid=publication["source_head_oid"],
            scope_base_oid=publication["source_scope_oid"], projection_digest=publication["projection_digest"],
            clauses=clauses,
        )
        return publication, marker
    except (ContractError, TypeError, ValueError) as exc:
        raise PublicationError("EVENT_PROJECTION_INVALID", str(exc)) from exc


def _prior(journal: PublicationJournal, action_id: str) -> dict[str, Any] | None:
    found = [item for item in journal.read_action_state()["actions"]
             if item.get("action_id") == action_id]
    if len(found) > 1:
        raise JournalError("duplicate publication action")
    return found[0] if found else None


def _task_intent_identity(adapter: Any, journal: PublicationJournal,
                          publication: dict[str, Any], pr_binding: dict[str, Any]
                          ) -> tuple[str, dict[str, Any]]:
    resolve_login = getattr(adapter, "resolve_publisher_login", None)
    if not callable(resolve_login):
        raise PublicationError("NETWORK_UNCERTAIN", "authenticated GitHub login resolver is unavailable")
    try:
        login = resolve_login()
    except Exception as exc:
        raise PublicationError("NETWORK_UNCERTAIN", f"authenticated GitHub login read failed: {exc}") from exc
    if (not isinstance(login, str) or not login.strip()
            or any(ord(character) < 33 for character in login)):
        raise PublicationError("NETWORK_UNCERTAIN", "authenticated GitHub login is unresolved")
    issue_number = getattr(adapter, "issue_number", None)
    if type(issue_number) is not int or issue_number < 1:
        raise PublicationError("TASK_IDENTITY_CONFLICT", "canonical Task Issue number is unavailable")
    if not isinstance(pr_binding, dict):
        raise PublicationError("TASK_IDENTITY_CONFLICT", "exact Task PR binding state is unavailable")
    canonical_payload = publication_comment(publication)
    payload_bytes = canonical_payload.encode("utf-8")
    root = getattr(adapter, "root", None)
    if root is None:
        root = journal.canonical_worktree or journal.common_dir
    try:
        worktree = str(root.resolve())
        common_dir = str(journal.common_dir.resolve())
    except (AttributeError, OSError) as exc:
        raise PublicationError("TASK_IDENTITY_CONFLICT", "canonical worktree identity is unavailable") from exc
    identity = {
        "schema": "oasis7-pr-task-post-action/v1",
        "repository": publication["repository"],
        "repository_id": publication["repository_id"],
        "task_issue_number": issue_number,
        "task_uid": publication["task_uid"],
        "bootstrap_epoch": publication["bootstrap_epoch"],
        "publication": publication,
        "pr_binding": pr_binding,
        "canonical_worktree": worktree,
        "git_common_dir": common_dir,
        "source_ref": publication["source_ref"],
        "target_ref": publication["target_ref"],
        "source_head_oid": publication["source_head_oid"],
        "scope_base_oid": publication["source_scope_oid"],
        "planner_authority_oid": publication["planner_authority_oid"],
        "planner_config_sha256": publication["planner_config_sha256"],
        "policy_digest": publication["policy_digest"],
        "projection_digest": publication["projection_digest"],
        "publisher_login": login,
        "payload_utf8": canonical_payload,
        "payload_sha256": "sha256:" + hashlib.sha256(payload_bytes).hexdigest(),
    }
    return "task-intent:" + publication["publication_id"], identity


def _publication_author(result: dict[str, Any], publication_id: str) -> str | None:
    authors = result.get("publication_authors")
    if not isinstance(authors, list):
        return None
    matches = [item for item in authors
               if isinstance(item, dict) and item.get("publication_id") == publication_id]
    if len(matches) != 1:
        return None
    login = matches[0].get("author_login")
    return login if isinstance(login, str) else None


def _publication_body(result: dict[str, Any], publication_id: str) -> str | None:
    bodies = result.get("publication_bodies")
    if not isinstance(bodies, list):
        return None
    matches = [item for item in bodies
               if isinstance(item, dict) and item.get("publication_id") == publication_id]
    if len(matches) != 1:
        return None
    body = matches[0].get("body")
    return body if isinstance(body, str) else None


def _intent(adapter: Any, journal: PublicationJournal, publication: dict[str, Any], *,
            pr_binding: dict[str, Any], resume_action_id: str | None = None) -> None:
    action = "task-intent:" + publication["publication_id"]
    record_pr_action = "record-pr:" + publication["publication_id"]
    action, identity = _task_intent_identity(adapter, journal, publication, pr_binding)
    selecting_record_pr = resume_action_id == record_pr_action
    if (resume_action_id is not None
            and (not isinstance(resume_action_id, str)
                 or resume_action_id not in {action, record_pr_action})):
        raise PublicationError("TASK_IDENTITY_CONFLICT", "recovery selector does not match the exact publication action")
    if selecting_record_pr:
        record_pr_prior = _prior(journal, record_pr_action)
        expected = record_pr_prior.get("expected") if isinstance(record_pr_prior, dict) else None
        if (not isinstance(record_pr_prior, dict) or record_pr_prior.get("kind") != "record_pr"
                or record_pr_prior.get("state") != "uncertain" or not isinstance(expected, dict)
                or expected.get("publication_id") != publication["publication_id"]
                or expected.get("task_uid") != publication["task_uid"]
                or type(expected.get("pr_number")) is not int or expected["pr_number"] < 1):
            raise PublicationError("TASK_IDENTITY_CONFLICT", "recovery selector is not the exact current uncertain record-pr action")
    prior = _prior(journal, action)
    history_anchored = journal.has_task_post_tail()
    events = journal.read_task_events(action, allow_unanchored=True)
    resolved = any(item["event"] == "RESOLVED" for item in events)
    if selecting_record_pr and not resolved:
        raise PublicationError("TASK_IDENTITY_CONFLICT", "record-pr recovery requires the exact Task intent to be resolved")
    if events:
        original_identity = events[0]["identity"]
        if resolved:
            current_without_binding = {key: value for key, value in identity.items()
                                      if key != "pr_binding"}
            original_without_binding = {key: value for key, value in original_identity.items()
                                        if key != "pr_binding"}
            if current_without_binding != original_without_binding:
                raise JournalError("resolved Task publication action identity changed")
            if history_anchored:
                # Later publisher steps may bind the Task to the PR after this
                # comment action completed. Preserve its original unbound/bound
                # snapshot; a resolved action can never issue another POST.
                return
        elif original_identity != identity:
            raise JournalError("Task publication action identity changed during recovery")
    prior_event_ids = {
        item["action_id"] for item in journal.read_task_events(allow_unanchored=True)
    }
    if resume_action_id is not None and action not in prior_event_ids and prior is None:
        raise PublicationError("TASK_IDENTITY_CONFLICT", "recovery selector has no persisted publication action")
    attempted = any(item["event"] == "POST_ATTEMPTED" for item in events)
    if (events or prior is not None) and resume_action_id is None:
        raise PublicationError(
            "NETWORK_UNCERTAIN",
            f"pending Task publication requires explicit --resume-action-id {action}",
        )

    def append(event: str, **evidence: Any) -> None:
        journal.append_task_event(action, event, identity, evidence)

    def fail_read(phase: str, detail: str) -> None:
        if history_anchored:
            append("READ_FAILED", phase=phase, detail=detail)

    def read_publications(phase: str) -> tuple[list[dict[str, Any]], dict[str, Any]]:
        nonlocal history_anchored
        try:
            result = adapter.find_task_publications(publication["publication_id"])
        except Exception as exc:
            fail_read(phase, type(exc).__name__)
            raise PublicationError("NETWORK_UNCERTAIN", f"Task publication readback failed: {exc}") from exc
        if not isinstance(result, dict) or result.get("complete") is not True:
            fail_read(phase, "incomplete")
            raise PublicationError("NETWORK_UNCERTAIN", "Task publication lookup incomplete")
        records = result.get("publications")
        if not isinstance(records, list):
            fail_read(phase, "malformed")
            raise PublicationError("NETWORK_UNCERTAIN", "Task publication lookup is malformed")
        if len(records) > 1:
            if history_anchored:
                append("READ_CONFLICT", phase=phase, count=len(records))
            raise PublicationError("PUBLICATION_WRITE_CONFLICT", "Task publication lookup is ambiguous")
        if not records:
            if history_anchored:
                append("READ_EMPTY", phase=phase)
            return [], result
        try:
            current = validate_ci_publication(records[0])
        except ContractError as exc:
            if history_anchored:
                append("READ_CONFLICT", phase=phase, detail="invalid-publication")
            raise PublicationError("TASK_IDENTITY_CONFLICT", "Task publication is invalid") from exc
        if current != publication:
            if history_anchored:
                append("READ_CONFLICT", phase=phase, detail="identity-mismatch")
            raise PublicationError("TASK_IDENTITY_CONFLICT", "Task publication identity/content differs")
        if (_publication_body(result, publication["publication_id"]) != identity["payload_utf8"]
                or _publication_author(result, publication["publication_id"]) != identity["publisher_login"]):
            if history_anchored:
                append("READ_CONFLICT", phase=phase, detail="author-or-content-mismatch")
            raise PublicationError("TASK_IDENTITY_CONFLICT", "Task publication author or canonical content differs")
        if not history_anchored:
            journal.anchor_task_history_after_exact_readback()
            history_anchored = True
        if resolved:
            return records, result
        append("READ_MATCH", phase=phase,
               publication_id=publication["publication_id"],
               author_login=identity["publisher_login"],
               payload_sha256=identity["payload_sha256"])
        append("RESOLVED", publication_id=publication["publication_id"])
        return records, result

    # A prior v1 action has no durable boundary separating read from POST.
    # Exact remote readback may reconcile a publication that is actually
    # visible; an empty lookup can never authorize a new POST.
    legacy_history = prior is not None
    try:
        records, _result = read_publications("prewrite")
    except PublicationError:
        raise
    if records:
        return
    if not history_anchored:
        raise PublicationError(
            "NETWORK_UNCERTAIN",
            "Task publication event history lacks a durable root tail anchor; UNKNOWN and pending",
        )
    if legacy_history:
        raise PublicationError("NETWORK_UNCERTAIN", "legacy Task publication history is UNKNOWN; empty lookup cannot authorize POST")
    if attempted:
        raise PublicationError("NETWORK_UNCERTAIN", "Task POST was attempted; empty readback remains pending")

    # Repeating WRITE_INTENT after a prior pre-attempt crash is safe only
    # because the just-appended complete-empty read is the latest evidence.
    append("WRITE_INTENT", payload_sha256=identity["payload_sha256"])
    append("POST_ATTEMPTED", payload_sha256=identity["payload_sha256"])
    confirmed = journal.read_task_events(action, identity=identity)
    if not confirmed or confirmed[-1].get("event") != "POST_ATTEMPTED":
        raise PublicationError("NETWORK_UNCERTAIN", "durable Task POST attempt marker could not be verified")
    try:
        adapter.publish_task_intent(publication)
    except Exception as exc:
        append("POST_RESPONSE_UNCERTAIN", detail=type(exc).__name__)
        raise PublicationError("NETWORK_UNCERTAIN", f"Task intent response uncertain: {exc}") from exc
    records, _result = read_publications("postattempt")
    if len(records) != 1:
        raise PublicationError("NETWORK_UNCERTAIN", "Task intent lacks exact unique readback")


def _push(adapter: Any, journal: PublicationJournal, publication: dict[str, Any],
          lease_oid: str | None) -> None:
    action = "push:" + publication["publication_id"]
    journal.intent(action, "push_source_ref", {
        "source_ref": publication["source_ref"], "new_oid": publication["source_head_oid"],
        "lease_oid": lease_oid,
    })
    try:
        current = adapter.read_source_ref(publication["source_ref"])
    except Exception as exc:
        journal.uncertain(action, "NETWORK_UNCERTAIN")
        raise PublicationError("NETWORK_UNCERTAIN", f"remote source ref readback failed: {exc}") from exc
    if current == publication["source_head_oid"]:
        journal.observe(action, {"oid": current}, phase="HEAD_CONFIRMED")
        return
    if current != lease_oid:
        journal.disposition("SUPERSEDED" if current else "CONFLICT")
        raise PublicationError("SOURCE_SUPERSEDED", f"remote source ref changed to {current!r}")
    # Repeating a push is safe only while the exact lease OID still matches.
    for attempt in range(2):
        try:
            adapter.push_source_ref(publication["source_ref"], publication["source_head_oid"], lease_oid)
        except PublicationError:
            raise
        except Exception:
            pass
        try:
            current = adapter.read_source_ref(publication["source_ref"])
        except Exception as exc:
            journal.uncertain(action, "NETWORK_UNCERTAIN")
            raise PublicationError("NETWORK_UNCERTAIN", f"remote source ref readback failed: {exc}") from exc
        if current == publication["source_head_oid"]:
            journal.observe(action, {"oid": current}, phase="HEAD_CONFIRMED")
            return
        if current != lease_oid:
            journal.disposition("SUPERSEDED" if current else "CONFLICT")
            raise PublicationError("SOURCE_SUPERSEDED", f"remote source ref changed to {current!r}")
        if attempt == 1:
            journal.uncertain(action, "NETWORK_UNCERTAIN")
            raise PublicationError("NETWORK_UNCERTAIN", "source push remains at lease OID after bounded retry")


def _pr_matches(pr: dict[str, Any], publication: dict[str, Any], body: str) -> bool:
    return (
        pr.get("repository") == publication["repository"]
        and pr.get("source_ref") == publication["source_ref"]
        and pr.get("target_ref") == publication["target_ref"]
        and pr.get("head_oid") == publication["source_head_oid"]
        and pr.get("body") == body
        and pr.get("state") == "open"
        and pr.get("merged") is False
        and pr.get("draft") is True
        and type(pr.get("number")) is int
        and pr["number"] > 0
    )


def _preflight_task_pr_binding(adapter: Any, publication: dict[str, Any], *,
                              expected_pr_number: int | None = None,
                              reconcile_exact_create: bool = False,
                              body: str | None = None) -> dict[str, Any] | None:
    """Read the canonical Task PR link before any publication mutation."""
    try:
        binding = adapter.read_task_pr_binding(publication["task_uid"])
    except Exception as exc:
        raise PublicationError("NETWORK_UNCERTAIN", f"Task PR binding preflight failed: {exc}") from exc
    if not isinstance(binding, dict) or binding.get("task_uid") != publication["task_uid"]:
        raise PublicationError("TASK_IDENTITY_CONFLICT", "live Task PR binding UID is invalid")
    number = binding.get("pr_number")
    if number is None:
        return None
    if type(number) is not int or number < 1:
        raise PublicationError("TASK_IDENTITY_CONFLICT", "live Task PR binding number is invalid")
    if expected_pr_number is not None:
        if number != expected_pr_number:
            raise PublicationError("TASK_IDENTITY_CONFLICT", "Task is bound to another PR")
        return {"task_uid": publication["task_uid"], "pr_number": number}
    if not reconcile_exact_create:
        raise PublicationError("TASK_IDENTITY_CONFLICT", "Task already has a PR binding")
    try:
        live_pr = adapter.read_pr(publication["repository"], number)
    except Exception as exc:
        raise PublicationError("NETWORK_UNCERTAIN", f"bound PR preflight failed: {exc}") from exc
    if not isinstance(live_pr, dict) or live_pr.get("number") != number:
        raise PublicationError("NETWORK_UNCERTAIN", "bound PR readback is incomplete")
    if not isinstance(body, str) or not _pr_matches(live_pr, publication, body):
        raise PublicationError(
            "TASK_IDENTITY_CONFLICT",
            "Task is bound to a PR that does not match this create candidate",
        )
    return live_pr


def _create_pr(adapter: Any, journal: PublicationJournal,
               publication: dict[str, Any], body: str, *,
               clock: Callable[[], float] = time.monotonic,
               sleep: Callable[[float], None] = time.sleep) -> dict[str, Any]:
    action = "create-pr:" + publication["publication_id"]
    prior = _prior(journal, action)
    journal.intent(action, "create_draft_pr", {
        "repository": publication["repository"], "source_ref": publication["source_ref"],
        "target_ref": publication["target_ref"], "head_oid": publication["source_head_oid"],
        "publication_id": publication["publication_id"],
    })
    try:
        result = _discover_prs(adapter, publication, 5.0)
    except Exception as exc:
        journal.uncertain(action, "NETWORK_UNCERTAIN")
        raise PublicationError("NETWORK_UNCERTAIN", f"PR discovery failed: {exc}") from exc
    if not isinstance(result, dict) or result.get("complete") is not True or not isinstance(result.get("pull_requests"), list):
        journal.uncertain(action, "NETWORK_UNCERTAIN")
        raise PublicationError("NETWORK_UNCERTAIN", "exact PR discovery is incomplete")
    prs = result["pull_requests"]
    if len(prs) > 1:
        journal.disposition("CONFLICT")
        raise PublicationError("PUBLICATION_WRITE_CONFLICT", "multiple PRs match task/source/target")
    if prs:
        if not _pr_matches(prs[0], publication, body):
            journal.disposition("CONFLICT")
            raise PublicationError("PUBLICATION_WRITE_CONFLICT", "existing PR conflicts with frozen candidate")
        journal.observe(action, {"pr_number": prs[0]["number"]}, phase="METADATA_CONFIRMED")
        return prs[0]
    if prior is not None:
        journal.uncertain(action, "NETWORK_UNCERTAIN")
        raise PublicationError("NETWORK_UNCERTAIN", "create intent lacks exact PR readback; refusing duplicate POST")
    try:
        adapter.create_draft_pr(publication["repository"], publication["source_ref"],
                                 publication["target_ref"], body)
    except Exception as exc:
        journal.uncertain(action, "NETWORK_UNCERTAIN")
        # One exact readback may reconcile a lost response. Absence never permits another POST.
        recovered = _wait_created_pr(adapter, publication, body, clock=clock, sleep=sleep)
        if recovered is not None:
            journal.observe(action, {"pr_number": recovered["number"]}, phase="METADATA_CONFIRMED")
            return recovered
        raise PublicationError("NETWORK_UNCERTAIN", "create response lost; refusing duplicate PR POST") from exc
    created = _wait_created_pr(adapter, publication, body, clock=clock, sleep=sleep)
    if created is None:
        journal.uncertain(action, "NETWORK_UNCERTAIN")
        raise PublicationError("NETWORK_UNCERTAIN", "created PR lacks stable exact readback")
    journal.observe(action, {"pr_number": created["number"]}, phase="METADATA_CONFIRMED")
    return created


def _discover_prs(adapter: Any, publication: dict[str, Any], timeout_seconds: float) -> Any:
    method = adapter.find_task_prs
    if "timeout_seconds" in inspect.signature(method).parameters:
        return adapter.find_task_prs(
            publication["task_uid"], publication["source_ref"], publication["target_ref"],
            timeout_seconds=timeout_seconds,
        )
    return adapter.find_task_prs(
        publication["task_uid"], publication["source_ref"], publication["target_ref"],
    )


def _wait_created_pr(adapter: Any, publication: dict[str, Any], body: str, *,
                     clock: Callable[[], float], sleep: Callable[[float], None]) -> dict[str, Any] | None:
    deadline = clock() + 45.0
    for round_index, delay in enumerate((2.0, 5.0, 0.0)):
        pair: list[dict[str, Any] | None] = []
        for _ in range(2):
            remaining = deadline - clock()
            if remaining <= 0:
                break
            try:
                result = _discover_prs(adapter, publication, min(5.0, remaining))
            except Exception:
                pair.append(None)
                continue
            if not isinstance(result, dict) or result.get("complete") is not True:
                pair.append(None)
                continue
            prs = result.get("pull_requests")
            if not isinstance(prs, list):
                pair.append(None)
                continue
            if len(prs) > 1:
                raise PublicationError("PUBLICATION_WRITE_CONFLICT", "multiple PRs match task/source/target")
            if not prs:
                pair.append(None)
                continue
            if not _pr_matches(prs[0], publication, body):
                raise PublicationError("PUBLICATION_WRITE_CONFLICT", "created PR conflicts with frozen candidate")
            pair.append(prs[0])
        if len(pair) == 2 and pair[0] is not None and pair[1] is not None:
            if pair[0].get("number") != pair[1].get("number"):
                raise PublicationError("PUBLICATION_WRITE_CONFLICT", "PR discovery changed between stable reads")
            return pair[0]
        remaining = deadline - clock()
        if round_index < 2 and remaining > 0:
            sleep(min(delay, remaining))
    return None


def _record_and_bind(adapter: Any, journal: PublicationJournal,
                     publication: dict[str, Any], pr: dict[str, Any],
                     expected_draft: bool = True) -> dict[str, Any]:
    number = pr["number"]
    action = "record-pr:" + publication["publication_id"]
    prior = _prior(journal, action)
    if prior is not None:
        # A persisted record-pr action means a prior attempt may have changed
        # only some of the Task projections. Do not let the ordinary helper
        # path infer recovery authority from an Issue poststate or cache.
        # First-time publication keeps the existing all-pre path.
        require_admission = getattr(adapter, "require_record_pr_recovery_admission", None)
        if not callable(require_admission):
            raise PublicationError(
                "NETWORK_UNCERTAIN",
                "recovery admission check is unavailable for a persisted record-pr action",
            )
        try:
            # The canonical publisher owns the handoff and binds it to the
            # exact current Task/action. Preserve the zero-argument adapter
            # shape used by older in-memory adapters without weakening the
            # production command, which accepts both selectors.
            try:
                inspect.signature(require_admission).bind(
                    publication["task_uid"], action,
                )
            except (TypeError, ValueError):
                require_admission()
            else:
                require_admission(publication["task_uid"], action)
        except PublicationError:
            raise
        except Exception as exc:
            raise PublicationError(
                "NETWORK_UNCERTAIN", f"record-pr recovery admission check failed: {exc}"
            ) from exc
    journal.intent(action, "record_pr", {
        "publication_id": publication["publication_id"], "task_uid": publication["task_uid"],
        "pr_number": number,
    })
    try:
        live = adapter.read_task_pr_binding(publication["task_uid"])
    except Exception as exc:
        journal.uncertain(action, "NETWORK_UNCERTAIN")
        raise PublicationError("NETWORK_UNCERTAIN", f"Task PR binding readback failed: {exc}") from exc
    if isinstance(live, dict) and live.get("task_uid") != publication["task_uid"]:
        journal.disposition("CONFLICT")
        raise PublicationError("TASK_IDENTITY_CONFLICT", "live Task PR binding UID mismatch")
    if isinstance(live, dict) and live.get("pr_number") not in (None, number):
        journal.disposition("CONFLICT")
        raise PublicationError("TASK_IDENTITY_CONFLICT", "Task is bound to another PR")
    # The Issue URL is only one projection of record-pr.  Re-run the canonical
    # live transition on every publication retry: an observed local journal
    # action cannot prove that Issue, Project, or mapping state has not drifted.
    try:
        adapter.record_pr(publication["task_uid"], number, publication["publication_id"])
    except Exception as exc:
        journal.uncertain(action, "NETWORK_UNCERTAIN")
        raise PublicationError("NETWORK_UNCERTAIN", f"record-pr transition did not confirm: {exc}") from exc
    try:
        live = adapter.read_task_pr_binding(publication["task_uid"])
    except Exception as exc:
        journal.uncertain(action, "NETWORK_UNCERTAIN")
        raise PublicationError("NETWORK_UNCERTAIN", f"Task PR binding readback failed: {exc}") from exc
    if not isinstance(live, dict) or live.get("task_uid") != publication["task_uid"] or live.get("pr_number") != number:
        journal.uncertain(action, "NETWORK_UNCERTAIN")
        raise PublicationError("NETWORK_UNCERTAIN", "record-pr lacks exact Task readback")
    recorded_pr = adapter.read_pr(publication["repository"], number)
    _check_pr(recorded_pr, publication, number, publication["source_head_oid"], expected_draft)
    if recorded_pr.get("body") != pr.get("body"):
        raise PublicationError("PUBLICATION_WRITE_CONFLICT", "PR body changed during record-pr")
    journal.observe(action, {"pr_number": number}, phase="METADATA_CONFIRMED")

    url = f"https://github.com/{publication['repository']}/pull/{number}"
    binding = build_publication_binding(publication, number, url)
    bind_action = "reciprocal-binding:" + publication["publication_id"]
    prior_binding = _prior(journal, bind_action)
    journal.intent(bind_action, "publish_reciprocal_binding", {
        "publication_id": publication["publication_id"], "pr_number": number,
        "binding_digest": binding["binding_digest"],
    })
    try:
        current = adapter.find_task_publication_bindings(publication["publication_id"])
    except Exception as exc:
        journal.uncertain(bind_action, "NETWORK_UNCERTAIN")
        raise PublicationError("NETWORK_UNCERTAIN", f"reciprocal binding readback failed: {exc}") from exc
    if not isinstance(current, dict) or current.get("complete") is not True:
        journal.uncertain(bind_action, "NETWORK_UNCERTAIN")
        raise PublicationError("NETWORK_UNCERTAIN", "reciprocal binding readback incomplete")
    bindings = current.get("bindings")
    if not isinstance(bindings, list) or len(bindings) > 1:
        journal.disposition("CONFLICT")
        raise PublicationError("PUBLICATION_WRITE_CONFLICT", "reciprocal binding is ambiguous")
    if bindings:
        try:
            observed = validate_publication_binding(bindings[0], publication)
        except ContractError as exc:
            journal.disposition("CONFLICT")
            raise PublicationError("TASK_IDENTITY_CONFLICT", "reciprocal binding is invalid") from exc
        if observed != binding:
            journal.disposition("CONFLICT")
            raise PublicationError("TASK_IDENTITY_CONFLICT", "Task is reciprocally bound to another PR")
    else:
        if prior_binding is not None:
            journal.uncertain(bind_action, "NETWORK_UNCERTAIN")
            raise PublicationError("NETWORK_UNCERTAIN", "reciprocal write may have landed; refusing duplicate comment")
        try:
            adapter.publish_reciprocal_binding(binding)
        except Exception as exc:
            journal.uncertain(bind_action, "NETWORK_UNCERTAIN")
            raise PublicationError("NETWORK_UNCERTAIN", f"reciprocal binding response uncertain: {exc}") from exc
        current = adapter.find_task_publication_bindings(publication["publication_id"])
        bindings = current.get("bindings") if isinstance(current, dict) and current.get("complete") is True else None
        if not isinstance(bindings, list) or len(bindings) != 1:
            journal.uncertain(bind_action, "NETWORK_UNCERTAIN")
            raise PublicationError("NETWORK_UNCERTAIN", "reciprocal binding lacks unique readback")
        observed = validate_publication_binding(bindings[0], publication)
        if observed != binding:
            journal.disposition("CONFLICT")
            raise PublicationError("TASK_IDENTITY_CONFLICT", "reciprocal binding readback differs")
    journal.observe(bind_action, {"binding_digest": binding["binding_digest"]}, phase="METADATA_CONFIRMED")
    return binding


def publish_create(adapter: Any, journal: PublicationJournal, *, publication: dict[str, Any],
                   projection: dict[str, Any], body: str, expected_remote_oid: str | None = None,
                   legacy_projection_b64: str | None = None,
                   resume_action_id: str | None = None,
                   clock: Callable[[], float] = time.monotonic,
                   sleep: Callable[[float], None] = time.sleep) -> dict[str, Any]:
    """Task intent/readback → H push → draft create → record-pr → reciprocal binding."""
    publication, marker = _candidate(publication, projection)
    body = replace_projection_marker(body, marker, legacy_projection_b64=legacy_projection_b64)
    try:
        with journal.locked():
            adapter.reservation_fd = journal.inherited_lock_fd()
            bound_pr = _preflight_task_pr_binding(
                adapter, publication, reconcile_exact_create=True, body=body,
            )
            pr_binding = (
                {"state": "bound", "pr_number": bound_pr["number"]}
                if bound_pr is not None else {"state": "unbound"}
            )
            _intent(adapter, journal, publication, pr_binding=pr_binding,
                    resume_action_id=resume_action_id)
            if bound_pr is not None:
                pr = bound_pr
            else:
                _push(adapter, journal, publication, expected_remote_oid)
                pr = _create_pr(adapter, journal, publication, body, clock=clock, sleep=sleep)
            binding = _record_and_bind(adapter, journal, publication, pr)
            return {"status": "published", "task_uid": publication["task_uid"],
                    "publication_id": publication["publication_id"], "pr_number": pr["number"],
                    "head_oid": publication["source_head_oid"], "projection_digest": publication["projection_digest"],
                    "binding": binding}
    except JournalError as exc:
        raise PublicationError("PUBLICATION_WRITE_CONFLICT", str(exc)) from exc
    finally:
        if hasattr(adapter, "reservation_fd"):
            adapter.reservation_fd = None


def _check_pr(pr: dict[str, Any], publication: dict[str, Any], number: int, head: str,
              expected_draft: bool = True) -> None:
    if not _is_full_sha1_oid(head) or not _is_full_sha1_oid(pr.get("head_oid")):
        raise PublicationError("TASK_IDENTITY_CONFLICT", "PR head is not a full SHA-1 commit OID")
    if type(expected_draft) is not bool or type(pr.get("draft")) is not bool:
        raise PublicationError("TASK_IDENTITY_CONFLICT", "PR draft expectation is invalid")
    actual = (pr.get("repository"), pr.get("number"), pr.get("source_ref"),
              pr.get("target_ref"), pr.get("head_oid"), pr.get("state"), pr.get("merged"),
              pr.get("draft"))
    expected = (publication["repository"], number, publication["source_ref"],
                publication["target_ref"], head, "open", False, expected_draft)
    if actual != expected:
        raise PublicationError("TASK_IDENTITY_CONFLICT", "live PR repository/ref/head/state identity mismatch")


def publish_update(adapter: Any, journal: PublicationJournal, *, publication: dict[str, Any],
                   projection: dict[str, Any], pr_number: int, old_head_oid: str,
                   body: str, legacy_projection_b64: str | None = None,
                   expected_draft: bool = True, existing_ready_update: bool = False,
                   resume_action_id: str | None = None) -> dict[str, Any]:
    """Patch and verify P(H1) before pushing H1 under a lease on H0."""
    publication, marker = _candidate(publication, projection)
    if (type(expected_draft) is not bool or type(existing_ready_update) is not bool
            or expected_draft == existing_ready_update):
        raise PublicationError("TASK_IDENTITY_CONFLICT", "explicit ready update and draft expectation disagree")
    if type(pr_number) is not int or pr_number < 1:
        raise PublicationError("TASK_IDENTITY_CONFLICT", "PR number is invalid")
    if not _is_full_sha1_oid(old_head_oid):
        raise PublicationError("TASK_IDENTITY_CONFLICT", "old PR head is not a full SHA-1 commit OID")
    body = replace_projection_marker(body, marker, legacy_projection_b64=legacy_projection_b64)
    try:
        with journal.locked():
            adapter.reservation_fd = journal.inherited_lock_fd()
            task_binding = _preflight_task_pr_binding(
                adapter, publication, expected_pr_number=pr_number,
            )
            if existing_ready_update:
                admission = adapter.read_task_pr_binding(publication["task_uid"])
                if (admission.get("existing_ready_update") is not True
                        or admission.get("task_uid") != publication["task_uid"]
                        or admission.get("pr_number") != pr_number):
                    raise PublicationError("TASK_IDENTITY_CONFLICT", "live existing-ready-update admission is missing")
            state_action = "update-state:" + publication["publication_id"]
            prior_state = _prior(journal, state_action)
            if prior_state is not None:
                pinned = prior_state.get("expected")
                if (prior_state.get("kind") != "pin_update_state"
                        or not isinstance(pinned, dict)
                        or pinned.get("pr_number") != pr_number
                        or pinned.get("expected_draft") is not expected_draft
                        or pinned.get("existing_ready_update") is not existing_ready_update
                        or not _is_full_sha1_oid(pinned.get("old_head_oid"))):
                    raise PublicationError("TASK_IDENTITY_CONFLICT", "pinned update state differs")
                old_head_oid = pinned["old_head_oid"]
            current = adapter.read_pr(publication["repository"], pr_number)
            head = current.get("head_oid") if isinstance(current, dict) else None
            if head not in (old_head_oid, publication["source_head_oid"]):
                raise PublicationError("SOURCE_SUPERSEDED", f"live PR head advanced to {head!r}")
            _check_pr(current, publication, pr_number, head, expected_draft)
            if head == publication["source_head_oid"] and prior_state is None:
                # Preserve legacy draft recovery with its original patch/push lease.
                prior_patch = _prior(journal, "patch-body:" + publication["publication_id"])
                if existing_ready_update:
                    raise PublicationError("TASK_IDENTITY_CONFLICT", "H1 recovery lacks pinned update identity")
                if prior_patch is not None:
                    recovery_expected = prior_patch.get("expected")
                    recovery_head = (recovery_expected.get("old_head_oid")
                                     if isinstance(recovery_expected, dict) else None)
                    if not _is_full_sha1_oid(recovery_head):
                        raise PublicationError(
                            "TASK_IDENTITY_CONFLICT", "recovery PR head is not a full SHA-1 commit OID",
                        )
                    old_head_oid = recovery_head
            old_body = current.get("body")
            if not isinstance(old_body, str):
                raise PublicationError("EVENT_PROJECTION_INVALID", "live PR body is not text")
            body = replace_projection_marker(
                old_body, marker, legacy_projection_b64=legacy_projection_b64,
            )
            journal.intent(state_action, "pin_update_state", {
                "pr_number": pr_number, "expected_draft": expected_draft,
                "existing_ready_update": existing_ready_update, "old_head_oid": old_head_oid,
            })
            pr_binding = (
                {"state": "bound", "pr_number": task_binding["pr_number"]}
                if task_binding is not None
                else {"state": "unbound", "candidate_pr_number": pr_number}
            )
            _intent(adapter, journal, publication, pr_binding=pr_binding,
                    resume_action_id=resume_action_id)

            action = "patch-body:" + publication["publication_id"]
            body_hash = lambda text: "sha256:" + hashlib.sha256(text.encode("utf-8")).hexdigest()
            journal.intent(action, "patch_projection_body", {
                "pr_number": pr_number, "new_body_sha256": body_hash(body),
                "old_head_oid": old_head_oid,
            })
            if head == publication["source_head_oid"]:
                if old_body == body:
                    journal.observe(action, {"body_sha256": body_hash(body)}, phase="METADATA_CONFIRMED")
                else:
                    # Recovery may find H1 already pushed while metadata was
                    # repaired or overwritten. Reconcile the exact observed
                    # body under the same single-publisher lock.
                    for attempt in range(2):
                        current = adapter.read_pr(publication["repository"], pr_number)
                        _check_pr(current, publication, pr_number, publication["source_head_oid"], expected_draft)
                        live_body = current.get("body")
                        if live_body == body:
                            journal.observe(action, {"body_sha256": body_hash(body)}, phase="METADATA_CONFIRMED")
                            break
                        if live_body != old_body:
                            journal.disposition("CONFLICT")
                            raise PublicationError("PUBLICATION_WRITE_CONFLICT", "PR body changed during H1 recovery")
                        try:
                            adapter.patch_pr_body(publication["repository"], pr_number, body)
                        except Exception:
                            pass
                        observed = adapter.read_pr(publication["repository"], pr_number)
                        _check_pr(observed, publication, pr_number, publication["source_head_oid"], expected_draft)
                        if observed.get("body") == body:
                            journal.observe(action, {"body_sha256": body_hash(body)}, phase="METADATA_CONFIRMED")
                            break
                        if observed.get("body") != old_body:
                            journal.disposition("CONFLICT")
                            raise PublicationError("PUBLICATION_WRITE_CONFLICT", "H1 body repair readback conflicts")
                        if attempt == 1:
                            journal.uncertain(action, "NETWORK_UNCERTAIN")
                            raise PublicationError("NETWORK_UNCERTAIN", "H1 projection repair did not read back")
            else:
                current = adapter.read_pr(publication["repository"], pr_number)
                _check_pr(current, publication, pr_number, old_head_oid, expected_draft)
                if current.get("body") == body:
                    journal.observe(action, {"body_sha256": body_hash(body)}, phase="METADATA_CONFIRMED")
                elif current.get("body") != old_body:
                    journal.disposition("CONFLICT")
                    raise PublicationError("PUBLICATION_WRITE_CONFLICT", "PR body changed during preflight")
                else:
                    # Retrying PATCH is allowed only while exact old body and H0 remain live.
                    for attempt in range(2):
                        try:
                            adapter.patch_pr_body(publication["repository"], pr_number, body)
                        except Exception:
                            pass
                        observed = adapter.read_pr(publication["repository"], pr_number)
                        _check_pr(observed, publication, pr_number, old_head_oid, expected_draft)
                        if observed.get("body") == body:
                            journal.observe(action, {"body_sha256": body_hash(body)}, phase="METADATA_CONFIRMED")
                            break
                        if observed.get("body") != old_body:
                            journal.disposition("CONFLICT")
                            raise PublicationError("PUBLICATION_WRITE_CONFLICT", "PATCH readback conflicts with candidate")
                        if attempt == 1:
                            journal.uncertain(action, "NETWORK_UNCERTAIN")
                            raise PublicationError("NETWORK_UNCERTAIN", "projection PATCH did not read back")

            before_push = adapter.read_pr(publication["repository"], pr_number)
            _check_pr(before_push, publication, pr_number, head, expected_draft)
            if before_push.get("body") != body:
                raise PublicationError("PUBLICATION_WRITE_CONFLICT", "projection body changed before H1 push")
            _push(adapter, journal, publication, old_head_oid)
            final = adapter.read_pr(publication["repository"], pr_number)
            _check_pr(final, publication, pr_number, publication["source_head_oid"], expected_draft)
            if final.get("body") != body:
                journal.disposition("CONFLICT")
                raise PublicationError("PUBLICATION_WRITE_CONFLICT", "projection body changed after H1 push")
            binding = _record_and_bind(adapter, journal, publication, final, expected_draft)
            return {"status": "published", "task_uid": publication["task_uid"],
                    "publication_id": publication["publication_id"], "pr_number": pr_number,
                    "head_oid": publication["source_head_oid"], "projection_digest": publication["projection_digest"],
                    "binding": binding}
    except JournalError as exc:
        raise PublicationError("PUBLICATION_WRITE_CONFLICT", str(exc)) from exc
    finally:
        if hasattr(adapter, "reservation_fd"):
            adapter.reservation_fd = None


def publish(update: Callable[[str], Any], **kwargs: Any) -> dict[str, Any]:
    """Compatibility wrapper retained for the original pure helper API."""
    contract, marker = prepare(**kwargs)
    update(marker)
    return validate_contract(contract)
