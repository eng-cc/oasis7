# Pre-commit governance

## Document model

- `README.md` is the first-read router.
- `pre-commit.prd.md` owns the current ordinary-commit contract, legacy hook setup, and explicit repair sequence.
- `pre-commit.project.md` preserves completed implementation history.

CI tier definitions and command coverage stay outside this topic, in
`doc/testing/ci/` and `scripts/ci-tests.sh`.

## Authority boundary

This page is the first-read router for `doc/scripts/precommit/`; it does not duplicate commands, failure procedures, or CI policy.

| Question | Canonical source |
| --- | --- |
| What runs before a local commit? | Nothing; see `pre-commit.prd.md` and the legacy no-op `scripts/pre-commit.sh` |
| How do I repair a formatting or explicit-validation failure? | `pre-commit.prd.md` plus `scripts/fix-precommit.sh` |
| What does each CI tier cover? | `doc/testing/ci/ci-tiered-execution.prd.md` plus `scripts/ci-tests.sh` |
| What was completed for this topic? | `pre-commit.project.md` |
| Where is scripts-module scope or current task status? | `doc/scripts/prd.md` and `doc/scripts/project.md` |

## Maintenance

- Keep the two-file topic chain addressable in `doc/scripts/prd.index.md`.
- Update this router in the same change when a new pre-commit topic changes a first-read path or authority boundary.
- Completed project history stays in `pre-commit.project.md`; current behavior and operator actions stay in `pre-commit.prd.md`.
- Do not add redirect-only design or compatibility leaves. Historical review
  logs retain audit conclusions; exact deleted-path provenance stays in Git
  history.
