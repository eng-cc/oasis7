# Oasis7 Command Center

macOS desktop implementation of [PR #4518](https://github.com/eng-cc/oasis7/pull/4518), following the [product contract](../../doc/engineering/command-center/command-center.prd.md) and [system design](../../doc/engineering/command-center/command-center.design.md).

This application implements the two foundation increments: local document/Git reading, Chinese search, durable decisions and reading history, manual work-to-goal associations, and an optional read-only GitHub PR/check window in four Chinese views. The optional experimental Codex execution increment is not implemented. There are no model tasks or owned execution processes; existing Codex work remains in its original tool. Missing runtime observation is displayed as unknown, never as an empty task inventory.

## Run and package

Use the repository Rust toolchain, Node 22 or newer, and the macOS developer tools. From this directory:

```sh
npm --prefix ui ci
npm --prefix ui run desktop:dev
npm --prefix ui run desktop:build
```

The build produces a `.app` in `target/release/bundle/macos/`. Its frontend is embedded; launching it does not require Node, Vite, a terminal backend, or the game stack. This is a local ad-hoc signed development build. Developer ID signing, notarization, public distribution, other CPU architectures and older macOS versions require separate verification. The configured deployment target is not evidence of validation on that system.

Choose a Git worktree with the native picker or enter its absolute path. The first load validates the actual repository/worktree and reads its tracked Markdown, HEAD, branch, dirty changes and worktree inventory. Refresh only reads; it never fetches, changes branches, commits or launches a model. Close the window to retain the menu bar entry; use its open action to return and its quit action to exit.

## Sources and personal data

Current-stage sections come from `doc/core/prd.md`'s current delivery/P0 heading. Other goals and rules retain their source path, content version and committed baseline. Document identity does not establish implementation acceptance. The UI does not synthesize a completion percentage or turn long-term goals into current blockers.

The bounded local index covers at most 3,000 tracked paths, 1 MiB per file and 16 MiB in total. It excludes untracked files and escaped symlinks and reports partial coverage. Short Chinese queries use parameterized literal substring search; longer queries use SQLite FTS5 trigram. Document content is rendered as escaped text, with no raw HTML or remote media execution.

SQLite lives in the platform application-data directory (`com.oasis7.command-center`), outside the repository. Personal decisions, confirmed-but-unwritten decisions, work associations and reading versions are scoped to the project/worktree and survive source-cache rebuilding. Saving a decision does not update the source document or prove implementation synchronization. Saved reading versions can be opened after the current content changes. Back up the application-data directory with the App closed; do not remove the database to rebuild the source index. Future schema versions are rejected without clearing data.

GitHub is explicitly connected from the execution view. The app uses an existing `gh` login, checking common absolute installation locations; a different absolute executable path can be selected. It currently supports `github.com` and reads a bounded window of the latest 100 PRs, including the returned check summaries. This window is partial, not a whole-repository snapshot. Tested commit IDs remain unknown when the source does not provide them. The account, last success and coverage stay visible; failures retain the last successful facts. Credentials are owned by gh, never copied into UI assets or SQLite. Native token-entry/Keychain management and enterprise hosts are not supplied in this foundation.

## Verification

```sh
env -u RUSTC_WRAPPER cargo test --locked -p oasis7-command-center-core
env -u RUSTC_WRAPPER cargo run --locked -p oasis7-command-center-core --example export_types
env -u RUSTC_WRAPPER cargo fmt --all -- --check
npm --prefix ui test
npm --prefix ui run build
env -u RUSTC_WRAPPER cargo clippy --locked --workspace --all-targets -- -D warnings
```

The independent workspace owns a single Cargo lockfile. Do not change `CARGO_TARGET_DIR`. Generated TypeScript contracts are committed and checked against Rust. [Command Center CI](../../.github/workflows/command-center.yml) runs core regressions, frontend tests/build and native host checks/build on macOS. The normal repository checks also continue to apply.

A debug first load on macOS 26.5 / arm64 read 7,499 sections from the implementation worktree in about 20 seconds, using an in-memory SQLite database. This is a single observed sample, not a startup guarantee or a production resource benchmark.

Desktop acceptance must additionally exercise the packaged App: native folder selection; a real repository with a dirty Markdown file; two-character Chinese search; decision save, quit/reopen and cache rebuild; history after source edits; one PR linked to two goals; unavailable gh preserving local capability; close/reopen through the menu bar; keyboard focus and Chinese composition. Browser tests verify UI behavior, not VoiceOver or macOS IME quality. Signing, older-system compatibility and the optional runtime-control acceptance remain unproven until separately tested.
