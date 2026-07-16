# Repository Health Manual Inspection Runbook

## Purpose
This runbook defines the manually triggered repository-health inspection used by engineering governance owners.

The inspection is a human-triaged health review. It does not add a GitHub Actions hard gate, and it does not replace the normal task/worktree, professional slice, verification, PR, or merge workflow.

## Trigger

The inspection is started manually by the engineering governance owner. There is no repository-managed scheduler, GitHub Actions schedule, or other automatic trigger for this workflow.

When the owner starts an inspection, still enter the standard `oasis7` workflow: create or enter one task worktree, bind one GitHub-backed task, and dispatch `repository_health_engineer` for professional repository-health judgment.

## Checklist
Run the checks from the task worktree and record the command outputs or summaries in GitHub task issue evidence comments.

```bash
./scripts/doc-inventory-report.sh
./scripts/doc-governance-check.sh
./scripts/lint-skills.sh
./scripts/worktree-gc-report.sh --prunable-only
./scripts/pm/lint.sh
./scripts/ci-rust-governance-report.sh --out-dir "output/rust-governance/repository-health-$(date +%Y%m%d)"
```

For code-health sampling that may be expensive, choose the narrowest tier that matches the finding and record why it was selected:

```bash
./scripts/ci-tests.sh required
```

## Interpretation
- `doc-governance-check` failure: treat as P0/P1 engineering-governance follow-up candidate.
- `lint-skills` failure: treat as P0/P1 workflow-surface follow-up candidate.
- `doc-inventory-report` `action_required`: classify by module or hotspot path, then decide whether to create a focused path-governance task or leave as quarterly trend evidence.
- `worktree-gc-report --prunable-only`: read-only only. Do not copy cleanup commands blindly; confirm the reported worktree is not the main worktree, has no useful dirty state, and is not part of an active task.
- `pm lint` failure: classify current-task failures separately from known historical execution-log evidence debt. Do not make historical debt a blocking inspection finding unless a follow-up task has scoped it.
- `ci-rust-governance-report` findings: treat cargo-deny policy failures, duplicate dependency clusters, and unsafe-usage hotspots as repository-health inputs. Read the generated duplicate dependency counts and top-crate list in addition to command status codes. Do not perform dependency upgrades inside the inspection task; create focused follow-up tasks for version upgrades, duplicate-prune work, unsafe-boundary review, or dependency-surface reduction.
- `ci-tests.sh required` failures: classify whether the failure is formatting, RustSec advisory, file-size/code-health, scoped test, or workflow-surface breakage. Only mark the inspection blocked when the current inspection task introduced the failure or the finding is an active release/merge blocker.

## Code And Dependency Health Focus
- Code health: look for oversized Rust files, repeated warning/fmt/clippy signatures, unsafe usage growth, duplicate logic hotspots, stale generated artifacts, and modules whose tests are missing from the required/support gate.
- Rust style guide: include `third_party/rust-skills/AGENTS.md` as a read-only inspection input for owned Rust code. In a fresh task worktree, initialize the submodule first with `git submodule update --init -- third_party/rust-skills`; if the file is still unavailable, record the missing input as inspection evidence and capture a focused workflow follow-up instead of silently skipping the check. Check for drift against its Rust 2024/rust-version/lint defaults, 100-character style guidance, `?` over `unwrap()` in library code, and required `// SAFETY:` comments around unsafe blocks.
- Dependency health: review `Cargo.toml`, `Cargo.lock`, cargo-deny output, duplicate dependency output, and release/build workflow version pins. Separate security/advisory upgrades from routine version refreshes and from dependency-prune opportunities.
- Upgrade and style triage: create one focused task per coherent upgrade, prune, or style-drift surface. Include the owner role, affected crates/packages, expected compatibility checks, rollback plan, and whether the work needs `runtime_engineer`, `wasm_platform_engineer`, `viewer_engineer`, `agent_engineer`, or `qa_engineer`.
- Evidence rule: record report paths such as `output/rust-governance/.../summary.md`, command exit status, and the role-attributed verdict in the inspection task log. Report-only findings are not automatically blockers.

## Code Evidence Sampling Model
Code-style and code-health conclusions need code evidence, but the inspection should not default to manually reading every code file.

- Full-repository coverage comes from automated scans and reports: required formatting/lint/test outputs, warning signatures, oversized-file checks, unsafe/dependency reports, targeted searches such as `unwrap()` and `unsafe`, and changed-path planners.
- Human review samples the code that the reports or current task make risky: changed diffs, report-hit files, near-limit or oversized Rust files, repeated warning/fmt/clippy hotspots, unsafe blocks and their `// SAFETY:` comments, public API boundaries, dependency-upgrade impact surfaces, and modules with missing or stale tests.
- Escalate from sampling to path-level deep reading when a finding cannot be classified from the report, a required/release blocker has unclear ownership, security or dependency-closure impact is uncertain, unsafe or workflow source-of-truth drift appears systemic, or the same pattern repeats across multiple modules.
- Do not claim repository-wide style conformance from reports alone. Record the sampled paths, why they were selected, what code evidence was read, and whether the result is a follow-up candidate or acceptable residual risk.

## Evidence Sink
The canonical inspection evidence is the GitHub task issue evidence comment stream
backed by `.pm/github-project-sync/tasks.json`.

The chat summary should only report the role-attributed findings, follow-up candidates, and residual risk. It is not a replacement for GitHub-backed task evidence.

High-value but not-yet-owned follow-up work should first be captured as a reflection signal:

```bash
./scripts/pm/capture-todo.sh --source-ref <path> --summary "<finding summary>"
```

Promote a signal to a formal task only after the owner chooses to create the follow-up.

## Escalation
- Repository-health, documentation/code alignment, semantic clarity, workflow drift, task evidence debt: keep with `repository_health_engineer`.
- Verification sufficiency or release blocking judgment: dispatch `qa_engineer`.
- Rust code health, dependency closure, duplicate dependencies, RustSec/cargo-deny findings, unsafe usage, or version-upgrade impact: dispatch `repository_health_engineer` first for triage, then the matching implementation role before changing code or dependency versions.
- Runtime, Viewer/Web, WASM, agent, blockchain ops, gameplay, or visual/interaction findings: dispatch the matching professional role before presenting a domain conclusion.
- External-facing community, incident, or player promise wording: dispatch `liveops_community`.

## Quarterly Review
When the owner chooses to run a quarterly review, compare recent manual inspection trends:
- repeated `doc-inventory-report` hotspots
- repeated worktree cleanup candidates
- `pm lint` historical-debt burn-down or growth
- repeated Rust governance report findings, Rust style-guide drift, duplicate dependency clusters, unsafe-usage hotspots, and dependency-upgrade backlog growth
- recurring `doc-governance-check` or `lint-skills` failure signatures
- whether any reflection signals should become committed `.pm` tasks

Quarterly conclusions should update the relevant engineering governance project/topic docs only when they change policy, thresholds, or active follow-up ownership.

Record the review in the same GitHub-backed task evidence stream as the inspection. Use this compact structure instead of creating a separate dated template document:

```markdown
## Quarterly repository-health review

- Review ID / quarter / date:
- Owner role: `repository_health_engineer`
- Review partners: roles required by the findings; include `qa_engineer` only for verification sufficiency or release-blocking judgment
- Trigger: `quarterly` / `ad-hoc-governance-change`
- Inputs: trend baseline, inspection command evidence, relevant engineering project/topic docs
- Trend verdict: `pass` / `watchlist` / `fix_required`

| ID | Category | Scope and evidence | Disposition | Owner | Priority |
| --- | --- | --- | --- | --- | --- |
| EQC-ISSUE-001 |  |  |  |  |  |

- Blocking findings:
- Follow-up reflection signals or GitHub-backed tasks:
- Evidence comment URL:
```

Do not copy forward an audit-round counter from the 2026-03 baseline. The review identity and current task evidence establish the new observation window; the baseline remains a comparison input, not a mutable template.
