#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys
from collections import OrderedDict
from datetime import datetime, timedelta

SAFE_SCALAR_RE = re.compile(r"[A-Za-z0-9_.:/+-]+")
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


def parse_scalar(value: str):
    value = value.strip()
    if value == "null":
        return None
    if value == "true":
        return True
    if value == "false":
        return False
    if value.startswith('"'):
        return json.loads(value)
    return value


def format_scalar(value) -> str:
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    value = str(value)
    if SAFE_SCALAR_RE.fullmatch(value):
        return value
    return json.dumps(value, ensure_ascii=False)


def parse_key_value(text: str) -> tuple[str, str]:
    key, sep, value = text.partition(": ")
    if not sep:
        raise ValueError(f"invalid key/value line: {text!r}")
    return key, value


def load_list_document(path: pathlib.Path, list_key: str) -> tuple[OrderedDict[str, object], list[OrderedDict[str, object]]]:
    header: OrderedDict[str, object] = OrderedDict()
    items: list[OrderedDict[str, object]] = []
    current: OrderedDict[str, object] | None = None
    active_list_key: str | None = None
    in_items = False

    for line_no, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not raw_line.strip():
            continue
        if not in_items:
            if raw_line == f"{list_key}: []":
                return header, []
            if raw_line == f"{list_key}:":
                in_items = True
                continue
            if raw_line.startswith(" "):
                raise ValueError(f"{path}:{line_no}: unexpected indentation before {list_key}")
            if raw_line.endswith(": []"):
                key = raw_line[:-4]
                header[key] = []
                continue
            key, value = parse_key_value(raw_line)
            header[key] = parse_scalar(value)
            continue

        if raw_line.startswith("  - "):
            if current is not None:
                items.append(current)
            current = OrderedDict()
            key, value = parse_key_value(raw_line[4:])
            current[key] = parse_scalar(value)
            active_list_key = None
            continue

        if raw_line.startswith("      - "):
            if current is None or active_list_key is None:
                raise ValueError(f"{path}:{line_no}: dangling nested list item")
            value = parse_scalar(raw_line[8:])
            current[active_list_key].append(value)
            continue

        if raw_line.startswith("    "):
            if current is None:
                raise ValueError(f"{path}:{line_no}: item field before item start")
            stripped = raw_line[4:]
            if stripped.endswith(": []"):
                key = stripped[:-4]
                current[key] = []
                active_list_key = None
                continue
            if stripped.endswith(":"):
                key = stripped[:-1]
                current[key] = []
                active_list_key = key
                continue
            key, value = parse_key_value(stripped)
            current[key] = parse_scalar(value)
            active_list_key = None
            continue

        raise ValueError(f"{path}:{line_no}: unsupported line: {raw_line!r}")

    if in_items and current is not None:
        items.append(current)
    return header, items


def dump_list_document(path: pathlib.Path, header: OrderedDict[str, object], list_key: str, items: list[OrderedDict[str, object]]) -> None:
    lines: list[str] = []
    for key, value in header.items():
        if isinstance(value, list):
            if value:
                raise ValueError(f"top-level lists other than {list_key} are not supported in {path}")
            lines.append(f"{key}: []")
        else:
            lines.append(f"{key}: {format_scalar(value)}")

    if not items:
        lines.append(f"{list_key}: []")
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return

    lines.append(f"{list_key}:")
    for item in items:
        first = True
        for key, value in item.items():
            prefix = "  - " if first else "    "
            if isinstance(value, list):
                if not value:
                    lines.append(f"{prefix}{key}: []")
                else:
                    lines.append(f"{prefix}{key}:")
                    for entry in value:
                        lines.append(f"      - {format_scalar(entry)}")
            else:
                lines.append(f"{prefix}{key}: {format_scalar(value)}")
            first = False

    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def load_mapping_document(path: pathlib.Path) -> OrderedDict[str, object]:
    data: OrderedDict[str, object] = OrderedDict()
    active_list_key: str | None = None
    for line_no, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not raw_line.strip():
            continue
        if raw_line.startswith("  - "):
            if active_list_key is None:
                raise ValueError(f"{path}:{line_no}: list item without list key")
            data[active_list_key].append(parse_scalar(raw_line[4:]))
            continue
        if raw_line.startswith(" "):
            raise ValueError(f"{path}:{line_no}: unsupported indentation in mapping doc")
        if raw_line.endswith(": []"):
            data[raw_line[:-4]] = []
            active_list_key = None
            continue
        if raw_line.endswith(":"):
            key = raw_line[:-1]
            data[key] = []
            active_list_key = key
            continue
        key, value = parse_key_value(raw_line)
        data[key] = parse_scalar(value)
        active_list_key = None
    return data


def dump_mapping_document(path: pathlib.Path, data: OrderedDict[str, object]) -> None:
    lines: list[str] = []
    for key, value in data.items():
        if isinstance(value, list):
            if not value:
                lines.append(f"{key}: []")
            else:
                lines.append(f"{key}:")
                for entry in value:
                    lines.append(f"  - {format_scalar(entry)}")
        else:
            lines.append(f"{key}: {format_scalar(value)}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


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


def find_registry_task(root: pathlib.Path, task_id: str) -> tuple[OrderedDict[str, object], list[OrderedDict[str, object]], OrderedDict[str, object], pathlib.Path]:
    registry_path = root / ".pm/registry/tasks.yaml"
    header, tasks = load_list_document(registry_path, "tasks")
    for entry in tasks:
        if entry.get("task_id") == task_id:
            return header, tasks, entry, registry_path
    raise ValueError(f"task not found in registry: {task_id}")


def load_task_context(root: pathlib.Path, task_id: str) -> dict[str, object]:
    _, _, registry_entry, _ = find_registry_task(root, task_id)
    task_path = root / str(registry_entry["task_path"])
    task_fields = load_mapping_document(task_path)
    return {
        "task_id": task_id,
        "owner_role": registry_entry.get("owner_role"),
        "status": task_fields.get("status"),
        "priority": task_fields.get("priority"),
        "title": task_fields.get("title"),
        "worktree_hint": task_fields.get("worktree_hint"),
        "execution_log_path": task_fields.get("execution_log_path"),
        "last_started_at": task_fields.get("last_started_at"),
        "last_closed_at": task_fields.get("last_closed_at"),
        "updated_at": task_fields.get("updated_at"),
    }


def task_execution_log_relative_path(task_id: str) -> str:
    return f".pm/tasks/{task_id}.execution.md"


def init_task_execution_log(
    root: pathlib.Path,
    task_id: str,
    title: str,
    owner_role: str,
    worktree_hint: str | None,
    *,
    path_rel: str | None = None,
) -> None:
    relative_path = path_rel or task_execution_log_relative_path(task_id)
    path = root / relative_path
    if path.exists():
        return
    path.write_text(
        "\n".join(
            [
                f"# {task_id} Execution Log",
                "",
                f"- task_id: {task_id}",
                f"- title: {title}",
                f"- owner_role: {owner_role}",
                f"- worktree_hint: {worktree_hint or 'null'}",
                "",
                "<!-- Append entries using:",
                "## YYYY-MM-DD HH:MM:SS CST / role_name",
                "- 完成内容: ...",
                "- 遗留事项: ...",
                "-->",
                "",
            ]
        ),
        encoding="utf-8",
    )


def record_task_workflow_phase(root: pathlib.Path, task_id: str, role: str, phase: str) -> dict[str, object]:
    if phase not in {"start", "close"}:
        raise ValueError(f"unsupported workflow record phase: {phase}")

    registry_header, registry_entries, registry_entry, registry_path = find_registry_task(root, task_id)
    owner_role = str(registry_entry["owner_role"])
    if owner_role != role:
        raise ValueError(f"task owner_role mismatch for workflow report: {task_id} -> {owner_role} != {role}")

    updated_at = now_iso()
    task_path = root / str(registry_entry["task_path"])
    task_fields = load_mapping_document(task_path)
    if phase == "start":
        task_fields["last_started_at"] = updated_at
    else:
        task_fields["last_closed_at"] = updated_at
    task_fields["updated_at"] = updated_at
    registry_entry["updated_at"] = updated_at

    dump_list_document(registry_path, registry_header, "tasks", registry_entries)
    dump_mapping_document(task_path, task_fields)
    return load_task_context(root, task_id)


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


def redact_text(text: str) -> tuple[str, int]:
    redacted = text
    replacements = 0
    for pattern, replacement in REDACTION_PATTERNS:
        redacted, count = pattern.subn(replacement, redacted)
        replacements += count
    return redacted, replacements


def working_memory_dir(root: pathlib.Path) -> pathlib.Path:
    return root / ".pm/working_memory"


def working_memory_path(root: pathlib.Path, task_id: str) -> pathlib.Path:
    return working_memory_dir(root) / f"{task_id}.yaml"


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
    task_id: str,
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
        if str(entry.get("task_id") or "") == task_id:
            continue
        retained.append(entry)

    retained.append(
        OrderedDict(
            [
                ("task_id", task_id),
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
        "task_id": task_id,
        "role": role,
        "session_id": session_id,
        "thread_name": thread_name,
        "worktree_hint": worktree_hint,
        "codex_dir": str(codex_dir),
        "updated_at": updated_at,
        "path": str(path),
    }


def resolve_task_worktree_hint(root: pathlib.Path, task_id: str | None) -> str | None:
    if not task_id:
        return None
    task_file = root / f".pm/tasks/{task_id}.yaml"
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
    task_id: str | None,
    worktree_hint: str | None,
    thread_name_pattern: str | None,
) -> tuple[str, str, str | None]:
    if session_id:
        metadata = load_codex_session_metadata(codex_dir, session_id)
        return session_id, "explicit", str(metadata.get("thread_name") or "")

    if task_id:
        _, _, registry_entries = load_codex_sessions_registry(root)
        for entry in reversed(registry_entries):
            if str(entry.get("task_id") or "") == task_id:
                resolved_session_id = str(entry.get("session_id") or "")
                if not resolved_session_id:
                    break
                metadata = load_codex_session_metadata(codex_dir, resolved_session_id)
                return resolved_session_id, "registry", str(metadata.get("thread_name") or "")

    derived_worktree_hint = worktree_hint or resolve_task_worktree_hint(root, task_id)
    pattern = thread_name_pattern or derived_worktree_hint
    if not pattern:
        raise ValueError("missing session resolution input: provide --session-id, --task-id with saved mapping, --worktree-hint, or --thread-name-pattern")

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
    task_id: str,
    role: str | None = None,
    worktree_hint: str | None = None,
) -> tuple[pathlib.Path, OrderedDict[str, object], list[OrderedDict[str, object]]]:
    path = working_memory_path(root, task_id)
    if path.exists():
        header, entries = load_list_document(path, "entries")
        return path, header, entries

    header: OrderedDict[str, object] = OrderedDict(
        [
            ("version", 1),
            ("task_id", task_id),
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
    task_id: str | None,
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
        current_task_id = str(header.get("task_id") or path.stem)
        current_role = str(header.get("role") or "")
        if task_id and current_task_id != task_id:
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

        task_payloads[current_task_id] = {
            "task_id": current_task_id,
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
        "task_filter": task_id,
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
        task_id = str(header.get("task_id") or "")
        role = header.get("role")
        if not task_id:
            fail(f"{path.relative_to(root)} missing task_id")
        elif path.name != f"{task_id}.yaml":
            fail(f"{path.relative_to(root)} filename/task_id mismatch: {path.name} != {task_id}.yaml")

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
    task_id: str,
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

    path, header, existing_entries = load_working_memory_document(root, task_id, role=role, worktree_hint=worktree_hint)
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
        "task_id": task_id,
        "role": role,
        "worktree_hint": worktree_hint,
        "path": str(path),
        "added": added,
        "skipped": skipped,
        "entry_count": len(existing_entries),
    }


def resolve_working_memory_context(
    root: pathlib.Path,
    task_id: str,
    role: str | None,
) -> tuple[pathlib.Path, OrderedDict[str, object], list[OrderedDict[str, object]], str]:
    path, header, entries = load_working_memory_document(root, task_id)
    resolved_role = str(role or header.get("role") or "")
    if resolved_role not in load_roles(root):
        raise ValueError(f"unknown role: {resolved_role}")
    return path, header, entries, resolved_role


def plan_working_memory_signal_promotions(
    root: pathlib.Path,
    task_id: str,
    role: str | None,
    entry_ids: list[str],
    severity: str,
) -> dict[str, object]:
    if severity not in SEVERITY_ORDER:
        raise ValueError(f"unsupported severity: {severity}")
    if not entry_ids:
        raise ValueError("at least one --entry-id is required")

    path, header, entries, resolved_role = resolve_working_memory_context(root, task_id, role)
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
        "task_id": task_id,
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
        "task_id": plan["task_id"],
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


def next_task_id(root: pathlib.Path) -> str:
    header, _ = load_list_document(root / ".pm/registry/tasks.yaml", "tasks")
    next_sequence = header.get("next_sequence")
    if next_sequence is None or not str(next_sequence).isdigit():
        raise ValueError("tasks registry missing numeric next_sequence")
    return f"TASK-PM-{int(next_sequence):04d}"


def create_candidate_task(
    root: pathlib.Path,
    owner_role: str,
    title: str,
    priority: str,
    source_signal: str | None,
    source_refs: list[str],
    related_prd: list[str],
    acceptance: list[str],
    handoff_to: list[str],
    worktree_hint: str | None,
) -> dict[str, object]:
    if owner_role not in load_roles(root):
        raise ValueError(f"unknown owner role: {owner_role}")
    if priority not in PRIORITY_ORDER:
        raise ValueError(f"unsupported priority: {priority}")

    registry_path = root / ".pm/registry/tasks.yaml"
    registry_header, registry_entries = load_list_document(registry_path, "tasks")
    next_sequence = registry_header.get("next_sequence")
    if next_sequence is None or not str(next_sequence).isdigit():
        raise ValueError("tasks registry missing numeric next_sequence")

    task_id = f"TASK-PM-{int(next_sequence):04d}"
    task_path_rel = f".pm/tasks/{task_id}.yaml"
    execution_log_path_rel = task_execution_log_relative_path(task_id)
    task_path = root / task_path_rel
    if task_path.exists():
        raise ValueError(f"task file already exists: {task_path_rel}")

    updated_at = now_iso()
    task_fields = OrderedDict(
        [
            ("task_id", task_id),
            ("title", title),
            ("owner_role", owner_role),
            ("worktree_hint", worktree_hint),
            ("execution_log_path", execution_log_path_rel),
            ("status", "candidate"),
            ("priority", priority),
            ("source_signal", source_signal),
            ("source_refs", list(source_refs)),
            ("doc_refs", []),
            ("related_prd", list(related_prd)),
            ("acceptance", list(acceptance)),
            ("handoff_to", list(handoff_to)),
            ("updated_at", updated_at),
        ]
    )
    dump_mapping_document(task_path, task_fields)
    init_task_execution_log(root, task_id, title, owner_role, worktree_hint, path_rel=execution_log_path_rel)

    registry_entries.append(
        OrderedDict(
            [
                ("task_id", task_id),
                ("owner_role", owner_role),
                ("task_path", task_path_rel),
                ("status", "candidate"),
                ("priority", priority),
                ("source_signal", source_signal),
                ("updated_at", updated_at),
            ]
        )
    )
    registry_header["next_sequence"] = int(next_sequence) + 1
    dump_list_document(registry_path, registry_header, "tasks", registry_entries)

    backlog_path = root / f".pm/roles/{owner_role}/backlog/candidate.yaml"
    backlog_header, backlog_entries = load_list_document(backlog_path, "tasks")
    backlog_entries.append(
        OrderedDict(
            [
                ("task_id", task_id),
                ("title", title),
                ("priority", priority),
                ("source_signal", source_signal),
                ("related_prd", list(related_prd)),
                ("acceptance", list(acceptance)),
                ("handoff_to", list(handoff_to)),
                ("status", "candidate"),
                ("task_path", task_path_rel),
            ]
        )
    )
    dump_list_document(backlog_path, backlog_header, "tasks", backlog_entries)

    return {
        "task_id": task_id,
        "task_path": task_path_rel,
        "execution_log_path": execution_log_path_rel,
        "backlog_path": str(backlog_path.relative_to(root)),
        "owner_role": owner_role,
        "priority": priority,
        "status": "candidate",
        "source_signal": source_signal,
        "updated_at": updated_at,
    }


def find_task_by_source_ref(root: pathlib.Path, source_ref: str) -> dict[str, object] | None:
    _, registry_entries = load_list_document(root / ".pm/registry/tasks.yaml", "tasks")
    for entry in registry_entries:
        task_path = root / str(entry.get("task_path") or "")
        if not task_path.exists():
            continue
        fields = load_mapping_document(task_path)
        if source_ref in [str(item) for item in fields.get("source_refs", [])]:
            return {
                "task_id": fields.get("task_id"),
                "task_path": str(entry.get("task_path")),
                "owner_role": fields.get("owner_role"),
                "status": fields.get("status"),
                "source_signal": fields.get("source_signal"),
            }
    return None


def promote_working_memory_to_signals(
    root: pathlib.Path,
    task_id: str,
    role: str | None,
    entry_ids: list[str],
    severity: str,
) -> dict[str, object]:
    plan = plan_working_memory_signal_promotions(root, task_id, role, entry_ids, severity)
    return apply_working_memory_signal_plan(plan, root)


def autoflow_working_memory(
    root: pathlib.Path,
    task_id: str,
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

    path, header, entries, resolved_role = resolve_working_memory_context(root, task_id, role)

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
        task_id=task_id,
        role=resolved_role,
        entry_ids=signal_candidates,
        severity=severity,
    ) if signal_candidates else None

    signal_result = summarize_working_memory_signal_plan(signal_plan, applied=False) if signal_plan is not None else {
        "task_id": task_id,
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
        path, header, entries = load_working_memory_document(root, task_id)
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
            if not dry_run and str(existing_task["task_id"]) not in entry.get("promoted_to", []):
                entry["promoted_to"].append(str(existing_task["task_id"]))
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
        if str(created_task["task_id"]) not in entry.get("promoted_to", []):
            entry["promoted_to"].append(str(created_task["task_id"]))
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
        "task_id": task_id,
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
    stale_cutoff = datetime.now().astimezone() - timedelta(days=stale_after_days)
    active_records: list[dict[str, object]] = []
    needs_review_records: list[dict[str, object]] = []
    superseded_records: list[dict[str, object]] = []
    role_summary: OrderedDict[str, OrderedDict[str, int]] = OrderedDict()
    eligible_roles = sorted(load_roles(root))
    if include_shared:
        eligible_roles.append("shared")

    def include_owner(owner: str) -> bool:
        if role_filter and owner != role_filter:
            return False
        if owner == "shared":
            return include_shared
        return True

    def ensure_role_summary(owner: str) -> OrderedDict[str, int]:
        if owner not in role_summary:
            role_summary[owner] = OrderedDict(
                [
                    ("active", 0),
                    ("needs_review", 0),
                    ("superseded", 0),
                ]
            )
        return role_summary[owner]

    for owner in eligible_roles:
        if include_owner(owner):
            ensure_role_summary(owner)

    def shape_record(owner: str, record: OrderedDict[str, object]) -> dict[str, object]:
        last_reviewed_at = parse_timestamp(record["last_reviewed_at"])
        review_state = "needs_review" if record.get("status") == "active" and last_reviewed_at <= stale_cutoff else "fresh"
        payload: OrderedDict[str, object] = OrderedDict(
            [
                ("id", record.get("id")),
                ("role", owner if owner != "shared" else "shared"),
                ("topic", record.get("topic")),
                ("status", record.get("status")),
                ("summary", record.get("summary")),
                ("effective_at", record.get("effective_at")),
                ("last_reviewed_at", record.get("last_reviewed_at")),
                ("review_state", review_state),
                ("source_refs", list(record.get("source_refs", []))),
                ("tags", list(record.get("tags", []))),
            ]
        )
        if record.get("status") == "active":
            payload["confidence"] = record.get("confidence")
            payload["promotion_reason"] = record.get("promotion_reason")
        else:
            payload["superseded_at"] = record.get("superseded_at")
            payload["superseded_by"] = record.get("superseded_by")
            payload["supersede_reason"] = record.get("supersede_reason")
        return payload

    for _, _, owner, record in iter_memory_records(root):
        if not include_owner(owner):
            continue
        shaped = shape_record(owner, record)
        summary = ensure_role_summary(owner)
        if record.get("status") == "active":
            active_records.append(shaped)
            summary["active"] += 1
            if shaped["review_state"] == "needs_review":
                needs_review_records.append(shaped)
                summary["needs_review"] += 1
        else:
            superseded_records.append(shaped)
            summary["superseded"] += 1

    active_records.sort(key=lambda item: (str(item["role"]), str(item["topic"]), str(item["effective_at"])), reverse=False)
    needs_review_records.sort(key=lambda item: str(item["last_reviewed_at"]))
    superseded_records.sort(key=lambda item: str(item.get("superseded_at") or ""), reverse=True)

    return {
        "generated_at": now_iso(),
        "stale_after_days": stale_after_days,
        "role_filter": role_filter,
        "include_shared": include_shared,
        "counts": {
            "active": len(active_records),
            "needs_review": len(needs_review_records),
            "superseded": len(superseded_records),
        },
        "roles": role_summary,
        "active": active_records,
        "needs_review": needs_review_records,
        "superseded": superseded_records,
    }


def task_sort_key(item: dict[str, object]) -> tuple[object, object, object]:
    return (
        PRIORITY_ORDER.get(str(item.get("priority")), 99),
        str(item.get("updated_at") or ""),
        str(item.get("task_id") or ""),
    )


def normalize_list_field(value: object) -> list[object]:
    if isinstance(value, list):
        return list(value)
    if value in {None, ""}:
        return []
    return [value]


def build_role_report(root: pathlib.Path, role_filter: str | None, stale_after_days: int) -> dict[str, object]:
    roles = sorted(load_roles(root))
    if role_filter and role_filter not in roles:
        raise ValueError(f"unknown role: {role_filter}")

    included_roles = [role_filter] if role_filter else roles
    memory_report = build_memory_report(
        root,
        role_filter=role_filter,
        include_shared=False,
        stale_after_days=stale_after_days,
    )

    registry_header, registry_entries = load_list_document(root / ".pm/registry/tasks.yaml", "tasks")
    del registry_header
    registry_by_id: dict[str, OrderedDict[str, object]] = {
        str(entry["task_id"]): entry for entry in registry_entries if entry.get("task_id")
    }
    task_field_cache: dict[str, OrderedDict[str, object]] = {}

    active_by_role: dict[str, list[dict[str, object]]] = {role: [] for role in included_roles}
    needs_review_by_role: dict[str, list[dict[str, object]]] = {role: [] for role in included_roles}
    superseded_by_role: dict[str, list[dict[str, object]]] = {role: [] for role in included_roles}

    for item in memory_report["active"]:
        active_by_role[str(item["role"])].append(item)
    for item in memory_report["needs_review"]:
        needs_review_by_role[str(item["role"])].append(item)
    for item in memory_report["superseded"]:
        superseded_by_role[str(item["role"])].append(item)

    def load_task_fields(task_path: str | None) -> OrderedDict[str, object]:
        if not task_path:
            return OrderedDict()
        if task_path not in task_field_cache:
            resolved = root / task_path
            task_field_cache[task_path] = load_mapping_document(resolved) if resolved.is_file() else OrderedDict()
        return task_field_cache[task_path]

    def shape_backlog_task(role: str, entry: OrderedDict[str, object]) -> dict[str, object]:
        del role
        task_id = str(entry.get("task_id") or "")
        registry_entry = registry_by_id.get(task_id, OrderedDict())
        task_path = str(entry.get("task_path") or registry_entry.get("task_path") or "")
        task_fields = load_task_fields(task_path)
        return {
            "task_id": task_id,
            "title": entry.get("title") or task_fields.get("title"),
            "priority": entry.get("priority") or registry_entry.get("priority") or task_fields.get("priority"),
            "status": entry.get("status") or task_fields.get("status") or registry_entry.get("status"),
            "source_signal": entry.get("source_signal") or task_fields.get("source_signal") or registry_entry.get("source_signal"),
            "task_path": task_path or None,
            "updated_at": task_fields.get("updated_at") or registry_entry.get("updated_at"),
            "related_prd": normalize_list_field(entry.get("related_prd") or task_fields.get("related_prd")),
            "acceptance": normalize_list_field(entry.get("acceptance") or task_fields.get("acceptance")),
            "handoff_to": normalize_list_field(entry.get("handoff_to") or task_fields.get("handoff_to")),
        }

    role_payloads: OrderedDict[str, dict[str, object]] = OrderedDict()
    backlog_totals: OrderedDict[str, int] = OrderedDict(
        (status, 0) for status in ("candidate", "committed", "blocked", "done", "deferred")
    )

    for role in included_roles:
        backlog_counts: OrderedDict[str, int] = OrderedDict(
            (status, 0) for status in ("candidate", "committed", "blocked", "done", "deferred")
        )
        tasks_by_status: OrderedDict[str, list[dict[str, object]]] = OrderedDict(
            (status, []) for status in ("candidate", "committed", "blocked", "done", "deferred")
        )

        for file_status in ("candidate", "committed", "blocked", "done"):
            path = root / f".pm/roles/{role}/backlog/{file_status}.yaml"
            _, entries = load_list_document(path, "tasks")
            for entry in entries:
                shaped = shape_backlog_task(role, entry)
                status = str(shaped.get("status") or file_status)
                if status not in tasks_by_status:
                    continue
                backlog_counts[status] += 1
                backlog_totals[status] += 1
                tasks_by_status[status].append(shaped)

        for task_items in tasks_by_status.values():
            task_items.sort(key=task_sort_key)

        role_payloads[role] = {
            "backlog_counts": backlog_counts,
            "memory_counts": OrderedDict(memory_report["roles"].get(role, {})),
            "tasks": tasks_by_status,
            "active_memory": active_by_role[role],
            "needs_review_memory": needs_review_by_role[role],
            "superseded_memory": superseded_by_role[role],
        }

    return {
        "generated_at": now_iso(),
        "stale_after_days": stale_after_days,
        "role_filter": role_filter,
        "role_count": len(included_roles),
        "backlog_totals": backlog_totals,
        "roles": role_payloads,
    }


def build_signal_summary(root: pathlib.Path, role_filter: str | None) -> dict[str, object]:
    roles = load_roles(root)
    if role_filter and role_filter not in roles:
        raise ValueError(f"unknown role: {role_filter}")

    promotion_counts: OrderedDict[str, int] = OrderedDict(
        (state, 0) for state in ("new", "triaged", "promoted_candidate_task", "discarded", "deferred")
    )
    memory_promotion_counts: OrderedDict[str, int] = OrderedDict(
        (state, 0) for state in ("pending", "promoted", "rejected", "deferred")
    )
    pending_signals: list[dict[str, object]] = []

    for payload in load_signal_entries(root):
        if role_filter and payload.get("role_hint") != role_filter:
            continue

        promotion_state = str(payload.get("promotion_state") or "new")
        if promotion_state in promotion_counts:
            promotion_counts[promotion_state] += 1

        memory_state = str(payload.get("memory_promotion_state") or "pending")
        if memory_state in memory_promotion_counts:
            memory_promotion_counts[memory_state] += 1

        # A signal stays pending until the operator has explicitly closed the
        # memory decision path. Rejected/deferred/promoted memory decisions
        # should no longer show up as pending workflow work.
        is_pending = memory_state == "pending" and promotion_state in {
            "new",
            "triaged",
            "promoted_candidate_task",
        }
        if is_pending:
            pending_signals.append(
                {
                    "signal_id": payload.get("signal_id"),
                    "role_hint": payload.get("role_hint"),
                    "severity": payload.get("severity"),
                    "source_type": payload.get("source_type"),
                    "source_ref": payload.get("source_ref"),
                    "summary": payload.get("summary"),
                    "promotion_state": promotion_state,
                    "memory_promotion_state": memory_state,
                }
            )

    pending_signals.sort(
        key=lambda item: (
            SEVERITY_ORDER.get(str(item.get("severity")), 99),
            str(item.get("signal_id") or ""),
        )
    )

    return {
        "role_filter": role_filter,
        "counts": promotion_counts,
        "memory_counts": memory_promotion_counts,
        "pending_count": len(pending_signals),
        "pending_signals": pending_signals,
    }


def build_reflection_summary(root: pathlib.Path, role_filter: str | None) -> dict[str, object]:
    roles = load_roles(root)
    if role_filter and role_filter not in roles:
        raise ValueError(f"unknown role: {role_filter}")

    _, registry_entries = load_list_document(root / ".pm/registry/tasks.yaml", "tasks")
    tasks_by_signal: dict[str, list[dict[str, object]]] = {}
    for entry in registry_entries:
        source_signal = str(entry.get("source_signal") or "")
        if not source_signal:
            continue
        task_path = root / str(entry.get("task_path") or "")
        fields = load_mapping_document(task_path) if task_path.exists() else OrderedDict()
        task_payload = {
            "task_id": fields.get("task_id") or entry.get("task_id"),
            "title": fields.get("title"),
            "owner_role": fields.get("owner_role") or entry.get("owner_role"),
            "status": fields.get("status") or entry.get("status"),
            "priority": fields.get("priority") or entry.get("priority"),
            "task_path": entry.get("task_path"),
        }
        tasks_by_signal.setdefault(source_signal, []).append(task_payload)

    counts = OrderedDict(
        [
            ("triaged", 0),
            ("promoted_candidate_task", 0),
            ("discarded", 0),
            ("deferred", 0),
        ]
    )
    items: list[dict[str, object]] = []
    for payload in load_signal_entries(root):
        if payload.get("source_type") != "reflection":
            continue
        if role_filter and payload.get("role_hint") != role_filter:
            continue
        promotion_state = str(payload.get("promotion_state") or "triaged")
        if promotion_state in counts:
            counts[promotion_state] += 1
        linked_tasks = list(tasks_by_signal.get(str(payload.get("signal_id") or ""), []))
        items.append(
            {
                "signal_id": payload.get("signal_id"),
                "role_hint": payload.get("role_hint"),
                "severity": payload.get("severity"),
                "promotion_state": promotion_state,
                "memory_promotion_state": payload.get("memory_promotion_state"),
                "source_ref": payload.get("source_ref"),
                "summary": payload.get("summary"),
                "linked_tasks": linked_tasks,
            }
        )

    items.sort(
        key=lambda item: (
            SEVERITY_ORDER.get(str(item.get("severity")), 99),
            str(item.get("signal_id") or ""),
        )
    )

    return {
        "role_filter": role_filter,
        "count": len(items),
        "counts": counts,
        "items": items,
    }


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
    checklist: list[OrderedDict[str, object]] = []
    backlog_counts = role_payload["backlog_counts"]
    memory_counts = role_payload["memory_counts"]
    gate_status = str(stage_report["gate"]["status"] or "")
    pending_signals = int(signal_summary["pending_count"])
    working_memory_entries = int(working_memory_summary["entry_count"])
    pending_reflections = int(reflection_summary["counts"]["triaged"])
    task_execution_log = None if task_context is None else str(task_context.get("execution_log_path") or "") or None

    def add(item_id: str, summary: str, command: str | None = None, reason: str | None = None) -> None:
        item = OrderedDict([("id", item_id), ("summary", summary)])
        if command:
            item["command"] = command
        if reason:
            item["reason"] = reason
        checklist.append(item)

    if phase == "start":
        if task_context is None:
            add(
                "bind-task",
                "若当前工作绑定到明确任务，补传 `--task-id <TASK-ID>` 记录 `last_started_at`，避免 `.pm` workflow 只停留在口头层。",
            )
        add(
            "read-docs",
            "先读目标模块 PRD / project，再开始编辑。",
        )
        if task_execution_log:
            add(
                "read-execution-log",
                "读取当前 task execution log，避免同任务上下文断档。",
                command=f"sed -n '1,200p' {task_execution_log}",
            )
        add(
            "role-report",
            f"读取 {role} 的 backlog 与 memory 现状，避免重复处理已知问题。",
            command=f"./scripts/pm/role-report.sh --role {role}",
        )
        if pending_signals > 0:
            add(
                "triage-signals",
                "先处理该角色尚未闭环的 signal，避免任务结论继续停留在 inbox。",
                reason=f"pending_signals={pending_signals}",
            )
        if int(backlog_counts["blocked"]) > 0:
            add(
                "inspect-blockers",
                "先看 blocked backlog，优先判断是否需要解除阻断或升级阶段风险。",
                command=f"./scripts/pm/role-report.sh --role {role}",
                reason=f"blocked_tasks={backlog_counts['blocked']}",
            )
        if int(backlog_counts["candidate"]) > 0:
            add(
                "review-candidates",
                "清理 candidate 池，决定哪些要升为 committed，哪些继续 deferred。",
                command=f"./scripts/pm/role-report.sh --role {role}",
                reason=f"candidate_tasks={backlog_counts['candidate']}",
            )
        if int(memory_counts["needs_review"]) > 0:
            add(
                "review-memory",
                "先 review stale memory，再写入新的长期结论，避免并行存在旧口径。",
                command=f"./scripts/pm/memory-report.sh --role {role} --no-shared",
                reason=f"needs_review_memory={memory_counts['needs_review']}",
            )
        if role == "producer_system_designer":
            add(
                "review-stage",
                "制作人开始推进前先看阶段和 gate 汇总，确认 blocker 与 claim envelope 是否已变化。",
                command="./scripts/pm/stage-report.sh",
                reason=f"gate_status={gate_status}",
            )
    elif phase == "close":
        if task_context is None:
            add(
                "bind-task",
                "若当前工作绑定到明确任务，补传 `--task-id <TASK-ID>` 记录 `last_closed_at`，否则不能宣称 `.pm` workflow 已完整接入。",
            )
        add(
            "write-execution-log",
            "先回写当前 task execution log，再做 signal / memory / backlog 的结构化收口。",
        )
        add(
            "extract-memory",
            "执行记忆抽取三问：这条结论是否跨任务复用、是否能避免其他 owner 重复踩坑、是否会影响 PRD/实现/测试/对外口径；任一为 yes 时，至少生成 signal、working_memory 或 memory 候选，而不是只写 execution log。",
        )
        if task_context is not None and working_memory_entries == 0:
            add(
                "bootstrap-working-memory",
                "当前 task 还没有 working_memory；若本轮主要过程发生在 Codex 会话里，先抽取一次 task-scoped working_memory，再决定是否需要 reflection signal / candidate task。",
                command=f"./scripts/pm/codex-working-memory.sh --task-id {task_context['task_id']} --role {role}",
            )
        elif working_memory_entries > 0:
            add(
                "review-working-memory",
                "先处理 task-scoped working_memory：提炼成 reflection signal、转 task/memory，或显式保留待过期，不要让过程认知悬空。",
                command="./scripts/pm/working-memory-report.sh --task-id <TASK-ID>",
                reason=f"working_memory_entries={working_memory_entries}",
            )
            add(
                "autoflow-working-memory",
                "可先用安全默认自动化把 working_memory 提成 reflection signal 和 candidate task，再进入 owner review。",
                command="./scripts/pm/working-memory-autoflow.sh --task-id <TASK-ID> --severity medium --priority P2",
            )
        if pending_reflections > 0:
            add(
                "review-reflection",
                "处理仍停留在 triaged 的 reflection signal，决定是否转 task/memory/deferred。",
                command=f"./scripts/pm/reflection-report.sh --role {role}",
                reason=f"triaged_reflections={pending_reflections}",
            )
        add(
            "subagent-review",
            "commit 前必须启动独立 subagent review 当前 diff；在 Codex 环境中默认通过 spawn_agent 派生独立 review agent。`codex exec review --uncommitted` 只算 shell 自检，不计作该流程完成；若运行环境禁止派生 agent，需显式记录为阻断。review 只用于暴露风险/回归/缺测，不替代 owner role，findings 处理后再提交。",
        )
        if role in {"qa_engineer", "liveops_community"} or pending_signals > 0:
            add(
                "promote-signals",
                "把新增的高价值 QA / liveops / incident 结论提升到 signal inbox，而不是只留在 task execution log。",
                command=f"./scripts/pm/promote-signal.sh ... --role-hint {role}",
            )
        add(
            "sync-backlog",
            "把本轮任务状态迁移回 backlog / task registry，避免 `.pm` 与实际执行脱节。",
            command="./scripts/pm/move-task.sh --task-id <TASK-ID> --to-status <candidate|committed|blocked|done|deferred>",
        )
        add(
            "sync-memory",
            "稳定结论进入 active memory，被新结论取代的记录显式 supersede，不允许直接覆盖旧口径。",
            command=f"./scripts/pm/promote-memory.sh --signal-id <SIG-ID> --role {role} --topic <topic> --promotion-reason <reason>",
        )
        if role == "producer_system_designer":
            add(
                "sync-stage",
                "若阶段判断、gate lane 或对外 claim envelope 有变化，同步更新 `.pm/stage/*.yaml` 并重跑阶段汇总。",
                command="./scripts/pm/stage-report.sh",
            )
        add(
            "pm-verify",
            "收口前复跑 PM 结构门禁，确认 report / lint 仍能读通本轮改动。",
            command="./scripts/pm/lint.sh",
        )
    else:
        add(
            "review-stage",
            "以 stage report 作为阶段评审输入，再结合角色视图确认 blocker、backlog 和长期结论。",
            command="./scripts/pm/stage-report.sh",
            reason=f"gate_status={gate_status}",
        )
        add(
            "review-role",
            f"读取 {role} 的 role report，确认 backlog / memory / blocked task 是否需要裁决。",
            command=f"./scripts/pm/role-report.sh --role {role}",
        )
        if pending_signals > 0:
            add(
                "review-signals",
                "阶段评审前先处理未闭环 signal，避免新风险没有进入正式候选池。",
                reason=f"pending_signals={pending_signals}",
            )
        if int(memory_counts["needs_review"]) > 0:
            add(
                "review-stale-memory",
                "处理 stale memory，避免阶段裁决继续引用过期结论。",
                command=f"./scripts/pm/memory-report.sh --role {role} --no-shared",
                reason=f"needs_review_memory={memory_counts['needs_review']}",
            )

    return checklist


def build_workflow_report(
    root: pathlib.Path,
    role: str,
    phase: str,
    stale_after_days: int,
    task_id: str | None,
) -> dict[str, object]:
    roles = load_roles(root)
    if role not in roles:
        raise ValueError(f"unknown role: {role}")
    if phase not in {"start", "close", "review"}:
        raise ValueError(f"unsupported phase: {phase}")

    task_context: dict[str, object] | None = None
    if task_id:
        task_context = load_task_context(root, task_id)

    role_report = build_role_report(root, role_filter=role, stale_after_days=stale_after_days)
    role_payload = role_report["roles"][role]
    stage_report = build_stage_report(root)
    signal_role_filter = None if (phase == "review" and role == "producer_system_designer") else role
    signal_summary = build_signal_summary(root, role_filter=signal_role_filter)
    if task_id:
        working_memory_summary = build_working_memory_report(root, task_id=task_id, role_filter=None)
    else:
        working_memory_summary = build_working_memory_report(root, task_id=None, role_filter=role)
    reflection_summary = build_reflection_summary(root, role_filter=signal_role_filter)
    checklist = build_workflow_checklist(
        role,
        phase,
        task_context,
        role_payload,
        signal_summary,
        stage_report,
        working_memory_summary,
        reflection_summary,
    )

    if task_id and phase in {"start", "close"}:
        task_context = record_task_workflow_phase(root, task_id, role, phase)

    return {
        "generated_at": now_iso(),
        "phase": phase,
        "role": role,
        "task_context": task_context,
        "stale_after_days": stale_after_days,
        "role_report": role_payload,
        "signal_summary": signal_summary,
        "stage_report": stage_report,
        "working_memory_summary": working_memory_summary,
        "reflection_summary": reflection_summary,
        "checklist": checklist,
    }


def run_task_backlog_lint(root: pathlib.Path) -> None:
    roles = load_roles(root)
    failures: list[str] = []

    def fail(message: str) -> None:
        failures.append(message)

    signal_ids, promoted_signal_ids = collect_signals(root)

    for line_no, raw_line in enumerate((root / ".pm/inbox/signals.jsonl").read_text(encoding="utf-8").splitlines(), start=1):
        line = raw_line.strip()
        if not line:
            continue
        payload = json.loads(line)
        missing = sorted(
            {
                "signal_id",
                "source_type",
                "source_ref",
                "role_hint",
                "severity",
                "summary",
                "promotion_state",
            }
            - payload.keys()
        )
        if missing:
            fail(f"signal missing keys at .pm/inbox/signals.jsonl:{line_no}: {', '.join(missing)}")
            continue
        if payload["role_hint"] not in roles:
            fail(f"signal role_hint not registered: {payload['signal_id']} -> {payload['role_hint']}")
        if payload["promotion_state"] not in ALLOWED_SIGNAL_STATES:
            fail(f"signal promotion_state invalid: {payload['signal_id']} -> {payload['promotion_state']}")
        memory_state = str(payload.get("memory_promotion_state", "pending"))
        if memory_state not in ALLOWED_MEMORY_PROMOTION_STATES:
            fail(f"signal memory_promotion_state invalid: {payload['signal_id']} -> {memory_state}")
        if memory_state == "promoted":
            missing_memory_keys = sorted(
                {
                    "memory_decision_at",
                    "memory_id",
                    "memory_promotion_reason",
                    "memory_role",
                    "memory_scope",
                    "memory_topic",
                }
                - payload.keys()
            )
            if missing_memory_keys:
                fail(
                    f"signal promoted memory missing keys: {payload['signal_id']} -> "
                    + ", ".join(missing_memory_keys)
                )
            elif payload["memory_promotion_reason"] not in ALLOWED_PROMOTION_REASONS:
                fail(
                    f"signal memory_promotion_reason invalid: "
                    f"{payload['signal_id']} -> {payload['memory_promotion_reason']}"
                )
        elif memory_state == "rejected":
            missing_rejection_keys = sorted({"memory_decision_at", "memory_rejection_reason"} - payload.keys())
            if missing_rejection_keys:
                fail(
                    f"signal rejected memory missing keys: {payload['signal_id']} -> "
                    + ", ".join(missing_rejection_keys)
                )
            elif payload["memory_rejection_reason"] not in ALLOWED_MEMORY_REJECTION_REASONS:
                fail(
                    f"signal memory_rejection_reason invalid: "
                    f"{payload['signal_id']} -> {payload['memory_rejection_reason']}"
                )
        elif memory_state == "deferred":
            missing_deferred_keys = sorted({"memory_decision_at", "memory_deferred_reason"} - payload.keys())
            if missing_deferred_keys:
                fail(
                    f"signal deferred memory missing keys: {payload['signal_id']} -> "
                    + ", ".join(missing_deferred_keys)
                )

    registry_header, registry_entries = load_list_document(root / ".pm/registry/tasks.yaml", "tasks")
    next_sequence = registry_header.get("next_sequence")
    if next_sequence is not None and (not str(next_sequence).isdigit()):
        fail("tasks registry missing numeric next_sequence")

    registry_by_id: dict[str, OrderedDict[str, object]] = {}
    for entry in registry_entries:
        task_id = str(entry.get("task_id") or "")
        if not task_id:
            fail("registry task missing task_id")
            continue
        if task_id in registry_by_id:
            fail(f"duplicate registry task_id: {task_id}")
            continue
        registry_by_id[task_id] = entry
        owner_role = entry.get("owner_role")
        status = entry.get("status")
        if owner_role not in roles:
            fail(f"registry task owner_role not registered: {task_id} -> {owner_role}")
        if status not in TASK_STATUSES:
            fail(f"registry task status invalid: {task_id} -> {status}")
        task_path = entry.get("task_path")
        if not task_path or not (root / str(task_path)).is_file():
            fail(f"registry task path missing: {task_id} -> {task_path}")
        source_signal = entry.get("source_signal")
        if source_signal and str(source_signal) not in signal_ids:
            fail(f"registry task source_signal missing from inbox: {task_id} -> {source_signal}")

    task_source_signals: set[str] = set()
    task_files = sorted(path for path in (root / ".pm/tasks").glob("TASK-PM-*.yaml") if path.is_file())
    if len(task_files) != len(registry_entries):
        fail(f"task file count mismatch: files={len(task_files)} registry={len(registry_entries)}")

    task_fields_by_id: dict[str, OrderedDict[str, object]] = {}
    for path in task_files:
        fields = load_mapping_document(path)
        task_id = fields.get("task_id")
        if not task_id:
            fail(f"task file missing task_id: {path.relative_to(root)}")
            continue
        task_id = str(task_id)
        task_fields_by_id[task_id] = fields
        owner_role = fields.get("owner_role")
        status = fields.get("status")
        if owner_role not in roles:
            fail(f"task file owner_role not registered: {task_id} -> {owner_role}")
        if status not in TASK_STATUSES:
            fail(f"task file status invalid: {task_id} -> {status}")
        registry_entry = registry_by_id.get(task_id)
        if registry_entry is None:
            fail(f"task file missing from registry: {task_id}")
        else:
            if registry_entry.get("owner_role") != owner_role:
                fail(f"registry owner_role mismatch: {task_id}")
            if registry_entry.get("status") != status:
                fail(f"registry status mismatch: {task_id}")
            if registry_entry.get("priority") != fields.get("priority"):
                fail(f"registry priority mismatch: {task_id}")
            expected_path = f".pm/tasks/{path.name}"
            if registry_entry.get("task_path") != expected_path:
                fail(f"registry task_path mismatch: {task_id} -> {registry_entry.get('task_path')} != {expected_path}")
        source_signal = fields.get("source_signal")
        if source_signal:
            task_source_signals.add(str(source_signal))
            if str(source_signal) not in signal_ids:
                fail(f"task source_signal missing from inbox: {task_id} -> {source_signal}")
        for key in ("last_started_at", "last_closed_at"):
            value = fields.get(key)
            if value in {None, ""}:
                continue
            try:
                datetime.fromisoformat(str(value))
            except ValueError:
                fail(f"task file invalid {key}: {task_id} -> {value}")
        if fields.get("last_started_at") not in {None, ""} and fields.get("last_closed_at") not in {None, ""}:
            try:
                started_at = datetime.fromisoformat(str(fields["last_started_at"]))
                closed_at = datetime.fromisoformat(str(fields["last_closed_at"]))
                if closed_at < started_at:
                    fail(f"task file close precedes start: {task_id}")
            except ValueError:
                pass
        if status in {"blocked", "done", "deferred"} and fields.get("last_started_at") in {None, ""}:
            fail(f"task file missing last_started_at for started workflow task: {task_id}")
        if status in {"done", "deferred"} and fields.get("last_closed_at") in {None, ""}:
            fail(f"task file missing last_closed_at for closed workflow task: {task_id}")
        execution_log_path = str(fields.get("execution_log_path") or "")
        expected_execution_log_path = task_execution_log_relative_path(task_id)
        if execution_log_path != expected_execution_log_path:
            fail(
                f"task file execution_log_path mismatch: {task_id} -> "
                f"{execution_log_path or '(missing)'} != {expected_execution_log_path}"
            )
            continue
        execution_log_file = root / execution_log_path
        if not execution_log_file.is_file():
            fail(f"task execution log missing: {task_id} -> {execution_log_path}")
            continue
        require_entry = status in {"blocked", "done", "deferred"} or fields.get("last_started_at") not in {None, ""}
        entry_count = 0
        active_entry_line: int | None = None
        entry_has_done = False
        entry_has_pending = False
        for line_no, raw_line in enumerate(execution_log_file.read_text(encoding="utf-8").splitlines(), start=1):
            if raw_line.startswith("## "):
                if active_entry_line is not None:
                    if not entry_has_done:
                        fail(
                            f"{execution_log_path}:{active_entry_line}: execution log entry missing 完成内容 for {task_id}"
                        )
                    if not entry_has_pending:
                        fail(
                            f"{execution_log_path}:{active_entry_line}: execution log entry missing 遗留事项 for {task_id}"
                        )
                match = TASK_EXECUTION_LOG_ENTRY_RE.fullmatch(raw_line)
                if not match:
                    fail(f"{execution_log_path}:{line_no}: invalid execution log heading for {task_id}")
                    active_entry_line = None
                    entry_has_done = False
                    entry_has_pending = False
                    continue
                role_name = match.group(3)
                if role_name not in roles:
                    fail(f"{execution_log_path}:{line_no}: unknown role in execution log for {task_id}: {role_name}")
                entry_count += 1
                active_entry_line = line_no
                entry_has_done = False
                entry_has_pending = False
                continue
            if active_entry_line is None:
                continue
            if raw_line.startswith("- 完成内容:"):
                entry_has_done = True
            elif raw_line.startswith("- 遗留事项:"):
                entry_has_pending = True
        if active_entry_line is not None:
            if not entry_has_done:
                fail(f"{execution_log_path}:{active_entry_line}: execution log entry missing 完成内容 for {task_id}")
            if not entry_has_pending:
                fail(f"{execution_log_path}:{active_entry_line}: execution log entry missing 遗留事项 for {task_id}")
        if require_entry and entry_count == 0:
            fail(f"task execution log requires at least one entry: {task_id}")

    for signal_id in promoted_signal_ids:
        if signal_id not in task_source_signals:
            fail(f"promoted signal has no task file: {signal_id}")

    backlog_membership: dict[str, list[tuple[str, str, OrderedDict[str, object]]]] = {}
    for role in sorted(roles):
        for file_status in ("candidate", "committed", "blocked", "done"):
            path = root / f".pm/roles/{role}/backlog/{file_status}.yaml"
            header, entries = load_list_document(path, "tasks")
            if header.get("role") != role:
                fail(f"{path.relative_to(root)} role header mismatch: {header.get('role')} != {role}")
            if header.get("status") != file_status:
                fail(f"{path.relative_to(root)} status header mismatch: {header.get('status')} != {file_status}")
            for entry in entries:
                task_id = entry.get("task_id")
                if not task_id:
                    fail(f"{path.relative_to(root)} entry missing task_id")
                    continue
                task_id = str(task_id)
                entry_status = entry.get("status")
                if file_status != "done" and entry_status != file_status:
                    fail(f"{path.relative_to(root)} {task_id} entry status mismatch: {entry_status} != {file_status}")
                if file_status == "done" and entry_status not in {"done", "deferred"}:
                    fail(f"{path.relative_to(root)} {task_id} invalid done-lane status: {entry_status}")
                backlog_membership.setdefault(task_id, []).append((role, file_status, entry))

    for task_id, registry_entry in registry_by_id.items():
        memberships = backlog_membership.get(task_id, [])
        if len(memberships) != 1:
            fail(f"task backlog membership mismatch: {task_id} has {len(memberships)} entries")
            continue
        role, file_status, entry = memberships[0]
        if role != registry_entry.get("owner_role"):
            fail(f"task backlog owner mismatch: {task_id} -> {role} != {registry_entry.get('owner_role')}")
        expected_file_status = backlog_file_for_status(str(registry_entry.get("status")))
        if file_status != expected_file_status[:-5]:
            fail(f"task backlog lane mismatch: {task_id} -> {file_status} != {expected_file_status[:-5]}")
        task_fields = task_fields_by_id.get(task_id)
        if task_fields is None:
            continue
        for key in ("title", "priority", "source_signal", "status"):
            if entry.get(key) != task_fields.get(key):
                fail(f"task backlog field mismatch: {task_id} -> {key}")

    for task_id, memberships in backlog_membership.items():
        if task_id not in registry_by_id:
            fail(f"backlog task missing from registry: {task_id}")
            continue
        task_fields = task_fields_by_id.get(task_id)
        if task_fields is None:
            continue
        for role, file_status, entry in memberships:
            if entry.get("status") != task_fields.get("status"):
                fail(f"backlog/task status mismatch: {task_id}")
            if file_status != backlog_file_for_status(str(task_fields.get('status')))[:-5]:
                fail(f"backlog/task lane mismatch: {task_id}")

    if next_sequence is not None and registry_by_id:
        max_sequence = max(int(task_id.rsplit("-", 1)[1]) for task_id in registry_by_id)
        if int(next_sequence) <= max_sequence:
            fail(f"next_sequence not ahead of existing tasks: next_sequence={next_sequence} max_task={max_sequence}")

    if failures:
        for failure in failures:
            print(f"pm-lint: FAIL: {failure}", file=sys.stderr)
        raise SystemExit(1)


def run_stage_lint(root: pathlib.Path) -> None:
    failures: list[str] = []

    def fail(message: str) -> None:
        failures.append(message)

    stage_current = load_mapping_document(root / ".pm/stage/current.yaml")
    gate = load_mapping_document(root / ".pm/stage/gate.yaml")
    _, registry_entries = load_list_document(root / ".pm/registry/tasks.yaml", "tasks")
    task_ids = {str(entry.get("task_id")) for entry in registry_entries if entry.get("task_id")}

    def require_keys(doc_name: str, payload: OrderedDict[str, object], required: set[str]) -> None:
        missing = sorted(required - set(payload.keys()))
        if missing:
            fail(f"{doc_name} missing keys: {', '.join(missing)}")

    require_keys(
        ".pm/stage/current.yaml",
        stage_current,
        {"version", "current_stage", "candidate_stage", "claim_envelope", "decision_date", "updated_from", "blocking_tasks"},
    )
    require_keys(
        ".pm/stage/gate.yaml",
        gate,
        {"version", "gate_id", "status", "lane_status", "blocking_tasks", "updated_from"},
    )

    producer_stage_memory = load_active_memory_record(
        root,
        root / ".pm/roles/producer_system_designer/memory/active.yaml",
        "stage.current",
    )
    shared_claim_memory = load_active_memory_record(
        root,
        root / ".pm/shared/memory/active.yaml",
        "gate.claim_envelope",
    )

    current_stage = stage_current.get("current_stage")
    claim_envelope = stage_current.get("claim_envelope")
    updated_from = stage_current.get("updated_from")
    gate_status = gate.get("status")
    gate_updated_from = gate.get("updated_from")

    if current_stage is None and producer_stage_memory is not None:
        fail("stage current is null while producer active memory still declares topic stage.current")
    if claim_envelope is None and shared_claim_memory is not None:
        fail("claim_envelope is null while shared active memory still declares topic gate.claim_envelope")
    if current_stage is not None:
        if claim_envelope is None:
            fail("current_stage is set but claim_envelope is null")
        if not isinstance(updated_from, list) or not updated_from:
            fail("stage current requires non-empty updated_from once current_stage is set")
        decision_date = stage_current.get("decision_date")
        if decision_date in {None, ""}:
            fail("stage current requires decision_date once current_stage is set")
        else:
            try:
                datetime.fromisoformat(str(decision_date))
            except ValueError:
                fail(f"stage current has invalid decision_date: {decision_date}")

    if gate_status != "draft":
        if not isinstance(gate_updated_from, list) or not gate_updated_from:
            fail("gate requires non-empty updated_from once status leaves draft")
        if gate.get("gate_id") in {None, ""}:
            fail("gate requires gate_id once status leaves draft")

    for doc_name, blocking_tasks in (
        (".pm/stage/current.yaml", stage_current.get("blocking_tasks", [])),
        (".pm/stage/gate.yaml", gate.get("blocking_tasks", [])),
    ):
        if not isinstance(blocking_tasks, list):
            fail(f"{doc_name} blocking_tasks must be a list")
            continue
        for task_id in blocking_tasks:
            if str(task_id) not in task_ids:
                fail(f"{doc_name} references missing blocking task: {task_id}")

    tracked_blocking_ids = {
        str(task_id) for task_id in list(stage_current.get("blocking_tasks", [])) + list(gate.get("blocking_tasks", []))
    }
    for entry in registry_entries:
        if entry.get("status") != "blocked":
            continue
        task_id = str(entry.get("task_id"))
        if task_id not in tracked_blocking_ids:
            fail(f"blocked task missing from stage/gate blocking_tasks: {task_id}")

    if failures:
        for failure in failures:
            print(f"stage-lint: FAIL: {failure}", file=sys.stderr)
        raise SystemExit(1)


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


def cmd_stage_lint(args: argparse.Namespace) -> int:
    run_stage_lint(args.root)
    print("stage-lint: OK")
    return 0


def cmd_set_stage(args: argparse.Namespace) -> int:
    if not args.source_ref:
        raise ValueError("at least one --source-ref is required")

    stage_current_path = args.root / ".pm/stage/current.yaml"
    gate_path = args.root / ".pm/stage/gate.yaml"
    stage_current = load_mapping_document(stage_current_path)
    gate = load_mapping_document(gate_path)

    source_refs = list(dict.fromkeys(args.source_ref))
    stage_changed = False
    gate_changed = False

    def assign_stage(key: str, value) -> None:
        nonlocal stage_changed
        if stage_current.get(key) != value:
            stage_current[key] = value
            stage_changed = True

    def assign_gate(key: str, value) -> None:
        nonlocal gate_changed
        if gate.get(key) != value:
            gate[key] = value
            gate_changed = True

    if args.current_stage is not None:
        assign_stage("current_stage", args.current_stage)
    if args.candidate_stage is not None:
        assign_stage("candidate_stage", args.candidate_stage)
    elif args.clear_candidate_stage:
        assign_stage("candidate_stage", None)
    if args.claim_envelope is not None:
        assign_stage("claim_envelope", args.claim_envelope)
    if args.decision_date is not None:
        assign_stage("decision_date", args.decision_date)
    elif stage_changed and stage_current.get("decision_date") in {None, ""}:
        assign_stage("decision_date", datetime.now().astimezone().date().isoformat())

    if args.gate_id is not None:
        assign_gate("gate_id", args.gate_id)
    elif args.clear_gate_id:
        assign_gate("gate_id", None)
    if args.gate_status is not None:
        assign_gate("status", args.gate_status)
    if args.lane_status:
        assign_gate("lane_status", list(args.lane_status))
    elif args.clear_lane_status:
        assign_gate("lane_status", [])
    if args.blocking_task:
        blocking = list(dict.fromkeys(args.blocking_task))
        assign_stage("blocking_tasks", blocking)
        assign_gate("blocking_tasks", blocking)
    elif args.clear_blocking_tasks:
        assign_stage("blocking_tasks", [])
        assign_gate("blocking_tasks", [])

    if stage_changed:
        stage_current["updated_from"] = source_refs
    if gate_changed:
        gate["updated_from"] = source_refs

    if not stage_changed and not gate_changed:
        raise ValueError("set-stage received no effective changes")

    original_stage_text = stage_current_path.read_text(encoding="utf-8")
    original_gate_text = gate_path.read_text(encoding="utf-8")
    try:
        dump_mapping_document(stage_current_path, stage_current)
        dump_mapping_document(gate_path, gate)
        run_stage_lint(args.root)
    except BaseException:
        stage_current_path.write_text(original_stage_text, encoding="utf-8")
        gate_path.write_text(original_gate_text, encoding="utf-8")
        raise

    result = {
        "current_stage": stage_current.get("current_stage"),
        "candidate_stage": stage_current.get("candidate_stage"),
        "claim_envelope": stage_current.get("claim_envelope"),
        "decision_date": stage_current.get("decision_date"),
        "gate_id": gate.get("gate_id"),
        "gate_status": gate.get("status"),
        "lane_status": list(gate.get("lane_status", [])),
        "blocking_tasks": list(stage_current.get("blocking_tasks", [])),
        "updated_from": source_refs,
    }
    if args.json:
        print(json.dumps(result, ensure_ascii=False))
    else:
        print(
            "set-stage: updated "
            f"current_stage={result['current_stage']} claim_envelope={result['claim_envelope']} "
            f"gate_status={result['gate_status']}"
        )
    return 0


def cmd_move_task(args: argparse.Namespace) -> int:
    validate_status(args.to_status)
    root = args.root
    updated_at = now_iso()

    registry_header, registry_entries, registry_entry, registry_path = find_registry_task(root, args.task_id)
    owner_role = str(registry_entry["owner_role"])
    current_status = str(registry_entry["status"])
    target_status = args.to_status

    task_path = root / str(registry_entry["task_path"])
    task_fields = load_mapping_document(task_path)

    source_paths = [
        root / f".pm/roles/{owner_role}/backlog/candidate.yaml",
        root / f".pm/roles/{owner_role}/backlog/committed.yaml",
        root / f".pm/roles/{owner_role}/backlog/blocked.yaml",
        root / f".pm/roles/{owner_role}/backlog/done.yaml",
    ]

    found_memberships: list[tuple[pathlib.Path, OrderedDict[str, object], list[OrderedDict[str, object]], OrderedDict[str, object]]] = []
    for backlog_path in source_paths:
        header, entries = load_list_document(backlog_path, "tasks")
        for entry in entries:
            if entry.get("task_id") == args.task_id:
                found_memberships.append((backlog_path, header, entries, entry))

    if len(found_memberships) != 1:
        raise ValueError(f"task backlog membership mismatch before move: {args.task_id} has {len(found_memberships)} entries")

    source_path, source_header, source_entries, source_entry = found_memberships[0]
    destination_path = root / f".pm/roles/{owner_role}/backlog/{backlog_file_for_status(target_status)}"
    destination_header, destination_entries = load_list_document(destination_path, "tasks")

    source_entries.remove(source_entry)
    moved_entry = OrderedDict(source_entry)
    moved_entry["status"] = target_status

    if source_path == destination_path:
        destination_entries = source_entries
        destination_entries.append(moved_entry)
    else:
        destination_entries.append(moved_entry)

    registry_entry["status"] = target_status
    registry_entry["updated_at"] = updated_at
    task_fields["status"] = target_status
    task_fields["updated_at"] = updated_at

    dump_list_document(registry_path, registry_header, "tasks", registry_entries)
    dump_mapping_document(task_path, task_fields)
    dump_list_document(source_path, source_header, "tasks", source_entries)
    if destination_path != source_path:
        dump_list_document(destination_path, destination_header, "tasks", destination_entries)

    result = {
        "task_id": args.task_id,
        "owner_role": owner_role,
        "from_status": current_status,
        "to_status": target_status,
        "updated_at": updated_at,
    }
    if args.json:
        print(json.dumps(result, ensure_ascii=False))
    else:
        print(f"move-task: moved {args.task_id} {current_status} -> {target_status}")
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
    if args.role == "shared":
        raise ValueError("--role=shared is invalid; use --include-shared without --role")
    if args.stale_after_days < 0:
        raise ValueError("--stale-after-days must be >= 0")
    if args.role and args.role not in load_roles(args.root):
        raise ValueError(f"unknown role: {args.role}")

    report = build_memory_report(
        args.root,
        role_filter=args.role,
        include_shared=args.include_shared,
        stale_after_days=args.stale_after_days,
    )
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
        return 0

    lines = [
        "oasis7 memory report",
        f"- generated_at: {report['generated_at']}",
        f"- stale_after_days: {report['stale_after_days']}",
        f"- role_filter: {report['role_filter'] or '(all roles)'}",
        f"- include_shared: {'yes' if report['include_shared'] else 'no'}",
        f"- counts: active={report['counts']['active']}, needs_review={report['counts']['needs_review']}, superseded={report['counts']['superseded']}",
        "- role_summary:",
    ]
    if report["roles"]:
        for role, counts in report["roles"].items():
            lines.append(
                f"  - {role}: active={counts['active']}, needs_review={counts['needs_review']}, superseded={counts['superseded']}"
            )
    else:
        lines.append("  - (none)")

    lines.append("- needs_review:")
    if report["needs_review"]:
        for item in report["needs_review"]:
            lines.append(
                f"  - {item['id']} / {item['role']} / {item['topic']} / last_reviewed_at={item['last_reviewed_at']}"
            )
    else:
        lines.append("  - (none)")

    lines.append("- active:")
    if report["active"]:
        for item in report["active"]:
            lines.append(f"  - {item['id']} / {item['role']} / {item['topic']} / {item['review_state']}")
    else:
        lines.append("  - (none)")

    lines.append("- superseded:")
    if report["superseded"]:
        for item in report["superseded"]:
            lines.append(
                f"  - {item['id']} / {item['role']} / {item['topic']} / superseded_by={item['superseded_by']}"
            )
    else:
        lines.append("  - (none)")

    print("\n".join(lines))
    return 0


def cmd_working_memory_report(args: argparse.Namespace) -> int:
    report = build_working_memory_report(args.root, task_id=args.task_id, role_filter=args.role)
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
        return 0

    lines = [
        "oasis7 working memory report",
        f"- generated_at: {report['generated_at']}",
        f"- task_filter: {report['task_filter'] or '(all tasks)'}",
        f"- role_filter: {report['role_filter'] or '(all roles)'}",
        f"- task_count: {report['task_count']}",
        f"- entry_count: {report['entry_count']}",
        "- counts_by_kind: " + ", ".join(f"{kind}={count}" for kind, count in report["counts_by_kind"].items()),
        "- tasks:",
    ]
    if report["tasks"]:
        for task_id, payload in report["tasks"].items():
            lines.append(
                f"  - {task_id} / {payload['role']} / {payload.get('worktree_hint') or '(no worktree_hint)'} / "
                f"entries={payload['entry_count']}"
            )
            for entry in payload["entries"][:5]:
                lines.append(
                    f"    - {entry['entry_id']} / {entry['entry_kind']} / {entry['captured_at']} / {entry['summary']}"
                )
    else:
        lines.append("  - (none)")

    print("\n".join(lines))
    return 0


def cmd_reflection_report(args: argparse.Namespace) -> int:
    report = build_reflection_summary(args.root, role_filter=args.role)
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
        return 0

    lines = [
        "oasis7 reflection report",
        f"- role_filter: {report['role_filter'] or '(all roles)'}",
        f"- count: {report['count']}",
        "- counts: " + ", ".join(f"{key}={value}" for key, value in report["counts"].items()),
        "- items:",
    ]
    if report["items"]:
        for item in report["items"]:
            linked_tasks = item["linked_tasks"]
            linked_summary = (
                ", ".join(str(task["task_id"]) for task in linked_tasks)
                if linked_tasks
                else "(none)"
            )
            lines.append(
                f"  - {item['signal_id']} / {item['promotion_state']} / "
                f"{item['memory_promotion_state']} / linked_tasks={linked_summary} / {item['summary']}"
            )
    else:
        lines.append("  - (none)")

    print("\n".join(lines))
    return 0


def cmd_role_report(args: argparse.Namespace) -> int:
    if args.stale_after_days < 0:
        raise ValueError("--stale-after-days must be >= 0")

    report = build_role_report(args.root, role_filter=args.role, stale_after_days=args.stale_after_days)
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
        return 0

    lines = [
        "oasis7 role report",
        f"- generated_at: {report['generated_at']}",
        f"- stale_after_days: {report['stale_after_days']}",
        f"- role_filter: {report['role_filter'] or '(all roles)'}",
        f"- backlog_totals: {', '.join(f'{k}={v}' for k, v in report['backlog_totals'].items())}",
    ]

    for role, payload in report["roles"].items():
        lines.append(f"- role: {role}")
        lines.append(
            "  backlog_counts: "
            + ", ".join(f"{status}={count}" for status, count in payload["backlog_counts"].items())
        )
        lines.append(
            "  memory_counts: "
            + ", ".join(f"{status}={count}" for status, count in payload["memory_counts"].items())
        )

        lines.append("  blocked_tasks:")
        if payload["tasks"]["blocked"]:
            for item in payload["tasks"]["blocked"]:
                lines.append(f"    - {item['task_id']} / {item['priority']} / {item['title']}")
        else:
            lines.append("    - (none)")

        lines.append("  needs_review_memory:")
        if payload["needs_review_memory"]:
            for item in payload["needs_review_memory"]:
                lines.append(f"    - {item['id']} / {item['topic']} / {item['last_reviewed_at']}")
        else:
            lines.append("    - (none)")

    print("\n".join(lines))
    return 0


def cmd_workflow_report(args: argparse.Namespace) -> int:
    if args.stale_after_days < 0:
        raise ValueError("--stale-after-days must be >= 0")

    report = build_workflow_report(
        args.root,
        role=args.role,
        phase=args.phase,
        stale_after_days=args.stale_after_days,
        task_id=args.task_id,
    )
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
        return 0

    backlog_counts = report["role_report"]["backlog_counts"]
    memory_counts = report["role_report"]["memory_counts"]
    signal_counts = report["signal_summary"]["counts"]
    memory_signal_counts = report["signal_summary"]["memory_counts"]

    lines = [
        "oasis7 workflow report",
        f"- generated_at: {report['generated_at']}",
        f"- phase: {report['phase']}",
        f"- role: {report['role']}",
        f"- task_id: {(report['task_context'] or {}).get('task_id') or '(none)'}",
        f"- stale_after_days: {report['stale_after_days']}",
        f"- current_stage: {report['stage_report']['current_stage']}",
        f"- gate_status: {report['stage_report']['gate']['status']}",
        "- backlog_counts: " + ", ".join(f"{status}={count}" for status, count in backlog_counts.items()),
        "- memory_counts: " + ", ".join(f"{status}={count}" for status, count in memory_counts.items()),
        f"- working_memory_entries: {report['working_memory_summary']['entry_count']}",
        "- signal_counts: " + ", ".join(f"{status}={count}" for status, count in signal_counts.items()),
        "- memory_signal_counts: "
        + ", ".join(f"{status}={count}" for status, count in memory_signal_counts.items()),
        "- reflection_counts: "
        + ", ".join(f"{status}={count}" for status, count in report["reflection_summary"]["counts"].items()),
        f"- blocked_tasks: {len(report['stage_report']['blocking_tasks'])}",
        "- pending_signals:",
    ]

    if report["task_context"] is not None:
        lines.insert(
            6,
            f"- task_status: {report['task_context']['status']} / "
            f"last_started_at={(report['task_context'].get('last_started_at') or '(none)')} / "
            f"last_closed_at={(report['task_context'].get('last_closed_at') or '(none)')}",
        )
        lines.insert(7, f"- execution_log_path: {report['task_context'].get('execution_log_path') or '(none)'}")

    if report["signal_summary"]["pending_signals"]:
        for item in report["signal_summary"]["pending_signals"]:
            lines.append(
                f"  - {item['signal_id']} / {item.get('role_hint') or '(unknown role)'} / "
                f"{item['severity']} / {item['promotion_state']} / "
                f"{item['memory_promotion_state']} / {item['summary']}"
            )
    else:
        lines.append("  - (none)")

    lines.append("- checklist:")
    for index, item in enumerate(report["checklist"], start=1):
        line = f"  {index}. {item['summary']}"
        if item.get("reason"):
            line += f" [{item['reason']}]"
        lines.append(line)
        if item.get("command"):
            lines.append(f"     cmd: {item['command']}")

    print("\n".join(lines))
    return 0


def cmd_codex_transcript_report(args: argparse.Namespace) -> int:
    codex_dir = pathlib.Path(args.codex_dir).expanduser()
    session_id, resolution_source, resolved_thread_name = resolve_codex_session_id(
        args.root,
        codex_dir=codex_dir,
        session_id=args.session_id,
        task_id=args.task_id,
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
        task_id=args.task_id,
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
            task_id=args.task_id,
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
        task_id=args.task_id,
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
        task_id=args.task_id,
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
    registry_header, registry_entries = load_list_document(root / ".pm/registry/tasks.yaml", "tasks")
    del registry_header
    tasks_by_id: dict[str, OrderedDict[str, object]] = {
        str(entry["task_id"]): entry for entry in registry_entries if entry.get("task_id")
    }

    role_counts: OrderedDict[str, OrderedDict[str, int]] = OrderedDict()
    for role in sorted(load_roles(root)):
        counts: OrderedDict[str, int] = OrderedDict(
            (status, 0) for status in ("candidate", "committed", "blocked", "done", "deferred")
        )
        for file_status in ("candidate", "committed", "blocked", "done"):
            path = root / f".pm/roles/{role}/backlog/{file_status}.yaml"
            _, entries = load_list_document(path, "tasks")
            for entry in entries:
                status = str(entry.get("status"))
                if status in counts:
                    counts[status] += 1
        role_counts[role] = counts

    task_counts: OrderedDict[str, int] = OrderedDict(
        (status, 0) for status in ("candidate", "committed", "blocked", "done", "deferred")
    )
    for entry in registry_entries:
        status = str(entry.get("status"))
        if status in task_counts:
            task_counts[status] += 1

    stage_current = load_mapping_document(root / ".pm/stage/current.yaml")
    gate = load_mapping_document(root / ".pm/stage/gate.yaml")

    def detail_task(task_id: str) -> dict[str, object]:
        entry = tasks_by_id.get(task_id)
        if entry is None:
            return {"task_id": task_id, "missing": True}
        task_path = root / str(entry["task_path"])
        task_fields = load_mapping_document(task_path) if task_path.is_file() else OrderedDict()
        return {
            "task_id": task_id,
            "missing": False,
            "owner_role": entry.get("owner_role"),
            "status": entry.get("status"),
            "priority": entry.get("priority"),
            "title": task_fields.get("title"),
            "task_path": entry.get("task_path"),
        }

    blocking_ids: list[str] = []
    for task_id in stage_current.get("blocking_tasks", []):
        blocking_ids.append(str(task_id))
    for task_id in gate.get("blocking_tasks", []):
        task_id = str(task_id)
        if task_id not in blocking_ids:
            blocking_ids.append(task_id)
    untracked_blocked_ids: list[str] = []
    for entry in registry_entries:
        if entry.get("status") != "blocked":
            continue
        task_id = str(entry["task_id"])
        if task_id not in blocking_ids:
            untracked_blocked_ids.append(task_id)

    producer_active_header, producer_active_records = load_list_document(
        root / ".pm/roles/producer_system_designer/memory/active.yaml",
        "records",
    )
    del producer_active_header
    shared_active_header, shared_active_records = load_list_document(
        root / ".pm/shared/memory/active.yaml",
        "records",
    )
    del shared_active_header

    def memory_summary(records: list[OrderedDict[str, object]]) -> list[dict[str, object]]:
        result: list[dict[str, object]] = []
        for record in records:
            result.append(
                {
                    "id": record.get("id"),
                    "topic": record.get("topic"),
                    "summary": record.get("summary"),
                    "effective_at": record.get("effective_at"),
                }
            )
        return result

    return {
        "current_stage": stage_current.get("current_stage"),
        "candidate_stage": stage_current.get("candidate_stage"),
        "claim_envelope": stage_current.get("claim_envelope"),
        "decision_date": stage_current.get("decision_date"),
        "updated_from": list(stage_current.get("updated_from", [])),
        "gate": {
            "gate_id": gate.get("gate_id"),
            "status": gate.get("status"),
            "lane_status": list(gate.get("lane_status", [])),
            "updated_from": list(gate.get("updated_from", [])),
        },
        "task_counts": task_counts,
        "role_counts": role_counts,
        "blocking_tasks": [detail_task(task_id) for task_id in blocking_ids],
        "untracked_blocked_tasks": [detail_task(task_id) for task_id in untracked_blocked_ids],
        "memory_inputs": {
            "producer_active": memory_summary(producer_active_records),
            "shared_active": memory_summary(shared_active_records),
        },
    }


def cmd_stage_report(args: argparse.Namespace) -> int:
    report = build_stage_report(args.root)
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
        return 0

    lines = [
        "oasis7 stage report",
        f"- current_stage: {report['current_stage']}",
        f"- candidate_stage: {report['candidate_stage']}",
        f"- claim_envelope: {report['claim_envelope']}",
        f"- decision_date: {report['decision_date']}",
        f"- gate_id: {report['gate']['gate_id']}",
        f"- gate_status: {report['gate']['status']}",
        f"- task_counts: {', '.join(f'{k}={v}' for k, v in report['task_counts'].items())}",
        f"- updated_from: {', '.join(report['updated_from']) or '(none)'}",
        f"- gate_updated_from: {', '.join(report['gate']['updated_from']) or '(none)'}",
        f"- gate_lane_status: {', '.join(report['gate']['lane_status']) or '(none)'}",
        "- blocking_tasks:",
    ]
    if report["blocking_tasks"]:
        for item in report["blocking_tasks"]:
            if item.get("missing"):
                lines.append(f"  - {item['task_id']} (missing)")
            else:
                lines.append(
                    f"  - {item['task_id']} [{item['status']}] {item['owner_role']} / "
                    f"{item['priority']} / {item['title']}"
                )
    else:
        lines.append("  - (none)")

    lines.append("- role_counts:")
    for role, counts in report["role_counts"].items():
        lines.append(
            f"  - {role}: " + ", ".join(f"{status}={count}" for status, count in counts.items())
        )

    lines.append("- untracked_blocked_tasks:")
    if report["untracked_blocked_tasks"]:
        for item in report["untracked_blocked_tasks"]:
            if item.get("missing"):
                lines.append(f"  - {item['task_id']} (missing)")
            else:
                lines.append(
                    f"  - {item['task_id']} [{item['status']}] {item['owner_role']} / "
                    f"{item['priority']} / {item['title']}"
                )
    else:
        lines.append("  - (none)")

    lines.append("- producer_active_memory:")
    if report["memory_inputs"]["producer_active"]:
        for item in report["memory_inputs"]["producer_active"]:
            lines.append(f"  - {item['id']} / {item['topic']} / {item['summary']}")
    else:
        lines.append("  - (none)")

    lines.append("- shared_active_memory:")
    if report["memory_inputs"]["shared_active"]:
        for item in report["memory_inputs"]["shared_active"]:
            lines.append(f"  - {item['id']} / {item['topic']} / {item['summary']}")
    else:
        lines.append("  - (none)")

    print("\n".join(lines))
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="oasis7 .pm store helper")
    subparsers = parser.add_subparsers(dest="command", required=True)

    task_lint = subparsers.add_parser("task-lint")
    task_lint.add_argument("root", type=pathlib.Path)
    task_lint.set_defaults(func=cmd_task_lint)

    task_execution_log_lint = subparsers.add_parser("task-execution-log-lint")
    task_execution_log_lint.add_argument("root", type=pathlib.Path)
    task_execution_log_lint.set_defaults(func=cmd_task_execution_log_lint)

    memory_lint = subparsers.add_parser("memory-lint")
    memory_lint.add_argument("root", type=pathlib.Path)
    memory_lint.set_defaults(func=cmd_memory_lint)

    working_memory_lint = subparsers.add_parser("working-memory-lint")
    working_memory_lint.add_argument("root", type=pathlib.Path)
    working_memory_lint.set_defaults(func=cmd_working_memory_lint)

    memory_report = subparsers.add_parser("memory-report")
    memory_report.add_argument("root", type=pathlib.Path)
    memory_report.add_argument("--role")
    memory_report.add_argument("--include-shared", dest="include_shared", action="store_true")
    memory_report.add_argument("--no-shared", dest="include_shared", action="store_false")
    memory_report.set_defaults(include_shared=True)
    memory_report.add_argument("--stale-after-days", type=int, default=DEFAULT_MEMORY_REVIEW_STALE_DAYS)
    memory_report.add_argument("--json", action="store_true")
    memory_report.set_defaults(func=cmd_memory_report)

    working_memory_report = subparsers.add_parser("working-memory-report")
    working_memory_report.add_argument("root", type=pathlib.Path)
    working_memory_report.add_argument("--task-id")
    working_memory_report.add_argument("--role")
    working_memory_report.add_argument("--json", action="store_true")
    working_memory_report.set_defaults(func=cmd_working_memory_report)

    reflection_report = subparsers.add_parser("reflection-report")
    reflection_report.add_argument("root", type=pathlib.Path)
    reflection_report.add_argument("--role")
    reflection_report.add_argument("--json", action="store_true")
    reflection_report.set_defaults(func=cmd_reflection_report)

    role_report = subparsers.add_parser("role-report")
    role_report.add_argument("root", type=pathlib.Path)
    role_report.add_argument("--role")
    role_report.add_argument("--stale-after-days", type=int, default=DEFAULT_MEMORY_REVIEW_STALE_DAYS)
    role_report.add_argument("--json", action="store_true")
    role_report.set_defaults(func=cmd_role_report)

    workflow_report = subparsers.add_parser("workflow-report")
    workflow_report.add_argument("root", type=pathlib.Path)
    workflow_report.add_argument("--role", required=True)
    workflow_report.add_argument("--phase", choices=("start", "close", "review"), default="start")
    workflow_report.add_argument("--task-id")
    workflow_report.add_argument("--stale-after-days", type=int, default=DEFAULT_MEMORY_REVIEW_STALE_DAYS)
    workflow_report.add_argument("--json", action="store_true")
    workflow_report.set_defaults(func=cmd_workflow_report)

    promote_memory = subparsers.add_parser("promote-memory")
    promote_memory.add_argument("root", type=pathlib.Path)
    promote_memory.add_argument("--signal-id", required=True)
    promote_memory.add_argument("--scope", choices=("role", "shared"), default="role")
    promote_memory.add_argument("--role")
    promote_memory.add_argument("--memory-id")
    promote_memory.add_argument("--topic")
    promote_memory.add_argument("--summary")
    promote_memory.add_argument("--source-ref", action="append", default=[])
    promote_memory.add_argument("--tag", action="append", default=[])
    promote_memory.add_argument("--confidence", default="confirmed")
    promote_memory.add_argument("--promotion-reason")
    promote_memory.add_argument("--reject-reason")
    promote_memory.add_argument("--defer-reason")
    promote_memory.add_argument("--effective-at")
    promote_memory.add_argument("--json", action="store_true")
    promote_memory.set_defaults(func=cmd_promote_memory)

    move_task = subparsers.add_parser("move-task")
    move_task.add_argument("root", type=pathlib.Path)
    move_task.add_argument("--task-id", required=True)
    move_task.add_argument("--to-status", required=True, choices=sorted(TASK_STATUSES))
    move_task.add_argument("--json", action="store_true")
    move_task.set_defaults(func=cmd_move_task)

    supersede_memory = subparsers.add_parser("supersede-memory")
    supersede_memory.add_argument("root", type=pathlib.Path)
    supersede_memory.add_argument("--scope", choices=("role", "shared"), default="role")
    supersede_memory.add_argument("--role")
    supersede_memory.add_argument("--memory-id", required=True)
    supersede_memory.add_argument("--superseded-by", required=True)
    supersede_memory.add_argument("--supersede-reason", required=True)
    supersede_memory.add_argument("--json", action="store_true")
    supersede_memory.set_defaults(func=cmd_supersede_memory)

    stage_report = subparsers.add_parser("stage-report")
    stage_report.add_argument("root", type=pathlib.Path)
    stage_report.add_argument("--json", action="store_true")
    stage_report.set_defaults(func=cmd_stage_report)

    stage_lint = subparsers.add_parser("stage-lint")
    stage_lint.add_argument("root", type=pathlib.Path)
    stage_lint.set_defaults(func=cmd_stage_lint)

    set_stage = subparsers.add_parser("set-stage")
    set_stage.add_argument("root", type=pathlib.Path)
    set_stage.add_argument("--current-stage")
    set_stage.add_argument("--candidate-stage")
    set_stage.add_argument("--clear-candidate-stage", action="store_true")
    set_stage.add_argument("--claim-envelope")
    set_stage.add_argument("--decision-date")
    set_stage.add_argument("--gate-id")
    set_stage.add_argument("--clear-gate-id", action="store_true")
    set_stage.add_argument("--gate-status")
    set_stage.add_argument("--lane-status", action="append", default=[])
    set_stage.add_argument("--clear-lane-status", action="store_true")
    set_stage.add_argument("--blocking-task", action="append", default=[])
    set_stage.add_argument("--clear-blocking-tasks", action="store_true")
    set_stage.add_argument("--source-ref", action="append", default=[])
    set_stage.add_argument("--json", action="store_true")
    set_stage.set_defaults(func=cmd_set_stage)

    codex_transcript_report = subparsers.add_parser("codex-transcript-report")
    codex_transcript_report.add_argument("root", type=pathlib.Path)
    codex_transcript_report.add_argument("--session-id")
    codex_transcript_report.add_argument("--task-id")
    codex_transcript_report.add_argument("--worktree-hint")
    codex_transcript_report.add_argument("--thread-name-pattern")
    codex_transcript_report.add_argument("--codex-dir", default="~/.codex")
    codex_transcript_report.add_argument("--after-ts")
    codex_transcript_report.add_argument("--before-ts")
    codex_transcript_report.add_argument("--json", action="store_true")
    codex_transcript_report.set_defaults(func=cmd_codex_transcript_report)

    import_working_memory = subparsers.add_parser("import-working-memory")
    import_working_memory.add_argument("root", type=pathlib.Path)
    import_working_memory.add_argument("--task-id", required=True)
    import_working_memory.add_argument("--role", required=True)
    import_working_memory.add_argument("--worktree-hint")
    import_working_memory.add_argument("--input-json", required=True)
    import_working_memory.add_argument("--expires-days", type=int, default=DEFAULT_WORKING_MEMORY_EXPIRES_DAYS)
    import_working_memory.add_argument("--session-id")
    import_working_memory.add_argument("--thread-name")
    import_working_memory.add_argument("--codex-dir", default="~/.codex")
    import_working_memory.add_argument("--mapping-updated-at")
    import_working_memory.add_argument("--transcript-source")
    import_working_memory.add_argument("--captured-until-ts")
    import_working_memory.add_argument("--json", action="store_true")
    import_working_memory.set_defaults(func=cmd_import_working_memory)

    promote_working_memory_signal = subparsers.add_parser("promote-working-memory-signal")
    promote_working_memory_signal.add_argument("root", type=pathlib.Path)
    promote_working_memory_signal.add_argument("--task-id", required=True)
    promote_working_memory_signal.add_argument("--role")
    promote_working_memory_signal.add_argument("--entry-id", action="append", default=[])
    promote_working_memory_signal.add_argument("--severity", choices=("low", "medium", "high", "critical"), default="medium")
    promote_working_memory_signal.add_argument("--json", action="store_true")
    promote_working_memory_signal.set_defaults(func=cmd_promote_working_memory_signal)

    working_memory_autoflow = subparsers.add_parser("working-memory-autoflow")
    working_memory_autoflow.add_argument("root", type=pathlib.Path)
    working_memory_autoflow.add_argument("--task-id", required=True)
    working_memory_autoflow.add_argument("--role")
    working_memory_autoflow.add_argument("--entry-id", action="append", default=[])
    working_memory_autoflow.add_argument("--severity", choices=("low", "medium", "high", "critical"), default="medium")
    working_memory_autoflow.add_argument("--priority", choices=tuple(PRIORITY_ORDER.keys()), default="P2")
    working_memory_autoflow.add_argument("--dry-run", action="store_true")
    working_memory_autoflow.add_argument("--json", action="store_true")
    working_memory_autoflow.set_defaults(func=cmd_working_memory_autoflow)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    try:
        return args.func(args)
    except ValueError as exc:
        die(str(exc))
    except FileNotFoundError as exc:
        die(str(exc))


if __name__ == "__main__":
    raise SystemExit(main())
