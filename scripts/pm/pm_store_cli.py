from __future__ import annotations

import argparse
import pathlib
from collections.abc import Callable

PmHandler = Callable[[argparse.Namespace], int]


def add_task_uid_argument(parser: argparse.ArgumentParser, *, required: bool = False) -> None:
    parser.add_argument("--task-uid", dest="task_uid", required=required)


def build_parser(
    *,
    handlers: dict[str, PmHandler],
    default_memory_review_stale_days: int,
    default_working_memory_expires_days: int,
    priority_choices: tuple[str, ...],
    task_statuses: tuple[str, ...],
) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="oasis7 .pm store helper")
    subparsers = parser.add_subparsers(dest="command", required=True)

    sync_views = subparsers.add_parser("sync-views")
    sync_views.add_argument("root", type=pathlib.Path)
    sync_views.add_argument("--json", action="store_true")
    sync_views.set_defaults(func=handlers["cmd_sync_views"])

    task_lint = subparsers.add_parser("task-lint")
    task_lint.add_argument("root", type=pathlib.Path)
    task_lint.set_defaults(func=handlers["cmd_task_lint"])

    task_execution_log_lint = subparsers.add_parser("task-execution-log-lint")
    task_execution_log_lint.add_argument("root", type=pathlib.Path)
    task_execution_log_lint.set_defaults(func=handlers["cmd_task_execution_log_lint"])

    memory_lint = subparsers.add_parser("memory-lint")
    memory_lint.add_argument("root", type=pathlib.Path)
    memory_lint.set_defaults(func=handlers["cmd_memory_lint"])

    working_memory_lint = subparsers.add_parser("working-memory-lint")
    working_memory_lint.add_argument("root", type=pathlib.Path)
    working_memory_lint.set_defaults(func=handlers["cmd_working_memory_lint"])

    memory_report = subparsers.add_parser("memory-report")
    memory_report.add_argument("root", type=pathlib.Path)
    memory_report.add_argument("--role")
    memory_report.add_argument("--include-shared", dest="include_shared", action="store_true")
    memory_report.add_argument("--no-shared", dest="include_shared", action="store_false")
    memory_report.set_defaults(include_shared=True)
    memory_report.add_argument("--stale-after-days", type=int, default=default_memory_review_stale_days)
    memory_report.add_argument("--json", action="store_true")
    memory_report.set_defaults(func=handlers["cmd_memory_report"])

    new_task = subparsers.add_parser("new-task")
    new_task.add_argument("root", type=pathlib.Path)
    new_task.add_argument("--owner-role", required=True)
    new_task.add_argument("--title", required=True)
    new_task.add_argument("--priority", choices=priority_choices, default="P2")
    new_task.add_argument("--source-signal")
    new_task.add_argument("--source-ref", action="append", default=[], required=True)
    new_task.add_argument("--doc-ref", action="append", default=[])
    new_task.add_argument("--related-prd", action="append", default=[])
    new_task.add_argument("--acceptance", action="append", default=[])
    new_task.add_argument("--handoff-to", action="append", default=[])
    new_task.add_argument("--worktree-hint")
    new_task.add_argument("--json", action="store_true")
    new_task.set_defaults(func=handlers["cmd_new_task"])

    working_memory_report = subparsers.add_parser("working-memory-report")
    working_memory_report.add_argument("root", type=pathlib.Path)
    add_task_uid_argument(working_memory_report)
    working_memory_report.add_argument("--role")
    working_memory_report.add_argument("--json", action="store_true")
    working_memory_report.set_defaults(func=handlers["cmd_working_memory_report"])

    reflection_report = subparsers.add_parser("reflection-report")
    reflection_report.add_argument("root", type=pathlib.Path)
    reflection_report.add_argument("--role")
    reflection_report.add_argument("--json", action="store_true")
    reflection_report.set_defaults(func=handlers["cmd_reflection_report"])

    role_report = subparsers.add_parser("role-report")
    role_report.add_argument("root", type=pathlib.Path)
    role_report.add_argument("--role")
    role_report.add_argument("--stale-after-days", type=int, default=default_memory_review_stale_days)
    role_report.add_argument("--json", action="store_true")
    role_report.set_defaults(func=handlers["cmd_role_report"])

    workflow_report = subparsers.add_parser("workflow-report")
    workflow_report.add_argument("root", type=pathlib.Path)
    workflow_report.add_argument("--role", required=True)
    workflow_report.add_argument("--phase", choices=("start", "close", "review"), default="start")
    add_task_uid_argument(workflow_report)
    workflow_report.add_argument("--stale-after-days", type=int, default=default_memory_review_stale_days)
    workflow_report.add_argument("--json", action="store_true")
    workflow_report.set_defaults(func=handlers["cmd_workflow_report"])

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
    promote_memory.set_defaults(func=handlers["cmd_promote_memory"])

    move_task = subparsers.add_parser("move-task")
    move_task.add_argument("root", type=pathlib.Path)
    add_task_uid_argument(move_task, required=True)
    move_task.add_argument("--to-status", required=True, choices=task_statuses)
    move_task.add_argument("--json", action="store_true")
    move_task.set_defaults(func=handlers["cmd_move_task"])

    supersede_memory = subparsers.add_parser("supersede-memory")
    supersede_memory.add_argument("root", type=pathlib.Path)
    supersede_memory.add_argument("--scope", choices=("role", "shared"), default="role")
    supersede_memory.add_argument("--role")
    supersede_memory.add_argument("--memory-id", required=True)
    supersede_memory.add_argument("--superseded-by", required=True)
    supersede_memory.add_argument("--supersede-reason", required=True)
    supersede_memory.add_argument("--json", action="store_true")
    supersede_memory.set_defaults(func=handlers["cmd_supersede_memory"])

    stage_report = subparsers.add_parser("stage-report")
    stage_report.add_argument("root", type=pathlib.Path)
    stage_report.add_argument("--json", action="store_true")
    stage_report.set_defaults(func=handlers["cmd_stage_report"])

    stage_lint = subparsers.add_parser("stage-lint")
    stage_lint.add_argument("root", type=pathlib.Path)
    stage_lint.set_defaults(func=handlers["cmd_stage_lint"])

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
    set_stage.set_defaults(func=handlers["cmd_set_stage"])

    codex_transcript_report = subparsers.add_parser("codex-transcript-report")
    codex_transcript_report.add_argument("root", type=pathlib.Path)
    codex_transcript_report.add_argument("--session-id")
    add_task_uid_argument(codex_transcript_report)
    codex_transcript_report.add_argument("--worktree-hint")
    codex_transcript_report.add_argument("--thread-name-pattern")
    codex_transcript_report.add_argument("--codex-dir", default="~/.codex")
    codex_transcript_report.add_argument("--after-ts")
    codex_transcript_report.add_argument("--before-ts")
    codex_transcript_report.add_argument("--json", action="store_true")
    codex_transcript_report.set_defaults(func=handlers["cmd_codex_transcript_report"])

    import_working_memory = subparsers.add_parser("import-working-memory")
    import_working_memory.add_argument("root", type=pathlib.Path)
    add_task_uid_argument(import_working_memory, required=True)
    import_working_memory.add_argument("--role", required=True)
    import_working_memory.add_argument("--worktree-hint")
    import_working_memory.add_argument("--input-json", required=True)
    import_working_memory.add_argument("--expires-days", type=int, default=default_working_memory_expires_days)
    import_working_memory.add_argument("--session-id")
    import_working_memory.add_argument("--thread-name")
    import_working_memory.add_argument("--codex-dir", default="~/.codex")
    import_working_memory.add_argument("--mapping-updated-at")
    import_working_memory.add_argument("--transcript-source")
    import_working_memory.add_argument("--captured-until-ts")
    import_working_memory.add_argument("--json", action="store_true")
    import_working_memory.set_defaults(func=handlers["cmd_import_working_memory"])

    promote_working_memory_signal = subparsers.add_parser("promote-working-memory-signal")
    promote_working_memory_signal.add_argument("root", type=pathlib.Path)
    add_task_uid_argument(promote_working_memory_signal, required=True)
    promote_working_memory_signal.add_argument("--role")
    promote_working_memory_signal.add_argument("--entry-id", action="append", default=[])
    promote_working_memory_signal.add_argument("--severity", choices=("low", "medium", "high", "critical"), default="medium")
    promote_working_memory_signal.add_argument("--json", action="store_true")
    promote_working_memory_signal.set_defaults(func=handlers["cmd_promote_working_memory_signal"])

    working_memory_autoflow = subparsers.add_parser("working-memory-autoflow")
    working_memory_autoflow.add_argument("root", type=pathlib.Path)
    add_task_uid_argument(working_memory_autoflow, required=True)
    working_memory_autoflow.add_argument("--role")
    working_memory_autoflow.add_argument("--entry-id", action="append", default=[])
    working_memory_autoflow.add_argument("--severity", choices=("low", "medium", "high", "critical"), default="medium")
    working_memory_autoflow.add_argument("--priority", choices=priority_choices, default="P2")
    working_memory_autoflow.add_argument("--dry-run", action="store_true")
    working_memory_autoflow.add_argument("--json", action="store_true")
    working_memory_autoflow.set_defaults(func=handlers["cmd_working_memory_autoflow"])

    migrate_task_identity_parser = subparsers.add_parser("migrate-task-identity")
    migrate_task_identity_parser.add_argument("root", type=pathlib.Path)
    migrate_task_identity_parser.add_argument("--json", action="store_true")
    migrate_task_identity_parser.set_defaults(func=handlers["cmd_migrate_task_identity"])

    return parser
