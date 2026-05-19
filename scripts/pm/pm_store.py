#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys
import uuid
from collections import OrderedDict
from datetime import datetime, timedelta

from pm_store_cli import build_parser
from pm_store_docio import (
    dump_list_document,
    dump_mapping_document,
    format_scalar,
    load_list_document,
    load_mapping_document,
    parse_key_value,
    parse_scalar,
)
from pm_store_reporting import (
    build_memory_report as build_memory_report_helper,
    build_reflection_summary as build_reflection_summary_helper,
    build_role_report as build_role_report_helper,
    build_signal_summary as build_signal_summary_helper,
    build_workflow_checklist as build_workflow_checklist_helper,
    build_workflow_report as build_workflow_report_helper,
    cmd_memory_report as cmd_memory_report_helper,
    cmd_reflection_report as cmd_reflection_report_helper,
    cmd_role_report as cmd_role_report_helper,
    cmd_workflow_report as cmd_workflow_report_helper,
    cmd_working_memory_report as cmd_working_memory_report_helper,
)
from pm_store_stage import (
    build_stage_report as build_stage_report_helper,
    cmd_set_stage as cmd_set_stage_helper,
    cmd_stage_lint as cmd_stage_lint_helper,
    cmd_stage_report as cmd_stage_report_helper,
    run_stage_lint as run_stage_lint_helper,
)
from pm_store_task_lint import run_task_backlog_lint as run_task_backlog_lint_helper
TASK_UID_RE = re.compile(r"^task_[0-9a-f]{32}$")
TASK_STATUSES = {"candidate", "committed", "blocked", "done", "deferred"}
LIVE_BACKLOG_STATUSES = {"candidate", "committed", "blocked"}
ALLOWED_SIGNAL_STATES = {"new", "triaged", "promoted_candidate_task", "discarded", "deferred"}
ALLOWED_MEMORY_PROMOTION_STATES = {"pending", "promoted", "rejected", "deferred"}
ALLOWED_PROMOTION_REASONS = {
    "abi_contract",
    "agent_behavior",
    "community_pattern",
    "engineering_constraint",
    "failure_signature",
    "incident_pattern",
    "policy_boundary",
    "repro_pattern",
    "stable_pattern",
    "stage_decision",
    "runtime_contract",
    "test_strategy",
    "ux_constraint",
}
ALLOWED_MEMORY_REJECTION_REASONS = {
    "one_off_operation",
    "short_lived_execution_detail",
    "task_status_update",
    "unverified_hypothesis",
}
ROLE_MEMORY_PREFIXES = {
    "agent_engineer": "AGENT",
    "liveops_community": "LIVEOPS",
    "producer_system_designer": "PRODUCER",
    "qa_engineer": "QA",
    "runtime_engineer": "RUNTIME",
    "shared": "SHARED",
    "viewer_engineer": "VIEWER",
    "wasm_platform_engineer": "WASM",
}
DEFAULT_MEMORY_REVIEW_STALE_DAYS = 7
DEFAULT_WORKING_MEMORY_EXPIRES_DAYS = 2
PRIORITY_ORDER = {"P0": 0, "P1": 1, "P2": 2, "P3": 3}
SEVERITY_ORDER = {"critical": 0, "high": 1, "medium": 2, "low": 3}
WORKING_MEMORY_ENTRY_KINDS = {
    "attempt",
    "hypothesis",
    "decision",
    "open_question",
    "next_step",
}
TASK_EXECUTION_LOG_ENTRY_RE = re.compile(r"^## (\d{4}-\d{2}-\d{2}) (\d{2}:\d{2}:\d{2}) CST / ([a-z_][a-z0-9_]*)$")
REDACTION_PATTERNS = [
    (re.compile(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b"), "[REDACTED_EMAIL]"),
    (
        re.compile(r"\b(?:sk|pk|rk|ghp|gho|ghu|ghs|github_pat|AIza|xox[baprs])[-_A-Za-z0-9]{10,}\b"),
        "[REDACTED_TOKEN]",
    ),
    (re.compile(r"(?i)\bBearer\s+[A-Za-z0-9._-]{10,}\b"), "Bearer [REDACTED_TOKEN]"),
]


def now_iso() -> str:
    return datetime.now().astimezone().isoformat(timespec="seconds")


def die(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(1)


def replace_text_tokens(text: str, replacements: dict[str, str]) -> str:
    updated = text
    for old, new in sorted(replacements.items(), key=lambda item: len(item[0]), reverse=True):
        updated = updated.replace(old, new)
    return updated


def rewrite_object_strings(value, replacements: dict[str, str]):
    if isinstance(value, str):
        return replace_text_tokens(value, replacements)
    if isinstance(value, list):
        return [rewrite_object_strings(item, replacements) for item in value]
    if isinstance(value, OrderedDict):
        return OrderedDict((key, rewrite_object_strings(item, replacements)) for key, item in value.items())
    if isinstance(value, dict):
        return {key: rewrite_object_strings(item, replacements) for key, item in value.items()}
    return value


def load_roles(root: pathlib.Path) -> set[str]:
    roles = set()
    for line in (root / ".pm/registry/roles.yaml").read_text(encoding="utf-8").splitlines():
        if line.startswith("  - role_name: "):
            roles.add(line.split(": ", 1)[1].strip())
    return roles


def load_active_memory_record(root: pathlib.Path, path: pathlib.Path, topic: str) -> OrderedDict[str, object] | None:
    _, records = load_list_document(path, "records")
    for record in records:
        if record.get("status") == "active" and record.get("topic") == topic:
            return record
    return None


def backlog_file_for_status(status: str) -> str:
    if status in LIVE_BACKLOG_STATUSES:
        return f"{status}.yaml"
    if status in {"done", "deferred"}:
        return "done.yaml"
    raise ValueError(f"unsupported backlog status: {status}")


def validate_status(status: str) -> None:
    if status not in TASK_STATUSES:
        raise ValueError(f"unsupported task status: {status}")


def task_relative_path(task_uid: str) -> str:
    return f".pm/tasks/{task_uid}.yaml"


def task_execution_log_relative_path(task_uid: str) -> str:
    return f".pm/tasks/{task_uid}.execution.md"


def generate_task_uid(seed: str | None = None) -> str:
    token = uuid.uuid5(uuid.NAMESPACE_URL, seed).hex if seed else uuid.uuid4().hex
    return f"task_{token}"


def task_file_path(root: pathlib.Path, task_uid: str) -> pathlib.Path:
    return root / task_relative_path(task_uid)


def find_task_file(root: pathlib.Path, task_uid: str) -> tuple[pathlib.Path, OrderedDict[str, object]]:
    path = task_file_path(root, task_uid)
    if not path.is_file():
        raise ValueError(f"task not found: {task_uid}")
    fields = load_mapping_document(path)
    if str(fields.get("task_uid") or "") != task_uid:
        raise ValueError(f"task file task_uid mismatch: {path.relative_to(root)}")
    return path, fields


def iter_task_files(
    root: pathlib.Path,
) -> list[tuple[pathlib.Path, OrderedDict[str, object]]]:
    directory = root / ".pm/tasks"
    if not directory.exists():
        return []
    records: list[tuple[pathlib.Path, OrderedDict[str, object]]] = []
    for path in sorted(directory.glob("*.yaml")):
        records.append((path, load_mapping_document(path)))
    return records


def task_order_key(fields: OrderedDict[str, object]) -> tuple[object, object, object]:
    return (
        PRIORITY_ORDER.get(str(fields.get("priority")), 99),
        str(fields.get("updated_at") or ""),
        str(fields.get("task_uid") or ""),
    )


def task_registry_path(root: pathlib.Path) -> pathlib.Path:
    return root / ".pm/registry/tasks.yaml"


def role_backlog_path(root: pathlib.Path, role: str, file_status: str) -> pathlib.Path:
    return root / f".pm/roles/{role}/backlog/{file_status}.yaml"


def rebuild_task_views(root: pathlib.Path) -> None:
    task_records = []
    for path, fields in iter_task_files(root):
        task_uid = str(fields.get("task_uid") or "")
        if not task_uid:
            continue
        task_records.append((path, fields))

    task_records.sort(key=lambda item: task_order_key(item[1]))

    registry_header = OrderedDict(
        [
            ("version", 2),
            ("identity_key", "task_uid"),
            ("generated_from", ".pm/tasks/*.yaml"),
        ]
    )
    registry_entries: list[OrderedDict[str, object]] = []
    for path, fields in task_records:
        registry_entries.append(
            OrderedDict(
                [
                    ("task_uid", fields.get("task_uid")),
                    ("owner_role", fields.get("owner_role")),
                    ("task_path", str(path.relative_to(root))),
                    ("status", fields.get("status")),
                    ("priority", fields.get("priority")),
                    ("source_signal", fields.get("source_signal")),
                    ("updated_at", fields.get("updated_at")),
                ]
            )
        )
    registry_path = task_registry_path(root)
    registry_path.parent.mkdir(parents=True, exist_ok=True)
    dump_list_document(registry_path, registry_header, "tasks", registry_entries)

    for role in sorted(load_roles(root)):
        for file_status in ("candidate", "committed", "blocked", "done"):
            backlog_path = role_backlog_path(root, role, file_status)
            if backlog_path.exists():
                header, _ = load_list_document(backlog_path, "tasks")
            else:
                header = OrderedDict([("version", 1), ("role", role), ("status", file_status)])
            items: list[OrderedDict[str, object]] = []
            for path, fields in task_records:
                if str(fields.get("owner_role") or "") != role:
                    continue
                status = str(fields.get("status") or "")
                if backlog_file_for_status(status) != f"{file_status}.yaml":
                    continue
                items.append(
                    OrderedDict(
                        [
                            ("task_uid", fields.get("task_uid")),
                            ("title", fields.get("title")),
                            ("priority", fields.get("priority")),
                            ("source_signal", fields.get("source_signal")),
                            ("related_prd", list(fields.get("related_prd", []))),
                            ("acceptance", list(fields.get("acceptance", []))),
                            ("handoff_to", list(fields.get("handoff_to", []))),
                            ("status", status),
                            ("task_path", str(path.relative_to(root))),
                        ]
                    )
                )
            backlog_path.parent.mkdir(parents=True, exist_ok=True)
            dump_list_document(backlog_path, header, "tasks", items)


def sync_task_views(root: pathlib.Path) -> dict[str, object]:
    rebuild_task_views(root)
    return {
        "task_registry_path": str(task_registry_path(root).relative_to(root)),
        "task_count": sum(1 for _ in iter_task_files(root)),
        "role_count": len(load_roles(root)),
    }


def find_registry_task(root: pathlib.Path, task_uid: str) -> tuple[OrderedDict[str, object], list[OrderedDict[str, object]], OrderedDict[str, object], pathlib.Path]:
    sync_task_views(root)
    registry_path = task_registry_path(root)
    header, tasks = load_list_document(registry_path, "tasks")
    for entry in tasks:
        if entry.get("task_uid") == task_uid:
            return header, tasks, entry, registry_path
    raise ValueError(f"task not found in registry: {task_uid}")


def load_task_context(root: pathlib.Path, task_uid: str) -> dict[str, object]:
    _, task_fields = find_task_file(root, task_uid)
    return {
        "task_uid": task_uid,
        "owner_role": task_fields.get("owner_role"),
        "status": task_fields.get("status"),
        "priority": task_fields.get("priority"),
        "title": task_fields.get("title"),
        "worktree_hint": task_fields.get("worktree_hint"),
        "execution_log_path": task_fields.get("execution_log_path"),
        "last_started_at": task_fields.get("last_started_at"),
        "last_closed_at": task_fields.get("last_closed_at"),
        "updated_at": task_fields.get("updated_at"),
    }


def init_task_execution_log(
    root: pathlib.Path,
    task_uid: str,
    title: str,
    owner_role: str,
    worktree_hint: str | None,
    *,
    path_rel: str | None = None,
) -> None:
    relative_path = path_rel or task_execution_log_relative_path(task_uid)
    path = root / relative_path
    if path.exists():
        return
    path.write_text(
        "\n".join(
            [
                f"# {task_uid} Execution Log",
                "",
                f"- task_uid: {task_uid}",
                f"- title: {title}",
                f"- owner_role: {owner_role}",
                f"- worktree_hint: {worktree_hint or 'null'}",
                "",
                "<!-- Append entries using:",
                "Example:",
                "  ## YYYY-MM-DD HH:MM:SS CST / role_name",
                "  - 完成内容: ...",
                "  - 遗留事项: ...",
                "-->",
                "",
            ]
        ),
        encoding="utf-8",
    )


def record_task_workflow_phase(root: pathlib.Path, task_uid: str, role: str, phase: str) -> dict[str, object]:
    if phase not in {"start", "close"}:
        raise ValueError(f"unsupported workflow record phase: {phase}")

    task_path, task_fields = find_task_file(root, task_uid)
    owner_role = str(task_fields.get("owner_role") or "")
    if owner_role != role:
        raise ValueError(f"task owner_role mismatch for workflow report: {task_uid} -> {owner_role} != {role}")

    updated_at = now_iso()
    if phase == "start":
        task_fields["last_started_at"] = updated_at
    else:
        task_fields["last_closed_at"] = updated_at
    task_fields["updated_at"] = updated_at

    dump_mapping_document(task_path, task_fields)
    rebuild_task_views(root)
    return load_task_context(root, task_uid)


def collect_signals(root: pathlib.Path) -> tuple[set[str], set[str]]:
    signal_ids: set[str] = set()
    promoted_signal_ids: set[str] = set()
    for payload in load_signal_entries(root):
        signal_id = payload["signal_id"]
        if signal_id in signal_ids:
            raise ValueError(f"duplicate signal_id in inbox: {signal_id}")
        signal_ids.add(signal_id)
        if payload["promotion_state"] == "promoted_candidate_task":
            promoted_signal_ids.add(signal_id)
    return signal_ids, promoted_signal_ids


def load_signal_entries(root: pathlib.Path) -> list[OrderedDict[str, object]]:
    signals_path = root / ".pm/inbox/signals.jsonl"
    entries: list[OrderedDict[str, object]] = []
    if not signals_path.exists():
        return entries
    for raw_line in signals_path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line:
            continue
        entries.append(json.loads(line, object_pairs_hook=OrderedDict))
    return entries


def dump_signal_entries(root: pathlib.Path, entries: list[OrderedDict[str, object]]) -> None:
    signals_path = root / ".pm/inbox/signals.jsonl"
    with signals_path.open("w", encoding="utf-8") as handle:
        for payload in entries:
            handle.write(json.dumps(payload, ensure_ascii=False) + "\n")


def find_signal_entry(
    root: pathlib.Path,
    signal_id: str,
) -> tuple[list[OrderedDict[str, object]], OrderedDict[str, object]]:
    entries = load_signal_entries(root)
    for payload in entries:
        if payload.get("signal_id") == signal_id:
            return entries, payload
    raise ValueError(f"signal not found in inbox: {signal_id}")


def parse_reference_path(value: str) -> str:
    return value.split("#", 1)[0]


def resolve_source_ref_path(root: pathlib.Path, source_ref: str) -> pathlib.Path:
    source_path = parse_reference_path(source_ref)
    if not source_path:
        raise ValueError("empty source_ref path")
    path = pathlib.Path(source_path).expanduser()
    if path.is_absolute():
        return path
    return root / path


def is_devlog_archive_reference(source_ref: str) -> bool:
    source_path = parse_reference_path(str(source_ref))
    if not source_path:
        return False
    normalized = source_path.replace("\\", "/")
    parts = pathlib.PurePosixPath(normalized).parts
    return len(parts) >= 2 and parts[0] == "doc" and parts[1] == "devlog"


def ensure_non_devlog_runtime_source_ref(source_ref: str, context: str) -> None:
    if is_devlog_archive_reference(source_ref):
        raise ValueError(f"{context} must not use doc/devlog archive as runtime source_ref: {source_ref}")


def validate_runtime_source_ref(root: pathlib.Path, source_ref: str, context: str) -> pathlib.Path:
    ensure_non_devlog_runtime_source_ref(source_ref, context)
    resolved_source = resolve_source_ref_path(root, source_ref)
    if not resolved_source.exists():
        raise ValueError(f"{context} missing: {parse_reference_path(str(source_ref))}")
    return resolved_source


def redact_text(text: str) -> tuple[str, int]:
    redacted = text
    replacements = 0
    for pattern, replacement in REDACTION_PATTERNS:
        redacted, count = pattern.subn(replacement, redacted)
        replacements += count
    return redacted, replacements


def working_memory_dir(root: pathlib.Path) -> pathlib.Path:
    return root / ".pm/working_memory"


def working_memory_path(root: pathlib.Path, task_uid: str) -> pathlib.Path:
    return working_memory_dir(root) / f"{task_uid}.yaml"


def working_memory_relative_path(task_uid: str) -> str:
    return f".pm/working_memory/{task_uid}.yaml"


def codex_sessions_registry_path(root: pathlib.Path) -> pathlib.Path:
    return root / ".pm/registry/codex-sessions.yaml"


def load_codex_sessions_registry(
    root: pathlib.Path,
) -> tuple[pathlib.Path, OrderedDict[str, object], list[OrderedDict[str, object]]]:
    path = codex_sessions_registry_path(root)
    if path.exists():
        header, entries = load_list_document(path, "sessions")
        return path, header, entries
    return path, OrderedDict([("version", 1)]), []


def remember_codex_session(
    root: pathlib.Path,
    task_uid: str,
    role: str,
    session_id: str,
    thread_name: str | None,
    worktree_hint: str | None,
    codex_dir: pathlib.Path,
    updated_at: str,
) -> dict[str, object]:
    path, header, entries = load_codex_sessions_registry(root)
    path.parent.mkdir(parents=True, exist_ok=True)

    retained: list[OrderedDict[str, object]] = []
    for entry in entries:
        if str(entry.get("task_uid") or "") == task_uid:
            continue
        retained.append(entry)

    retained.append(
        OrderedDict(
            [
                ("task_uid", task_uid),
                ("role", role),
                ("session_id", session_id),
                ("thread_name", thread_name),
                ("worktree_hint", worktree_hint),
                ("codex_dir", str(codex_dir)),
                ("updated_at", updated_at),
            ]
        )
    )
    dump_list_document(path, header, "sessions", retained)
    return {
        "task_uid": task_uid,
        "role": role,
        "session_id": session_id,
        "thread_name": thread_name,
        "worktree_hint": worktree_hint,
        "codex_dir": str(codex_dir),
        "updated_at": updated_at,
        "path": str(path),
    }


def resolve_task_worktree_hint(root: pathlib.Path, task_uid: str | None) -> str | None:
    if not task_uid:
        return None
    task_file = task_file_path(root, task_uid)
    if not task_file.exists():
        return None
    fields = load_mapping_document(task_file)
    value = fields.get("worktree_hint")
    if value in {None, ""}:
        return None
    return str(value)


def resolve_codex_session_id(
    root: pathlib.Path,
    codex_dir: pathlib.Path,
    session_id: str | None,
    task_uid: str | None,
    worktree_hint: str | None,
    thread_name_pattern: str | None,
) -> tuple[str, str, str | None]:
    if session_id:
        metadata = load_codex_session_metadata(codex_dir, session_id)
        return session_id, "explicit", str(metadata.get("thread_name") or "")

    if task_uid:
        _, _, registry_entries = load_codex_sessions_registry(root)
        for entry in reversed(registry_entries):
            if str(entry.get("task_uid") or "") == task_uid:
                resolved_session_id = str(entry.get("session_id") or "")
                if not resolved_session_id:
                    break
                metadata = load_codex_session_metadata(codex_dir, resolved_session_id)
                return resolved_session_id, "registry", str(metadata.get("thread_name") or "")

    derived_worktree_hint = worktree_hint or resolve_task_worktree_hint(root, task_uid)
    pattern = thread_name_pattern or derived_worktree_hint
    if not pattern:
        raise ValueError("missing session resolution input: provide --session-id, --task-uid with saved mapping, --worktree-hint, or --thread-name-pattern")

    index_path = codex_dir / "session_index.jsonl"
    if not index_path.exists():
        raise ValueError(f"missing Codex session index: {index_path}")

    normalized_pattern = pattern.casefold()
    candidates: list[OrderedDict[str, object]] = []
    for raw_line in index_path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line:
            continue
        payload = json.loads(line, object_pairs_hook=OrderedDict)
        thread_name = str(payload.get("thread_name") or "")
        if normalized_pattern in thread_name.casefold():
            candidates.append(payload)

    if not candidates:
        raise ValueError(f"no Codex session matched pattern: {pattern}")

    candidates.sort(key=lambda item: str(item.get("updated_at") or ""), reverse=True)
    selected = candidates[0]
    return str(selected.get("id")), "pattern", str(selected.get("thread_name") or "")


def load_working_memory_document(
    root: pathlib.Path,
    task_uid: str,
    role: str | None = None,
    worktree_hint: str | None = None,
) -> tuple[pathlib.Path, OrderedDict[str, object], list[OrderedDict[str, object]]]:
    path = working_memory_path(root, task_uid)
    if path.exists():
        header, entries = load_list_document(path, "entries")
        return path, header, entries

    header: OrderedDict[str, object] = OrderedDict(
        [
            ("version", 1),
            ("task_uid", task_uid),
            ("role", role),
            ("worktree_hint", worktree_hint),
        ]
    )
    return path, header, []


def iter_working_memory_documents(
    root: pathlib.Path,
) -> list[tuple[pathlib.Path, OrderedDict[str, object], list[OrderedDict[str, object]]]]:
    directory = working_memory_dir(root)
    if not directory.exists():
        return []
    documents: list[tuple[pathlib.Path, OrderedDict[str, object], list[OrderedDict[str, object]]]] = []
    for path in sorted(directory.glob("*.yaml")):
        header, entries = load_list_document(path, "entries")
        documents.append((path, header, entries))
    return documents


def next_working_memory_entry_id(entries: list[OrderedDict[str, object]]) -> str:
    max_sequence = 0
    for entry in entries:
        entry_id = str(entry.get("entry_id") or "")
        match = re.fullmatch(r"WM-(\d{4})", entry_id)
        if match:
            max_sequence = max(max_sequence, int(match.group(1)))
    return f"WM-{max_sequence + 1:04d}"


def build_working_memory_report(
    root: pathlib.Path,
    task_uid: str | None,
    role_filter: str | None,
) -> dict[str, object]:
    if role_filter and role_filter not in load_roles(root):
        raise ValueError(f"unknown role: {role_filter}")

    task_payloads: OrderedDict[str, dict[str, object]] = OrderedDict()
    counts_by_kind: OrderedDict[str, int] = OrderedDict(
        (kind, 0) for kind in sorted(WORKING_MEMORY_ENTRY_KINDS)
    )
    total_entries = 0

    for path, header, entries in iter_working_memory_documents(root):
        current_task_uid = str(header.get("task_uid") or path.stem)
        current_role = str(header.get("role") or "")
        if task_uid and current_task_uid != task_uid:
            continue
        if role_filter and current_role != role_filter:
            continue

        payload_counts: OrderedDict[str, int] = OrderedDict(
            (kind, 0) for kind in sorted(WORKING_MEMORY_ENTRY_KINDS)
        )
        shaped_entries: list[OrderedDict[str, object]] = []
        for entry in sorted(entries, key=lambda item: str(item.get("captured_at") or ""), reverse=True):
            entry_kind = str(entry.get("entry_kind") or "")
            if entry_kind in payload_counts:
                payload_counts[entry_kind] += 1
                counts_by_kind[entry_kind] += 1
            total_entries += 1
            shaped_entries.append(
                OrderedDict(
                    [
                        ("entry_id", entry.get("entry_id")),
                        ("entry_kind", entry.get("entry_kind")),
                        ("summary", entry.get("summary")),
                        ("source_refs", list(entry.get("source_refs", []))),
                        ("captured_at", entry.get("captured_at")),
                        ("expires_at", entry.get("expires_at")),
                        ("promoted_to", list(entry.get("promoted_to", []))),
                    ]
                )
            )

        task_payloads[current_task_uid] = {
            "task_uid": current_task_uid,
            "role": header.get("role"),
            "worktree_hint": header.get("worktree_hint"),
            "source_session_id": header.get("source_session_id"),
            "source_thread_name": header.get("source_thread_name"),
            "transcript_source": header.get("transcript_source"),
            "last_extracted_ts": header.get("last_extracted_ts"),
            "captured_until_ts": header.get("captured_until_ts"),
            "entry_count": len(entries),
            "counts_by_kind": payload_counts,
            "entries": shaped_entries,
        }

    return {
        "generated_at": now_iso(),
        "task_filter": task_uid,
        "role_filter": role_filter,
        "task_count": len(task_payloads),
        "entry_count": total_entries,
        "counts_by_kind": counts_by_kind,
        "tasks": task_payloads,
    }


def run_working_memory_lint(root: pathlib.Path) -> None:
    failures: list[str] = []
    roles = load_roles(root)

    def fail(message: str) -> None:
        failures.append(message)

    directory = working_memory_dir(root)
    if not directory.exists():
        fail("missing directory: .pm/working_memory")
    for path, header, entries in iter_working_memory_documents(root):
        task_uid = str(header.get("task_uid") or "")
        role = header.get("role")
        if not task_uid:
            fail(f"{path.relative_to(root)} missing task_uid")
        elif path.name != f"{task_uid}.yaml":
            fail(f"{path.relative_to(root)} filename/task_uid mismatch: {path.name} != {task_uid}.yaml")

        if role is None or str(role) not in roles:
            fail(f"{path.relative_to(root)} unknown role: {role}")

        entry_ids: set[str] = set()
        for entry in entries:
            entry_id = str(entry.get("entry_id") or "")
            if not entry_id:
                fail(f"{path.relative_to(root)} entry missing entry_id")
                continue
            if entry_id in entry_ids:
                fail(f"{path.relative_to(root)} duplicate entry_id: {entry_id}")
            entry_ids.add(entry_id)

            entry_kind = str(entry.get("entry_kind") or "")
            if entry_kind not in WORKING_MEMORY_ENTRY_KINDS:
                fail(f"{path.relative_to(root)} {entry_id} invalid entry_kind: {entry_kind}")

            summary = str(entry.get("summary") or "").strip()
            if not summary:
                fail(f"{path.relative_to(root)} {entry_id} missing summary")

            source_refs = entry.get("source_refs")
            if not isinstance(source_refs, list) or not source_refs:
                fail(f"{path.relative_to(root)} {entry_id} source_refs must be a non-empty list")
            else:
                for source_ref in source_refs:
                    try:
                        resolved = resolve_source_ref_path(root, str(source_ref))
                    except ValueError as exc:
                        fail(f"{path.relative_to(root)} {entry_id} invalid source_ref: {exc}")
                        continue
                    if not resolved.exists():
                        fail(f"{path.relative_to(root)} {entry_id} source_ref missing: {resolved}")

            for key in ("captured_at", "expires_at"):
                try:
                    datetime.fromisoformat(str(entry.get(key)))
                except ValueError:
                    fail(f"{path.relative_to(root)} {entry_id} invalid timestamp: {key}={entry.get(key)}")

            try:
                captured_at = datetime.fromisoformat(str(entry.get("captured_at")))
                expires_at = datetime.fromisoformat(str(entry.get("expires_at")))
                if expires_at < captured_at:
                    fail(f"{path.relative_to(root)} {entry_id} expires_at before captured_at")
            except ValueError:
                pass

            promoted_to = entry.get("promoted_to")
            if not isinstance(promoted_to, list):
                fail(f"{path.relative_to(root)} {entry_id} promoted_to must be a list")

    if failures:
        for failure in failures:
            print(f"working-memory-lint: FAIL: {failure}", file=sys.stderr)
        raise SystemExit(1)


def load_codex_session_metadata(codex_dir: pathlib.Path, session_id: str) -> OrderedDict[str, object]:
    index_path = codex_dir / "session_index.jsonl"
    if index_path.exists():
        for raw_line in index_path.read_text(encoding="utf-8").splitlines():
            line = raw_line.strip()
            if not line:
                continue
            payload = json.loads(line, object_pairs_hook=OrderedDict)
            if payload.get("id") == session_id:
                return payload

    rollout_path = find_codex_session_rollout_path(codex_dir, session_id)
    if rollout_path is not None:
        metadata = load_codex_session_metadata_from_rollout(rollout_path, session_id)
        if metadata is not None:
            return metadata

    if not index_path.exists():
        raise ValueError(f"missing Codex session index: {index_path}")
    raise ValueError(f"session_id not found in session_index.jsonl or sessions rollout files: {session_id}")


def find_codex_session_rollout_path(codex_dir: pathlib.Path, session_id: str) -> pathlib.Path | None:
    sessions_dir = codex_dir / "sessions"
    if not sessions_dir.exists():
        return None

    matches = sorted(sessions_dir.rglob(f"*{session_id}*.jsonl"))
    if matches:
        return matches[-1]
    return None


def load_codex_session_metadata_from_rollout(
    rollout_path: pathlib.Path,
    session_id: str,
) -> OrderedDict[str, object] | None:
    for raw_line in rollout_path.read_text(encoding="utf-8", errors="ignore").splitlines():
        line = raw_line.strip()
        if not line:
            continue
        payload = json.loads(line, object_pairs_hook=OrderedDict)
        if payload.get("type") != "session_meta":
            continue
        session_payload = payload.get("payload")
        if not isinstance(session_payload, dict):
            continue
        if session_payload.get("id") != session_id:
            continue
        return OrderedDict(
            [
                ("id", session_payload.get("id")),
                ("thread_name", session_payload.get("title") or session_payload.get("thread_name")),
                ("updated_at", payload.get("timestamp") or session_payload.get("timestamp")),
            ]
        )
    return None


def extract_text_from_codex_message_content(content: object) -> str:
    if not isinstance(content, list):
        return ""

    parts: list[str] = []
    for item in content:
        if not isinstance(item, dict):
            continue
        text = item.get("text")
        if text:
            parts.append(str(text))
    return "\n".join(part for part in parts if part).strip()


def load_codex_rollout_messages(
    codex_dir: pathlib.Path,
    session_id: str,
) -> tuple[list[OrderedDict[str, object]], int, str | None]:
    rollout_path = find_codex_session_rollout_path(codex_dir, session_id)
    if rollout_path is None:
        return [], 0, None

    messages: list[OrderedDict[str, object]] = []
    redaction_count = 0
    seen: set[tuple[str, str, str]] = set()

    for line_no, raw_line in enumerate(
        rollout_path.read_text(encoding="utf-8", errors="ignore").splitlines(),
        start=1,
    ):
        line = raw_line.strip()
        if not line:
            continue
        payload = json.loads(line, object_pairs_hook=OrderedDict)
        record_type = payload.get("type")
        record_payload = payload.get("payload")
        if not isinstance(record_payload, dict):
            continue

        role = ""
        text = ""
        if record_type == "event_msg":
            event_type = str(record_payload.get("type") or "")
            if event_type == "user_message":
                role = "user"
                text = str(record_payload.get("message") or "")
            elif event_type == "agent_message":
                role = "assistant"
                text = str(record_payload.get("message") or "")
        elif record_type == "response_item" and record_payload.get("type") == "message":
            role = str(record_payload.get("role") or "")
            text = extract_text_from_codex_message_content(record_payload.get("content"))

        text = text.strip()
        timestamp = str(payload.get("timestamp") or "")
        if not timestamp or not role or not text:
            continue

        dedupe_key = (timestamp, role, text)
        if dedupe_key in seen:
            continue
        seen.add(dedupe_key)

        redacted_text, replacements = redact_text(text)
        redaction_count += replacements
        messages.append(
            OrderedDict(
                [
                    ("session_id", session_id),
                    ("ts", timestamp),
                    ("role", role),
                    ("text", redacted_text),
                    ("source_ref", f"{rollout_path}#L{line_no}"),
                ]
            )
        )

    return messages, redaction_count, str(rollout_path)


def codex_message_sort_key(item: OrderedDict[str, object]) -> tuple[int, str]:
    raw_ts = item.get("ts")
    if isinstance(raw_ts, int):
        return (0, f"{raw_ts:020d}")

    ts_text = str(raw_ts or "")
    if re.fullmatch(r"\d+", ts_text):
        return (0, f"{int(ts_text):020d}")

    return (1, ts_text)


def codex_ts_in_window(ts: object, after_ts: object | None, before_ts: object | None) -> bool:
    ts_key = codex_message_sort_key(OrderedDict([("ts", ts)]))
    if after_ts not in {None, ""}:
        after_key = codex_message_sort_key(OrderedDict([("ts", after_ts)]))
        if ts_key <= after_key:
            return False
    if before_ts not in {None, ""}:
        before_key = codex_message_sort_key(OrderedDict([("ts", before_ts)]))
        if ts_key > before_key:
            return False
    return True


def build_codex_transcript_report(
    codex_dir: pathlib.Path,
    session_id: str,
    after_ts: object | None = None,
    before_ts: object | None = None,
) -> dict[str, object]:
    history_path = codex_dir / "history.jsonl"
    session = load_codex_session_metadata(codex_dir, session_id)
    messages: list[OrderedDict[str, object]] = []
    redaction_count = 0
    transcript_source = "history_jsonl"
    found_history_messages = False
    if history_path.exists():
        for raw_line in history_path.read_text(encoding="utf-8").splitlines():
            line = raw_line.strip()
            if not line:
                continue
            payload = json.loads(line, object_pairs_hook=OrderedDict)
            if payload.get("session_id") != session_id:
                continue
            found_history_messages = True
            ts = payload.get("ts")
            if not codex_ts_in_window(ts, after_ts=after_ts, before_ts=before_ts):
                continue
            redacted_text, replacements = redact_text(str(payload.get("text") or ""))
            redaction_count += replacements
            messages.append(
                OrderedDict(
                    [
                        ("session_id", session_id),
                        ("ts", ts),
                        ("text", redacted_text),
                        ("source_ref", f"{history_path}#session_id={session_id}&ts={ts}"),
                    ]
                )
            )

    if not messages:
        rollout_messages, rollout_redaction_count, rollout_path = load_codex_rollout_messages(codex_dir, session_id)
        messages = [
            message
            for message in rollout_messages
            if codex_ts_in_window(message.get("ts"), after_ts=after_ts, before_ts=before_ts)
        ]
        found_rollout_messages = bool(rollout_messages)
        if messages:
            redaction_count = rollout_redaction_count
            transcript_source = "sessions_rollout"
        elif not found_history_messages and not found_rollout_messages:
            if not history_path.exists():
                raise ValueError(
                    f"missing Codex transcript sources: {history_path} and {codex_dir / 'sessions'}"
                )
            raise ValueError(f"no transcript messages found for session_id={session_id}")
    else:
        rollout_path = None

    messages.sort(key=codex_message_sort_key)
    return {
        "generated_at": now_iso(),
        "codex_dir": str(codex_dir),
        "session": OrderedDict(
            [
                ("id", session.get("id")),
                ("thread_name", session.get("thread_name")),
                ("updated_at", session.get("updated_at")),
                ("source_ref", f"{codex_dir / 'session_index.jsonl'}#id={session_id}"),
            ]
        ),
        "transcript_source": transcript_source,
        "rollout_source_ref": f"{rollout_path}#session_id={session_id}" if rollout_path else None,
        "after_ts": after_ts,
        "before_ts": before_ts,
        "message_count": len(messages),
        "redaction_count": redaction_count,
        "messages": messages,
    }


def import_working_memory_entries(
    root: pathlib.Path,
    task_uid: str,
    role: str,
    worktree_hint: str | None,
    entries_payload: list[dict[str, object]],
    expires_days: int,
    source_session_id: str | None = None,
    source_thread_name: str | None = None,
    transcript_source: str | None = None,
    last_extracted_ts: object | None = None,
    captured_until_ts: object | None = None,
) -> dict[str, object]:
    if role not in load_roles(root):
        raise ValueError(f"unknown role: {role}")
    if expires_days < 0:
        raise ValueError("--expires-days must be >= 0")

    path, header, existing_entries = load_working_memory_document(root, task_uid, role=role, worktree_hint=worktree_hint)
    directory = working_memory_dir(root)
    directory.mkdir(parents=True, exist_ok=True)
    header["role"] = role
    header["worktree_hint"] = worktree_hint
    if source_session_id:
        header["source_session_id"] = source_session_id
    if source_thread_name:
        header["source_thread_name"] = source_thread_name
    if transcript_source:
        header["transcript_source"] = transcript_source
    if last_extracted_ts not in {None, ""}:
        header["last_extracted_ts"] = last_extracted_ts
    if captured_until_ts not in {None, ""}:
        header["captured_until_ts"] = captured_until_ts

    existing_keys = {
        (
            str(entry.get("entry_kind") or ""),
            str(entry.get("summary") or "").strip(),
            tuple(str(item) for item in entry.get("source_refs", [])),
        )
        for entry in existing_entries
    }

    added = 0
    skipped = 0
    captured_at = now_iso()
    expires_at = (datetime.now().astimezone() + timedelta(days=expires_days)).isoformat(timespec="seconds")

    for payload in entries_payload:
        entry_kind = str(payload.get("entry_kind") or "")
        if entry_kind not in WORKING_MEMORY_ENTRY_KINDS:
            raise ValueError(f"invalid working_memory entry_kind: {entry_kind}")
        summary = str(payload.get("summary") or "").strip()
        if not summary:
            raise ValueError("working_memory entry missing summary")
        source_refs = [str(item) for item in payload.get("source_refs", [])]
        if not source_refs:
            raise ValueError("working_memory entry missing source_refs")

        dedupe_key = (entry_kind, summary, tuple(source_refs))
        if dedupe_key in existing_keys:
            skipped += 1
            continue
        existing_keys.add(dedupe_key)

        existing_entries.append(
            OrderedDict(
                [
                    ("entry_id", next_working_memory_entry_id(existing_entries)),
                    ("entry_kind", entry_kind),
                    ("summary", summary),
                    ("source_refs", source_refs),
                    ("captured_at", captured_at),
                    ("expires_at", expires_at),
                    ("promoted_to", []),
                ]
            )
        )
        added += 1

    dump_list_document(path, header, "entries", existing_entries)
    return {
        "task_uid": task_uid,
        "role": role,
        "worktree_hint": worktree_hint,
        "path": str(path),
        "added": added,
        "skipped": skipped,
        "entry_count": len(existing_entries),
    }


def resolve_working_memory_context(
    root: pathlib.Path,
    task_uid: str,
    role: str | None,
) -> tuple[pathlib.Path, OrderedDict[str, object], list[OrderedDict[str, object]], str]:
    path, header, entries = load_working_memory_document(root, task_uid)
    resolved_role = str(role or header.get("role") or "")
    if resolved_role not in load_roles(root):
        raise ValueError(f"unknown role: {resolved_role}")
    return path, header, entries, resolved_role


def plan_working_memory_signal_promotions(
    root: pathlib.Path,
    task_uid: str,
    role: str | None,
    entry_ids: list[str],
    severity: str,
) -> dict[str, object]:
    if severity not in SEVERITY_ORDER:
        raise ValueError(f"unsupported severity: {severity}")
    if not entry_ids:
        raise ValueError("at least one --entry-id is required")

    path, header, entries, resolved_role = resolve_working_memory_context(root, task_uid, role)
    entries_by_id = {str(entry.get("entry_id") or ""): entry for entry in entries}
    signal_entries = load_signal_entries(root)
    relative_source_path = str(path.relative_to(root))
    plans: list[dict[str, object]] = []

    for entry_id in entry_ids:
        entry = entries_by_id.get(entry_id)
        if entry is None:
            raise ValueError(f"working_memory entry not found: {entry_id}")

        source_ref = f"{relative_source_path}#entry_id={entry_id}"
        summary = str(entry.get("summary") or "")
        existing_signal = None
        for payload in signal_entries:
            if (
                payload.get("source_type") == "reflection"
                and payload.get("source_ref") == source_ref
                and payload.get("summary") == summary
                and payload.get("role_hint") == resolved_role
            ):
                existing_signal = payload
                break

        plan: dict[str, object] = {
            "entry_id": entry_id,
            "source_ref": source_ref,
            "summary": summary,
        }
        if existing_signal is not None:
            plan["decision"] = "reuse"
            plan["signal_id"] = str(existing_signal["signal_id"])
        else:
            plan["decision"] = "create"
        plans.append(plan)

    return {
        "task_uid": task_uid,
        "role": resolved_role,
        "severity": severity,
        "working_memory_path": relative_source_path,
        "path": path,
        "header": header,
        "entries": entries,
        "plans": plans,
    }


def summarize_working_memory_signal_plan(
    plan: dict[str, object],
    *,
    applied: bool,
) -> dict[str, object]:
    created: list[dict[str, object]] = []
    reused: list[dict[str, object]] = []
    for item in plan["plans"]:
        shaped = {
            "entry_id": item["entry_id"],
            "source_ref": item["source_ref"],
            "summary": item["summary"],
        }
        if item["decision"] == "reuse":
            shaped["signal_id"] = item["signal_id"]
            reused.append(shaped)
        else:
            if "signal_id" in item:
                shaped["signal_id"] = item["signal_id"]
            created.append(shaped)

    return {
        "task_uid": plan["task_uid"],
        "role": plan["role"],
        "severity": plan["severity"],
        "working_memory_path": plan["working_memory_path"],
        "applied": applied,
        "created": created,
        "reused": reused,
    }


def apply_working_memory_signal_plan(plan: dict[str, object], root: pathlib.Path) -> dict[str, object]:
    path = plan["path"]
    header = plan["header"]
    entries = plan["entries"]
    entries_by_id = {str(entry.get("entry_id") or ""): entry for entry in entries}
    signal_entries = load_signal_entries(root)

    for item in plan["plans"]:
        if item["decision"] == "reuse":
            signal_id = str(item["signal_id"])
            entry = entries_by_id[str(item["entry_id"])]
            if signal_id not in entry["promoted_to"]:
                entry["promoted_to"].append(signal_id)
            continue

        signal_id = next_signal_id(root)
        payload = OrderedDict(
            [
                ("signal_id", signal_id),
                ("source_type", "reflection"),
                ("source_ref", item["source_ref"]),
                ("role_hint", plan["role"]),
                ("severity", plan["severity"]),
                ("summary", item["summary"]),
                ("promotion_state", "triaged"),
                ("memory_promotion_state", "pending"),
            ]
        )
        signal_entries.append(payload)
        item["signal_id"] = signal_id
        entry = entries_by_id[str(item["entry_id"])]
        if signal_id not in entry["promoted_to"]:
            entry["promoted_to"].append(signal_id)

    dump_signal_entries(root, signal_entries)
    dump_list_document(path, header, "entries", entries)
    return summarize_working_memory_signal_plan(plan, applied=True)


def next_signal_id(root: pathlib.Path) -> str:
    max_sequence = 0
    for payload in load_signal_entries(root):
        signal_id = str(payload.get("signal_id") or "")
        match = re.fullmatch(r"SIG-PM-(\d{4})", signal_id)
        if match:
            max_sequence = max(max_sequence, int(match.group(1)))
    return f"SIG-PM-{max_sequence + 1:04d}"


def next_task_uid(root: pathlib.Path) -> str:
    while True:
        task_uid = generate_task_uid()
        if not task_file_path(root, task_uid).exists():
            return task_uid


def create_candidate_task(
    root: pathlib.Path,
    owner_role: str,
    title: str,
    priority: str,
    source_signal: str | None,
    source_refs: list[str],
    doc_refs: list[str],
    related_prd: list[str],
    acceptance: list[str],
    handoff_to: list[str],
    worktree_hint: str | None,
) -> dict[str, object]:
    roles = load_roles(root)
    if owner_role not in roles:
        raise ValueError(f"unknown owner role: {owner_role}")
    for role in handoff_to:
        if role not in roles:
            raise ValueError(f"unknown handoff role: {role}")
    if priority not in PRIORITY_ORDER:
        raise ValueError(f"unsupported priority: {priority}")
    if not source_refs:
        raise ValueError("task source_refs must be a non-empty list")
    for source_ref in source_refs:
        validate_runtime_source_ref(root, str(source_ref), "task source_ref")

    task_uid = next_task_uid(root)
    task_path_rel = task_relative_path(task_uid)
    execution_log_path_rel = task_execution_log_relative_path(task_uid)
    task_path = task_file_path(root, task_uid)
    if task_path.exists():
        raise ValueError(f"task file already exists: {task_path_rel}")

    updated_at = now_iso()
    task_fields = OrderedDict(
        [
            ("task_uid", task_uid),
            ("title", title),
            ("owner_role", owner_role),
            ("worktree_hint", worktree_hint),
            ("execution_log_path", execution_log_path_rel),
            ("status", "candidate"),
            ("priority", priority),
            ("source_signal", source_signal),
            ("source_refs", list(source_refs)),
            ("doc_refs", list(doc_refs)),
            ("related_prd", list(related_prd)),
            ("acceptance", list(acceptance)),
            ("handoff_to", list(handoff_to)),
            ("updated_at", updated_at),
        ]
    )
    dump_mapping_document(task_path, task_fields)
    init_task_execution_log(root, task_uid, title, owner_role, worktree_hint, path_rel=execution_log_path_rel)
    rebuild_task_views(root)

    return {
        "task_uid": task_uid,
        "task_path": task_path_rel,
        "execution_log_path": execution_log_path_rel,
        "backlog_path": f".pm/roles/{owner_role}/backlog/candidate.yaml",
        "owner_role": owner_role,
        "priority": priority,
        "status": "candidate",
        "source_signal": source_signal,
        "updated_at": updated_at,
    }


def find_task_by_source_ref(root: pathlib.Path, source_ref: str) -> dict[str, object] | None:
    for task_path, fields in iter_task_files(root):
        if source_ref in [str(item) for item in fields.get("source_refs", [])]:
            return {
                "task_uid": fields.get("task_uid"),
                "task_path": str(task_path.relative_to(root)),
                "owner_role": fields.get("owner_role"),
                "status": fields.get("status"),
                "source_signal": fields.get("source_signal"),
            }
    return None


def ordered_task_fields(fields: OrderedDict[str, object], task_uid: str) -> OrderedDict[str, object]:
    rewritten = rewrite_object_strings(fields, {})
    ordered = OrderedDict()
    ordered["task_uid"] = task_uid
    ordered["title"] = rewritten.get("title")
    ordered["owner_role"] = rewritten.get("owner_role")
    ordered["worktree_hint"] = rewritten.get("worktree_hint")
    ordered["execution_log_path"] = task_execution_log_relative_path(task_uid)
    ordered["status"] = rewritten.get("status")
    ordered["priority"] = rewritten.get("priority")
    ordered["source_signal"] = rewritten.get("source_signal")
    ordered["source_refs"] = list(rewritten.get("source_refs", []))
    ordered["doc_refs"] = list(rewritten.get("doc_refs", []))
    ordered["related_prd"] = list(rewritten.get("related_prd", []))
    ordered["acceptance"] = list(rewritten.get("acceptance", []))
    ordered["handoff_to"] = list(rewritten.get("handoff_to", []))
    if "last_started_at" in rewritten:
        ordered["last_started_at"] = rewritten.get("last_started_at")
    if "last_closed_at" in rewritten:
        ordered["last_closed_at"] = rewritten.get("last_closed_at")
    ordered["updated_at"] = rewritten.get("updated_at")
    for key, value in rewritten.items():
        if key in ordered or key == "task_id":
            continue
        ordered[key] = value
    return ordered


def ordered_working_memory_header(header: OrderedDict[str, object], task_uid: str) -> OrderedDict[str, object]:
    rewritten = rewrite_object_strings(header, {})
    ordered = OrderedDict()
    ordered["version"] = rewritten.get("version", 1)
    ordered["task_uid"] = task_uid
    ordered["role"] = rewritten.get("role")
    ordered["worktree_hint"] = rewritten.get("worktree_hint")
    for key in (
        "source_session_id",
        "source_thread_name",
        "transcript_source",
        "last_extracted_ts",
        "captured_until_ts",
    ):
        if key in rewritten:
            ordered[key] = rewritten.get(key)
    for key, value in rewritten.items():
        if key in ordered or key == "task_id":
            continue
        ordered[key] = value
    return ordered


def migrate_task_identity(root: pathlib.Path) -> dict[str, object]:
    # Legacy `task_id` compatibility is migration-only; runtime commands use `task_uid` exclusively.
    task_records = [(path, load_mapping_document(path)) for path in sorted((root / ".pm/tasks").glob("*.yaml"))]
    if not task_records:
        rebuild_task_views(root)
        return {"migrated": 0, "task_count": 0, "mapping": OrderedDict()}

    id_mapping: OrderedDict[str, str] = OrderedDict()
    seen_task_uids: set[str] = set()
    for path, fields in task_records:
        task_uid = str(fields.get("task_uid") or "")
        task_id = str(fields.get("task_id") or "")
        if task_uid:
            if not TASK_UID_RE.fullmatch(task_uid):
                raise ValueError(f"invalid task_uid in task file: {path.relative_to(root)} -> {task_uid}")
            seen_task_uids.add(task_uid)
            continue
        if not task_id:
            raise ValueError(f"task file missing task_uid/task_id: {path.relative_to(root)}")
        migrated_uid = generate_task_uid(f"legacy:{task_id}")
        if migrated_uid in seen_task_uids:
            raise ValueError(f"duplicate migrated task_uid: {task_id} -> {migrated_uid}")
        id_mapping[task_id] = migrated_uid
        seen_task_uids.add(migrated_uid)

    if not id_mapping:
        rebuild_task_views(root)
        return {"migrated": 0, "task_count": len(task_records), "mapping": OrderedDict()}

    replacements: OrderedDict[str, str] = OrderedDict()
    for task_id, task_uid in id_mapping.items():
        replacements[f".pm/tasks/{task_id}.execution.md"] = task_execution_log_relative_path(task_uid)
        replacements[f".pm/tasks/{task_id}.yaml"] = task_relative_path(task_uid)
        replacements[f".pm/working_memory/{task_id}.yaml"] = working_memory_relative_path(task_uid)
        replacements[task_id] = task_uid

    stale_paths: list[pathlib.Path] = []

    for original_path, original_fields in task_records:
        current_task_uid = str(original_fields.get("task_uid") or "") or id_mapping[str(original_fields.get("task_id"))]
        rewritten_fields = rewrite_object_strings(original_fields, replacements)
        ordered_fields = ordered_task_fields(rewritten_fields, current_task_uid)
        new_path = task_file_path(root, current_task_uid)
        dump_mapping_document(new_path, ordered_fields)
        if new_path != original_path:
            stale_paths.append(original_path)

        old_log_rel = str(
            original_fields.get("execution_log_path")
            or task_execution_log_relative_path(str(original_fields.get("task_id") or current_task_uid))
        )
        old_log_path = root / old_log_rel
        new_log_path = root / task_execution_log_relative_path(current_task_uid)
        if old_log_path.exists():
            lines = replace_text_tokens(old_log_path.read_text(encoding="utf-8"), replacements).splitlines()
            if lines:
                lines[0] = f"# {current_task_uid} Execution Log"
            else:
                lines = [f"# {current_task_uid} Execution Log", ""]
            metadata_updated = False
            for index, line in enumerate(lines):
                if line.startswith("- task_id:") or line.startswith("- task_uid:"):
                    lines[index] = f"- task_uid: {current_task_uid}"
                    metadata_updated = True
                    break
            if not metadata_updated:
                lines.insert(2 if len(lines) > 1 else 1, f"- task_uid: {current_task_uid}")
            new_log_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
            if new_log_path != old_log_path:
                stale_paths.append(old_log_path)

    working_memory_root = working_memory_dir(root)
    if working_memory_root.exists():
        for path in sorted(working_memory_root.glob("*.yaml")):
            header, entries = load_list_document(path, "entries")
            legacy_task_id = str(header.get("task_id") or "")
            task_uid = str(header.get("task_uid") or "") or id_mapping.get(legacy_task_id, "")
            if not task_uid:
                raise ValueError(f"working_memory missing task identity: {path.relative_to(root)}")
            rewritten_header = ordered_working_memory_header(rewrite_object_strings(header, replacements), task_uid)
            rewritten_entries = [rewrite_object_strings(entry, replacements) for entry in entries]
            new_path = working_memory_path(root, task_uid)
            dump_list_document(new_path, rewritten_header, "entries", rewritten_entries)
            if new_path != path:
                stale_paths.append(path)

    for path, _, _, header, records in collect_memory_documents(root):
        rewritten_header = rewrite_object_strings(header, replacements)
        rewritten_records = [rewrite_object_strings(record, replacements) for record in records]
        dump_list_document(path, rewritten_header, "records", rewritten_records)

    codex_sessions_path, codex_sessions_header, codex_sessions_entries = load_codex_sessions_registry(root)
    rewritten_sessions: list[OrderedDict[str, object]] = []
    for entry in codex_sessions_entries:
        legacy_task_id = str(entry.get("task_id") or "")
        task_uid = str(entry.get("task_uid") or "") or id_mapping.get(legacy_task_id, "")
        if not task_uid:
            raise ValueError(f"codex session entry missing task identity: {entry}")
        rewritten_entry = OrderedDict()
        rewritten_entry["task_uid"] = task_uid
        for key, value in rewrite_object_strings(entry, replacements).items():
            if key == "task_id":
                continue
            if key == "task_uid":
                continue
            rewritten_entry[key] = value
        rewritten_sessions.append(rewritten_entry)
    dump_list_document(codex_sessions_path, codex_sessions_header, "sessions", rewritten_sessions)

    for stage_path in (root / ".pm/stage/current.yaml", root / ".pm/stage/gate.yaml"):
        stage_payload = rewrite_object_strings(load_mapping_document(stage_path), replacements)
        dump_mapping_document(stage_path, stage_payload)

    rewritten_signals = [rewrite_object_strings(payload, replacements) for payload in load_signal_entries(root)]
    dump_signal_entries(root, rewritten_signals)

    for stale_path in sorted(set(stale_paths), reverse=True):
        if stale_path.exists():
            stale_path.unlink()

    rebuild_task_views(root)
    return {
        "migrated": len(id_mapping),
        "task_count": len(task_records),
        "mapping": id_mapping,
    }


def promote_working_memory_to_signals(
    root: pathlib.Path,
    task_uid: str,
    role: str | None,
    entry_ids: list[str],
    severity: str,
) -> dict[str, object]:
    plan = plan_working_memory_signal_promotions(root, task_uid, role, entry_ids, severity)
    return apply_working_memory_signal_plan(plan, root)


def autoflow_working_memory(
    root: pathlib.Path,
    task_uid: str,
    role: str | None,
    entry_ids: list[str] | None,
    severity: str,
    priority: str,
    dry_run: bool,
) -> dict[str, object]:
    if severity not in SEVERITY_ORDER:
        raise ValueError(f"unsupported severity: {severity}")
    if priority not in PRIORITY_ORDER:
        raise ValueError(f"unsupported priority: {priority}")

    path, header, entries, resolved_role = resolve_working_memory_context(root, task_uid, role)

    selected_ids = set(entry_ids or [])
    selected_entries: list[OrderedDict[str, object]] = []
    for entry in entries:
        entry_id = str(entry.get("entry_id") or "")
        if selected_ids and entry_id not in selected_ids:
            continue
        selected_entries.append(entry)

    if entry_ids and len(selected_entries) != len(selected_ids):
        missing = sorted(selected_ids - {str(entry.get("entry_id") or "") for entry in selected_entries})
        raise ValueError(f"working_memory entries not found: {', '.join(missing)}")

    signal_candidates = [
        str(entry.get("entry_id"))
        for entry in selected_entries
        if str(entry.get("entry_kind") or "") in {"decision", "hypothesis", "open_question", "next_step"}
    ]

    signal_plan = plan_working_memory_signal_promotions(
        root,
        task_uid=task_uid,
        role=resolved_role,
        entry_ids=signal_candidates,
        severity=severity,
    ) if signal_candidates else None

    signal_result = summarize_working_memory_signal_plan(signal_plan, applied=False) if signal_plan is not None else {
        "task_uid": task_uid,
        "role": resolved_role,
        "severity": severity,
        "working_memory_path": str(path.relative_to(root)),
        "applied": False,
        "created": [],
        "reused": [],
    }

    signals_by_entry_id: dict[str, str] = {}
    for item in [*signal_result["created"], *signal_result["reused"]]:
        signal_id = item.get("signal_id")
        if signal_id:
            signals_by_entry_id[str(item["entry_id"])] = str(signal_id)

    if not dry_run and signal_plan is not None:
        signal_result = apply_working_memory_signal_plan(signal_plan, root)
        path, header, entries = load_working_memory_document(root, task_uid)
        signals_by_entry_id = {}
        for item in [*signal_result["created"], *signal_result["reused"]]:
            signal_id = item.get("signal_id")
            if signal_id:
                signals_by_entry_id[str(item["entry_id"])] = str(signal_id)

    task_actions: list[dict[str, object]] = []
    for entry in entries:
        entry_id = str(entry.get("entry_id") or "")
        if selected_ids and entry_id not in selected_ids:
            continue
        entry_kind = str(entry.get("entry_kind") or "")
        if entry_kind not in {"open_question", "next_step"}:
            continue

        source_ref = f"{path.relative_to(root)}#entry_id={entry_id}"
        existing_task = find_task_by_source_ref(root, source_ref)
        if existing_task is not None:
            if not dry_run and str(existing_task["task_uid"]) not in entry.get("promoted_to", []):
                entry["promoted_to"].append(str(existing_task["task_uid"]))
            task_actions.append(
                {
                    "entry_id": entry_id,
                    "decision": "reused",
                    "task": existing_task,
                }
            )
            continue

        if dry_run:
            task_actions.append(
                {
                    "entry_id": entry_id,
                    "decision": "would_create",
                    "task": {
                        "owner_role": resolved_role,
                        "priority": priority,
                        "title": str(entry.get("summary") or ""),
                        "source_signal": signals_by_entry_id.get(entry_id),
                        "source_ref": source_ref,
                    },
                }
            )
            continue

        created_task = create_candidate_task(
            root,
            owner_role=resolved_role,
            title=str(entry.get("summary") or ""),
            priority=priority,
            source_signal=signals_by_entry_id.get(entry_id),
            source_refs=[source_ref],
            doc_refs=[],
            related_prd=[],
            acceptance=[],
            handoff_to=[],
            worktree_hint=str(header.get("worktree_hint") or "") or None,
        )
        signal_id = signals_by_entry_id.get(entry_id)
        if signal_id:
            signal_entries, signal_entry = find_signal_entry(root, signal_id)
            signal_entry["promotion_state"] = "promoted_candidate_task"
            dump_signal_entries(root, signal_entries)
        if str(created_task["task_uid"]) not in entry.get("promoted_to", []):
            entry["promoted_to"].append(str(created_task["task_uid"]))
        task_actions.append(
            {
                "entry_id": entry_id,
                "decision": "created",
                "task": created_task,
            }
        )

    if not dry_run:
        dump_list_document(path, header, "entries", entries)

    return {
        "task_uid": task_uid,
        "role": resolved_role,
        "severity": severity,
        "priority": priority,
        "dry_run": dry_run,
        "signal_result": signal_result,
        "task_actions": task_actions,
    }


def collect_memory_documents(root: pathlib.Path) -> list[tuple[pathlib.Path, str, str, OrderedDict[str, object], list[OrderedDict[str, object]]]]:
    documents: list[tuple[pathlib.Path, str, str, OrderedDict[str, object], list[OrderedDict[str, object]]]] = []
    for role in sorted(load_roles(root)):
        for kind in ("active", "superseded"):
            path = root / f".pm/roles/{role}/memory/{kind}.yaml"
            header, records = load_list_document(path, "records")
            documents.append((path, "role", role, header, records))
    for kind in ("active", "superseded"):
        path = root / f".pm/shared/memory/{kind}.yaml"
        header, records = load_list_document(path, "records")
        documents.append((path, "shared", "shared", header, records))
    return documents


def resolve_memory_documents(
    root: pathlib.Path,
    scope: str,
    role: str | None,
) -> tuple[pathlib.Path, pathlib.Path, str, str]:
    if scope == "shared":
        if role != "producer_system_designer":
            raise ValueError("shared memory promotion is restricted to producer_system_designer")
        return (
            root / ".pm/shared/memory/active.yaml",
            root / ".pm/shared/memory/superseded.yaml",
            "shared",
            "shared",
        )

    if not role:
        raise ValueError("--role is required when --scope=role")
    if role not in load_roles(root):
        raise ValueError(f"unknown role: {role}")
    return (
        root / f".pm/roles/{role}/memory/active.yaml",
        root / f".pm/roles/{role}/memory/superseded.yaml",
        role,
        role,
    )


def parse_timestamp(value: object) -> datetime:
    return datetime.fromisoformat(str(value))


def iter_memory_records(
    root: pathlib.Path,
    include_active: bool = True,
    include_superseded: bool = True,
):
    for path, scope_type, owner, header, records in collect_memory_documents(root):
        del header
        is_active = path.name == "active.yaml"
        if is_active and not include_active:
            continue
        if (not is_active) and not include_superseded:
            continue
        for record in records:
            yield path, scope_type, owner, record


def next_memory_id(root: pathlib.Path, record_owner: str) -> str:
    prefix = ROLE_MEMORY_PREFIXES.get(record_owner)
    if prefix is None:
        raise ValueError(f"missing memory prefix for owner: {record_owner}")

    max_sequence = 0
    for path, _, owner, _, records in collect_memory_documents(root):
        del path
        if owner != record_owner:
            continue
        for record in records:
            record_id = str(record.get("id") or "")
            match = re.fullmatch(rf"MEM-{prefix}-(\d{{4}})", record_id)
            if match:
                max_sequence = max(max_sequence, int(match.group(1)))
    return f"MEM-{prefix}-{max_sequence + 1:04d}"


def run_memory_lint(root: pathlib.Path) -> None:
    roles = load_roles(root)
    failures: list[str] = []
    all_memory_ids: dict[str, tuple[pathlib.Path, OrderedDict[str, object]]] = {}
    active_topics: dict[tuple[str, str], str] = {}

    def fail(message: str) -> None:
        failures.append(message)

    for path, scope_type, owner, header, records in collect_memory_documents(root):
        expected_kind = "memory_active" if path.name == "active.yaml" else "memory_superseded"
        if header.get("kind") != expected_kind:
            fail(f"{path.relative_to(root)} kind mismatch: {header.get('kind')} != {expected_kind}")
        if scope_type == "role":
            if header.get("role") != owner:
                fail(f"{path.relative_to(root)} role header mismatch: {header.get('role')} != {owner}")
        else:
            if header.get("scope") != "shared":
                fail(f"{path.relative_to(root)} scope header mismatch: {header.get('scope')} != shared")

        for record in records:
            record_id = record.get("id")
            if not record_id:
                fail(f"{path.relative_to(root)} record missing id")
                continue
            if record_id in all_memory_ids:
                fail(f"duplicate memory id: {record_id}")
            else:
                all_memory_ids[str(record_id)] = (path, record)

            required = {
                "id",
                "topic",
                "summary",
                "source_refs",
                "effective_at",
                "last_reviewed_at",
                "status",
            }
            if expected_kind == "memory_active":
                required |= {"confidence", "promotion_reason"}
            else:
                required |= {"superseded_by", "superseded_at", "supersede_reason"}

            missing = sorted(key for key in required if key not in record)
            if missing:
                fail(f"{path.relative_to(root)} {record_id} missing fields: {', '.join(missing)}")
                continue

            if scope_type == "role":
                if record.get("role") != owner:
                    fail(f"{path.relative_to(root)} {record_id} role mismatch: {record.get('role')} != {owner}")
            elif "role" in record and record.get("role") not in {None, "shared"}:
                fail(f"{path.relative_to(root)} {record_id} shared record has unexpected role: {record.get('role')}")

            if record.get("status") != ("active" if expected_kind == "memory_active" else "superseded"):
                fail(f"{path.relative_to(root)} {record_id} status mismatch: {record.get('status')}")

            source_refs = record.get("source_refs")
            if not isinstance(source_refs, list) or not source_refs:
                fail(f"{path.relative_to(root)} {record_id} source_refs must be a non-empty list")
            else:
                for source_ref in source_refs:
                    source_path = parse_reference_path(str(source_ref))
                    if not source_path:
                        fail(f"{path.relative_to(root)} {record_id} has empty source_ref")
                        continue
                    if is_devlog_archive_reference(str(source_ref)):
                        fail(
                            f"{path.relative_to(root)} {record_id} source_ref must not use doc/devlog archive: "
                            f"{source_ref}"
                        )
                    if not (root / source_path).exists():
                        fail(f"{path.relative_to(root)} {record_id} source_ref missing: {source_path}")

            for key in ("effective_at", "last_reviewed_at"):
                try:
                    datetime.fromisoformat(str(record[key]))
                except ValueError:
                    fail(f"{path.relative_to(root)} {record_id} invalid timestamp: {key}={record[key]}")

            if expected_kind == "memory_superseded":
                try:
                    datetime.fromisoformat(str(record["superseded_at"]))
                except ValueError:
                    fail(f"{path.relative_to(root)} {record_id} invalid superseded_at={record['superseded_at']}")

            promotion_reason = record.get("promotion_reason")
            if promotion_reason is not None and promotion_reason not in ALLOWED_PROMOTION_REASONS:
                fail(f"{path.relative_to(root)} {record_id} invalid promotion_reason: {promotion_reason}")

            if expected_kind == "memory_active":
                topic_key = (owner, str(record["topic"]))
                if topic_key in active_topics:
                    fail(
                        f"active memory topic conflict for {owner}/{record['topic']}: "
                        f"{active_topics[topic_key]} and {record_id}"
                    )
                else:
                    active_topics[topic_key] = str(record_id)

    for record_id, (path, record) in all_memory_ids.items():
        if record.get("status") != "superseded":
            continue
        superseded_by = record.get("superseded_by")
        if not superseded_by:
            fail(f"{path.relative_to(root)} {record_id} missing superseded_by")
            continue
        if superseded_by == record_id:
            fail(f"{path.relative_to(root)} {record_id} superseded_by points to itself")
            continue
        if superseded_by not in all_memory_ids:
            fail(f"{path.relative_to(root)} {record_id} superseded_by missing target: {superseded_by}")

    if failures:
        for failure in failures:
            print(f"memory-lint: FAIL: {failure}", file=sys.stderr)
        raise SystemExit(1)


def build_memory_report(
    root: pathlib.Path,
    role_filter: str | None,
    include_shared: bool,
    stale_after_days: int,
) -> dict[str, object]:
    return build_memory_report_helper(
        root,
        role_filter,
        include_shared,
        stale_after_days,
        now_iso=now_iso,
        load_roles=load_roles,
        parse_timestamp=parse_timestamp,
        iter_memory_records=iter_memory_records,
    )


def build_role_report(root: pathlib.Path, role_filter: str | None, stale_after_days: int) -> dict[str, object]:
    return build_role_report_helper(
        root,
        role_filter,
        stale_after_days,
        now_iso=now_iso,
        sync_task_views=sync_task_views,
        load_roles=load_roles,
        build_memory_report_impl=build_memory_report,
        load_list_document=load_list_document,
        task_registry_path=task_registry_path,
        load_mapping_document=load_mapping_document,
        role_backlog_path=role_backlog_path,
        priority_order=PRIORITY_ORDER,
    )


def build_signal_summary(root: pathlib.Path, role_filter: str | None) -> dict[str, object]:
    return build_signal_summary_helper(
        root,
        role_filter,
        load_roles=load_roles,
        load_signal_entries=load_signal_entries,
        severity_order=SEVERITY_ORDER,
    )


def build_reflection_summary(root: pathlib.Path, role_filter: str | None) -> dict[str, object]:
    return build_reflection_summary_helper(
        root,
        role_filter,
        sync_task_views=sync_task_views,
        load_roles=load_roles,
        load_list_document=load_list_document,
        task_registry_path=task_registry_path,
        load_mapping_document=load_mapping_document,
        load_signal_entries=load_signal_entries,
        severity_order=SEVERITY_ORDER,
    )


def build_workflow_checklist(
    role: str,
    phase: str,
    task_context: dict[str, object] | None,
    role_payload: dict[str, object],
    signal_summary: dict[str, object],
    stage_report: dict[str, object],
    working_memory_summary: dict[str, object],
    reflection_summary: dict[str, object],
) -> list[OrderedDict[str, object]]:
    return build_workflow_checklist_helper(
        role,
        phase,
        task_context,
        role_payload,
        signal_summary,
        stage_report,
        working_memory_summary,
        reflection_summary,
    )


def build_workflow_report(
    root: pathlib.Path,
    role: str,
    phase: str,
    stale_after_days: int,
    task_uid: str | None,
) -> dict[str, object]:
    return build_workflow_report_helper(
        root,
        role,
        phase,
        stale_after_days,
        task_uid,
        now_iso=now_iso,
        load_roles=load_roles,
        load_task_context=load_task_context,
        build_role_report_impl=build_role_report,
        build_stage_report_impl=build_stage_report,
        build_signal_summary_impl=build_signal_summary,
        build_working_memory_report=build_working_memory_report,
        build_reflection_summary_impl=build_reflection_summary,
        build_workflow_checklist_impl=build_workflow_checklist,
        record_task_workflow_phase=record_task_workflow_phase,
    )


def run_task_backlog_lint(root: pathlib.Path) -> None:
    run_task_backlog_lint_helper(
        root,
        sync_task_views=sync_task_views,
        load_roles=load_roles,
        collect_signals=collect_signals,
        is_devlog_archive_reference=is_devlog_archive_reference,
        resolve_source_ref_path=resolve_source_ref_path,
        parse_reference_path=parse_reference_path,
        load_list_document=load_list_document,
        task_registry_path=task_registry_path,
        load_mapping_document=load_mapping_document,
        task_execution_log_relative_path=task_execution_log_relative_path,
        role_backlog_path=role_backlog_path,
        backlog_file_for_status=backlog_file_for_status,
        task_execution_log_entry_re=TASK_EXECUTION_LOG_ENTRY_RE,
        allowed_signal_states=ALLOWED_SIGNAL_STATES,
        allowed_memory_promotion_states=ALLOWED_MEMORY_PROMOTION_STATES,
        allowed_promotion_reasons=ALLOWED_PROMOTION_REASONS,
        allowed_memory_rejection_reasons=ALLOWED_MEMORY_REJECTION_REASONS,
        task_statuses=TASK_STATUSES,
    )


def run_stage_lint(root: pathlib.Path) -> None:
    run_stage_lint_helper(
        root,
        sync_task_views=sync_task_views,
        load_mapping_document=load_mapping_document,
        load_list_document=load_list_document,
        task_registry_path=task_registry_path,
        load_active_memory_record=load_active_memory_record,
        validate_runtime_source_ref=validate_runtime_source_ref,
    )


def cmd_memory_lint(args: argparse.Namespace) -> int:
    run_memory_lint(args.root)
    print("memory-lint: OK")
    return 0


def cmd_working_memory_lint(args: argparse.Namespace) -> int:
    run_working_memory_lint(args.root)
    print("working-memory-lint: OK")
    return 0


def cmd_task_lint(args: argparse.Namespace) -> int:
    run_task_backlog_lint(args.root)
    return 0


def cmd_task_execution_log_lint(args: argparse.Namespace) -> int:
    run_task_backlog_lint(args.root)
    print("task-execution-log-lint: OK")
    return 0


def cmd_sync_views(args: argparse.Namespace) -> int:
    result = sync_task_views(args.root)
    if args.json:
        print(json.dumps(result, ensure_ascii=False))
    else:
        print(
            "sync-views: refreshed "
            f"{result['task_registry_path']} from {result['task_count']} canonical task files"
        )
    return 0


def cmd_stage_lint(args: argparse.Namespace) -> int:
    return cmd_stage_lint_helper(args, run_stage_lint_impl=run_stage_lint)


def cmd_set_stage(args: argparse.Namespace) -> int:
    return cmd_set_stage_helper(
        args,
        validate_runtime_source_ref=validate_runtime_source_ref,
        load_mapping_document=load_mapping_document,
        dump_mapping_document=dump_mapping_document,
        run_stage_lint_impl=run_stage_lint,
    )


def cmd_new_task(args: argparse.Namespace) -> int:
    result = create_candidate_task(
        args.root,
        owner_role=args.owner_role,
        title=args.title,
        priority=args.priority,
        source_signal=args.source_signal,
        source_refs=list(args.source_ref),
        doc_refs=list(args.doc_ref),
        related_prd=list(args.related_prd),
        acceptance=list(args.acceptance),
        handoff_to=list(args.handoff_to),
        worktree_hint=args.worktree_hint,
    )
    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        print(f"new-task: created {result['task_uid']} ({result['task_path']})")
    return 0


def cmd_migrate_task_identity(args: argparse.Namespace) -> int:
    result = migrate_task_identity(args.root)
    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        print(
            "migrate-task-identity: "
            f"migrated={result['migrated']} task_count={result['task_count']}"
        )
    return 0


def cmd_move_task(args: argparse.Namespace) -> int:
    validate_status(args.to_status)
    root = args.root
    updated_at = now_iso()

    task_path, task_fields = find_task_file(root, args.task_uid)
    owner_role = str(task_fields.get("owner_role") or "")
    current_status = str(task_fields.get("status") or "")
    target_status = args.to_status
    task_fields["status"] = target_status
    task_fields["updated_at"] = updated_at

    dump_mapping_document(task_path, task_fields)
    rebuild_task_views(root)

    result = {
        "task_uid": args.task_uid,
        "owner_role": owner_role,
        "from_status": current_status,
        "to_status": target_status,
        "updated_at": updated_at,
    }
    if args.json:
        print(json.dumps(result, ensure_ascii=False))
    else:
        print(f"move-task: moved {args.task_uid} {current_status} -> {target_status}")
    return 0


def cmd_supersede_memory(args: argparse.Namespace) -> int:
    root = args.root
    updated_at = now_iso()

    if args.scope == "shared":
        active_path = root / ".pm/shared/memory/active.yaml"
        superseded_path = root / ".pm/shared/memory/superseded.yaml"
    else:
        if not args.role:
            raise ValueError("--role is required when --scope=role")
        active_path = root / f".pm/roles/{args.role}/memory/active.yaml"
        superseded_path = root / f".pm/roles/{args.role}/memory/superseded.yaml"

    active_header, active_records = load_list_document(active_path, "records")
    superseded_header, superseded_records = load_list_document(superseded_path, "records")

    target_record: OrderedDict[str, object] | None = None
    for record in active_records:
        if record.get("id") == args.memory_id:
            target_record = record
            break
    if target_record is None:
        raise ValueError(f"active memory not found: {args.memory_id}")

    if any(record.get("id") == args.memory_id for record in superseded_records):
        raise ValueError(f"memory already exists in superseded file: {args.memory_id}")

    active_records.remove(target_record)
    moved_record = OrderedDict(target_record)
    moved_record["status"] = "superseded"
    moved_record["last_reviewed_at"] = updated_at
    moved_record["superseded_by"] = args.superseded_by
    moved_record["superseded_at"] = updated_at
    moved_record["supersede_reason"] = args.supersede_reason
    superseded_records.append(moved_record)

    dump_list_document(active_path, active_header, "records", active_records)
    dump_list_document(superseded_path, superseded_header, "records", superseded_records)

    result = {
        "memory_id": args.memory_id,
        "superseded_by": args.superseded_by,
        "supersede_reason": args.supersede_reason,
        "superseded_at": updated_at,
        "scope": args.scope,
        "role": args.role,
    }
    if args.json:
        print(json.dumps(result, ensure_ascii=False))
    else:
        print(f"supersede-memory: moved {args.memory_id} to superseded")
    return 0


def cmd_memory_report(args: argparse.Namespace) -> int:
    return cmd_memory_report_helper(args, load_roles=load_roles, build_memory_report_impl=build_memory_report)


def cmd_working_memory_report(args: argparse.Namespace) -> int:
    return cmd_working_memory_report_helper(args, build_working_memory_report=build_working_memory_report)


def cmd_reflection_report(args: argparse.Namespace) -> int:
    return cmd_reflection_report_helper(args, build_reflection_summary_impl=build_reflection_summary)


def cmd_role_report(args: argparse.Namespace) -> int:
    return cmd_role_report_helper(args, build_role_report_impl=build_role_report)


def cmd_workflow_report(args: argparse.Namespace) -> int:
    return cmd_workflow_report_helper(args, build_workflow_report_impl=build_workflow_report)


def cmd_codex_transcript_report(args: argparse.Namespace) -> int:
    codex_dir = pathlib.Path(args.codex_dir).expanduser()
    session_id, resolution_source, resolved_thread_name = resolve_codex_session_id(
        args.root,
        codex_dir=codex_dir,
        session_id=args.session_id,
        task_uid=args.task_uid,
        worktree_hint=args.worktree_hint,
        thread_name_pattern=args.thread_name_pattern,
    )
    report = build_codex_transcript_report(
        codex_dir=codex_dir,
        session_id=session_id,
        after_ts=args.after_ts,
        before_ts=args.before_ts,
    )
    report["resolution_source"] = resolution_source
    if resolved_thread_name and not report["session"].get("thread_name"):
        report["session"]["thread_name"] = resolved_thread_name
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
        return 0

    lines = [
        "oasis7 codex transcript report",
        f"- generated_at: {report['generated_at']}",
        f"- codex_dir: {report['codex_dir']}",
        f"- resolution_source: {report.get('resolution_source')}",
        f"- session_id: {report['session']['id']}",
        f"- thread_name: {report['session'].get('thread_name') or '(none)'}",
        f"- updated_at: {report['session'].get('updated_at')}",
        f"- message_count: {report['message_count']}",
        f"- redaction_count: {report['redaction_count']}",
        "- messages:",
    ]
    if report["messages"]:
        for item in report["messages"][:10]:
            lines.append(f"  - ts={item['ts']} / {item['text']}")
    else:
        lines.append("  - (none)")

    print("\n".join(lines))
    return 0


def cmd_import_working_memory(args: argparse.Namespace) -> int:
    payload = json.loads(pathlib.Path(args.input_json).read_text(encoding="utf-8"))
    entries = payload.get("entries")
    if not isinstance(entries, list):
        raise ValueError("input json must contain an entries list")

    result = import_working_memory_entries(
        args.root,
        task_uid=args.task_uid,
        role=args.role,
        worktree_hint=args.worktree_hint,
        entries_payload=entries,
        expires_days=args.expires_days,
        source_session_id=args.session_id,
        source_thread_name=args.thread_name,
        transcript_source=args.transcript_source,
        last_extracted_ts=args.captured_until_ts,
        captured_until_ts=args.captured_until_ts,
    )
    if args.session_id:
        codex_mapping = remember_codex_session(
            args.root,
            task_uid=args.task_uid,
            role=args.role,
            session_id=args.session_id,
            thread_name=args.thread_name,
            worktree_hint=args.worktree_hint,
            codex_dir=pathlib.Path(args.codex_dir).expanduser(),
            updated_at=args.mapping_updated_at or now_iso(),
        )
        result["codex_session_mapping"] = codex_mapping
    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        print(
            f"import-working-memory: wrote {result['added']} entries to {result['path']} "
            f"(skipped {result['skipped']})"
        )
    return 0


def cmd_promote_working_memory_signal(args: argparse.Namespace) -> int:
    result = promote_working_memory_to_signals(
        args.root,
        task_uid=args.task_uid,
        role=args.role,
        entry_ids=list(args.entry_id),
        severity=args.severity,
    )
    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        print(
            "promote-working-memory-signal: "
            f"created={len(result['created'])} reused={len(result['reused'])} "
            f"path={result['working_memory_path']}"
        )
    return 0


def cmd_working_memory_autoflow(args: argparse.Namespace) -> int:
    result = autoflow_working_memory(
        args.root,
        task_uid=args.task_uid,
        role=args.role,
        entry_ids=list(args.entry_id),
        severity=args.severity,
        priority=args.priority,
        dry_run=args.dry_run,
    )
    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        print(
            "working-memory-autoflow: "
            f"mode={'plan' if args.dry_run else 'apply'} "
            f"signals(created={len(result['signal_result']['created'])}, reused={len(result['signal_result']['reused'])}) "
            f"tasks={len(result['task_actions'])} dry_run={'yes' if result['dry_run'] else 'no'}"
        )
    return 0


def cmd_promote_memory(args: argparse.Namespace) -> int:
    root = args.root
    updated_at = args.effective_at or now_iso()
    signal_entries, signal_entry = find_signal_entry(root, args.signal_id)

    memory_state = str(signal_entry.get("memory_promotion_state", "pending"))
    if memory_state != "pending":
        raise ValueError(f"signal already decided for memory promotion: {args.signal_id} -> {memory_state}")

    if args.reject_reason:
        if args.reject_reason not in ALLOWED_MEMORY_REJECTION_REASONS:
            raise ValueError(f"unsupported rejection reason: {args.reject_reason}")
        signal_entry["memory_promotion_state"] = "rejected"
        signal_entry["memory_rejection_reason"] = args.reject_reason
        signal_entry["memory_decision_at"] = updated_at
        dump_signal_entries(root, signal_entries)
        result = {
            "signal_id": args.signal_id,
            "decision": "rejected",
            "rejection_reason": args.reject_reason,
            "decided_at": updated_at,
        }
        if args.json:
            print(json.dumps(result, ensure_ascii=False))
        else:
            print(f"promote-memory: rejected {args.signal_id} ({args.reject_reason})")
        return 0

    if args.defer_reason:
        signal_entry["memory_promotion_state"] = "deferred"
        signal_entry["memory_deferred_reason"] = args.defer_reason
        signal_entry["memory_decision_at"] = updated_at
        dump_signal_entries(root, signal_entries)
        result = {
            "signal_id": args.signal_id,
            "decision": "deferred",
            "defer_reason": args.defer_reason,
            "decided_at": updated_at,
        }
        if args.json:
            print(json.dumps(result, ensure_ascii=False))
        else:
            print(f"promote-memory: deferred {args.signal_id} ({args.defer_reason})")
        return 0

    if args.promotion_reason not in ALLOWED_PROMOTION_REASONS:
        raise ValueError(f"unsupported promotion_reason: {args.promotion_reason}")
    if not args.topic:
        raise ValueError("--topic is required when promoting memory")

    role = args.role or str(signal_entry["role_hint"])
    active_path, _, record_owner, record_role = resolve_memory_documents(root, args.scope, role)
    active_header, active_records = load_list_document(active_path, "records")

    if any(str(record.get("topic")) == args.topic for record in active_records):
        raise ValueError(f"active memory topic already exists for {record_owner}: {args.topic}")

    memory_id = args.memory_id or next_memory_id(root, record_owner)
    if any(str(record.get("id")) == memory_id for _, _, _, _, records in collect_memory_documents(root) for record in records):
        raise ValueError(f"memory id already exists: {memory_id}")

    source_refs: list[str] = []
    for source_ref in [str(signal_entry["source_ref"]), *args.source_ref]:
        ensure_non_devlog_runtime_source_ref(str(source_ref), "memory source_ref")
        if source_ref not in source_refs:
            source_refs.append(source_ref)

    summary = args.summary or str(signal_entry["summary"])
    record = OrderedDict(
        [
            ("id", memory_id),
            ("role", record_role),
            ("topic", args.topic),
            ("summary", summary),
            ("source_refs", source_refs),
            ("tags", list(args.tag)),
            ("effective_at", updated_at),
            ("last_reviewed_at", updated_at),
            ("status", "active"),
            ("confidence", args.confidence),
            ("promotion_reason", args.promotion_reason),
        ]
    )
    active_records.append(record)
    dump_list_document(active_path, active_header, "records", active_records)

    signal_entry["memory_promotion_state"] = "promoted"
    signal_entry["memory_decision_at"] = updated_at
    signal_entry["memory_id"] = memory_id
    signal_entry["memory_scope"] = args.scope
    signal_entry["memory_role"] = record_role
    signal_entry["memory_topic"] = args.topic
    signal_entry["memory_promotion_reason"] = args.promotion_reason
    dump_signal_entries(root, signal_entries)

    result = {
        "signal_id": args.signal_id,
        "decision": "promoted",
        "memory_id": memory_id,
        "scope": args.scope,
        "role": record_role,
        "topic": args.topic,
        "promotion_reason": args.promotion_reason,
        "effective_at": updated_at,
    }
    if args.json:
        print(json.dumps(result, ensure_ascii=False))
    else:
        print(f"promote-memory: wrote {memory_id} from {args.signal_id}")
    return 0


def build_stage_report(root: pathlib.Path) -> dict[str, object]:
    return build_stage_report_helper(
        root,
        sync_task_views=sync_task_views,
        load_list_document=load_list_document,
        task_registry_path=task_registry_path,
        load_roles=load_roles,
        role_backlog_path=role_backlog_path,
        load_mapping_document=load_mapping_document,
    )


def cmd_stage_report(args: argparse.Namespace) -> int:
    return cmd_stage_report_helper(args, build_stage_report_impl=build_stage_report)

def main() -> int:
    parser = build_parser(
        handlers={
            "cmd_sync_views": cmd_sync_views,
            "cmd_task_lint": cmd_task_lint,
            "cmd_task_execution_log_lint": cmd_task_execution_log_lint,
            "cmd_memory_lint": cmd_memory_lint,
            "cmd_working_memory_lint": cmd_working_memory_lint,
            "cmd_memory_report": cmd_memory_report,
            "cmd_new_task": cmd_new_task,
            "cmd_working_memory_report": cmd_working_memory_report,
            "cmd_reflection_report": cmd_reflection_report,
            "cmd_role_report": cmd_role_report,
            "cmd_workflow_report": cmd_workflow_report,
            "cmd_promote_memory": cmd_promote_memory,
            "cmd_move_task": cmd_move_task,
            "cmd_supersede_memory": cmd_supersede_memory,
            "cmd_stage_report": cmd_stage_report,
            "cmd_stage_lint": cmd_stage_lint,
            "cmd_set_stage": cmd_set_stage,
            "cmd_codex_transcript_report": cmd_codex_transcript_report,
            "cmd_import_working_memory": cmd_import_working_memory,
            "cmd_promote_working_memory_signal": cmd_promote_working_memory_signal,
            "cmd_working_memory_autoflow": cmd_working_memory_autoflow,
            "cmd_migrate_task_identity": cmd_migrate_task_identity,
        },
        default_memory_review_stale_days=DEFAULT_MEMORY_REVIEW_STALE_DAYS,
        default_working_memory_expires_days=DEFAULT_WORKING_MEMORY_EXPIRES_DAYS,
        priority_choices=tuple(PRIORITY_ORDER.keys()),
        task_statuses=tuple(sorted(TASK_STATUSES)),
    )
    args = parser.parse_args()
    try:
        return args.func(args)
    except ValueError as exc:
        die(str(exc))
    except FileNotFoundError as exc:
        die(str(exc))


if __name__ == "__main__":
    raise SystemExit(main())
