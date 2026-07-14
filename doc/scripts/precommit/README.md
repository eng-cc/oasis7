# Pre-commit governance

## Start here

- Need the ordinary-commit no-op contract, legacy hook setup, or the boundary to authoritative `required` checks: read `pre-commit.prd.md`.
- Need the repeatable repair sequence after a pre-commit failure: read `precommit-remediation-playbook.prd.md` and use `scripts/fix-precommit.sh`.
- Need the executable command matrix rather than this navigation layer: read `scripts/ci-tests.sh`; CI-tier authority stays in `doc/testing/ci/ci-tiered-execution.prd.md`.
- Need current scripts-module scope or task status: return to `doc/scripts/prd.md` or `doc/scripts/project.md`.

## Authority boundary

This page is the first-read router for `doc/scripts/precommit/`; it does not duplicate commands, failure procedures, or CI policy.

| Question | Canonical source |
| --- | --- |
| What runs before a local commit? | Nothing; see `pre-commit.prd.md` and the legacy no-op `scripts/pre-commit.sh` |
| How do I repair a repeatable pre-commit failure? | `precommit-remediation-playbook.prd.md` plus `scripts/fix-precommit.sh` |
| What does each CI tier cover? | `doc/testing/ci/ci-tiered-execution.prd.md` plus `scripts/ci-tests.sh` |
| What was completed for either topic? | The matching `*.project.md` file |

## Maintenance

- Keep the two topic triplets file-addressable in `doc/scripts/prd.index.md`.
- Update this router in the same change when a new pre-commit topic changes a first-read path or authority boundary.
- Retire a leaf only after its current successor and lack of active callers are both demonstrated; completed project records remain execution evidence.
