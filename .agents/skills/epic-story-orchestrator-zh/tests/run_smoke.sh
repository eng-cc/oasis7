#!/usr/bin/env bash
set -euo pipefail

skill_root=".agents/skills/epic-story-orchestrator-zh"

for p in \
  "$skill_root/SKILL.md" \
  "$skill_root/README.md" \
  "$skill_root/templates/world-bible.template.md" \
  "$skill_root/templates/character-registry.template.md" \
  "$skill_root/templates/timeline.template.md" \
  "$skill_root/templates/plot-branches.template.md" \
  "$skill_root/templates/chapter-card.template.md" \
  "$skill_root/templates/canon-log.template.md" \
  "$skill_root/templates/consistency-report.template.md" \
  "$skill_root/tests/smoke-scaffold.md" \
  "$skill_root/tests/smoke-chapter-draft.md" \
  "$skill_root/tests/smoke-canon-audit.md" \
  "$skill_root/tests/fixtures/writeback.sample.json" \
  "$skill_root/tests/validate_writeback.py"
do
  test -f "$p"
done

python3 "$skill_root/tests/validate_writeback.py"
git diff --check
./scripts/doc-governance-check.sh

if [[ "${RUN_PM_LINT:-0}" == "1" ]]; then
  ./scripts/pm/lint.sh
else
  echo "skip pm lint (set RUN_PM_LINT=1 to enable)"
fi

echo "epic-story-orchestrator-zh smoke: OK"
