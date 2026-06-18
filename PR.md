# Polish viewer visual hierarchy

- PR: https://github.com/eng-cc/oasis7/pull/524
- Task UID: task_e7760ad76a0046dfa5a17d0a5a89e59c
- Source Branch: task/viewer-viewer-visual-polish
- Base Branch: main
- Purpose: normal_pr_ci_watch

## Verification

- `PATH=/Users/scc/.nvm/versions/node/v24.14.1/bin:$PATH npm --prefix crates/oasis7_viewer run test:ui`
- `PATH=/Users/scc/.nvm/versions/node/v24.14.1/bin:$PATH npm --prefix crates/oasis7_viewer run build:software-safe`
- `PATH=/Users/scc/.nvm/versions/node/v24.14.1/bin:$PATH npm --prefix crates/oasis7_viewer run test:pixel-world:visual`

## Review Evidence

- Pre-PR local role review: passed
- Roles: game_visual_interaction_designer, viewer_engineer, qa_engineer
- Findings disposition: no_findings
