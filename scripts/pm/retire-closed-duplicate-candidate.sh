#!/usr/bin/env bash
set -euo pipefail

# Human-operated launcher: bind the executable closure to the authenticated
# live default-branch commit/tree before starting Python or reading the map.
readonly canonical_repo="eng-cc/oasis7"
readonly launcher_path="scripts/pm/retire-closed-duplicate-candidate.sh"
readonly helper_path="scripts/pm/retire-closed-duplicate-candidate.py"
readonly tool_paths=(
  "scripts/pm/portable_file_lock.py"
  "scripts/pm/retire-closed-duplicate-candidate.py"
  "scripts/pm/retire-closed-duplicate-candidate.sh"
  "scripts/pm/workflow-durable-store.py"
)

usage() {
  cat <<'EOF'
Usage: retire-closed-duplicate-candidate.sh
  --mapping-root <registered-worktree>
  --task-uid <TASK-UID>
  --disposition-comment-id <server-comment-id>
  (--preflight | --apply)

Preflight is mutation-free. Apply only updates the selected local tasks.json
mapping and its append-only retirement ledger.
EOF
}

if [[ $# == 1 && ( "$1" == "--help" || "$1" == "-h" ) ]]; then
  usage
  exit 0
fi

unset OASIS7_RETIREMENT_LAUNCH_CONTEXT_V1 OASIS7_RETIREMENT_TOOL_ROOT_V1 OASIS7_RETIREMENT_AUTHORITY_FILE_V1
command -v gh >/dev/null 2>&1 || { echo "retirement launcher requires authenticated gh CLI" >&2; exit 1; }
command -v python3 >/dev/null 2>&1 || { echo "retirement launcher requires python3" >&2; exit 1; }

umask 077
tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-retirement-tool.XXXXXXXX")"
cleanup() { rm -rf -- "$tmp_root"; }
trap cleanup EXIT INT TERM

pin_query='query RetirementToolAuthority { repository(owner: "eng-cc", name: "oasis7") { defaultBranchRef { name target { oid ... on Commit { tree { oid } } } } } }'
pin_json="$(gh api graphql -f "query=$pin_query")" || {
  echo "retirement launcher could not read the canonical live default branch" >&2
  exit 1
}
pin_json="$(python3 -c '
import json, re, sys
try:
    ref = json.load(sys.stdin)["data"]["repository"]["defaultBranchRef"]
    name, target = ref["name"], ref["target"]
    commit, tree = target["oid"], target["tree"]["oid"]
except (KeyError, TypeError, json.JSONDecodeError) as exc:
    raise SystemExit(f"canonical live default-branch response is incomplete: {exc}")
if not isinstance(name, str) or not name or any(char in name for char in "\r\n\0"):
    raise SystemExit("canonical live default-branch name is malformed")
if not isinstance(commit, str) or re.fullmatch(r"[0-9a-f]{40}", commit) is None:
    raise SystemExit("canonical live commit OID is malformed")
if not isinstance(tree, str) or re.fullmatch(r"[0-9a-f]{40}", tree) is None:
    raise SystemExit("canonical live tree OID is malformed")
print(json.dumps({"default_branch": name, "commit_oid": commit, "tree_oid": tree}, separators=(",", ":")))
' <<<"$pin_json")"
read -r default_branch commit_oid tree_oid < <(
  python3 -c 'import json,sys; v=json.load(sys.stdin); print(v["default_branch"],v["commit_oid"],v["tree_oid"])' <<<"$pin_json"
)

manifest_entries='[]'
for source_path in "${tool_paths[@]}"; do
  parent_tree="$tree_oid"
  IFS='/' read -r -a components <<<"$source_path"
  final_mode=""
  final_blob=""
  for ((index=0; index<${#components[@]}; index++)); do
    component="${components[index]}"
    tree_json="$(gh api "repos/$canonical_repo/git/trees/$parent_tree")" || {
      echo "retirement launcher could not read pinned source tree for $source_path" >&2
      exit 1
    }
    entry_json="$(python3 -c '
import json, sys
component, expected_tree = sys.argv[1:]
try:
    page = json.load(sys.stdin)
    if page.get("sha") != expected_tree:
        raise ValueError("tree response OID differs from the pinned parent")
    if page.get("truncated") is True:
        raise ValueError("tree response is truncated")
    matches = [entry for entry in page["tree"] if entry.get("path") == component]
    if len(matches) != 1:
        raise ValueError("tree entry is missing or ambiguous")
    entry = matches[0]
    print(json.dumps({"type": entry["type"], "mode": entry["mode"], "sha": entry["sha"]}, separators=(",", ":")))
except (KeyError, TypeError, ValueError, json.JSONDecodeError) as exc:
    raise SystemExit(f"pinned source tree entry is invalid: {exc}")
' "$component" "$parent_tree" <<<"$tree_json")"
    read -r entry_type entry_mode entry_sha < <(
      python3 -c 'import json,sys; v=json.load(sys.stdin); print(v["type"],v["mode"],v["sha"])' <<<"$entry_json"
    )
    [[ "$entry_sha" =~ ^[0-9a-f]{40}$ ]] || {
      echo "pinned source tree contains a malformed object ID" >&2
      exit 1
    }
    if (( index + 1 < ${#components[@]} )); then
      [[ "$entry_type" == "tree" && "$entry_mode" == "040000" ]] || {
        echo "pinned source path has a non-directory component: $source_path" >&2
        exit 1
      }
      parent_tree="$entry_sha"
    else
      [[ "$entry_type" == "blob" && ( "$entry_mode" == "100644" || "$entry_mode" == "100755" ) ]] || {
        echo "pinned source path is not a regular file: $source_path" >&2
        exit 1
      }
      final_mode="$entry_mode"
      final_blob="$entry_sha"
    fi
  done

  blob_json="$(gh api "repos/$canonical_repo/git/blobs/$final_blob")" || {
    echo "retirement launcher could not read pinned source blob $source_path" >&2
    exit 1
  }
  destination="$tmp_root/$source_path"
  mkdir -p -- "$(dirname -- "$destination")"
  python3 -c '
import base64, hashlib, json, pathlib, sys
expected_oid, expected_mode, destination = sys.argv[1:]
try:
    blob = json.load(sys.stdin)
    if blob.get("encoding") != "base64" or blob.get("sha") != expected_oid:
        raise ValueError("blob identity/encoding differs from pinned tree")
    content = base64.b64decode("".join(blob["content"].split()), validate=True)
except (KeyError, TypeError, ValueError, json.JSONDecodeError) as exc:
    raise SystemExit(f"pinned source blob is invalid: {exc}")
actual_oid = hashlib.sha1(b"blob " + str(len(content)).encode("ascii") + b"\0" + content).hexdigest()
if actual_oid != expected_oid:
    raise SystemExit("pinned source blob content does not match its Git object ID")
path = pathlib.Path(destination)
path.write_bytes(content)
path.chmod(0o755 if expected_mode == "100755" else 0o644)
' "$final_blob" "$final_mode" "$destination" <<<"$blob_json"
  manifest_entries="$(python3 -c '
import json, sys
entries = json.loads(sys.argv[1])
entries.append({"path": sys.argv[2], "mode": sys.argv[3], "blob_oid": sys.argv[4]})
print(json.dumps(entries, ensure_ascii=True, sort_keys=True, separators=(",", ":")))
' "$manifest_entries" "$source_path" "$final_mode" "$final_blob")"
done

launcher_source="${BASH_SOURCE[0]}"
[[ ! -L "$launcher_source" && -f "$launcher_source" ]] || {
  echo "selected retirement launcher must be a regular non-symlink file" >&2
  exit 1
}
cmp -s -- "$launcher_source" "$tmp_root/$launcher_path" || {
  echo "selected launcher bytes differ from the live default-branch closure" >&2
  exit 1
}

authority_file="$tmp_root/tool-authority.json"
python3 -c '
import hashlib, json, pathlib, sys
default_branch, commit_oid, tree_oid, manifest_json, destination = sys.argv[1:]
manifest = json.loads(manifest_json)
manifest.sort(key=lambda entry: entry["path"].encode("ascii"))
preimage = json.dumps(manifest, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode("utf-8")
authority = {
    "schema": "oasis7.retirement-tool-authority/v1",
    "canonical_repository": "eng-cc/oasis7",
    "default_branch": default_branch,
    "commit_oid": commit_oid,
    "tree_oid": tree_oid,
    "launcher_path": "scripts/pm/retire-closed-duplicate-candidate.sh",
    "helper_path": "scripts/pm/retire-closed-duplicate-candidate.py",
    "closure_manifest": manifest,
    "closure_sha256": hashlib.sha256(preimage).hexdigest(),
}
path = pathlib.Path(destination)
path.write_text(json.dumps(authority, ensure_ascii=True, sort_keys=True, separators=(",", ":")), encoding="utf-8")
path.chmod(0o600)
' "$default_branch" "$commit_oid" "$tree_oid" "$manifest_entries" "$authority_file"

export OASIS7_RETIREMENT_LAUNCH_CONTEXT_V1="pinned-live-source"
export OASIS7_RETIREMENT_TOOL_ROOT_V1="$tmp_root"
export OASIS7_RETIREMENT_AUTHORITY_FILE_V1="$authority_file"
python3 -I "$tmp_root/$helper_path" "$@"
