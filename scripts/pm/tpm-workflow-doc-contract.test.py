#!/usr/bin/env python3
"""Executable RED contract for a concise, unambiguous TPM workflow spec."""

from __future__ import annotations

import json
import re
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "doc/engineering/workflow/source-of-truth.md"
AGENTS = ROOT / "AGENTS.md"
FINISHING = ROOT / ".agents/skills/finishing-a-development-branch/SKILL.md"
EXECUTING_PROJECT_TASKS = ROOT / ".agents/skills/executing-project-tasks/SKILL.md"
TPM_ROLE = ROOT / ".agents/roles/tpm.md"
PM_README = ROOT / ".pm/README.md"
VERIFICATION_SKILL = ROOT / ".agents/skills/verification-before-completion/SKILL.md"
SUPERVISOR_SKILL = ROOT / ".agents/skills/tpm-production-supervisor/SKILL.md"
SUPERVISOR_DESIGN = ROOT / "doc/engineering/workflow/production-supervisor-runtime.design.md"
ENGINEERING_README = ROOT / "doc/engineering/README.md"
ENGINEERING_PRD = ROOT / "doc/engineering/prd.md"
ENGINEERING_PRD_INDEX = ROOT / "doc/engineering/prd.index.md"
PM_REPORTING = ROOT / "scripts/pm/pm_store_reporting.py"
SKILLS_README = ROOT / ".agents/skills/README.md"
PROJECT_TASK = ROOT / "scripts/pm/github-project-task.py"
PROJECT_SYNC = ROOT / "scripts/pm/github-project-sync.py"
PROJECT_WORKFLOW = ROOT / "scripts/pm/github-project-workflow.py"
FINALIZER = ROOT / "scripts/pm/post-merge-finalize.py"
WORKFLOW_EVAL = ROOT / "scripts/pm/workflow-behavior-eval.sh"
ROLE_FIT = ROOT / "scripts/pm/verify-codex-subagent-role-fit.sh"
HUMAN_REVIEW_ENTRYPOINTS = (
    ROOT / "scripts/pm/record-pre-pr-review.sh",
    ROOT / "scripts/prepare-task-pr.sh",
    ROOT / "scripts/pm/task-closeout.sh",
)
BOOTSTRAP_SKILL = ROOT / ".agents/skills/default-workflow-bootstrap/SKILL.md"
RECEIVING_CODE_REVIEW_SKILL = ROOT / ".agents/skills/receiving-code-review/SKILL.md"
WORKFLOW_ROUTER_SKILL = ROOT / ".agents/skills/repo-owned-workflow-router/SKILL.md"
PREPARE_TASK_PR = ROOT / "scripts/prepare-task-pr.sh"
SCRIPTS_PRD = ROOT / "doc/scripts/prd.md"


class WorkflowDocumentationContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = SOURCE.read_text(encoding="utf-8")
        cls.supervisor_design = SUPERVISOR_DESIGN.read_text(encoding="utf-8")

    def section(self, heading: str) -> str:
        match = re.search(
            rf"(?ms)^##+\s+{re.escape(heading)}\s*$\n(.*?)(?=^##+\s|\Z)", self.text
        )
        self.assertIsNotNone(match, f"missing canonical section: {heading}")
        return match.group(1)

    def canonical_source_line_budget(self) -> int:
        match = re.search(
            r"one (\d+)-line budget, enforced by `scripts/pm/tpm-workflow-doc-contract\.test\.py`",
            self.text,
        )
        self.assertIsNotNone(match, "source must define one canonical line budget")
        return int(match.group(1))

    def test_capability_status_table_distinguishes_reality(self) -> None:
        table = self.section("Capability status")
        for state in ("implemented", "test-only", "blocked"):
            self.assertRegex(table, rf"(?im)^\|[^\n]*\b{state}\b[^\n]*\|$")
        self.assertIn("production supervisor", table.lower())
        self.assertRegex(table.lower(), r"production supervisor[^\n]*\bblocked\b")

    def test_target_supervisor_contract_is_canonical_and_companion_is_non_normative(self) -> None:
        capability = self.section("Capability status")
        states = self.section("Workflow states")
        self.assertIn("production-supervisor-runtime.design.md", capability)
        declaration_match = re.search(
            r"(?is)requires\s+four\s+independently observable producer classes:\s*"
            r"(?P<declaration>[^.]+)\.\s*",
            capability,
        )
        self.assertIsNotNone(
            declaration_match,
            "Capability status must publish one four-class producer declaration",
        )
        raw_producers = re.split(
            r"\s*,\s*|\s+and\s+", declaration_match.group("declaration").strip()
        )
        producers = set()
        for producer in raw_producers:
            normalized_producer = producer.strip().lower()
            normalized_producer = re.sub(r"^and\s+", "", normalized_producer)
            normalized_producer = re.sub(
                r"^(?:task-bound|runtime-issued|durable)\s+", "", normalized_producer
            )
            normalized_producer = re.sub(r"/readback$", "", normalized_producer)
            normalized_producer = re.sub(
                r"/return\s+attestation", "/attestation", normalized_producer
            )
            normalized_producer = re.sub(r"\s+delivery$", "", normalized_producer)
            producers.add(normalized_producer)
        expected_producers = {
            "mechanical/bootstrap action",
            "independent live validator",
            "collaboration/attestation",
            "wake/scheduler",
        }
        self.assertEqual(4, len(raw_producers))
        self.assertEqual(expected_producers, producers)
        self.assertIn("all four", capability.lower())
        self.assertIn("QA", capability)
        self.assertIn("wait_class", states)
        self.assertNotIn("external_wait.system", states)
        self.assertNotIn("external_wait.policy", states)
        self.assertRegex(self.supervisor_design, r"(?i)non-normative")

    def test_supervisor_milestones_and_live_evaluation_boundary_are_explicit(self) -> None:
        milestone_headings = (
            "M1 — durable supervisor core",
            "M2 — trusted mechanical path",
            "M3 — trusted collaboration",
            "M4 — wake/event runtime",
        )
        positions = []
        for heading in milestone_headings:
            with self.subTest(heading=heading):
                self.assertIn(heading, self.supervisor_design)
                positions.append(self.supervisor_design.index(heading))
        self.assertEqual(sorted(positions), positions)

        canonical = re.sub(r"\s+", " ", self.text.lower())
        self.assertRegex(
            canonical,
            r"no (?:single )?milestone(?: or fixture)?[^.]{0,120}"
            r"(?:change|alter)[^.]{0,80}(?:capability|status)",
        )
        self.assertRegex(
            canonical,
            r"(?is)(?=[^.!?]*(?:live|staging)[^.!?]*(?:evaluation|evidence))"
            r"(?=[^.!?]*supervisor[- ]runtime)"
            r"(?=[^.!?]*(?:target(?:ed)?|promotion))"
            r"(?=[^.!?]*(?:not|rather than)[^.!?]*"
            r"(?:universal|every|each)[^.!?]*(?:workflow|document))"
            r"[^.!?]+",
        )

    def test_supervisor_thin_surfaces_link_without_copying_target_matrices(self) -> None:
        surfaces = (
            ENGINEERING_README,
            ENGINEERING_PRD_INDEX,
            SKILLS_README,
            SUPERVISOR_SKILL,
        )
        supervisor_links = (
            "production-supervisor-runtime.design.md",
            "source-of-truth.md#appendix-a-target-supervisor-contract",
            "source-of-truth.md#capability-and-ownership",
        )
        producer_rows = (
            "trusted mechanical/bootstrap action",
            "independent live validator/readback",
            "trusted collaboration/attestation",
            "wake/scheduler runtime",
        )
        milestone_rows = (
            "m1 — durable supervisor core",
            "m2 — trusted mechanical path",
            "m3 — trusted collaboration",
            "m4 — wake/event runtime",
        )
        for path in surfaces:
            normalized = re.sub(r"\s+", " ", path.read_text(encoding="utf-8").lower())
            with self.subTest(path=path):
                self.assertTrue(
                    any(link in normalized for link in supervisor_links),
                    f"{path} must link the canonical supervisor contract or companion",
                )
                self.assertFalse(
                    all(row in normalized for row in producer_rows),
                    f"{path} must not copy the four-row producer matrix",
                )
                self.assertFalse(
                    all(row in normalized for row in milestone_rows),
                    f"{path} must not copy the M1-M4 acceptance matrix",
                )

    def test_stable_required_gate_wait_uses_bounded_codex_heartbeat(self) -> None:
        budget = self.section("GitHub query budget and terminal defaults")
        normalized = re.sub(r"\s+", " ", budget.lower())
        for term in (
            "required-gate",
            "codex surface",
            "must stop polling in the active turn",
            "continuation/heartbeat",
            "roughly ten minutes",
            "one batched current-head gate read",
            "unchanged state stays quiet",
            "workflow violation",
            "timeout without a meaningful state change",
            "not an unattended production supervisor",
            "finite, bounded",
        ):
            with self.subTest(term=term):
                self.assertIn(term, normalized)

        self.assertNotIn("back off to 300", normalized)

        finishing = FINISHING.read_text(encoding="utf-8").lower()
        self.assertIn("source-of-truth.md#stable-required-gate-wait", finishing)
        self.assertIn("non-codex surface", finishing)
        self.assertIn(
            "./scripts/pm/pr-lifecycle-gate.py <pr-number> --task-uid <task_uid> --json",
            finishing,
        )
        for duplicated_policy in (
            "roughly ten minutes",
            "one batched current-head gate read",
            "unchanged state stays quiet",
            "not an unattended production supervisor",
        ):
            with self.subTest(duplicated_policy=duplicated_policy):
                self.assertNotIn(duplicated_policy, finishing)

    def test_early_not_planned_outcome_routes_to_non_merge_terminal_entry(self) -> None:
        router = WORKFLOW_ROUTER_SKILL.read_text(encoding="utf-8")
        finishing = FINISHING.read_text(encoding="utf-8")
        for phase in ("bootstrap", "planning", "execution"):
            self.assertIn(phase, router)
        self.assertIn("not_planned", router)
        self.assertIn("finishing-a-development-branch", router)
        self.assertLess(
            router.index("A classified non-merge outcome"),
            router.index("Clear execution truth"),
        )
        self.assertIn("takes priority over", router)
        self.assertIn("not_planned", finishing)
        self.assertIn("without implementation\nverification", finishing)

    def test_bounded_slices_default_to_head_bound_task_packets(self) -> None:
        dispatch = self.section("5.2 TPM planning and subagent dispatch")
        normalized = re.sub(r"\s+", " ", dispatch.lower())
        for term in (
            "minimal, head-bound task packet",
            "use no inherited history or the smallest recent-turn window",
            "do not copy the full parent conversation by default",
            "full-thread/full-history delivery is an explicit escalation",
            "task uid",
            "frozen/current head",
            "packet producer/time",
            "regenerate it rather than append an unbounded transcript",
        ):
            with self.subTest(term=term):
                self.assertIn(term, normalized)

        agents = AGENTS.read_text(encoding="utf-8")
        self.assertIn("最小 task packet", agents)
        self.assertIn("full-history fork 仅用于已记录具体原因的升级", agents)

    def test_mandatory_slice_context_checklist_is_complete(self) -> None:
        """The dispatch contract must preserve every authority/context boundary."""
        dispatch = self.section("5.2 TPM planning and subagent dispatch")
        match = re.search(
            r"(?ms)^- The mandatory context checklist must include:\n((?:^  - .*\n)+)",
            dispatch,
        )
        self.assertIsNotNone(match, "5.2 must publish the mandatory context checklist")
        checklist = re.sub(r"\s+", " ", match.group(1).lower())
        required_items = (
            "identity and authority: assigned role, role card path, owner role, and tpm integration owner",
            "workflow governance: `agents.md`, `doc/engineering/workflow/source-of-truth.md`, and the selected workflow skills",
            "task truth: current github issue, github project item/status, `.pm/github-project-sync/tasks.json` mapping record, canonical worktree, branch, base ref, and pr link/status when present",
            "user intent and acceptance target: original request summary, current work item, explicit non-goals, and done/verification expectations",
            "scoped repo context: relevant `prd.md`, design, handoff, changed paths, current diff or evidence summary, and known constraints such as `third_party` read-only boundaries",
            "collaboration boundary: sibling slices, write-scope conflicts, integration order, allowed commands, return contract, and formal sink",
        )
        for item in required_items:
            with self.subTest(item=item):
                self.assertIn(item, checklist)

    def test_efficiency_helpers_have_bounded_enforcement_contracts(self) -> None:
        friction = self.section("1.2.1 Friction Controls After Task Truth")
        dispatch = self.section("5.2 TPM planning and subagent dispatch")
        budget = self.section("GitHub query budget and terminal defaults")
        self.assertIn("immutable machine-readable snapshot", friction)
        self.assertIn("never a second mutable task store", friction)
        self.assertIn("one expected role/slice batch", dispatch)
        self.assertIn("does not claim to control Codex host concurrency", dispatch)
        self.assertIn("bounded-command-output.py", budget)
        self.assertIn("does not claim control over Codex host logs", budget)

    def test_plan_gap_evidence_has_a_structured_fail_closed_step_contract(self) -> None:
        """Execution cannot start from a prose-only or partially mapped plan."""
        dispatch = self.section("5.2 TPM planning and subagent dispatch")
        normalized = re.sub(r"\s+", " ", dispatch.lower())
        self.assertIn("plan-gap evidence", normalized)
        self.assertRegex(
            normalized,
            r"each (?:ordered )?(?:execution )?step.{0,220}must (?:record|include).{0,120}all",
        )

        required_fields = (
            "step_id",
            "acceptance_refs",
            "dependencies",
            "verification_command",
            "verification_evidence",
            "write_scope",
            "out_of_scope",
            "required_role_slices",
        )
        for field in required_fields:
            with self.subTest(field=field):
                self.assertIn(f"`{field}`", dispatch)

        self.assertRegex(
            normalized,
            r"missing.{0,180}(?:field|evidence|mapping).{0,180}fail(?:s|ure)?[- ]closed",
        )
        self.assertRegex(
            normalized,
            r"(?:before|prior to).{0,100}(?:editing|execution).{0,160}plan-gap evidence",
        )

    def test_execution_skill_enforces_the_same_plan_gap_schema(self) -> None:
        """The operational entrypoint must consume, not weaken, the canonical schema."""
        dispatch = self.section("5.2 TPM planning and subagent dispatch")
        skill = EXECUTING_PROJECT_TASKS.read_text(encoding="utf-8")
        skill_normalized = re.sub(r"\s+", " ", skill.lower())
        self.assertIn("plan-gap evidence", skill_normalized)
        self.assertIn("github-backed task truth", skill_normalized)
        for field in (
            "step_id",
            "acceptance_refs",
            "dependencies",
            "verification_command",
            "verification_evidence",
            "write_scope",
            "out_of_scope",
            "required_role_slices",
        ):
            with self.subTest(field=field):
                self.assertIn(f"`{field}`", dispatch)
                self.assertIn(f"`{field}`", skill)
        self.assertRegex(
            skill_normalized,
            r"missing.{0,180}(?:field|evidence|mapping).{0,180}fail(?:s|ure)?[- ]closed",
        )
        self.assertRegex(
            skill_normalized,
            r"(?:before|prior to).{0,100}(?:editing|execution).{0,160}plan-gap evidence",
        )

    def test_terminal_helpers_are_current_while_supervisor_automation_is_blocked(self) -> None:
        table = self.section("Capability status")
        self.assertRegex(table, r"(?im)^\|[^\n]*(?:main[- ]sync|safe[- ]cleanup)[^\n]*\|\s*implemented\s*\|")
        self.assertRegex(table, r"(?im)^\|[^\n]*production supervisor[^\n]*\|\s*blocked\s*\|")

    def test_one_and_only_one_current_continuation_owner(self) -> None:
        ownership = self.section("Lifecycle ownership")
        normalized = re.sub(r"\s+", " ", ownership.lower())
        self.assertEqual(1, normalized.count("continuation owner"))
        self.assertRegex(normalized, r"tpm.{0,120}continuation owner")
        self.assertRegex(
            ownership,
            r"(?is)target production supervisor.*runtime executor.*not.*accountability owner",
        )

    def test_terminal_order_is_merge_receipt_done_sync_cleanup(self) -> None:
        machine = self.section("Canonical state machine")
        normalized = re.sub(r"\s+", " ", machine.lower())
        expected = "merge receipt -> task done -> main sync -> safe cleanup"
        self.assertIn(expected, normalized)
        positions = [normalized.index(term) for term in expected.split(" -> ")]
        self.assertEqual(sorted(positions), positions)

    def test_workflow_behavior_eval_covers_terminal_transition_order_contract(self) -> None:
        """The default behavior eval must run the dedicated terminal-order regression."""
        evaluation = WORKFLOW_EVAL.read_text(encoding="utf-8")
        terminal_contract = (
            'python3 "$ROOT_DIR/scripts/pm/terminal-transition-order.test.py" >/dev/null'
        )
        supervisor_contract = (
            'python3 "$ROOT_DIR/scripts/pm/tpm-production-supervisor.test.py" >/dev/null'
        )
        self.assertIn(terminal_contract, evaluation)
        self.assertEqual(1, evaluation.count(supervisor_contract))

    def test_state_enum_is_canonical_and_closed(self) -> None:
        states = self.section("Workflow states")
        declared = set(re.findall(r"`([a-z][a-z0-9_]*)`", states))
        expected = {
            "running", "action_required", "external_wait", "capability_blocked",
            "completed", "failed",
        }
        self.assertEqual(expected, declared)

    def test_ready_and_done_are_distinct_gates(self) -> None:
        gates = self.section("Ready and Done")
        self.assertRegex(gates, r"(?is)Ready.*pre-PR.*not.*Done")
        self.assertRegex(gates, r"(?is)Done.*merge receipt.*task done")

    def test_unimplemented_automation_is_not_claimed_in_present_tense(self) -> None:
        forbidden = (
            "TPM automatically runs to completion",
            "supervisor automatically consumes every action",
            "cross-turn wake owner is installed",
            "uninterrupted production workflow is implemented",
        )
        for claim in forbidden:
            with self.subTest(claim=claim):
                self.assertNotIn(claim.lower(), self.text.lower())

    def test_critical_rules_have_one_canonical_definition(self) -> None:
        for heading in (
            "Lifecycle ownership", "Canonical state machine", "Workflow states", "Ready and Done",
        ):
            with self.subTest(heading=heading):
                self.assertEqual(1, len(re.findall(rf"(?m)^## {re.escape(heading)}$", self.text)))

    def test_missing_attestation_has_one_capability_blocked_term(self) -> None:
        states = self.section("Workflow states")
        self.assertRegex(
            states,
            r"(?is)`capability_blocked`.*missing.*runtime.*attestation",
        )
        conflicting_lines = [
            line for line in self.text.splitlines()
            if "attestation" in line.lower()
            and ("missing" in line.lower() or "unavailable" in line.lower())
            and "external_wait" in line
        ]
        self.assertEqual([], conflicting_lines)

    def test_missing_attestation_is_never_classified_as_external_wait_anywhere(self) -> None:
        for path in (SOURCE, AGENTS, FINISHING):
            text = path.read_text(encoding="utf-8")
            conflicting = [
                line for line in text.splitlines()
                if "attestation" in line.lower()
                and ("missing" in line.lower() or "unavailable" in line.lower())
                and "external_wait" in line
            ]
            self.assertEqual([], conflicting, f"{path}: {conflicting}")

    def test_agents_admin_merge_is_link_only(self) -> None:
        agents = re.sub(r"\s+", " ", AGENTS.read_text(encoding="utf-8").lower())
        self.assertIn("source-of-truth.md#ready-and-done", agents)
        self.assertNotRegex(agents, r"admin merge.{0,700}complete-ruleset")

    def test_admin_merge_defaults_for_approval_only_and_requires_live_recheck(self) -> None:
        normalized = re.sub(r"\s+", " ", SOURCE.read_text(encoding="utf-8").lower())
        self.assertRegex(normalized, r"review_required.{0,240}admin merge path by default")
        self.assertIn("does not require a per-task authority comment", normalized)
        for marker in ("required checks", "mergeability", "requested changes", "comments", "review threads"):
            self.assertIn(marker, normalized)
        self.assertNotIn("complete-ruleset runtime receipt", normalized)

    def test_receiving_review_skill_matches_approval_and_thread_disposition_policy(self) -> None:
        """Review handling must not add authorization or push requirements absent from the gate."""
        skill = re.sub(
            r"\s+", " ", RECEIVING_CODE_REVIEW_SKILL.read_text(encoding="utf-8").lower()
        )
        self.assertIn("source-of-truth.md#ready-and-done", skill)
        self.assertIn("approval-only admin path needs no additional authorization", skill)
        self.assertRegex(skill, r"push only when.{0,100}code change.{0,140}resolve")
        self.assertRegex(
            skill,
            r"stale or incorrect.{0,160}no code change.{0,160}evidence-backed disposition",
        )
        self.assertNotIn("explicitly authorizes skipping", skill)

    def test_prepare_task_pr_help_matches_approval_only_admin_merge_policy(self) -> None:
        """The operator-facing helper must link to, not add, approval-only authority."""
        help_text = subprocess.run(
            [str(PREPARE_TASK_PR), "--help"],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=True,
        ).stdout.lower()
        normalized = re.sub(r"\s+", " ", help_text)
        self.assertIn("source-of-truth.md#ready-and-done", normalized)
        self.assertRegex(normalized, r"fresh live gate.{0,120}use_admin_merge: true")
        self.assertIn("no additional authorization", normalized)
        self.assertNotIn("user/task policy explicitly allows", normalized)

    def test_finishing_pre_pr_transition_is_ready_not_task_closeout(self) -> None:
        finishing = FINISHING.read_text(encoding="utf-8")
        pre_pr = finishing[: finishing.lower().find("## post-pr / pre-merge gates")]
        self.assertRegex(pre_pr, r"(?is)pre-PR.*\bReady\b")
        self.assertNotRegex(pre_pr, r"(?is)(close|complete) the task.*(?:before|pre-PR|PR creation)")

    def test_state_gate_and_pm_mapping_is_explicit_and_total(self) -> None:
        mapping = self.section("State, gate, and PM mapping")
        for state in (
            "running", "action_required", "external_wait", "capability_blocked",
            "completed", "failed",
        ):
            self.assertRegex(mapping, rf"(?m)^\|[^\n]*`{state}`[^\n]*\|$")
        for column in ("Workflow state", "Gate meaning", "GitHub Project status", "Resume authority"):
            self.assertIn(column, mapping)

    def test_historical_changelog_is_not_in_normative_spec(self) -> None:
        self.assertNotRegex(self.text, r"(?im)^##+\s+.*change log")
        self.assertRegex(self.text, r"(?m)^Version:\s+\*\*v[0-9.]+\*\*$")

    def test_secondary_workflow_docs_are_thin_operational_entrypoints(self) -> None:
        policy = self.section("Documentation policy")
        for surface in ("AGENTS.md", "finishing-a-development-branch", "tpm.md", ".pm/README.md"):
            self.assertIn(surface, policy)
        self.assertRegex(policy, r"(?is)thin operational entrypoints?.*must not.*restate|must not.*restate.*thin operational entrypoints?")
        self.assertNotIn("link-only", policy.lower())

    def test_tpm_is_consistently_coordinator_integrator_not_orchestrator_or_executor(self) -> None:
        for path in (SOURCE, AGENTS, FINISHING, ROOT / ".agents/roles/tpm.md"):
            text = path.read_text(encoding="utf-8").lower()
            if path == SOURCE:
                text = text.split("## 7. change log", 1)[0]
            self.assertIn("coordinator", text, path)
            self.assertIn("integrator", text, path)
            self.assertNotRegex(text, r"\btpm\b[^\n]{0,80}\b(orchestrator|executor)\b")

    def test_agents_and_finishing_do_not_duplicate_long_canonical_merge_rules(self) -> None:
        agents = AGENTS.read_text(encoding="utf-8")
        finishing = FINISHING.read_text(encoding="utf-8")
        # Secondary surfaces may carry one short eval marker/link, not multiple
        # independently maintained paragraphs defining the same merge behavior.
        for marker in ("mergeStateStatus=BEHIND", "REVIEW_REQUIRED", "complete-ruleset"):
            self.assertLessEqual(agents.count(marker), 1, ("AGENTS.md", marker))
            self.assertLessEqual(finishing.count(marker), 1, ("finishing", marker))
        self.assertLessEqual(
            max((len(line) for line in agents.splitlines() if "admin merge" in line.lower()), default=0),
            420,
            "AGENTS admin rule should link to the canonical definition instead of restating it",
        )
        self.assertLessEqual(
            max((len(line) for line in finishing.splitlines() if "admin merge" in line.lower()), default=0),
            420,
            "finishing guidance should link to the canonical definition instead of restating it",
        )

    def test_secondary_surfaces_link_to_canonical_workflow_sections(self) -> None:
        paths = (AGENTS, FINISHING, TPM_ROLE, PM_README)
        anchors = (
            "source-of-truth.md#lifecycle-ownership",
            "source-of-truth.md#canonical-state-machine",
            "source-of-truth.md#workflow-states",
            "source-of-truth.md#ready-and-done",
        )
        for path in paths:
            text = path.read_text(encoding="utf-8")
            with self.subTest(path=path):
                for anchor in anchors:
                    self.assertIn(anchor, text, f"{path} missing canonical anchor {anchor}")

    def test_every_secondary_canonical_anchor_resolves_in_source(self) -> None:
        """A copied anchor string is not useful unless GitHub can resolve it."""
        explicit = set(re.findall(r'<a\s+id="([^"]+)"', self.text))
        generated = set()
        for heading in re.findall(r"(?m)^#{1,6}\s+(.+?)\s*$", self.text):
            slug = heading.strip().lower()
            slug = re.sub(r"[^\w\- ]", "", slug, flags=re.UNICODE)
            slug = re.sub(r"\s+", "-", slug)
            generated.add(slug)
        available = explicit | generated
        for path in (AGENTS, FINISHING, TPM_ROLE, PM_README):
            links = re.findall(r"source-of-truth\.md#([a-z0-9_.-]+)", path.read_text(encoding="utf-8"))
            self.assertTrue(links, f"{path} has no canonical workflow anchors")
            for anchor in links:
                with self.subTest(path=path, anchor=anchor):
                    self.assertIn(anchor, available, f"{path} has broken canonical anchor #{anchor}")

    def test_secondary_surfaces_do_not_redeclare_canonical_ownership_or_terminal_rules(self) -> None:
        forbidden = (
            "canonical workflow owner",
            "sole lifecycle loop owner",
            "Canonical definition: lifecycle owner",
            "Canonical definition: terminal order",
            "Canonical definition: workflow states",
            "Canonical definition: ready versus done",
        )
        for path in (AGENTS, FINISHING, TPM_ROLE, PM_README):
            text = path.read_text(encoding="utf-8")
            for phrase in forbidden:
                with self.subTest(path=path, phrase=phrase):
                    self.assertNotIn(phrase.lower(), text.lower())

    def test_ready_wording_is_pre_pr_gate_not_task_closeout(self) -> None:
        gates = self.section("Ready and Done")
        self.assertRegex(gates, r"(?is)Ready.*pre-PR gate")
        self.assertNotRegex(gates, r"(?is)(close|complete).*task.*Ready")

    def test_agents_links_instead_of_repeating_terminal_sequence(self) -> None:
        agents = AGENTS.read_text(encoding="utf-8")
        normalized = re.sub(r"\s+", " ", agents.lower())
        self.assertIn("source-of-truth.md#canonical-state-machine", normalized)
        self.assertNotIn("merge receipt -> task done -> main sync -> safe cleanup", normalized)

    def test_diagram_marks_supervisor_automation_as_target_blocked(self) -> None:
        machine = self.section("Canonical state machine")
        self.assertRegex(
            machine,
            r"(?is)production supervisor.*target.*blocked|blocked.*production supervisor.*target",
        )

    def test_mermaid_names_only_the_target_blocked_runtime(self) -> None:
        diagram = self.section("1. Phase Diagram").split("```", 2)[1]
        self.assertNotIn("TPM lifecycle controller", diagram)
        self.assertRegex(
            diagram,
            r"(?is)(target[^\n]*production supervisor|production supervisor[^\n]*target)[^\n]*blocked",
        )

    def test_capability_status_is_the_single_current_target_definition(self) -> None:
        self.assertEqual(1, len(re.findall(r"(?m)^\*\*Current:\*\*", self.text)))
        self.assertEqual(1, len(re.findall(r"(?m)^\*\*Target:\*\*", self.text)))
        self.assertNotRegex(self.text, r"(?m)^\*\*(Current behavior|Target contract):\*\*")

    def test_gate_and_review_packet_schemas_have_one_canonical_definition(self) -> None:
        self.assertEqual(1, len(re.findall(r"(?m)^## Ready and Done$", self.text)))
        self.assertEqual(1, len(re.findall(r"(?m)^#### Pre-PR review packet$", self.text)))

    def test_header_has_current_version_and_update_date(self) -> None:
        header = re.search(r"(?m)^Version:\s+\*\*(v[0-9.]+)\*\*$", self.text)
        self.assertIsNotNone(header)
        self.assertRegex(self.text, r"(?m)^Last Updated:\s+\*\*20[0-9]{2}-[0-9]{2}-[0-9]{2}\*\*$")

    def test_secondary_surfaces_have_thin_entrypoint_size_budgets(self) -> None:
        budgets = {AGENTS: 100, FINISHING: 130, TPM_ROLE: 80, PM_README: 120}
        for path, maximum in budgets.items():
            lines = path.read_text(encoding="utf-8").splitlines()
            self.assertLessEqual(len(lines), maximum, f"{path} exceeds thin-entrypoint budget")

    def test_source_omits_changelog_and_static_completed_checklist(self) -> None:
        self.assertNotRegex(self.text, r"(?im)^##+\s+.*change log")
        self.assertNotRegex(self.text, r"(?m)^- \[[xX]\]")

    def test_recorded_action_authority_replaces_undefined_loop_owner(self) -> None:
        normative = self.text.split("## 7. Change Log", 1)[0]
        self.assertNotRegex(normative, r"(?i)\bloop owner\b")
        states = self.section("State, gate, and PM mapping")
        self.assertIn("recorded action authority", states.lower())

    def test_evidence_only_commit_head_identity_and_rereview_are_explicit(self) -> None:
        gates = self.section("Ready and Done")
        packet = self.section("Pre-PR review packet")
        finishing = FINISHING.read_text(encoding="utf-8")
        combined = "\n".join((gates, packet, finishing))
        for term in ("final implementation head", "reviewed pr head", "evidence-only commit"):
            self.assertIn(term, combined.lower())
        self.assertRegex(
            combined,
            r"(?is)(HEAD changes?|changes? HEAD).{0,300}(new|renew|repeat|re-run|reissue).{0,200}(packet|verify|verification).{0,200}review",
        )

    def test_finishing_requires_final_head_rereview_after_any_evidence_commit(self) -> None:
        finishing = FINISHING.read_text(encoding="utf-8")
        self.assertRegex(
            finishing,
            r"(?is)evidence-only.{0,240}(changes? HEAD|HEAD change).{0,240}"
            r"(re-run|repeat|new).{0,160}(verification|verify).{0,160}review.{0,160}"
            r"(new|reissue|renew).{0,100}packet",
        )
        self.assertRegex(finishing, r"(?is)(final|reviewed) PR head")

    def test_human_pre_pr_is_not_blocked_by_unattended_attestation(self) -> None:
        status = self.section("Capability status")
        self.assertRegex(
            status,
            r"(?is)human-operated[^\n]*(pre-PR|review)[^\n]*implemented",
        )
        self.assertRegex(status, r"(?is)unattended[^\n]*attestation[^\n]*blocked")

    def test_human_review_entrypoints_use_review_evidence_vocabulary(self) -> None:
        for path in HUMAN_REVIEW_ENTRYPOINTS:
            text = path.read_text(encoding="utf-8")
            text = text.replace("validate-review-provenance.py", "")
            self.assertNotRegex(text, r"(?i)provenance|attest|trusted dispatch", str(path))

    def test_bare_closeout_is_not_a_gate_or_phase_name(self) -> None:
        self.assertNotRegex(
            self.text,
            r"(?im)^#{2,6}\s+(?:claim\s*/\s*)?(?:task\s+|workflow\s+)?closeout(?:\s+chain)?\s*$",
        )
        self.assertNotRegex(self.text, r"(?im)^\|[^\n]*`closeout`[^\n]*\|$")
        self.assertNotRegex(self.text, r"(?im)^- \*\*closeout:\*\*")

    def test_owner_terms_are_not_ambiguous(self) -> None:
        ownership = self.section("Lifecycle ownership")
        for term in ("continuation owner", "phase owner", "task owner role"):
            self.assertIn(term, ownership.lower())
        self.assertRegex(
            ownership,
            r"(?is)phase owner.*owns only its bounded slice",
        )
        self.assertEqual(1, ownership.lower().count("continuation owner"))
        self.assertRegex(
            ownership,
            r"(?is)production supervisor.*runtime executor.*not.*accountability owner",
        )

    def test_ownership_mapping_has_one_canonical_definition(self) -> None:
        self.assertNotIn("Canonical definition:", self.text)
        ownership = self.section("Lifecycle ownership")
        self.assertRegex(ownership, r"(?is)TPM.*coordinator.*continuation owner")

    def test_repeated_rules_link_to_canonical_markers(self) -> None:
        for marker, anchor in (
            ("lifecycle owner", "#lifecycle-ownership"),
            ("terminal order", "#canonical-state-machine"),
            ("workflow states", "#workflow-states"),
            ("ready versus done", "#ready-and-done"),
        ):
            with self.subTest(marker=marker):
                occurrences = [
                    line for line in self.text.splitlines()
                    if marker in line.lower()
                    and "Canonical definition:" not in line
                    and not line.lstrip().startswith("#")
                ]
                for line in occurrences:
                    self.assertIn(anchor, line, line)

    def test_current_and_target_capabilities_are_visibly_separated(self) -> None:
        status = self.section("Capability status")
        self.assertRegex(status, r"(?is)Current.*production supervisor.*blocked")
        self.assertRegex(status, r"(?is)Target.*intake.*merge.*cleanup")
        self.assertNotRegex(status, r"(?im)^\|[^\n]*production supervisor[^\n]*implemented[^\n]*\|$")

    def test_reflection_signal_marker_is_unique_and_not_a_second_policy(self) -> None:
        self.assertEqual(1, self.text.count("Reflection signal"))
        line = next(line for line in self.text.splitlines() if "Reflection signal" in line)
        self.assertIn("capture-todo.sh", line)
        self.assertNotIn("Canonical definition:", line)

    def test_current_target_and_secondary_policy_have_no_residual_restatement(self) -> None:
        status = self.section("Capability status")
        without_status = self.text.replace(status, "")
        self.assertNotRegex(without_status, r"(?m)^\*\*(?:Current|Target):\*\*")
        policy = self.section("Documentation policy")
        self.assertLessEqual(len(policy.splitlines()), 8)
        self.assertNotIn("all five steps", self.text.lower())

    def test_gate_definitions_separate_pre_pr_post_pr_and_terminal(self) -> None:
        gates = self.section("Ready and Done")
        for label in ("Pre-PR Ready", "Post-PR merge-ready", "Terminal Done"):
            self.assertEqual(1, gates.count(label), label)

    def test_secondary_docs_are_short_link_surfaces(self) -> None:
        forbidden_definitions = (
            "merge receipt -> task done -> main sync -> safe cleanup",
            "sole lifecycle loop owner",
            "Post-PR merge-ready",
        )
        for path in (AGENTS, FINISHING, TPM_ROLE, PM_README):
            text = path.read_text(encoding="utf-8")
            for phrase in forbidden_definitions:
                self.assertNotIn(phrase.lower(), text.lower(), f"{path} restates {phrase}")

    def test_workflow_docs_use_profiles_not_caller_commands_for_task_closeout(self) -> None:
        for path in (SOURCE, AGENTS, FINISHING, TPM_ROLE, PM_README, VERIFICATION_SKILL):
            text = path.read_text(encoding="utf-8")
            normalized = re.sub(r"\\\s*\n\s*", " ", text)
            with self.subTest(path=path):
                self.assertNotRegex(
                    normalized,
                    r"task-closeout\.sh[^\n`]{0,300}--verify-command",
                    "task-closeout must not execute a caller-authored verification command",
                )
                invocations = re.findall(r"task-closeout\.sh[^\n`]{0,300}--role[^\n`]*", normalized)
                if invocations:
                    self.assertTrue(
                        any("--verification-profile" in command for command in invocations),
                        f"{path} must show task-closeout with a repository-owned verification profile",
                    )

    def test_normative_tail_links_instead_of_repeating_gate_terminal_and_role_matrices(self) -> None:
        claim = self.section("5.4 Claim and lifecycle transitions")
        pr_chain = self.section("5.5 PR and review chain").split("#### Pre-PR review packet", 1)[0]

        self.assertLessEqual(len(claim.split()), 250, "5.4 must be a concise operational delta")
        self.assertIn("#ready-and-done", claim)
        self.assertIn("#canonical-state-machine", claim)
        self.assertNotIn("finish implementation ->", claim.lower())

        self.assertLessEqual(len(pr_chain.split()), 220, "5.5 must link to the role matrix")
        self.assertIn("#specialist-review-role-selection", pr_chain)
        roles = (
            "producer_system_designer", "gameplay_designer", "game_visual_interaction_designer",
            "runtime_engineer", "blockchain_ops_engineer", "wasm_platform_engineer",
            "agent_engineer", "viewer_engineer", "qa_engineer",
            "repository_health_engineer", "liveops_community",
        )
        self.assertLessEqual(sum(pr_chain.count(role) for role in roles), 3)

    def test_source_has_conciseness_and_canonical_marker_budgets(self) -> None:
        self.assertLessEqual(len(self.text.splitlines()), self.canonical_source_line_budget())
        self.assertLessEqual(self.text.count("Canonical definition:"), 6)
        self.assertLessEqual(len(re.findall(r"(?m)^## ", self.text)), 16)
        self.assertLessEqual(len(re.findall(r"(?m)^### ", self.text)), 22)
        markers = re.findall(r"(?m)^Canonical definition: ([^.]+)\.$", self.text)
        self.assertEqual(len(markers), len(set(markers)), "canonical markers must be unique")
        self.assertLessEqual(len(markers), 6)

    def test_claim_ready_examples_use_the_real_claim_type_option(self) -> None:
        """Operator-facing commands must be directly copyable."""
        for path in (SOURCE, AGENTS, FINISHING, TPM_ROLE, PM_README, VERIFICATION_SKILL):
            text = path.read_text(encoding="utf-8")
            with self.subTest(path=path):
                self.assertNotRegex(text, r"claim-ready\.sh[^\n`]*\s--claim(?:\s|=)")
        help_text = subprocess.run(
            [str(ROOT / "scripts/pm/claim-ready.sh"), "--help"],
            cwd=ROOT, text=True, capture_output=True, check=True,
        ).stdout
        self.assertIn("--claim-type", help_text)
        self.assertNotRegex(help_text, r"\s--claim(?:\s|=)")

    def test_workflow_phase_enum_and_writers_use_one_canonical_vocabulary(self) -> None:
        contract = self.section("1.2.3 GitHub Project-Backed PM Contract")
        enum_match = re.search(
            r"(?im)^\|[ \t]*`Workflow Phase`[ \t]*\|[^\n]*\|[^\n]*\|[ \t]*([^\n]+)\|$",
            contract,
        )
        self.assertIsNotNone(enum_match, "Workflow Phase must publish one closed enum")
        canonical = set(re.findall(r"[a-z][a-z0-9_]*", enum_match.group(1)))
        self.assertIn("pre_pr_ready", canonical)
        self.assertIn("done", canonical)
        self.assertNotIn("post_merge_done", canonical)
        self.assertFalse({"close", "closeout"} & canonical)
        illegal_persisted_phase = re.compile(
            r'''(?x)
            (?:
                record\[\s*["']workflow_phase["']\s*\]\s*=
                |
                ["']Workflow\ Phase["']\s*:
            )
            \s*["'](?:close|closeout)["']
            '''
        )
        for path in (PROJECT_TASK, PROJECT_SYNC, PROJECT_WORKFLOW):
            text = path.read_text(encoding="utf-8")
            with self.subTest(path=path):
                self.assertNotRegex(text, illegal_persisted_phase)
        for illegal_assignment in (
            'record["workflow_phase"] = "close"',
            'record["workflow_phase"] = "closeout"',
            '"Workflow Phase": "close"',
            '"Workflow Phase": "closeout"',
        ):
            with self.subTest(illegal_assignment=illegal_assignment):
                self.assertRegex(illegal_assignment, illegal_persisted_phase)
        task_text = PROJECT_TASK.read_text(encoding="utf-8")
        self.assertRegex(
            task_text,
            r"[\"']Workflow Phase[\"']\s*:\s*[\"']task_done[\"']\s+if\s+[^\n]+\s+else\s+[\"']pre_pr_ready[\"']",
        )
        self.assertNotIn("post_merge_done", function_source := re.search(
            r"(?ms)^def command_closeout_task\b.*?(?=^def |\Z)", task_text
        ).group(0))
        self.assertIn("post_merge_done", FINALIZER.read_text(encoding="utf-8"))

    def test_dispatch_producer_blockers_have_unambiguous_classification(self) -> None:
        skill = SUPERVISOR_SKILL.read_text(encoding="utf-8")
        missing_windows = [
            skill[max(0, match.start() - 180):match.end() + 260]
            for match in re.finditer(r"(?i)(?:missing|unavailable) dispatch (?:producer|attestation)", skill)
        ]
        self.assertTrue(missing_windows, "skill must name the missing dispatch producer case")
        for window in missing_windows:
            self.assertIn("capability_blocked", window)
            self.assertNotIn("external_wait", window)
        self.assertRegex(
            skill,
            r"(?is)trusted dispatch producer.{0,220}temporary.{0,220}(readback|delivery).{0,220}`external_wait`",
        )

    def test_terminal_runbook_is_unique_copyable_and_complete(self) -> None:
        matches = list(re.finditer(r"(?m)^###?\s+Terminal runbook\s*$", self.text))
        self.assertEqual(1, len(matches), "there must be exactly one terminal runbook")
        start = matches[0].end()
        next_heading = re.search(r"(?m)^#{2,3}\s+", self.text[start:])
        runbook = self.text[start:start + next_heading.start()] if next_heading else self.text[start:]
        normalized = re.sub(r"\\\s*\n\s*", " ", runbook)
        self.assertRegex(runbook, r'(?m)^RECEIPT_ROOT="\$\(python3 scripts/pm/canonical-receipt-root\.py \\')
        self.assertRegex(runbook, r'--default-worktree <canonical-default-worktree>.*\n\s*--task-uid <TASK-UID> --create\)"')
        receipt = re.search(r"pr-merge-receipt\.py[^\n]+>\s*[^\s`]+\.json", normalized)
        done = re.search(r"task-closeout\.sh[^\n]+--task-uid[^\n]+--pr-receipt\s+[^\s`]+\.json", normalized)
        sync = re.search(r"post-merge-main-sync\.sh[^\n]+--repo-root[^\n]+--main-ref", normalized)
        cleanup = re.search(
            r"post-merge-cleanup\.sh[^\n]+--repo-root[^\n]+--worktree[^\n]+--branch"
            r"[^\n]+--main-ref[^\n]+--task-uid[^\n]+--pr-receipt\s+[^\s`]+\.json",
            normalized,
        )
        for label, command in (("receipt", receipt), ("task done", done), ("main sync", sync), ("cleanup", cleanup)):
            self.assertIsNotNone(command, f"terminal runbook lacks copyable {label} command")
        positions = [command.start() for command in (receipt, done, sync, cleanup)]
        self.assertEqual(sorted(positions), positions)
        for helper in ("pr-merge-receipt.py", "task-closeout.sh", "post-merge-main-sync.sh", "post-merge-cleanup.sh"):
            self.assertTrue((ROOT / "scripts/pm" / helper).is_file(), f"missing runbook helper {helper}")
        invocations = re.findall(
            r"(?:python3\s+)?(?:\./)?scripts/pm/(?:pr-merge-receipt\.py|task-closeout\.sh|"
            r"refresh-task-cache\.sh|post-merge-main-sync\.sh|post-merge-cleanup\.sh|"
            r"post-merge-finalize\.py)", runbook)
        self.assertEqual(6, len(invocations), invocations)
        self.assertRegex(runbook, r"(?i)all six (?:commands|transitions|steps)")

    def test_non_merge_terminal_route_is_copyable_and_synchronized(self) -> None:
        route = re.compile(
            r"(?:\./)?scripts/pm/non-merge-finalize\.py\s+"
            r"--repo-root\s+<canonical-default-worktree>\s+"
            r"--task-uid\s+<TASK-UID>\s+"
            r"--reason\s+<reason>\s+"
            r"--evidence-file\s+<path>\s+--json"
        )

        def normalized(text: str) -> str:
            return re.sub(r"\\\s*\n\s*", " ", text)

        surfaces = {
            "canonical source": self.text,
            "finishing skill": FINISHING.read_text(encoding="utf-8"),
        }
        matches = {}
        for name, text in surfaces.items():
            match = route.search(normalized(text))
            self.assertIsNotNone(
                match,
                f"{name} must publish a complete copyable non-merge-finalize route",
            )
            matches[name] = re.sub(r"\s+", " ", match.group(0)).strip()
        self.assertEqual(
            matches["canonical source"],
            matches["finishing skill"],
            "non-merge terminal route drifted between canonical source and finishing skill",
        )

    def test_terminal_runbook_enters_default_worktree_before_any_helper(self) -> None:
        match = re.search(r"(?ms)^###?\s+Terminal runbook\s*$\n(.*?)(?=^#{2,3}\s+|\Z)", self.text)
        self.assertIsNotNone(match)
        runbook = match.group(1)
        cd = re.search(r"(?m)^cd <canonical-default-worktree>\s*$", runbook)
        helper = re.search(r"(?:python3\s+|\./)scripts/pm/", runbook)
        self.assertIsNotNone(cd)
        self.assertIsNotNone(helper)
        self.assertLess(cd.start(), helper.start(), "runbook must enter default worktree before its first helper")

    def test_terminal_runbook_separates_ordinary_and_squash_retry_lanes(self) -> None:
        match = re.search(r"(?ms)^###?\s+Terminal runbook\s*$\n(.*?)(?=^#{2,3}\s+|\Z)", self.text)
        self.assertIsNotNone(match)
        runbook = match.group(1)
        ordinary = re.search(r"(?ms)4\. Main sync.*?```bash\n(.*?)```", runbook)
        self.assertIsNotNone(ordinary)
        self.assertNotIn("--patch-equivalence-receipt", ordinary.group(1))
        retry = re.search(r"(?m)^Squash/rebase retry:.*$", runbook)
        self.assertIsNotNone(retry)
        self.assertIn("patch-equivalence-receipt.sh", retry.group(0))
        self.assertIn("--patch-equivalence-receipt", retry.group(0))
        self.assertIn("projected tree", retry.group(0))
        self.assertIn("integration commit remains an ancestor", retry.group(0))

    def test_failed_is_escalation_or_new_epoch_not_resume_authority(self) -> None:
        failed_windows = [
            self.text[max(0, m.start() - 160):m.end() + 240]
            for m in re.finditer(r"(?i)(?:`failed`|\|\s*failed\s*\|)", self.text)
        ]
        self.assertTrue(failed_windows)
        for window in failed_windows:
            self.assertRegex(window, r"(?i)(escalat|new epoch)")
            self.assertNotRegex(window, r"(?i)resume (?:authority|owner)")

    def test_secondary_policy_has_one_thin_entrypoint_definition_and_no_placeholders(self) -> None:
        phrase = re.findall(r"(?i)thin operational entrypoints?", self.text)
        self.assertEqual(1, len(phrase), "secondary policy must have one canonical definition")
        self.assertNotRegex(
            self.text,
            r"(?:\bTBD\b|(?<![-/])\bTODO\b(?![-/])|(?i:\bto be defined\b|<placeholder>|\bplaceholder\b))",
        )

    def test_section_three_does_not_define_a_second_gate_taxonomy_or_controller(self) -> None:
        section_three_match = re.search(
            r"(?ms)^## 3\. Lifecycle prerequisites and conditional phases\s*$\n(.*?)(?=^## 4\.|\Z)",
            self.text,
        )
        self.assertIsNotNone(section_three_match)
        section_three = section_three_match.group(1)
        self.assertRegex(section_three, r"(?m)^###\s+3\.1\s+Prerequisites\s*$")
        self.assertRegex(section_three, r"(?m)^###\s+3\.2\s+Conditional phases\s*$")
        self.assertNotRegex(section_three, r"(?m)^###\s+3\.[12]\s+(?:Required|Optional) Gates")
        self.assertNotIn("Uninterrupted lifecycle controller contract", section_three)
        self.assertEqual(
            1,
            len(re.findall(r"(?im)^###\s+Appendix A: Unattended automation invariants\s*$", self.text)),
        )

    def test_phase_diagram_freeze_is_not_commit_and_reaches_post_merge_done(self) -> None:
        diagram = self.section("1. Phase Diagram").split("```", 2)[1]
        freeze_lines = [line for line in diagram.splitlines() if "freeze" in line.lower()]
        self.assertTrue(freeze_lines)
        self.assertTrue(all("commit" not in line.lower() for line in freeze_lines))
        self.assertIn("post_merge_done", diagram)

    def test_phase_diagram_separates_ready_optional_commit_and_pr_creation(self) -> None:
        diagram = self.section("1. Phase Diagram").split("```", 2)[1]
        labels = re.findall(r"\[([^\]]+)\]", diagram)
        ready = [label for label in labels if "Pre-PR Ready" in label]
        commits = [label for label in labels if "commit" in label.lower()]
        creates = [label for label in labels if "PR creat" in label]
        self.assertEqual(1, len(ready), labels)
        self.assertEqual(1, len(commits), labels)
        self.assertIn("optional", commits[0].lower())
        self.assertEqual(1, len(creates), labels)
        self.assertRegex(diagram, r"(?is)Pre-PR Local Role Review.{0,500}human-operated evidence validated")
        self.assertRegex(diagram, r"(?is)Pre-PR Ready.{0,500}optional.{0,500}Promote draft")

    def test_supervisor_positive_catalog_reaches_terminal_outcome(self) -> None:
        positive = re.search(
            r"(?ms)^The closed positive catalog .*?(?=^Fix/reverify loops)",
            self.supervisor_design,
        )
        self.assertIsNotNone(positive, "positive supervisor catalog is required")
        catalog = positive.group(0)
        sequence = re.search(r"exact phase sequence\s+`\[(.*?)\]`", catalog, re.S)
        self.assertIsNotNone(sequence, "positive catalog must publish its exact phase sequence")
        phases = [phase.strip() for phase in sequence.group(1).split(",")]
        expected = [
            "bootstrap", "planning", "execution", "verification", "pre_pr_review",
            "pre_pr_ready", "pr_watch", "done",
        ]
        self.assertEqual(expected, phases)
        self.assertEqual("done", phases[-1])
        self.assertEqual(8, len(phases), phases)
        self.assertRegex(catalog, r"final_phase=\{kind:canonical,value:done\}")
        self.assertRegex(
            catalog,
            r'terminal_outcome=\{schema:"tpm-supervisor-terminal-outcome/v1",code:post_merge_done\}',
        )
        advances = re.search(r"phase_advances\s*=\s*(\d+)", catalog)
        self.assertIsNotNone(advances)
        self.assertEqual("7", advances.group(1))
        self.assertIn("`expected_outcome={kind:code,code:accepted}`", catalog)
        self.assertIn("`observed_outcome={kind:code,code:accepted}`", catalog)
        self.assertIn("`verdict=pass` when all typed assertions match", catalog)
        self.assertRegex(
            self.supervisor_design,
            r"exact canonical 8-phase/7-advance sequence\s+with `pre_pr_ready` before promotion and terminal outcome `post_merge_done`",
        )

    def test_supervisor_positive_phase_sequence_matches_canonical_source_enum(self) -> None:
        contract = self.section("1.2.3 GitHub Project-Backed PM Contract")
        enum_match = re.search(
            r"(?im)^\|[ \t]*`Workflow Phase`[ \t]*\|[^\n]*\|[^\n]*\|[ \t]*([^\n]+)\|$",
            contract,
        )
        self.assertIsNotNone(enum_match, "Workflow Phase must publish one closed enum")
        assert enum_match is not None
        enum_text = enum_match.group(1).split(" (", 1)[0]
        canonical = re.findall(r"`([a-z][a-z0-9_]*)`", enum_text)
        self.assertEqual(
            [
                "bootstrap", "planning", "execution", "verification", "pre_pr_review",
                "pre_pr_ready", "pr_watch", "blocked", "done",
            ],
            canonical,
        )
        positive = re.search(
            r"(?ms)^The closed positive catalog .*?(?=^Fix/reverify loops)",
            self.supervisor_design,
        )
        self.assertIsNotNone(positive, "positive supervisor catalog is required")
        assert positive is not None
        sequence = re.search(r"exact phase sequence\s+`\[(.*?)\]`", positive.group(0), re.S)
        self.assertIsNotNone(sequence, "positive catalog must publish its exact phase sequence")
        assert sequence is not None
        phases = [phase.strip() for phase in sequence.group(1).split(",")]
        self.assertEqual([phase for phase in canonical if phase != "blocked"], phases)
        self.assertNotIn("closeout", phases)
        self.assertNotRegex(self.supervisor_design, r"(?m)^\| `closeout`\b")
        projection = re.search(
            r"workflow_phase_projection=\{schema:\"tpm-supervisor-workflow-phase/v1\",sequence:\[(.*?)\]\}",
            positive.group(0),
            re.S,
        )
        self.assertIsNotNone(projection, "positive workflow phase projection is required")
        assert projection is not None
        self.assertEqual(phases, [item.strip() for item in projection.group(1).split(",")])
        operations = re.search(
            r"implementation_operation_sequence=\{schema:\"tpm-supervisor-operation-sequence/v1\",sequence:\[(.*?)\]\}",
            positive.group(0),
            re.S,
        )
        self.assertIsNotNone(operations, "positive implementation operation sequence is required")
        assert operations is not None
        self.assertEqual(
            [
                "bootstrap", "route", "dispatch", "execute", "integrate", "freeze",
                "draft_candidate", "create_pr", "record_pr", "comment", "verify",
                "review", "ready", "promote_draft", "pr_watch", "merge",
                "merge_receipt", "task_done", "main_sync", "safe_cleanup",
                "post_merge_finalize",
            ],
            [item.strip() for item in operations.group(1).split(",")],
        )
        self.assertRegex(
            positive.group(0),
            r'terminal_outcome=\{schema:"tpm-supervisor-terminal-outcome/v1",code:post_merge_done\}',
        )
        self.assertIn("`verdict=pass` when all typed assertions match", positive.group(0))
        schema = re.search(
            r"(?ms)^### 10\.5 Machine-readable case summary and human report\n(.*?)(?=^### 10\.6)",
            self.supervisor_design,
        )
        self.assertIsNotNone(schema, "machine-readable case schema is required")
        assert schema is not None
        schema_text = re.sub(r"\s+", " ", schema.group(1))
        self.assertIn("typed `terminal_outcome`", schema_text)
        self.assertRegex(
            schema_text,
            r"positive cases require typed `?workflow_phase_projection`? and `?implementation_operation_sequence`?",
        )

    def test_supervisor_soak_timeout_is_distinct_from_ordinary_case_budget(self) -> None:
        budgets = re.search(
            r"(?ms)^### 10\.3 Initial staging budgets and deterministic repetitions\n(.*?)(?=^### 10\.4)",
            self.supervisor_design,
        )
        self.assertIsNotNone(budgets, "staging budgets are required")
        assert budgets is not None
        self.assertIn("`120 s` per matrix/operational/positive case", budgets.group(1))
        self.assertIn("Cross-process soak case timeout", budgets.group(1))
        self.assertIn("At least `900 s` per soak case", budgets.group(1))
        soak = re.search(
            r"(?ms)^The closed soak catalog .*?(?=^Every catalog row)",
            self.supervisor_design,
        )
        self.assertIsNotNone(soak, "closed soak catalog is required")
        assert soak is not None
        self.assertIn("case_timeout_s >= 900", soak.group(0))
        self.assertIn("ordinary case timeout remains `120 s`", soak.group(0))
        self.assertIn("`expected_outcome={kind:code,code:accepted}`", soak.group(0))
        self.assertIn("`observed_outcome={kind:code,code:accepted}`", soak.group(0))

    def test_supervisor_a05_typed_outcomes_follow_closed_mapping(self) -> None:
        matrix = re.search(
            r"(?ms)^\| Case ID \| Stage .*?(?=^The operational catalog)",
            self.supervisor_design,
        )
        self.assertIsNotNone(matrix, "supervisor fault matrix is required")
        assert matrix is not None
        row = re.search(r"(?m)^\| `A05` .*", matrix.group(0))
        self.assertIsNotNone(row, "A05 typed receipt variants are required")
        assert row is not None
        text = row.group(0)
        for mapping in (
            "forged -> rejected(authority)",
            "malformed -> rejected(malformed)",
            "wrong identity -> rejected(identity)",
            "stale/revision/different digest -> stale_receipt",
            "fencing -> stale_fencing",
        ):
            with self.subTest(mapping=mapping):
                self.assertIn(mapping, text)
        self.assertIn("new_epoch=false", text)
        self.assertIn("reject_same_epoch", text)
        self.assertIn("one case record per seed", self.supervisor_design)
        self.assertIn("total_case_count=87", self.supervisor_design)

    def test_external_wait_metadata_is_durable_checkpoint_only(self) -> None:
        states = self.section("Workflow states")
        for phrase in (
            "only a durable production-supervisor checkpoint emission",
            "Transient human-operated helper transport/status JSON",
            "`pr-watch-loop`",
            "current collaboration probe",
            "cannot be persisted or promoted as one",
        ):
            with self.subTest(phrase=phrase):
                self.assertIn(phrase, states)
        machine = re.sub(r"\s+", " ", self.section("Canonical state machine"))
        expected = (
            "draft_candidate -> create/record/comment draft PR -> CI verify -> review -> "
            "pre_pr_ready -> promote_draft -> pr_watch"
        )
        self.assertIn(expected, machine)
        design_state = re.search(
            r"(?m)^\| `running` \| Trusted temporary external condition.*$",
            self.supervisor_design,
        )
        self.assertIsNotNone(design_state, "design external_wait transition is required")
        assert design_state is not None
        self.assertIn("durable production-supervisor checkpoint", design_state.group(0))
        self.assertIn("transient human helper status is not a transition", design_state.group(0))

    def test_supervisor_pr_identity_readbacks_are_post_action_guards(self) -> None:
        matrix = re.search(
            r"(?ms)^\| Phase family \| Required validated evidence .*?(?=^The final two rows)",
            self.supervisor_design,
        )
        self.assertIsNotNone(matrix, "supervisor phase guard matrix is required")
        assert matrix is not None
        draft = re.search(r"(?m)^\| `draft_candidate` .*", matrix.group(0))
        self.assertIsNotNone(draft, "draft_candidate guard is required")
        assert draft is not None
        self.assertIn("candidate inputs", draft.group(0))
        self.assertNotIn("draft PR identity", draft.group(0))
        for phase, evidence, next_phase in (
            ("create_pr", "Draft PR create receipt", "record_pr"),
            ("record_pr", "Recorded PR identity", "comment"),
            ("comment", "Issue-comment receipt", "verify"),
        ):
            row = re.search(rf"(?m)^\| `{phase}` .*", matrix.group(0))
            self.assertIsNotNone(row, phase)
            assert row is not None
            self.assertIn(evidence, row.group(0), phase)
            self.assertIn(f"/ `{next_phase}`", row.group(0), phase)

        trust = re.search(
            r"(?ms)^#### Phase-to-producer and validator trust matrix\n(.*?)(?=^The executor receipt)",
            self.supervisor_design,
        )
        self.assertIsNotNone(trust, "phase-to-producer trust matrix is required")
        assert trust is not None
        trust_rows = trust.group(1)
        for phase, evidence in (
            ("draft_candidate", "candidate-input readback; no PR identity yet"),
            ("create_pr", "Draft PR create receipt and independent PR identity/readback"),
            ("record_pr", "Recorded PR identity plus independent task/PR readback"),
            ("comment", "Issue-comment receipt plus independent task/issue-comment readback"),
        ):
            row = re.search(rf"(?m)^\| `{phase}` .*", trust_rows)
            self.assertIsNotNone(row, phase)
            assert row is not None
            self.assertIn(evidence, row.group(0), phase)

    def test_supervisor_promote_receipt_enters_pr_watch_once(self) -> None:
        matrix = re.search(
            r"(?ms)^\| Phase family \| Required validated evidence .*?(?=^The final two rows)",
            self.supervisor_design,
        )
        self.assertIsNotNone(matrix, "supervisor phase transition matrix is required")
        assert matrix is not None
        pre_ready = re.search(r"(?m)^\| `pre_pr_ready` .*", matrix.group(0))
        self.assertIsNotNone(pre_ready, "pre_pr_ready transition is required")
        assert pre_ready is not None
        self.assertIn("Execute(promote_draft)", pre_ready.group(0))
        promote = re.search(r"(?m)^\| `promote_draft` .*", matrix.group(0))
        self.assertIsNotNone(promote, "promote_draft receipt guard is required")
        assert promote is not None
        self.assertIn("Successful validated `Execute(promote_draft)` receipt", promote.group(0))
        self.assertIn("`Wait(pr_watch)` once", promote.group(0))
        self.assertIn("no second `promote_draft` emission", promote.group(0))
        self.assertEqual(1, promote.group(0).count("Execute(promote_draft)"))
        positive = re.search(
            r"(?ms)^The closed positive catalog .*?(?=^Fix/reverify loops)",
            self.supervisor_design,
        )
        self.assertIsNotNone(positive, "positive supervisor catalog is required")
        assert positive is not None
        operations = re.search(
            r"implementation_operation_sequence=\{schema:\"tpm-supervisor-operation-sequence/v1\",sequence:\[(.*?)\]\}",
            positive.group(0),
            re.S,
        )
        self.assertIsNotNone(operations, "positive operation sequence is required")
        assert operations is not None
        self.assertEqual(1, operations.group(1).split(",").count("promote_draft"))

    def test_draft_candidate_command_uses_canonical_flags(self) -> None:
        canonical = "./scripts/prepare-task-pr.sh --draft-candidate --create"
        legacy = "./scripts/prepare-task-pr.sh --create " + "--draft"
        help_text = subprocess.run(
            [str(PREPARE_TASK_PR), "--help"],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=True,
        ).stdout
        surfaces = {
            "source-of-truth": self.text,
            "prepare-task-pr": PREPARE_TASK_PR.read_text(encoding="utf-8"),
            "prepare-task-pr --help": help_text,
            "finishing skill": FINISHING.read_text(encoding="utf-8"),
        }
        for name, surface in surfaces.items():
            with self.subTest(surface=name):
                self.assertIn(canonical, surface)
                self.assertNotIn(legacy, surface)
        finishing = surfaces["finishing skill"]
        self.assertRegex(
            finishing,
            r"(?is)after frozen-head draft-candidate creation and trusted exact-head CI,"
            r".*before draft promotion",
        )
        legacy_review_phrase = (
            "review packet recorded after immutable verification and "
            + "before PR creation"
        )
        self.assertNotIn(legacy_review_phrase, finishing.lower())

    def test_prepare_help_keeps_optional_closeout_before_fresh_promotion(self) -> None:
        help_text = subprocess.run(
            [str(PREPARE_TASK_PR), "--help"],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=True,
        ).stdout
        standard = re.search(r"(?m)^- standard path: (.*)$", help_text)
        self.assertIsNotNone(standard, "prepare-task-pr help must publish its standard path")
        assert standard is not None
        self.assertIn(
            "./scripts/prepare-task-pr.sh --promote-draft <fresh ci_ready_receipt.json>",
            standard.group(1),
        )
        self.assertNotIn("evidence-only", standard.group(1).lower())

        optional = re.search(
            r"(?ms)^- optional evidence-only closeout: (.*?)(?=^Options:)",
            help_text,
        )
        self.assertIsNotNone(
            optional,
            "prepare-task-pr help must describe evidence-only closeout separately",
        )
        assert optional is not None
        for phrase in (
            "if it changes HEAD",
            "rerun exact-head CI and local role review",
            "regenerate the packet and ci_ready receipt",
            "promote only with that new receipt",
        ):
            with self.subTest(phrase=phrase):
                self.assertIn(phrase, optional.group(1))
        self.assertNotIn(
            "evidence-only closeout commit -> ./scripts/prepare-task-pr.sh --promote-draft",
            help_text.lower(),
        )

    def test_task_bound_legacy_create_is_rejected_before_pr_recording(self) -> None:
        script = PREPARE_TASK_PR.read_text(encoding="utf-8")
        guard = (
            'if [[ "$CREATE_PR" == "1" && "$DRAFT_CANDIDATE" != "1" '
            '&& "$LOCAL_ROLE_REVIEW_STATUS" == "passed" '
            '&& -n "$LOCAL_ROLE_REVIEW_TASK_UID" ]]; then'
        )
        self.assertIn(guard, script)
        guard_index = script.index(guard)
        create_index = script.index('if [[ "$CREATE_PR" == "1" ]]; then', guard_index)
        record_index = script.index('github-project-task.py" record-pr', create_index)
        self.assertLess(guard_index, create_index)
        self.assertLess(guard_index, record_index)
        self.assertIn("legacy task-bound", script)
        self.assertIn("--create", script)

        help_text = subprocess.run(
            [str(PREPARE_TASK_PR), "--help"],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=True,
        ).stdout
        self.assertIn("legacy task-bound `--create` is rejected", help_text)
        for path in (PM_README, SCRIPTS_PRD):
            with self.subTest(surface=path):
                text = path.read_text(encoding="utf-8")
                self.assertIn("./scripts/prepare-task-pr.sh --draft-candidate --create", text)
                self.assertIn("--promote-draft <fresh ci_ready_receipt.json>", text)

    def test_draft_candidate_binding_is_canonical_and_pre_side_effect(self) -> None:
        script = PREPARE_TASK_PR.read_text(encoding="utf-8")
        validator = script.index("validate_draft_candidate_binding()")
        call = script.index("BOUND_TASK_FIELDS=\"$(validate_draft_candidate_binding", validator)
        role_status = script.index(
            'LOCAL_ROLE_REVIEW_OUTPUT=\"$(local_role_review_status',
            call,
        )
        push = script.index('git -C \"$SOURCE_WORKTREE\" push', call)
        gh_create = script.index('CREATE_CMD=("gh" "pr" "create"', call)
        self.assertLess(call, role_status)
        self.assertLess(call, push)
        self.assertLess(call, gh_create)
        for marker in (
            "canonical_worktree",
            "task_branch",
            "checked-out worktree HEAD differs",
            "source branch ref differs",
            "comparison ref differs",
            "expected exactly one canonical_worktree/task_branch match",
        ):
            with self.subTest(marker=marker):
                self.assertIn(marker, script)
        self.assertNotIn("canonical_hits or legacy_hits", script)
        self.assertIn(
            'local role-review task UID does not match canonical task binding',
            script,
        )

    def test_task_pr_docs_order_draft_identity_review_closeout_and_promotion(self) -> None:
        readme = PM_README.read_text(encoding="utf-8")
        readme_section = re.search(
            r"(?ms)^## Pre-PR and PR\n(.*?)(?=^## |\Z)", readme
        )
        self.assertIsNotNone(readme_section, "PM README must publish the Pre-PR sequence")
        assert readme_section is not None
        readme_text = readme_section.group(1)
        readme_positions = [
            readme_text.index("./scripts/prepare-task-pr.sh --draft-candidate --create"),
            readme_text.index("after exact-head CI and local role review"),
            readme_text.index("./scripts/pm/task-closeout.sh"),
            readme_text.index("./scripts/prepare-task-pr.sh --promote-draft"),
        ]
        self.assertEqual(sorted(readme_positions), readme_positions)
        self.assertIn("after exact-head CI and local role review", readme_text)
        self.assertIn("after task closeout succeeds", readme_text)
        self.assertIn("--ci-ready-receipt <ci_ready_receipt.json>", readme_text)

        prd = SCRIPTS_PRD.read_text(encoding="utf-8")
        sc12e = re.search(r"(?m)^\s*- SC-12E:.*$", prd)
        self.assertIsNotNone(sc12e, "scripts PRD must publish SC-12E")
        assert sc12e is not None
        sc12e_text = sc12e.group(0)
        identity = "Task UID`、`Source Worktree`、`Source Branch`、`Source Head` 与 `Comparison Ref`"
        self.assertIn(identity, sc12e_text)
        self.assertIn("draft candidate 创建不得要求 `Pre-PR Local Role Review: passed`", sc12e_text)
        self.assertLess(
            sc12e_text.index("task/frozen-head identity"),
            sc12e_text.index("Pre-PR Local Role Review: passed"),
        )
        self.assertNotIn(
            "缺少匹配 `Task UID`、`Source Worktree`、`Source Branch`、`Source Head`、`Comparison Ref` 与 `Pre-PR Local Role Review: passed`",
            sc12e_text,
        )
        ac18j = re.search(r"(?m)^\s*- AC-18J:.*$", prd)
        self.assertIsNotNone(ac18j, "scripts PRD must publish AC-18J")
        assert ac18j is not None
        ac18j_text = ac18j.group(0)
        self.assertIn("task-bound legacy `--create`", ac18j_text)
        self.assertIn("`--draft-candidate --create`", ac18j_text)
        self.assertIn("只在候选创建前强制校验 source-bound Task UID / frozen-head identity", ac18j_text)
        self.assertIn("把 pre-PR local role review evidence 留到 `Pre-PR Ready` / draft promotion", ac18j_text)
        self.assertNotIn(
            "`--create` 必须在 push / `gh pr create` 前强制要求 source-bound pre-PR local role review evidence",
            ac18j_text,
        )

        flow = re.search(r"(?m)^\s*6\. Flow-SCR-006:.*$", prd)
        self.assertIsNotNone(flow, "scripts PRD must publish Flow-SCR-006")
        assert flow is not None
        flow_text = flow.group(0)
        flow_positions = [
            flow_text.index("Task UID / frozen-head identity"),
            flow_text.index("创建 frozen-head draft candidate"),
            flow_text.index("exact-head CI"),
            flow_text.index("task-closeout"),
            flow_text.index("--promote-draft"),
        ]
        self.assertEqual(sorted(flow_positions), flow_positions)

        flow_d = re.search(r"(?m)^\s*6D\. Flow-SCR-006D:.*$", prd)
        self.assertIsNotNone(flow_d, "scripts PRD must publish Flow-SCR-006D")
        assert flow_d is not None
        flow_d_text = flow_d.group(0)
        self.assertIn("不要求 role-review packet", flow_d_text)
        flow_d_positions = [
            flow_d_text.index("Task UID / frozen-head identity"),
            flow_d_text.index("创建 draft candidate"),
            flow_d_text.index("exact-head CI"),
            flow_d_text.index("task-closeout"),
            flow_d_text.index("--promote-draft"),
        ]
        self.assertEqual(sorted(flow_d_positions), flow_d_positions)

    def test_engineering_prd_active_pr_flows_use_draft_candidate_order(self) -> None:
        prd = ENGINEERING_PRD.read_text(encoding="utf-8")
        flow_labels = ("SC-24A", "Flow-ENG-001", "Flow-ENG-014")
        markers = (
            "--draft-candidate --create",
            "exact-head CI",
            "pre-PR local role subagent review",
            "task-closeout",
            "--promote-draft <fresh ci_ready_receipt.json>",
        )
        for label in flow_labels:
            line = re.search(rf"(?m)^.*{re.escape(label)}:.*$", prd)
            self.assertIsNotNone(line, f"engineering PRD must publish {label}")
            assert line is not None
            flow = line.group(0)
            with self.subTest(flow=label):
                positions = [flow.index(marker) for marker in markers]
                self.assertEqual(sorted(positions), positions)
                self.assertNotIn("-> prepare-task-pr ->", flow)
                self.assertNotIn("-> 执行 prepare-task-pr GitHub PR preflight / create", flow)

    def test_active_workflow_emitters_keep_draft_first_order(self) -> None:
        prd = ENGINEERING_PRD.read_text(encoding="utf-8")
        for label, review_marker in (
            ("SC-23", "subagent review evidence packet"),
            ("角色协作工作流", "本地相关角色 subagent review evidence packet"),
        ):
            line = re.search(rf"(?m)^.*{re.escape(label)}.*$", prd)
            self.assertIsNotNone(line, f"engineering PRD must publish {label}")
            assert line is not None
            text = line.group(0)
            with self.subTest(surface=f"engineering-prd:{label}"):
                markers = (
                    "--draft-candidate --create",
                    "exact-head CI",
                    review_marker,
                    "task-closeout",
                    "--promote-draft",
                )
                positions = [text.index(marker) for marker in markers]
                self.assertEqual(sorted(positions), positions)
                self.assertNotIn("创建 PR 前必须先完成本地", text)
        sc24 = re.search(r"(?m)^.*SC-24:.*$", prd)
        self.assertIsNotNone(sc24, "engineering PRD must publish SC-24")
        assert sc24 is not None
        self.assertIn("draft candidate 创建不得等待 role review", sc24.group(0))
        self.assertIn("exact-head CI", sc24.group(0))
        self.assertIn("task-closeout", sc24.group(0))
        self.assertNotIn("创建 PR 前必须先完成本地", sc24.group(0))

        reporting = PM_REPORTING.read_text(encoding="utf-8")
        prepare = reporting.index('"prepare-pr-review"')
        promote = reporting.index('"promote-draft"', prepare)
        checklist = reporting[prepare:]
        markers = (
            "source-bound Task UID / frozen-head identity",
            "--draft-candidate --create",
            "exact-head CI",
            "Pre-PR Local Role Review: passed",
            "task-closeout",
            "--promote-draft",
        )
        positions = [checklist.index(marker) for marker in markers]
        self.assertEqual(sorted(positions), positions)
        self.assertLess(prepare, promote)
        self.assertNotIn("创建 PR 前必须先完成本地相关角色 subagent review", checklist)

        workflow_eval = WORKFLOW_EVAL.read_text(encoding="utf-8")
        usage_start = workflow_eval.index("Run the repo-owned workflow behavior eval")
        usage_end = workflow_eval.index("Options:", usage_start)
        usage = workflow_eval[usage_start:usage_end]
        usage_positions = [
            usage.index("--draft-candidate --create"),
            usage.index("exact-head CI"),
            usage.index("role review"),
            usage.index("task-closeout"),
            usage.index("--promote-draft"),
        ]
        self.assertEqual(sorted(usage_positions), usage_positions)
        workflow_line = next(
            line for line in workflow_eval.splitlines() if '"workflow_path":' in line
        )
        workflow_positions = [
            workflow_line.index("--draft-candidate --create"),
            workflow_line.index("exact-head CI"),
            workflow_line.index("role review"),
            workflow_line.index("task-closeout"),
            workflow_line.index("--promote-draft"),
        ]
        self.assertEqual(sorted(workflow_positions), workflow_positions)
        self.assertNotIn("task-closeout -> prepare-task-pr ->", workflow_eval)
        self.assertNotIn(
            "requesting-repo-owned-review -> prepare-task-pr ->",
            workflow_eval,
        )

    def test_supervisor_a05_subcases_are_explicit_nested_schema(self) -> None:
        section = re.search(
            r"(?ms)^### 10\.5 Machine-readable case summary and human report\n(.*?)(?=^### 10\.6)",
            self.supervisor_design,
        )
        self.assertIsNotNone(section, "machine-readable case schema is required")
        assert section is not None
        text = section.group(1)
        normalized = re.sub(r"\s+", " ", text)
        self.assertIn(
            "The `subcases` array is `[]` except for composite parent IDs `B02`, `A05`, `C03`, and `W01`",
            normalized,
        )
        for field in (
            "id", "fault", "injection", "expected_outcome", "observed_outcome", "verdict", "status", "phase",
            "cardinality", "recovery", "consume_result", "readback", "digest",
        ):
            with self.subTest(field=field):
                self.assertIn(f"`{field}`", text)
        self.assertIn("Aggregate cardinality is the component-wise sum of subcase cardinalities", normalized)
        self.assertIn("the parent passes iff every declared subcase", normalized)
        for case_id in ("B02", "A05", "C03", "W01"):
            self.assertIn(case_id, normalized)
        self.assertIn('"subcases":[{', normalized)
        example = next(
            example for example in re.findall(r"(?ms)^```json\n(.*?)^```", text)
            if '"schema": "tpm-supervisor-staging-case/v1"' in example
        )
        self.assertRegex(example, r'"subcases": \[\]')
        self.assertIn("one nested object per declared variant", text)

    def test_supervisor_proof_and_transition_bind_task_and_epoch_identity(self) -> None:
        self.assertIn("| Envelope | Required fields and invariants |", self.supervisor_design)
        for schema in ("tpm-validator-proof/v1", "tpm-supervisor-transition/v1"):
            row = re.search(rf"(?m)^\| `{re.escape(schema)}` \| .*", self.supervisor_design)
            self.assertIsNotNone(row, schema)
            assert row is not None
            for field in ("task_uid", "repository", "canonical_worktree", "bootstrap_epoch", "evidence_epoch"):
                with self.subTest(schema=schema, field=field):
                    self.assertIn(f"`{field}`", row.group(0))
            self.assertNotIn("run_id", row.group(0))
            self.assertNotIn("case_id", row.group(0))
        self.assertRegex(
            self.supervisor_design,
            r"evidence_epoch[\s\S]*?concrete on\s+all later route/action/receipt/proof/collaboration/wake/transition envelopes",
        )

    def test_supervisor_b02_identity_and_epoch_subcases_follow_closed_mapping(self) -> None:
        matrix = re.search(
            r"(?ms)^\| Case ID \| Stage .*?(?=^The operational catalog)",
            self.supervisor_design,
        )
        self.assertIsNotNone(matrix, "supervisor fault matrix is required")
        assert matrix is not None
        row = re.search(r"(?m)^\| `B02` .*", matrix.group(0))
        self.assertIsNotNone(row, "B02 row is required")
        assert row is not None
        for mapping in ("identity -> rejected(identity)", "epoch -> rejected(epoch)"):
            with self.subTest(mapping=mapping):
                self.assertIn(mapping, row.group(0))
        self.assertIn("typed_subcases aggregate:A=0/P=0/S=0/R=2/B=0/C=0", row.group(0))
        self.assertIn("typed_subcases per_subcase: reject_same_epoch", row.group(0))
        schema = re.search(
            r"(?ms)^### 10\.5 Machine-readable case summary and human report\n(.*?)(?=^### 10\.6)",
            self.supervisor_design,
        )
        self.assertIsNotNone(schema, "machine-readable case schema is required")
        assert schema is not None
        normalized = re.sub(r"\s+", " ", schema.group(1))
        self.assertIn("B02={identity,epoch}", normalized)
        self.assertIn('observed_outcome={"kind":"all_subcases_pass"', normalized)

    def test_supervisor_wake_rows_separate_external_wait_state_from_phase(self) -> None:
        phase_matrix = re.search(
            r"(?ms)^\| Phase family \| Required validated evidence .*?(?=^The final two rows)",
            self.supervisor_design,
        )
        self.assertIsNotNone(phase_matrix, "phase matrix is required")
        assert phase_matrix is not None
        valid_phases = set(re.findall(r"`([a-z][a-z0-9_]*)`", phase_matrix.group(0)))
        matrix = re.search(
            r"(?ms)^\| Case ID \| Stage .*?(?=^The operational catalog)",
            self.supervisor_design,
        )
        self.assertIsNotNone(matrix, "supervisor fault matrix is required")
        assert matrix is not None
        for case_id in ("W01", "W02", "W03"):
            row = re.search(
                rf"(?m)^\| `{case_id}` \| [^|]+ \| `([^`]+)` \| `([^`]+)` \|",
                matrix.group(0),
            )
            self.assertIsNotNone(row, case_id)
            assert row is not None
            state, phase = row.groups()
            self.assertEqual("external_wait", state, case_id)
            self.assertIn(phase, valid_phases, case_id)
            self.assertNotEqual("external_wait", phase, case_id)
        self.assertIn("input={workflow_state:external_wait,phase:execute}", self.supervisor_design)
        operational = re.search(
            r"(?ms)^\| Operational ID \| Fault family / injection .*?(?=^The closed positive catalog)",
            self.supervisor_design,
        )
        self.assertIsNotNone(operational, "operational matrix is required")
        assert operational is not None
        for state, phase in re.findall(r"\| `([^`]+)` / `([^`]+)` \|", operational.group(0)):
            self.assertIn(phase, valid_phases, f"{state}/{phase}")
            self.assertNotEqual("external_wait", phase, f"{state}/{phase}")

    def test_supervisor_return_and_wake_variants_follow_closed_mapping(self) -> None:
        matrix = re.search(
            r"(?ms)^\| Case ID \| Stage .*?(?=^The operational catalog)",
            self.supervisor_design,
        )
        self.assertIsNotNone(matrix, "supervisor fault matrix is required")
        assert matrix is not None
        for case_id, mappings in (
            (
                "C03",
                (
                    "forged -> rejected(authority)",
                    "stale -> stale_return(category=stale)",
                    "replayed -> stale_return(category=replay)",
                    "out_of_order -> stale_return(category=ordering)",
                    "partial -> rejected(malformed)",
                    "wrong_scope -> rejected(scope)",
                ),
            ),
            (
                "W01",
                (
                    "same_digest_duplicate -> accepted",
                    "different_digest_duplicate -> stale_receipt",
                    "stale -> stale_fencing",
                    "out_of_order -> stale_fencing",
                    "fencing -> stale_fencing",
                ),
            ),
        ):
            row = re.search(rf"(?m)^\| `{case_id}` .*", matrix.group(0))
            self.assertIsNotNone(row, case_id)
            assert row is not None
            for mapping in mappings:
                with self.subTest(case_id=case_id, mapping=mapping):
                    self.assertIn(mapping, row.group(0))
            self.assertIn("new_epoch=false", row.group(0), case_id)
        w01 = re.search(r"(?m)^\| `W01` .*", matrix.group(0))
        self.assertIsNotNone(w01)
        assert w01 is not None
        self.assertNotIn("accepted(existing_result)", w01.group(0))
        self.assertIn("consume_result=existing_result", w01.group(0))
        self.assertIn("readback=existing_result", w01.group(0))

    def test_supervisor_composite_subcases_bind_matrix_and_aggregate(self) -> None:
        matrix = re.search(
            r"(?ms)^\| Case ID \| Stage .*?(?=^The operational catalog)",
            self.supervisor_design,
        )
        self.assertIsNotNone(matrix, "supervisor fault matrix is required")
        assert matrix is not None
        expected_aggregates = {"B02": "R=2", "A05": "R=7", "C03": "R=6", "W01": "R=4"}
        for case_id, aggregate in expected_aggregates.items():
            row = re.search(rf"(?m)^\| `{case_id}` .*", matrix.group(0))
            self.assertIsNotNone(row, case_id)
            assert row is not None
            self.assertIn("typed_subcases aggregate:", row.group(0), case_id)
            self.assertIn(aggregate, row.group(0), case_id)
            self.assertNotIn("A=0/P=0/S=0/R=1", row.group(0), case_id)
            self.assertIn("typed_subcases per_subcase:", row.group(0), case_id)

        section = re.search(
            r"(?ms)^### 10\.5 Machine-readable case summary and human report\n(.*?)(?=^### 10\.6)",
            self.supervisor_design,
        )
        self.assertIsNotNone(section, "machine-readable case schema is required")
        assert section is not None
        schema = re.sub(r"\s+", " ", section.group(1))
        self.assertIn('`cardinality={"aggregate_from":"subcases","operation":"sum"}`', schema)
        self.assertIn('`recovery={"aggregate":"all_subcases"}`', schema)
        self.assertIn("never one tuple", schema)
        self.assertIn(
            'parent uses `expected_outcome={"kind":"subcases","child_ids":[...],"child_outcomes":[...]}`',
            schema,
        )
        self.assertIn(
            'observed_outcome={"kind":"all_subcases_pass","child_ids":[...],"child_outcomes":[...]}`',
            schema,
        )
        self.assertIn("separate closed `verdict` (`pass|fail|blocked`)", schema)
        self.assertIn("`verdict` is never an outcome code", schema)
        self.assertIn("aggregate verdict is `blocked` if any case is blocked", schema)
        self.assertIn("scalar parent code is invalid when subcases exist", schema)
        self.assertIn("a composite expected field", schema)
        self.assertIn(
            '"expected_outcome":{"kind":"code","code":"rejected(authority)"}',
            schema,
        )
        self.assertIn(
            '"observed_outcome":{"kind":"code","code":"rejected(authority)"}',
            schema,
        )
        self.assertIn('"verdict":"pass"', schema)
        self.assertIn("same-digest W01 has semantic outcome `accepted`, typed `consume_result=existing_result`/`readback=existing_result`, R=0, idempotent_existing", schema)
        self.assertIn("every other W01 subcase has R=1/reject_same_epoch", schema)
        for variant in (
            "B02={identity,epoch}",
            "A05={forged,malformed,wrong_identity,stale,revision,different_digest,fencing}",
            "C03={forged,stale,replayed,out_of_order,partial,wrong_scope}",
            "W01={same_digest_duplicate,different_digest_duplicate,stale,out_of_order,fencing}",
        ):
            self.assertIn(variant, schema)

    def test_supervisor_soak_wake_evidence_is_nested_and_aggregateable(self) -> None:
        section = re.search(
            r"(?ms)^### 10\.5 Machine-readable case summary and human report\n(.*?)(?=^### 10\.6)",
            self.supervisor_design,
        )
        self.assertIsNotNone(section, "machine-readable case schema is required")
        assert section is not None
        text = section.group(1)
        normalized = re.sub(r"\s+", " ", text)
        self.assertIn("`observed.wakes` is a typed nested array", normalized)
        for field in (
            "wake_id", "delivery_id", "attempt", "fencing_token",
            "consume_result", "readback", "digest",
        ):
            with self.subTest(field=field):
                self.assertIn(f"`{field}`", text)
        self.assertIn("len(wakes)=wake_deliveries=wake_consumes", normalized)
        self.assertIn('"wakes": []', text)

    def test_supervisor_unavailable_observed_outcome_requires_blocked_verdict(self) -> None:
        section = re.search(
            r"(?ms)^### 10\.5 Machine-readable case summary and human report\n(.*?)(?=^### 10\.6)",
            self.supervisor_design,
        )
        self.assertIsNotNone(section, "machine-readable case schema is required")
        assert section is not None
        normalized = re.sub(r"\s+", " ", section.group(1))
        unavailable = re.search(
            r"An unavailable observed field is (`\{\"kind\":\"unavailable\",\"reason\":\"<closed-reason>\",\"evidence_digest\":\"\.\.\.\"\}`) only with `verdict=blocked`",
            normalized,
        )
        self.assertIsNotNone(unavailable, "unavailable observed shape must be explicit")
        assert unavailable is not None
        self.assertEqual(
            {"kind": "unavailable", "reason": "<closed-reason>", "evidence_digest": "..."},
            json.loads(unavailable.group(1).strip("`")),
        )
        for reason in (
            "producer_missing", "validator_unavailable", "readback_unavailable",
            "wake_unavailable", "transport_unavailable",
        ):
            self.assertIn(reason, normalized)
        self.assertIn("`evidence_digest` binds retained evidence", normalized)
        self.assertIn("unavailable is invalid with pass/fail", normalized)

    def test_supervisor_outcome_catalog_maps_generic_rejections(self) -> None:
        catalog = re.search(
            r"(?ms)^`input_workflow_state` .*?(?=^Cardinality is)",
            self.supervisor_design,
        )
        self.assertIsNotNone(catalog, "supervisor outcome catalog is required")
        text = catalog.group(0)
        for code in (
            "stale_receipt",
            "identity_mismatch",
            "receipt_mismatch",
            "stale_fencing",
            "rejected",
        ):
            with self.subTest(code=code):
                self.assertIn(f"`{code}`", text)
        for category in (
            "schema", "identity", "epoch", "revision", "lease", "fencing",
            "scope", "ordering", "authority", "malformed",
        ):
            with self.subTest(category=category):
                self.assertIn(category, text)
        for mapping in (
            "schema|malformed -> rejected(category)",
            "identity|epoch|scope|authority -> rejected(category)",
            "revision|lease|ordering -> stale_receipt",
            "fencing -> stale_fencing",
            "same-digest duplicates use semantic outcome `accepted` plus typed consume/readback `existing_result`",
            "different-digest duplicates map to `stale_receipt`",
        ):
            with self.subTest(mapping=mapping):
                self.assertIn(mapping, text)
        matrix = re.search(
            r"(?ms)^\| Case ID \| Stage .*?(?=^The operational catalog)",
            self.supervisor_design,
        )
        self.assertIsNotNone(matrix, "supervisor fault matrix is required")
        assert matrix is not None
        m01 = re.search(r"(?m)^\| `M01` .*", matrix.group(0))
        self.assertIsNotNone(m01, "M01 merge-receipt case is required")
        assert m01 is not None
        self.assertIn("`rejected(malformed)`", m01.group(0))
        self.assertNotIn("`receipt_mismatch`", m01.group(0))

    def test_supervisor_fault_rows_do_not_implicitly_reset_epoch(self) -> None:
        matrix = re.search(
            r"(?ms)^\| Case ID \| Stage .*?(?=^The closed positive catalog)",
            self.supervisor_design,
        )
        self.assertIsNotNone(matrix, "supervisor fault and operational matrices are required")
        rows = matrix.group(0)
        no_reset_cases = ("B01", "B02", "A05", "V01", "C01", "C03", "W01", "R01", "M01", "O05", "O06", "O07", "O08")
        for case_id in no_reset_cases:
            row = re.search(rf"(?m)^\| `{case_id}` .*", rows)
            self.assertIsNotNone(row, case_id)
            self.assertIn("new_epoch=false", row.group(0), case_id)
            self.assertRegex(row.group(0), r"(?:capability_unchanged|reject_same_epoch)", case_id)
        for case_id in ("A04", "V02", "W03", "O01", "O02", "O04"):
            row = re.search(rf"(?m)^\| `{case_id}` .*", rows)
            self.assertIsNotNone(row, case_id)
            self.assertIn("authorized_reset(", row.group(0), case_id)
            self.assertIn("reset_authority=", row.group(0), case_id)
        for row in re.findall(r"(?m)^\| `[^`]+` .*", rows):
            if "new_epoch=true" in row:
                self.assertIn("authorized_reset(", row)
                self.assertIn("reset_authority=", row)

    def test_supervisor_embedded_recovery_examples_require_authorized_reset(self) -> None:
        examples = re.findall(r"(?ms)^```json\n(.*?)^```", self.supervisor_design)
        json_example = next(
            (example for example in examples if '"schema": "tpm-supervisor-staging-case/v1"' in example),
            None,
        )
        self.assertIsNotNone(json_example, "supervisor staging case JSON example is required")
        assert json_example is not None
        b01 = re.search(
            r'"case_id": "adversarial-B01-7001".*?(?=^  "expected")',
            json_example,
            re.S | re.M,
        )
        self.assertIsNotNone(b01, "embedded B01 case example is required")
        self.assertRegex(b01.group(0), r'"rule": "capability_unchanged"')
        self.assertRegex(b01.group(0), r'"new_epoch": false')
        recovery_lines = re.findall(r"(?m)^\s*\"recovery\": \{.*$", json_example)
        self.assertTrue(recovery_lines, "embedded recovery examples are required")
        for line in recovery_lines:
            if '"new_epoch": true' in line:
                self.assertRegex(line, r'"rule": "authorized_reset"')
                self.assertRegex(line, r'"reset_authority": "[^"]+"')

    def test_supervisor_b01_capability_block_transition_is_typed_and_coherent(self) -> None:
        examples = re.findall(r"(?ms)^```json\n(.*?)^```", self.supervisor_design)
        example = next(
            (item for item in examples if '"case_id": "adversarial-B01-7001"' in item),
            None,
        )
        self.assertIsNotNone(example, "embedded B01 case example is required")
        assert example is not None
        record = json.loads(example)
        transitions = record["observed"]["transitions"]
        self.assertEqual(1, len(transitions))
        transition = transitions[0]
        self.assertEqual("tpm-supervisor-transition/v1", transition["schema"])
        allowed_transition_fields = {
            "schema", "task_uid", "repository", "canonical_worktree", "bootstrap_epoch",
            "evidence_epoch", "transition_id", "event_id", "action_id", "receipt_id",
            "parent_event_digest", "parent_transition_digest", "parent_rejection_digest",
            "from_state", "to_state", "from_phase", "to_phase", "expected_revision",
            "new_revision", "lease_id", "fencing_token", "validator_proof_digest",
            "outcome_code", "status", "result", "expected_state_digest",
            "observed_state_digest", "state_digest", "committed_at", "transition_digest",
        }
        self.assertEqual(allowed_transition_fields, set(transition))
        self.assertNotIn("run_id", transition)
        self.assertNotIn("case_id", transition)
        for field in ("task_uid", "repository", "canonical_worktree", "bootstrap_epoch", "evidence_epoch"):
            self.assertEqual(record[field], transition[field], field)
        self.assertEqual("running", transition["from_state"])
        self.assertEqual("capability_blocked", transition["to_state"])
        self.assertEqual("bootstrap", transition["from_phase"])
        self.assertEqual("bootstrap", transition["to_phase"])
        self.assertEqual("producer_missing", transition["outcome_code"])
        self.assertEqual("CapabilityBlocked(producer_missing)", transition["result"])
        self.assertEqual(0, transition["expected_revision"])
        self.assertEqual(1, transition["new_revision"])
        self.assertIsNone(transition["action_id"])
        self.assertIsNone(transition["receipt_id"])
        self.assertIsNone(transition["validator_proof_digest"])
        self.assertEqual(transition["state_digest"], transition["observed_state_digest"])
        self.assertEqual([transition["transition_digest"]], record["transition_digests"])
        final_cardinality = record["observed"]["final"]["cardinality"]
        self.assertEqual(0, final_cardinality["accepted_external_effects"])
        self.assertEqual(0, final_cardinality["phase_advances"])
        self.assertEqual(1, final_cardinality["state_changes"])
        self.assertEqual(0, final_cardinality["classified_rejections"])
        self.assertEqual(1, final_cardinality["capability_blocks"])
        self.assertEqual([], record["observed"]["effects"])
        self.assertEqual([], record["observed"]["readbacks"])
        self.assertEqual(
            {"kind": "code", "code": "producer_missing"},
            record["expected"]["expected_outcome"],
        )
        self.assertEqual(
            {"kind": "code", "code": "producer_missing"},
            record["observed_outcome"],
        )
        self.assertEqual("pass", record["verdict"])
        self.assertNotIn("outcome", record)
        self.assertIn("nested repository envelopes never carry", self.supervisor_design)
        self.assertNotIn("staging-only `run_id`/`case_id` binding", self.supervisor_design)

    def test_supervisor_state_hash_has_canonical_preimage_and_ascii_lf(self) -> None:
        section = re.search(
            r"(?ms)^#### Replay envelope and canonical state hash\n(.*?)(?=^#### State and phase transition matrix)",
            self.supervisor_design,
        )
        self.assertIsNotNone(section, "supervisor state-hash contract is required")
        assert section is not None
        text = section.group(1)
        self.assertIn(
            '`SHA-256(ASCII("oasis7/tpm/tpm-supervisor-state-hash/v1") || byte[0x0A] || UTF-8(canonical_json(preimage)))`',
            text,
        )
        self.assertNotRegex(text, r"tpm-supervisor-state-hash/v1\\\\n")
        self.assertRegex(
            text,
            r"(?s)canonical_worktree.*preimage.*included",
        )
        field_order = re.search(r"exact\s+semantic\s+field\s+order.*?`\[(.*?)\]`", text, re.S)
        self.assertIsNotNone(field_order, "state-hash field order must be explicit")
        assert field_order is not None
        self.assertEqual(
            [
                "schema", "task_uid", "repository", "canonical_worktree", "task_branch",
                "bootstrap_epoch", "evidence_epoch", "workflow_state", "phase", "next_result",
                "revision", "head_oid", "base_oid", "comparison_oid", "event_parent_digest",
                "transition_parent_digest", "validator_proof_digest", "active_action", "wait", "effect_states",
                "rejection_parent_digest", "terminal_outcome",
            ],
            [field.strip() for field in field_order.group(1).split(",")],
        )
        self.assertIn("non-identity local paths", text)
        self.assertIn("`canonical_worktree` is the canonical identity string", text)
        self.assertIn("only the `transition_digest` of the transition currently being published is", text)
        self.assertIn("referenced `validator_proof_digest`/effect/receipt digests remain", text)
        self.assertIn("`validator_proof_digest` is included as a referenced proof input", self.supervisor_design)
        self.assertIn("`transition_parent_digest`", text)
        self.assertNotIn("Envelope digests, signatures", text)

    def test_supervisor_human_report_partitions_all_catalog_records(self) -> None:
        report = re.search(
            r"(?ms)^The human `staging-report\.md` .*?(?=^### 10\.6)",
            self.supervisor_design,
        )
        self.assertIsNotNone(report, "human staging-report contract is required")
        text = report.group(0)
        for phrase in (
            "81 adversarial rows",
            "one per positive seed (3)",
            "one per soak seed (3)",
            "exactly 87 case/seed rows",
        ):
            with self.subTest(phrase=phrase):
                self.assertIn(phrase, text)

    def test_eval_and_role_fit_own_disjoint_scratch_roots(self) -> None:
        eval_text = WORKFLOW_EVAL.read_text(encoding="utf-8")
        role_text = ROLE_FIT.read_text(encoding="utf-8")
        self.assertRegex(eval_text, r"(?:PM_ROOT_DIR|OASIS7_[A-Z_]*SCRATCH)[^\n]*\$TMP_DIR")
        self.assertRegex(role_text, r"(?:PM_ROOT_DIR|OASIS7_[A-Z_]*SCRATCH)[^\n]*\$TMP_DIR")
        self.assertNotRegex(eval_text, r"(?m)^\s*(?:rm|cp|mv).+ROOT_DIR.+\.pm")
        self.assertNotRegex(role_text, r"(?m)^\s*(?:rm|cp|mv).+ROOT_DIR.+\.pm")

    def test_tpm_is_coordinator_not_the_professional_task_owner(self) -> None:
        ownership = self.section("Lifecycle ownership")
        normalized = re.sub(r"\s+", " ", ownership.lower())
        self.assertRegex(normalized, r"tpm.{0,100}(coordinator|continuation owner)")
        self.assertRegex(normalized, r"task owner role.{0,120}(result|outcome|implementation|judgment)")
        self.assertNotRegex(normalized, r"tpm.{0,80}(professional|implementation|verification) owner")

    def test_unattended_invariants_are_minimal(self) -> None:
        appendix = self.section("Appendix A: Unattended automation invariants")
        self.assertLessEqual(len(appendix.splitlines()), 12, "hypothetical design obscures current runbook")
        for detail in ("CAS", "wake", "lease", "Retry-After", "scheduler"):
            self.assertNotIn(detail, appendix)

    def test_secondary_policy_is_thin_not_link_only(self) -> None:
        policy = self.section("Documentation policy")
        self.assertIn("thin operational entrypoint", policy.lower())
        self.assertNotIn("link-only", policy.lower())

    def test_canonical_spec_has_a_line_budget_and_no_boilerplate_labels(self) -> None:
        self.assertNotIn("Canonical definition:", self.text)

    def test_bootstrap_snapshot_example_supplies_required_repo_root(self) -> None:
        skill = BOOTSTRAP_SKILL.read_text(encoding="utf-8")
        self.assertRegex(
            skill,
            r"bootstrap-task-snapshot\.py validate-or-create"
            r" --repo-root <canonical-worktree> --task-uid <task_uid> --producer tpm",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
