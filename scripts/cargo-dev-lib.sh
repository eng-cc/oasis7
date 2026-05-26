#!/usr/bin/env bash

oasis7_cargo_dev_repo_root() {
  local repo_root
  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
  printf '%s\n' "$repo_root"
}

oasis7_cargo_dev_use_shared_target() {
  if [[ "${OASIS7_CARGO_DEV_SHARED:-1}" == "0" ]]; then
    return 1
  fi
  if [[ "${OASIS7_FORCE_RAW_CARGO:-0}" == "1" ]]; then
    return 1
  fi
  if [[ "${CI:-}" == "1" || "${CI:-}" == "true" ]]; then
    return 1
  fi
  return 0
}

oasis7_cargo_dev_target_dir() {
  local repo_root="${1:-$(oasis7_cargo_dev_repo_root)}"
  if oasis7_cargo_dev_use_shared_target; then
    "$repo_root/scripts/cargo-dev.sh" --print-target-dir
    return 0
  fi

  if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    if [[ "$CARGO_TARGET_DIR" == /* ]]; then
      printf '%s\n' "$CARGO_TARGET_DIR"
    else
      printf '%s\n' "$repo_root/$CARGO_TARGET_DIR"
    fi
  else
    printf '%s\n' "$repo_root/target"
  fi
}

oasis7_cargo_dev_debug_bin_dir() {
  local repo_root="${1:-$(oasis7_cargo_dev_repo_root)}"
  printf '%s/debug\n' "$(oasis7_cargo_dev_target_dir "$repo_root")"
}

oasis7_cargo_dev() {
  local repo_root="${OASIS7_CARGO_DEV_REPO_ROOT:-$(oasis7_cargo_dev_repo_root)}"
  if oasis7_cargo_dev_use_shared_target; then
    "$repo_root/scripts/cargo-dev.sh" "$@"
  else
    env -u RUSTC_WRAPPER cargo "$@"
  fi
}
