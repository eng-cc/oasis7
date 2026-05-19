from __future__ import annotations

import json
import pathlib
from collections import OrderedDict
from datetime import datetime, timedelta


def build_memory_report(
    root: pathlib.Path,
    role_filter: str | None,
    include_shared: bool,
    stale_after_days: int,
    *,
    now_iso,
    load_roles,
    parse_timestamp,
    iter_memory_records,
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


def task_sort_key(item: dict[str, object], *, priority_order) -> tuple[object, object, object]:
    return (
        priority_order.get(str(item.get("priority")), 99),
        str(item.get("updated_at") or ""),
        str(item.get("task_uid") or ""),
    )


def normalize_list_field(value: object) -> list[object]:
    if isinstance(value, list):
        return list(value)
    if value in {None, ""}:
        return []
    return [value]


def build_role_report(
    root: pathlib.Path,
    role_filter: str | None,
    stale_after_days: int,
    *,
    now_iso,
    sync_task_views,
    load_roles,
    build_memory_report_impl,
    load_list_document,
    task_registry_path,
    load_mapping_document,
    role_backlog_path,
    priority_order,
) -> dict[str, object]:
    sync_task_views(root)
    roles = sorted(load_roles(root))
    if role_filter and role_filter not in roles:
        raise ValueError(f"unknown role: {role_filter}")

    included_roles = [role_filter] if role_filter else roles
    memory_report = build_memory_report_impl(
        root,
        role_filter=role_filter,
        include_shared=False,
        stale_after_days=stale_after_days,
    )

    registry_header, registry_entries = load_list_document(task_registry_path(root), "tasks")
    del registry_header
    registry_by_id: dict[str, OrderedDict[str, object]] = {
        str(entry["task_uid"]): entry for entry in registry_entries if entry.get("task_uid")
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
        task_uid = str(entry.get("task_uid") or "")
        registry_entry = registry_by_id.get(task_uid, OrderedDict())
        task_path = str(entry.get("task_path") or registry_entry.get("task_path") or "")
        task_fields = load_task_fields(task_path)
        return {
            "task_uid": task_uid,
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
            path = role_backlog_path(root, role, file_status)
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
            task_items.sort(key=lambda item: task_sort_key(item, priority_order=priority_order))

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


def build_signal_summary(
    root: pathlib.Path,
    role_filter: str | None,
    *,
    load_roles,
    load_signal_entries,
    severity_order,
) -> dict[str, object]:
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
            severity_order.get(str(item.get("severity")), 99),
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


def build_reflection_summary(
    root: pathlib.Path,
    role_filter: str | None,
    *,
    sync_task_views,
    load_roles,
    load_list_document,
    task_registry_path,
    load_mapping_document,
    load_signal_entries,
    severity_order,
) -> dict[str, object]:
    sync_task_views(root)
    roles = load_roles(root)
    if role_filter and role_filter not in roles:
        raise ValueError(f"unknown role: {role_filter}")

    _, registry_entries = load_list_document(task_registry_path(root), "tasks")
    tasks_by_signal: dict[str, list[dict[str, object]]] = {}
    for entry in registry_entries:
        source_signal = str(entry.get("source_signal") or "")
        if not source_signal:
            continue
        task_path = root / str(entry.get("task_path") or "")
        fields = load_mapping_document(task_path) if task_path.exists() else OrderedDict()
        task_payload = {
            "task_uid": fields.get("task_uid") or entry.get("task_uid"),
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
            severity_order.get(str(item.get("severity")), 99),
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
                "若当前工作绑定到明确任务，补传 `--task-uid <TASK-UID>` 记录 `last_started_at`，避免 `.pm` workflow 只停留在口头层。",
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
                "若当前工作绑定到明确任务，补传 `--task-uid <TASK-UID>` 记录 `last_closed_at`，否则不能宣称 `.pm` workflow 已完整接入。",
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
                "当前 task 还没有 working_memory；默认不要直接从当前 live Codex session 自读。若确实需要 transcript extraction，请显式给出 `--session-id`，或显式传 `--allow-auto-session` 后再决定是否提炼为 reflection signal / candidate task。",
                command=f"./scripts/pm/codex-working-memory.sh --task-uid {task_context['task_uid']} --role {role} --session-id <session_id>",
            )
        elif working_memory_entries > 0:
            add(
                "review-working-memory",
                "先处理 task-scoped working_memory：提炼成 reflection signal、转 task/memory，或显式保留待过期，不要让过程认知悬空。",
                command="./scripts/pm/working-memory-report.sh --task-uid <TASK-UID>",
                reason=f"working_memory_entries={working_memory_entries}",
            )
            add(
                "autoflow-working-memory",
                "可先用安全默认自动化把 working_memory 提成 reflection signal 和 candidate task，再进入 owner review。",
                command="./scripts/pm/working-memory-autoflow.sh --task-uid <TASK-UID> --severity medium --priority P2",
            )
        if pending_reflections > 0:
            add(
                "review-reflection",
                "处理仍停留在 triaged 的 reflection signal，决定是否转 task/memory/deferred。",
                command=f"./scripts/pm/reflection-report.sh --role {role}",
                reason=f"triaged_reflections={pending_reflections}",
            )
        add(
            "fresh-claim-verification",
            "在宣称“完成 / 测试通过 / 可提 PR”前，先用 claim-ready helper 立即重跑一条 fresh verification 命令；旧结果、局部结果或 agent 自报成功都不能替代当前回合的执行证据。",
            command="./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command '<fresh verification command>'",
        )
        add(
            "prepare-pr-review",
            "默认评审边界在 GitHub PR：完成 commit 后通过 `./scripts/prepare-task-pr.sh` 执行 PR preflight / create，并以 required checks + review/approval 作为正式 review 流程；本地不再要求额外的 pre-commit review 脚本。",
            command="./scripts/prepare-task-pr.sh",
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
            command="./scripts/pm/move-task.sh --task-uid <TASK-UID> --to-status <candidate|committed|blocked|done|deferred>",
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
    task_uid: str | None,
    *,
    now_iso,
    load_roles,
    load_task_context,
    build_role_report_impl,
    build_stage_report_impl,
    build_signal_summary_impl,
    build_working_memory_report,
    build_reflection_summary_impl,
    build_workflow_checklist_impl,
    record_task_workflow_phase,
) -> dict[str, object]:
    roles = load_roles(root)
    if role not in roles:
        raise ValueError(f"unknown role: {role}")
    if phase not in {"start", "close", "review"}:
        raise ValueError(f"unsupported phase: {phase}")

    task_context: dict[str, object] | None = None
    if task_uid:
        task_context = load_task_context(root, task_uid)

    role_report = build_role_report_impl(root, role_filter=role, stale_after_days=stale_after_days)
    role_payload = role_report["roles"][role]
    stage_report = build_stage_report_impl(root)
    signal_role_filter = None if (phase == "review" and role == "producer_system_designer") else role
    signal_summary = build_signal_summary_impl(root, role_filter=signal_role_filter)
    if task_uid:
        working_memory_summary = build_working_memory_report(root, task_uid=task_uid, role_filter=None)
    else:
        working_memory_summary = build_working_memory_report(root, task_uid=None, role_filter=role)
    reflection_summary = build_reflection_summary_impl(root, role_filter=signal_role_filter)
    checklist = build_workflow_checklist_impl(
        role,
        phase,
        task_context,
        role_payload,
        signal_summary,
        stage_report,
        working_memory_summary,
        reflection_summary,
    )

    if task_uid and phase in {"start", "close"}:
        task_context = record_task_workflow_phase(root, task_uid, role, phase)

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


def cmd_memory_report(args, *, load_roles, build_memory_report_impl) -> int:
    if args.role == "shared":
        raise ValueError("--role=shared is invalid; use --include-shared without --role")
    if args.stale_after_days < 0:
        raise ValueError("--stale-after-days must be >= 0")
    if args.role and args.role not in load_roles(args.root):
        raise ValueError(f"unknown role: {args.role}")

    report = build_memory_report_impl(
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


def cmd_working_memory_report(args, *, build_working_memory_report) -> int:
    report = build_working_memory_report(args.root, task_uid=args.task_uid, role_filter=args.role)
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
        for task_uid, payload in report["tasks"].items():
            lines.append(
                f"  - {task_uid} / {payload['role']} / {payload.get('worktree_hint') or '(no worktree_hint)'} / "
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


def cmd_reflection_report(args, *, build_reflection_summary_impl) -> int:
    report = build_reflection_summary_impl(args.root, role_filter=args.role)
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
                ", ".join(str(task["task_uid"]) for task in linked_tasks)
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


def cmd_role_report(args, *, build_role_report_impl) -> int:
    if args.stale_after_days < 0:
        raise ValueError("--stale-after-days must be >= 0")

    report = build_role_report_impl(args.root, role_filter=args.role, stale_after_days=args.stale_after_days)
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
                lines.append(f"    - {item['task_uid']} / {item['priority']} / {item['title']}")
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


def cmd_workflow_report(args, *, build_workflow_report_impl) -> int:
    if args.stale_after_days < 0:
        raise ValueError("--stale-after-days must be >= 0")

    report = build_workflow_report_impl(
        args.root,
        role=args.role,
        phase=args.phase,
        stale_after_days=args.stale_after_days,
        task_uid=args.task_uid,
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
        f"- task_uid: {(report['task_context'] or {}).get('task_uid') or '(none)'}",
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
