#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys
from collections import OrderedDict
from datetime import datetime
SAFE_SCALAR_RE = re.compile(r"[A-Za-z0-9_.:/+-]+")
TASK_STATUSES = {"candidate", "committed", "blocked", "done", "deferred"}
LIVE_BACKLOG_STATUSES = {"candidate", "committed", "blocked"}
ALLOWED_SIGNAL_STATES = {"new", "triaged", "promoted_candidate_task", "discarded", "deferred"}
ALLOWED_MEMORY_PROMOTION_STATES = {"pending", "promoted", "rejected", "deferred"}
ALLOWED_PROMOTION_REASONS = {
    "engineering_constraint",
    "failure_signature",
    "policy_boundary",
    "stable_pattern",
    "stage_decision",
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


def cmd_memory_lint(args: argparse.Namespace) -> int:
    run_memory_lint(args.root)
    print("memory-lint: OK")
    return 0


def cmd_task_lint(args: argparse.Namespace) -> int:
    run_task_backlog_lint(args.root)
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
    for entry in registry_entries:
        if entry.get("status") == "blocked":
            task_id = str(entry["task_id"])
            if task_id not in blocking_ids:
                blocking_ids.append(task_id)

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

    memory_lint = subparsers.add_parser("memory-lint")
    memory_lint.add_argument("root", type=pathlib.Path)
    memory_lint.set_defaults(func=cmd_memory_lint)

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
