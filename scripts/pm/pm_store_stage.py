from __future__ import annotations

import json
import pathlib
import sys
from collections import OrderedDict
from datetime import datetime


def run_stage_lint(
    root: pathlib.Path,
    *,
    sync_task_views,
    load_mapping_document,
    load_list_document,
    task_registry_path,
    load_active_memory_record,
    validate_runtime_source_ref,
) -> None:
    sync_task_views(root)
    failures: list[str] = []

    def fail(message: str) -> None:
        failures.append(message)

    stage_current = load_mapping_document(root / ".pm/stage/current.yaml")
    gate = load_mapping_document(root / ".pm/stage/gate.yaml")
    _, registry_entries = load_list_document(task_registry_path(root), "tasks")
    task_uids = {str(entry.get("task_uid")) for entry in registry_entries if entry.get("task_uid")}

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
        else:
            for source_ref in updated_from:
                try:
                    validate_runtime_source_ref(root, str(source_ref), "stage current updated_from")
                except ValueError as exc:
                    fail(str(exc))
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
        else:
            for source_ref in gate_updated_from:
                try:
                    validate_runtime_source_ref(root, str(source_ref), "gate updated_from")
                except ValueError as exc:
                    fail(str(exc))
        if gate.get("gate_id") in {None, ""}:
            fail("gate requires gate_id once status leaves draft")

    for doc_name, blocking_tasks in (
        (".pm/stage/current.yaml", stage_current.get("blocking_tasks", [])),
        (".pm/stage/gate.yaml", gate.get("blocking_tasks", [])),
    ):
        if not isinstance(blocking_tasks, list):
            fail(f"{doc_name} blocking_tasks must be a list")
            continue
        for task_uid in blocking_tasks:
            if str(task_uid) not in task_uids:
                fail(f"{doc_name} references missing blocking task: {task_uid}")

    tracked_blocking_ids = {
        str(task_uid) for task_uid in list(stage_current.get("blocking_tasks", [])) + list(gate.get("blocking_tasks", []))
    }
    for entry in registry_entries:
        if entry.get("status") != "blocked":
            continue
        task_uid = str(entry.get("task_uid"))
        if task_uid not in tracked_blocking_ids:
            fail(f"blocked task missing from stage/gate blocking_tasks: {task_uid}")

    if failures:
        for failure in failures:
            print(f"stage-lint: FAIL: {failure}", file=sys.stderr)
        raise SystemExit(1)


def cmd_stage_lint(args, *, run_stage_lint_impl) -> int:
    run_stage_lint_impl(args.root)
    print("stage-lint: OK")
    return 0


def cmd_set_stage(
    args,
    *,
    validate_runtime_source_ref,
    load_mapping_document,
    dump_mapping_document,
    run_stage_lint_impl,
) -> int:
    if not args.source_ref:
        raise ValueError("at least one --source-ref is required")
    for source_ref in args.source_ref:
        validate_runtime_source_ref(args.root, str(source_ref), "set-stage source_ref")

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
        run_stage_lint_impl(args.root)
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


def build_stage_report(
    root: pathlib.Path,
    *,
    sync_task_views,
    load_list_document,
    task_registry_path,
    load_roles,
    role_backlog_path,
    load_mapping_document,
) -> dict[str, object]:
    sync_task_views(root)
    registry_header, registry_entries = load_list_document(task_registry_path(root), "tasks")
    del registry_header
    tasks_by_id: dict[str, OrderedDict[str, object]] = {
        str(entry["task_uid"]): entry for entry in registry_entries if entry.get("task_uid")
    }

    role_counts: OrderedDict[str, OrderedDict[str, int]] = OrderedDict()
    for role in sorted(load_roles(root)):
        counts: OrderedDict[str, int] = OrderedDict(
            (status, 0) for status in ("candidate", "committed", "blocked", "done", "deferred")
        )
        for file_status in ("candidate", "committed", "blocked", "done"):
            path = role_backlog_path(root, role, file_status)
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

    def detail_task(task_uid: str) -> dict[str, object]:
        entry = tasks_by_id.get(task_uid)
        if entry is None:
            return {"task_uid": task_uid, "missing": True}
        task_path = root / str(entry["task_path"])
        task_fields = load_mapping_document(task_path) if task_path.is_file() else OrderedDict()
        return {
            "task_uid": task_uid,
            "missing": False,
            "owner_role": entry.get("owner_role"),
            "status": entry.get("status"),
            "priority": entry.get("priority"),
            "title": task_fields.get("title"),
            "task_path": entry.get("task_path"),
        }

    blocking_ids: list[str] = []
    for task_uid in stage_current.get("blocking_tasks", []):
        blocking_ids.append(str(task_uid))
    for task_uid in gate.get("blocking_tasks", []):
        task_uid = str(task_uid)
        if task_uid not in blocking_ids:
            blocking_ids.append(task_uid)
    untracked_blocked_ids: list[str] = []
    for entry in registry_entries:
        if entry.get("status") != "blocked":
            continue
        task_uid = str(entry["task_uid"])
        if task_uid not in blocking_ids:
            untracked_blocked_ids.append(task_uid)

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
        "blocking_tasks": [detail_task(task_uid) for task_uid in blocking_ids],
        "untracked_blocked_tasks": [detail_task(task_uid) for task_uid in untracked_blocked_ids],
        "memory_inputs": {
            "producer_active": memory_summary(producer_active_records),
            "shared_active": memory_summary(shared_active_records),
        },
    }


def cmd_stage_report(args, *, build_stage_report_impl) -> int:
    report = build_stage_report_impl(args.root)
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
                lines.append(f"  - {item['task_uid']} (missing)")
            else:
                lines.append(
                    f"  - {item['task_uid']} [{item['status']}] {item['owner_role']} / "
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
                lines.append(f"  - {item['task_uid']} (missing)")
            else:
                lines.append(
                    f"  - {item['task_uid']} [{item['status']}] {item['owner_role']} / "
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
