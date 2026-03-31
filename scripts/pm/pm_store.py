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
DEFAULT_MEMORY_REVIEW_STALE_DAYS = 7
PRIORITY_ORDER = {"P0": 0, "P1": 1, "P2": 2, "P3": 3}
SEVERITY_ORDER = {"critical": 0, "high": 1, "medium": 2, "low": 3}


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


def build_workflow_checklist(
    role: str,
    phase: str,
    role_payload: dict[str, object],
    signal_summary: dict[str, object],
    stage_report: dict[str, object],
) -> list[OrderedDict[str, object]]:
    checklist: list[OrderedDict[str, object]] = []
    backlog_counts = role_payload["backlog_counts"]
    memory_counts = role_payload["memory_counts"]
    gate_status = str(stage_report["gate"]["status"] or "")
    pending_signals = int(signal_summary["pending_count"])

    def add(item_id: str, summary: str, command: str | None = None, reason: str | None = None) -> None:
        item = OrderedDict([("id", item_id), ("summary", summary)])
        if command:
            item["command"] = command
        if reason:
            item["reason"] = reason
        checklist.append(item)

    if phase == "start":
        add(
            "read-docs",
            "先读目标模块 PRD / project 和当天 devlog，再开始编辑。",
        )
        add(
            "role-report",
            f"读取 {role} 的 backlog 与 memory 现状，避免重复处理已知问题。",
            command=f"./scripts/pm/role-report.sh --role {role}",
        )
        if pending_signals > 0:
            add(
                "triage-signals",
                "先处理该角色尚未闭环的 signal，避免 devlog 结论继续停留在 inbox。",
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
        add(
            "write-devlog",
            "先回写当天 devlog，再做 signal / memory / backlog 的结构化收口。",
        )
        add(
            "subagent-review",
            "commit 前必须启动独立 subagent review 当前 diff；review 只用于暴露风险/回归/缺测，不替代 owner role，findings 处理后再提交。",
        )
        if role in {"qa_engineer", "liveops_community"} or pending_signals > 0:
            add(
                "promote-signals",
                "把新增的高价值 QA / liveops / incident 结论提升到 signal inbox，而不是只留在 devlog。",
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
) -> dict[str, object]:
    roles = load_roles(root)
    if role not in roles:
        raise ValueError(f"unknown role: {role}")
    if phase not in {"start", "close", "review"}:
        raise ValueError(f"unsupported phase: {phase}")

    role_report = build_role_report(root, role_filter=role, stale_after_days=stale_after_days)
    role_payload = role_report["roles"][role]
    stage_report = build_stage_report(root)
    signal_role_filter = None if (phase == "review" and role == "producer_system_designer") else role
    signal_summary = build_signal_summary(root, role_filter=signal_role_filter)
    checklist = build_workflow_checklist(role, phase, role_payload, signal_summary, stage_report)

    return {
        "generated_at": now_iso(),
        "phase": phase,
        "role": role,
        "stale_after_days": stale_after_days,
        "role_report": role_payload,
        "signal_summary": signal_summary,
        "stage_report": stage_report,
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
        f"- stale_after_days: {report['stale_after_days']}",
        f"- current_stage: {report['stage_report']['current_stage']}",
        f"- gate_status: {report['stage_report']['gate']['status']}",
        "- backlog_counts: " + ", ".join(f"{status}={count}" for status, count in backlog_counts.items()),
        "- memory_counts: " + ", ".join(f"{status}={count}" for status, count in memory_counts.items()),
        "- signal_counts: " + ", ".join(f"{status}={count}" for status, count in signal_counts.items()),
        "- memory_signal_counts: "
        + ", ".join(f"{status}={count}" for status, count in memory_signal_counts.items()),
        f"- blocked_tasks: {len(report['stage_report']['blocking_tasks'])}",
        "- pending_signals:",
    ]

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

    memory_report = subparsers.add_parser("memory-report")
    memory_report.add_argument("root", type=pathlib.Path)
    memory_report.add_argument("--role")
    memory_report.add_argument("--include-shared", dest="include_shared", action="store_true")
    memory_report.add_argument("--no-shared", dest="include_shared", action="store_false")
    memory_report.set_defaults(include_shared=True)
    memory_report.add_argument("--stale-after-days", type=int, default=DEFAULT_MEMORY_REVIEW_STALE_DAYS)
    memory_report.add_argument("--json", action="store_true")
    memory_report.set_defaults(func=cmd_memory_report)

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
