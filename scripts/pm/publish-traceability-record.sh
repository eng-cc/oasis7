#!/usr/bin/env bash
# Trust-boundary launcher for the coordinating-record publication CLI.
# It rejects untracked code from the only repository import root before Python
# starts, then runs an isolated interpreter with that verified root injected.

set -euo pipefail

die() {
    printf 'publisher launcher: %s\n' "$1" >&2
    exit 2
}

draft_arg=""
enable_publication=0
while (($#)); do
    case "$1" in
        --draft)
            (($# >= 2)) || die "--draft requires a file path"
            [[ -z "$draft_arg" ]] || die "--draft may be supplied only once"
            draft_arg="$2"
            shift 2
            ;;
        --enable-publication)
            [[ "$enable_publication" == 0 ]] || die "--enable-publication may be supplied only once"
            enable_publication=1
            shift
            ;;
        *)
            die "unsupported argument: $1"
            ;;
    esac
done
[[ -n "$draft_arg" ]] || die "usage: publish-traceability-record.sh --draft <canonical-json-file> [--enable-publication]"

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)" || die "cannot resolve launcher path"
repo_root="$(git -C "$script_dir/../.." rev-parse --show-toplevel 2>/dev/null)" || die "cannot resolve repository root"
repo_root="$(cd -- "$repo_root" && pwd -P)" || die "cannot resolve repository root"
pm_root="$repo_root/scripts/pm"
[[ -d "$pm_root" ]] || die "publisher import root is missing"
pm_root="$(cd -- "$pm_root" && pwd -P)" || die "cannot resolve publisher import root"
[[ "$script_dir" == "$pm_root" ]] || die "launcher must run from the repository publisher import root"

if ! git -C "$repo_root" diff --quiet HEAD --; then
    die "tracked checkout must be clean before publisher startup"
fi

if [[ "$draft_arg" == /* ]]; then
    draft_candidate="$draft_arg"
else
    draft_candidate="$PWD/$draft_arg"
fi
[[ -f "$draft_candidate" && ! -L "$draft_candidate" ]] || die "draft must be an existing regular non-symlink file"
draft_parent="$(cd -- "$(dirname -- "$draft_candidate")" && pwd -P)" || die "cannot resolve draft directory"
draft_path="$draft_parent/$(basename -- "$draft_candidate")"
case "$draft_path" in
    "$pm_root"/*) die "draft must be outside scripts/pm, the publisher import root" ;;
esac

# Do not use --exclude-standard: ignored/untracked import candidates are still
# executable Python inputs when scripts/pm is added to sys.path below.
while IFS= read -r -d '' untracked_path; do
    die "untracked publisher import input is not allowed: scripts/pm/$untracked_path"
done < <(git -C "$repo_root" ls-files --others -z -- scripts/pm)

bootstrap='import sys
publisher_root, repository_root, draft_path, enable = sys.argv[1:5]
sys.path.insert(0, publisher_root)
import loop_traceability
args = ["publish-record", "--draft", draft_path, "--repo-root", repository_root]
if enable == "1":
    args.append("--enable-publication")
raise SystemExit(loop_traceability._trusted_launcher_main(args))'

exec python3 -I -S -c "$bootstrap" "$pm_root" "$repo_root" "$draft_path" "$enable_publication"
