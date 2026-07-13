#!/usr/bin/env bash
set -euo pipefail

[[ "${1:-}" == --isolation-root && -n "${2:-}" && "${3:-}" == --fault && -n "${4:-}" && "${5:-}" == -- ]] || {
  echo "usage: $0 --isolation-root <temporary-root> --fault <fixed-point> -- <production cleanup command>" >&2
  exit 2
}
ISOLATION_ROOT="$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).resolve())' "$2")"
FAULT="$4"; shift 5
case "$FAULT" in
  TPM_CLEANUP_FAULT_AFTER_WORKTREE_REMOVE) EXIT_CODE=86 ;;
  TPM_CLEANUP_FAULT_AFTER_BRANCH_DELETE) EXIT_CODE=87 ;;
  *) echo "unsupported isolated fault: $FAULT" >&2; exit 2 ;;
esac

OUTPUT=""
ARGS=("$@")
REPO="" WORKTREE=""
for ((i=0; i<${#ARGS[@]}; i++)); do
  if [[ "${ARGS[$i]}" == --terminal-receipt-output ]]; then OUTPUT="${ARGS[$((i+1))]:-}"; fi
  if [[ "${ARGS[$i]}" == --repo-root ]]; then REPO="${ARGS[$((i+1))]:-}"; fi
  if [[ "${ARGS[$i]}" == --worktree ]]; then WORKTREE="${ARGS[$((i+1))]:-}"; fi
done
[[ -n "$OUTPUT" && "$OUTPUT" == /* ]] || { echo "isolated fault fixture requires absolute terminal receipt output" >&2; exit 2; }
python3 - "$ISOLATION_ROOT" "$OUTPUT" <<'PY'
import pathlib,subprocess,sys,tempfile
root=pathlib.Path(sys.argv[1]).resolve(); temp=pathlib.Path(tempfile.gettempdir()).resolve()
try: root.relative_to(temp)
except ValueError: raise SystemExit('fault fixture isolation root is outside the system temporary boundary')
paths=[pathlib.Path(v).resolve() for v in sys.argv[2:]]
for path in paths:
 try: path.relative_to(root)
 except ValueError: raise SystemExit(f'fault fixture path is outside isolation boundary: {path}')
PY
FIXTURE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PRODUCTION_CLEANUP="$(cd "$FIXTURE_DIR/.." && pwd)/post-merge-cleanup.sh"
[[ "${ARGS[0]:-}" == "$PRODUCTION_CLEANUP" ]] || { echo "fixture accepts only the fixed production cleanup executable" >&2; exit 2; }
python3 - "$ISOLATION_ROOT" "$REPO" "$WORKTREE" <<'PY'
import pathlib,subprocess,sys
root=pathlib.Path(sys.argv[1]).resolve(); paths=[pathlib.Path(v).resolve() for v in sys.argv[2:]]
for path in paths:
 try: path.relative_to(root)
 except ValueError: raise SystemExit(f'fault fixture repository/worktree is outside isolation boundary: {path}')
repo,worktree=paths
if not repo.is_dir() or not worktree.is_dir(): raise SystemExit('fault fixture repository/worktree must exist inside isolation root')
def common(path):
 raw=subprocess.check_output(['git','-C',str(path),'rev-parse','--git-common-dir'],text=True).strip()
 p=pathlib.Path(raw); return (p if p.is_absolute() else path/p).resolve()
if common(repo)!=common(worktree): raise SystemExit('fault fixture repository and worktree have different git common-dir')
PY

REAL_GIT="$(command -v git)"; SHIM_DIR="$(mktemp -d)"
trap 'rm -rf "$SHIM_DIR"' EXIT
cat >"$SHIM_DIR/git" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
"$CLEANUP_REAL_GIT" "$@"
matched=0
case "$CLEANUP_FAULT:$*" in
  TPM_CLEANUP_FAULT_AFTER_WORKTREE_REMOVE:*worktree\ remove*) matched=1 ;;
  TPM_CLEANUP_FAULT_AFTER_BRANCH_DELETE:*branch\ -d*) matched=1 ;;
esac
if [[ "$matched" == 1 ]]; then
  python3 - "$CLEANUP_INTENT" "$CLEANUP_FAULT" <<'PY'
import json,sys
p=sys.argv[1]; d=json.load(open(p,encoding='utf-8'))
if sys.argv[2].endswith('WORKTREE_REMOVE'): d['worktree_removed']=True
else: d['branch_deleted']=True
json.dump(d,open(p,'w',encoding='utf-8'),indent=2,sort_keys=True)
PY
  exit "$CLEANUP_EXIT_CODE"
fi
SH
chmod +x "$SHIM_DIR/git"
PATH="$SHIM_DIR:$PATH" CLEANUP_REAL_GIT="$REAL_GIT" CLEANUP_FAULT="$FAULT" \
  CLEANUP_EXIT_CODE="$EXIT_CODE" CLEANUP_INTENT="$(dirname "$OUTPUT")/cleanup-intent.json" "${ARGS[@]}"
