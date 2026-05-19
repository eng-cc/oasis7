from __future__ import annotations

import json
import pathlib
import sys
from collections import OrderedDict
from datetime import datetime


def run_task_backlog_lint(
    root: pathlib.Path,
    *,
    sync_task_views,
    load_roles,
    collect_signals,
    is_devlog_archive_reference,
    resolve_source_ref_path,
    parse_reference_path,
    load_list_document,
    task_registry_path,
    load_mapping_document,
    task_execution_log_relative_path,
    role_backlog_path,
    backlog_file_for_status,
    task_execution_log_entry_re,
    allowed_signal_states,
    allowed_memory_promotion_states,
    allowed_promotion_reasons,
    allowed_memory_rejection_reasons,
    task_statuses,
) -> None:
    sync_task_views(root)
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
        if str(payload.get("source_type") or "") == "devlog":
            fail(f"signal source_type must not be devlog archive: {payload['signal_id']}")
        source_ref = str(payload.get("source_ref") or "")
        if not source_ref:
            fail(f"signal source_ref missing: {payload['signal_id']}")
        else:
            if is_devlog_archive_reference(source_ref):
                fail(f"signal source_ref must not use doc/devlog archive: {payload['signal_id']} -> {source_ref}")
            try:
                resolved_signal_source = resolve_source_ref_path(root, source_ref)
            except ValueError as exc:
                fail(f"signal source_ref invalid: {payload['signal_id']} -> {exc}")
            else:
                if not resolved_signal_source.exists():
                    fail(
                        f"signal source_ref missing: {payload['signal_id']} -> "
                        f"{parse_reference_path(source_ref)}"
                    )
        if payload["promotion_state"] not in allowed_signal_states:
            fail(f"signal promotion_state invalid: {payload['signal_id']} -> {payload['promotion_state']}")
        memory_state = str(payload.get("memory_promotion_state", "pending"))
        if memory_state not in allowed_memory_promotion_states:
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
            elif payload["memory_promotion_reason"] not in allowed_promotion_reasons:
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
            elif payload["memory_rejection_reason"] not in allowed_memory_rejection_reasons:
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

    registry_header, registry_entries = load_list_document(task_registry_path(root), "tasks")
    if str(registry_header.get("identity_key") or "") != "task_uid":
        fail("tasks registry identity_key mismatch: expected task_uid")

    registry_by_id: dict[str, OrderedDict[str, object]] = {}
    for entry in registry_entries:
        task_uid = str(entry.get("task_uid") or "")
        if not task_uid:
            fail("registry task missing task_uid")
            continue
        if task_uid in registry_by_id:
            fail(f"duplicate registry task_uid: {task_uid}")
            continue
        registry_by_id[task_uid] = entry
        owner_role = entry.get("owner_role")
        status = entry.get("status")
        if owner_role not in roles:
            fail(f"registry task owner_role not registered: {task_uid} -> {owner_role}")
        if status not in task_statuses:
            fail(f"registry task status invalid: {task_uid} -> {status}")
        task_path = entry.get("task_path")
        if not task_path or not (root / str(task_path)).is_file():
            fail(f"registry task path missing: {task_uid} -> {task_path}")
        source_signal = entry.get("source_signal")
        if source_signal and str(source_signal) not in signal_ids:
            fail(f"registry task source_signal missing from inbox: {task_uid} -> {source_signal}")

    task_source_signals: set[str] = set()
    task_files = sorted(path for path in (root / ".pm/tasks").glob("*.yaml") if path.is_file())
    if len(task_files) != len(registry_entries):
        fail(f"task file count mismatch: files={len(task_files)} registry={len(registry_entries)}")

    task_fields_by_id: dict[str, OrderedDict[str, object]] = {}
    for path in task_files:
        fields = load_mapping_document(path)
        task_uid = fields.get("task_uid")
        if not task_uid:
            fail(f"task file missing task_uid: {path.relative_to(root)}")
            continue
        task_uid = str(task_uid)
        task_fields_by_id[task_uid] = fields
        owner_role = fields.get("owner_role")
        status = fields.get("status")
        if owner_role not in roles:
            fail(f"task file owner_role not registered: {task_uid} -> {owner_role}")
        if status not in task_statuses:
            fail(f"task file status invalid: {task_uid} -> {status}")
        registry_entry = registry_by_id.get(task_uid)
        if registry_entry is None:
            fail(f"task file missing from registry: {task_uid}")
        else:
            if registry_entry.get("owner_role") != owner_role:
                fail(f"registry owner_role mismatch: {task_uid}")
            if registry_entry.get("status") != status:
                fail(f"registry status mismatch: {task_uid}")
            if registry_entry.get("priority") != fields.get("priority"):
                fail(f"registry priority mismatch: {task_uid}")
            expected_path = f".pm/tasks/{path.name}"
            if registry_entry.get("task_path") != expected_path:
                fail(f"registry task_path mismatch: {task_uid} -> {registry_entry.get('task_path')} != {expected_path}")
        source_signal = fields.get("source_signal")
        if source_signal:
            task_source_signals.add(str(source_signal))
            if str(source_signal) not in signal_ids:
                fail(f"task source_signal missing from inbox: {task_uid} -> {source_signal}")
        source_refs = fields.get("source_refs")
        if not isinstance(source_refs, list) or not source_refs:
            fail(f"task file source_refs must be a non-empty list: {task_uid}")
        else:
            for source_ref in source_refs:
                if is_devlog_archive_reference(str(source_ref)):
                    fail(f"task file source_ref must not use doc/devlog archive: {task_uid} -> {source_ref}")
                try:
                    resolved_source = resolve_source_ref_path(root, str(source_ref))
                except ValueError as exc:
                    fail(f"task file source_ref invalid: {task_uid} -> {exc}")
                else:
                    if not resolved_source.exists():
                        fail(
                            f"task file source_ref missing: {task_uid} -> "
                            f"{parse_reference_path(str(source_ref))}"
                        )
        for key in ("last_started_at", "last_closed_at"):
            value = fields.get(key)
            if value in {None, ""}:
                continue
            try:
                datetime.fromisoformat(str(value))
            except ValueError:
                fail(f"task file invalid {key}: {task_uid} -> {value}")
        if fields.get("last_started_at") not in {None, ""} and fields.get("last_closed_at") not in {None, ""}:
            try:
                started_at = datetime.fromisoformat(str(fields["last_started_at"]))
                closed_at = datetime.fromisoformat(str(fields["last_closed_at"]))
                if closed_at < started_at:
                    fail(f"task file close precedes start: {task_uid}")
            except ValueError:
                pass
        if status in {"blocked", "done", "deferred"} and fields.get("last_started_at") in {None, ""}:
            fail(f"task file missing last_started_at for started workflow task: {task_uid}")
        if status in {"done", "deferred"} and fields.get("last_closed_at") in {None, ""}:
            fail(f"task file missing last_closed_at for closed workflow task: {task_uid}")
        execution_log_path = str(fields.get("execution_log_path") or "")
        expected_execution_log_path = task_execution_log_relative_path(task_uid)
        if execution_log_path != expected_execution_log_path:
            fail(
                f"task file execution_log_path mismatch: {task_uid} -> "
                f"{execution_log_path or '(missing)'} != {expected_execution_log_path}"
            )
            continue
        execution_log_file = root / execution_log_path
        if not execution_log_file.is_file():
            fail(f"task execution log missing: {task_uid} -> {execution_log_path}")
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
                            f"{execution_log_path}:{active_entry_line}: execution log entry missing 完成内容 for {task_uid}"
                        )
                    if not entry_has_pending:
                        fail(
                            f"{execution_log_path}:{active_entry_line}: execution log entry missing 遗留事项 for {task_uid}"
                        )
                match = task_execution_log_entry_re.fullmatch(raw_line)
                if not match:
                    fail(f"{execution_log_path}:{line_no}: invalid execution log heading for {task_uid}")
                    active_entry_line = None
                    entry_has_done = False
                    entry_has_pending = False
                    continue
                role_name = match.group(3)
                if role_name not in roles:
                    fail(f"{execution_log_path}:{line_no}: unknown role in execution log for {task_uid}: {role_name}")
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
                fail(f"{execution_log_path}:{active_entry_line}: execution log entry missing 完成内容 for {task_uid}")
            if not entry_has_pending:
                fail(f"{execution_log_path}:{active_entry_line}: execution log entry missing 遗留事项 for {task_uid}")
        if require_entry and entry_count == 0:
            fail(f"task execution log requires at least one entry: {task_uid}")

    for signal_id in promoted_signal_ids:
        if signal_id not in task_source_signals:
            fail(f"promoted signal has no task file: {signal_id}")

    backlog_membership: dict[str, list[tuple[str, str, OrderedDict[str, object]]]] = {}
    for role in sorted(roles):
        for file_status in ("candidate", "committed", "blocked", "done"):
            path = role_backlog_path(root, role, file_status)
            header, entries = load_list_document(path, "tasks")
            if header.get("role") != role:
                fail(f"{path.relative_to(root)} role header mismatch: {header.get('role')} != {role}")
            if header.get("status") != file_status:
                fail(f"{path.relative_to(root)} status header mismatch: {header.get('status')} != {file_status}")
            for entry in entries:
                task_uid = entry.get("task_uid")
                if not task_uid:
                    fail(f"{path.relative_to(root)} entry missing task_uid")
                    continue
                task_uid = str(task_uid)
                entry_status = entry.get("status")
                if file_status != "done" and entry_status != file_status:
                    fail(f"{path.relative_to(root)} {task_uid} entry status mismatch: {entry_status} != {file_status}")
                if file_status == "done" and entry_status not in {"done", "deferred"}:
                    fail(f"{path.relative_to(root)} {task_uid} invalid done-lane status: {entry_status}")
                backlog_membership.setdefault(task_uid, []).append((role, file_status, entry))

    for task_uid, registry_entry in registry_by_id.items():
        memberships = backlog_membership.get(task_uid, [])
        if len(memberships) != 1:
            fail(f"task backlog membership mismatch: {task_uid} has {len(memberships)} entries")
            continue
        role, file_status, entry = memberships[0]
        if role != registry_entry.get("owner_role"):
            fail(f"task backlog owner mismatch: {task_uid} -> {role} != {registry_entry.get('owner_role')}")
        expected_file_status = backlog_file_for_status(str(registry_entry.get("status")))
        if file_status != expected_file_status[:-5]:
            fail(f"task backlog lane mismatch: {task_uid} -> {file_status} != {expected_file_status[:-5]}")
        task_fields = task_fields_by_id.get(task_uid)
        if task_fields is None:
            continue
        for key in ("title", "priority", "source_signal", "status"):
            if entry.get(key) != task_fields.get(key):
                fail(f"task backlog field mismatch: {task_uid} -> {key}")

    for task_uid, memberships in backlog_membership.items():
        if task_uid not in registry_by_id:
            fail(f"backlog task missing from registry: {task_uid}")
            continue
        task_fields = task_fields_by_id.get(task_uid)
        if task_fields is None:
            continue
        for _, file_status, entry in memberships:
            if entry.get("status") != task_fields.get("status"):
                fail(f"backlog/task status mismatch: {task_uid}")
            if file_status != backlog_file_for_status(str(task_fields.get("status")))[:-5]:
                fail(f"backlog/task lane mismatch: {task_uid}")

    if failures:
        for failure in failures:
            print(f"pm-lint: FAIL: {failure}", file=sys.stderr)
        raise SystemExit(1)
