#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
pages_workflow="$repo_root/.github/workflows/pages.yml"
required_workflow="$repo_root/.github/workflows/rust.yml"
ci_tests="$repo_root/scripts/ci-tests.sh"

site_checks=(
  scripts/site-link-check.sh
  scripts/site-homepage-claim-check.sh
  scripts/site-manual-sync-check.sh
  scripts/site-download-check.sh
)

if grep -Fq '  push:' "$pages_workflow"; then
  echo "Pages workflow must not deploy automatically from a push" >&2
  exit 1
fi
if ! grep -Fq '  workflow_dispatch:' "$pages_workflow"; then
  echo "Pages workflow must remain manually dispatchable" >&2
  exit 1
fi

for check in "${site_checks[@]}"; do
  if [[ ! -x "$repo_root/$check" ]]; then
    echo "site quality check is missing or not executable: $check" >&2
    exit 1
  fi
  if ! grep -Fq "./$check" "$pages_workflow"; then
    echo "Pages workflow does not run site quality check: $check" >&2
    exit 1
  fi
  if ! grep -Fq "  run ./$check" "$ci_tests"; then
    echo "ci-tests site selector does not run site quality check: $check" >&2
    exit 1
  fi
done

python3 - "$ci_tests" "$required_workflow" "$repo_root/scripts/ci-required-scope.json" <<'PYTHON'
import json
from pathlib import Path
import re
import sys

driver, workflow = (Path(path).read_text() for path in sys.argv[1:3])
config = json.loads(Path(sys.argv[3]).read_text())
if not re.search(r'^\s*site_quality\)\s+run_site_contract_tests\s*;;', driver, re.M):
    raise SystemExit("site quality group does not execute its product checks")
def validate_execution(workflow):
    jobs = dict(re.findall(r'^  ([a-z][a-z0-9-]*):\n(.*?)(?=^  [a-z][a-z0-9-]*:|\Z)',
                           workflow.split('\njobs:\n', 1)[1], re.M | re.S))
    job = jobs.get('site-quality', '')
    for expected in ('needs: select', "needs.select.result == 'success'",
                     "needs.select.outputs.run_site_quality == 'true'",
                     "fromJSON(needs.select.outputs.plan).scope == 'full'",
                     'bash ./scripts/ci-tests.sh required --group "site_quality" --repo-root "$GITHUB_WORKSPACE"'):
        if expected not in job:
            raise ValueError("site quality job is missing selected group execution: " + expected)
    if 'strategy:' in job or 'ci-authority' in job or 'matrix.group' in job:
        raise ValueError("site quality job must execute the candidate group without a singleton matrix")
    needs = re.search(r'    needs: \[(.*)\]', jobs.get('required-gate', ''))
    if not needs or 'site-quality' not in needs.group(1).split(', '):
        raise ValueError("final required-gate omits site quality results")

try:
    validate_execution(workflow)
except ValueError as exc:
    raise SystemExit(str(exc))

# Isolated mutations must fail: route, selection and failure aggregation all matter.
for changed in (
        workflow.replace("needs.select.result == 'success'", 'true'),
        workflow.replace("fromJSON(needs.select.outputs.plan).scope == 'full'", 'false'),
        workflow.replace('bash ./scripts/ci-tests.sh required --group "site_quality"',
                         'bash "$RUNNER_TEMP/ci-authority/ci-tests.sh" required --group "site_quality"'),
        workflow.replace(', site-quality,', ', ')):
    try:
        validate_execution(changed)
    except ValueError:
        continue
    raise SystemExit("site quality contract accepted a broken execution fixture")
if 'site_quality' not in config['groups'] or not any(
        'site_quality' in rule.get('groups', []) and 'site/**' in rule['match']
        for rule in config['rules']):
    raise SystemExit("site directory changes do not select the site quality group")
PYTHON

# Isolated manual fixtures exercise the checker without starting a browser,
# serving a build, or executing any example command.
python3 - "$repo_root" <<'PYTHON'
import difflib
import html
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

repo = Path(sys.argv[1])
manuals = (
    "doc/world-simulator/viewer/viewer-manual.manual.md",
    "site/doc/cn/viewer-manual.html",
    "site/doc/en/viewer-manual.html",
)
originals = {name: (repo / name).read_text() for name in manuals}
mutations = (
    ("default must be release", '${VIEWER_CAPTURE_MODE:-release}', '${VIEWER_CAPTURE_MODE:-visual-test}'),
    ("release port", 'release) VIEWER_CAPTURE_PORT=4173 ;;', 'release) VIEWER_CAPTURE_PORT=4174 ;;'),
    ("test port", 'visual-test) VIEWER_CAPTURE_PORT=4174 ;;', 'visual-test) VIEWER_CAPTURE_PORT=4173 ;;'),
    ("invalid mode rejection", 'exit 2 ;;', 'exit 0 ;;'),
    ("profile-aware URL", '${VIEWER_CAPTURE_PORT}/viewer.html', '4173/'),
    ("explicit test build", 'run build:viewer:visual-test', 'run build:viewer'),
    ("isolated test directory", '--directory crates/oasis7_viewer/.viewer-test-dist', '--directory crates/oasis7_viewer/dist'),
    ("test build must succeed before serve", 'visual-test && \\\n', 'visual-test ; \\\n'),
    ("release API must be absent", "typeof window.__AW_TEST__ === 'undefined'", "typeof window.__AW_TEST__ === 'object'"),
    ("test API must be present", "typeof window.__AW_TEST__ === 'object'", "typeof window.__AW_TEST__ === 'undefined'"),
    ("state capture stays guarded", 'if [[ "$VIEWER_CAPTURE_MODE" == "visual-test" ]]; then\n  agent-browser --session "$AB_SESSION" eval', 'if true; then\n  agent-browser --session "$AB_SESSION" eval'),
    ("owned-session close", 'agent-browser --session "$AB_SESSION" close', 'agent-browser close'),
    ("primary failure preserved", 'exit "$main_status"', 'exit 0'),
    ("close failure preserved", 'exit "$close_status"', 'exit 0'),
    ("cleanup trap", 'trap cleanup EXIT', 'trap : EXIT'),
    ("cleanup status artifact", '"$VIEWER_EVIDENCE_DIR/cleanup.status"', '"$VIEWER_EVIDENCE_DIR/ignored.status"'),
    ("no global close", 'trap cleanup EXIT', 'agent-browser close-all\ntrap cleanup EXIT'),
    ("no unscoped open", 'trap cleanup EXIT', 'agent-browser --headed open "http://127.0.0.1:4174/viewer.html"\ntrap cleanup EXIT'),
    ("no networkidle", 'trap cleanup EXIT', 'agent-browser wait --load networkidle\ntrap cleanup EXIT'),
    ("no unguarded test API", 'trap cleanup EXIT', 'agent-browser --session "$AB_SESSION" eval "window.__AW_TEST__.getState()"\ntrap cleanup EXIT'),
    ("commented session creation", 'AB_SESSION="$(agent-browser session id', '# AB_SESSION="$(agent-browser session id'),
    ("commented session export", 'export AB_SESSION', '# export AB_SESSION'),
    ("commented open", 'agent-browser --session "$AB_SESSION" --headed open', '# agent-browser --session "$AB_SESSION" --headed open'),
    ("commented trap", 'trap cleanup EXIT', '# trap cleanup EXIT'),
    ("scoped networkidle with comment bait", 'agent-browser --session "$AB_SESSION" wait --load domcontentloaded', 'agent-browser --session "$AB_SESSION" wait --load networkidle\n# agent-browser --session "$AB_SESSION" wait --load domcontentloaded'),
    ("scoped global close", 'trap cleanup EXIT', 'agent-browser --session "$AB_SESSION" close-all\ntrap cleanup EXIT'),
    ("wrong API branch", 'if [[ "$VIEWER_CAPTURE_MODE" == "visual-test" ]]; then', 'if [[ "$VIEWER_CAPTURE_MODE" == "release" ]]; then'),
    ("dead outer branch", 'set -euo pipefail', 'if false; then\nset -euo pipefail'),
    ("here-doc wrapper", 'set -euo pipefail', "cat <<'CAPTURE'\nset -euo pipefail"),
)

with tempfile.TemporaryDirectory(prefix="site-manual-contract-") as directory:
    root = Path(directory)
    (root / "scripts").mkdir()
    checker = root / "scripts/site-manual-sync-check.sh"
    shutil.copyfile(repo / "scripts/site-manual-sync-check.sh", checker)
    for name, content in originals.items():
        target = root / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content)

    def check(label, expected):
        result = subprocess.run(["bash", str(checker)], text=True, capture_output=True)
        if (result.returncode == 0) != expected:
            raise SystemExit(f"manual checker fixture failed: {label}\n{result.stdout}{result.stderr}")
        if not expected and "error:" not in result.stderr:
            raise SystemExit(f"manual checker failed without its own diagnostic: {label}\n{result.stderr}")

    check("current source and both mirrors", True)
    count = 0
    positives = 2
    for name, original in originals.items():
        target = root / name
        # A harmless mutation through the same encoding path must still pass;
        # otherwise mirror encoding could make every negative a false positive.
        cases = (("harmless comment", 'set -euo pipefail', '# capture comment\nset -euo pipefail'),) + mutations
        for label, before, after in cases:
            # Mutate only the chosen executable recipe, never an unrelated
            # example or prose occurrence earlier in the same document.
            pattern = r"<pre><code>(.*?)</code></pre>" if name.endswith(".html") else r"```bash\n(.*?)```"
            kind = 'run build:viewer:visual-test' if label in (
                'explicit test build', 'isolated test directory',
                'test build must succeed before serve') else 'VIEWER_CAPTURE_MODE='
            matches = [match for match in re.finditer(pattern, original, re.S)
                       if kind in html.unescape(match.group(1))]
            if len(matches) != 1:
                raise SystemExit(f"ambiguous mutation recipe: {name}: {label}")
            match = matches[0]
            block = html.unescape(match.group(1)) if name.endswith(".html") else match.group(1)
            if before not in block:
                raise SystemExit(f"mutation anchor missing: {name}: {label}")
            changed = block.replace(before, after, 1)
            if label == 'dead outer branch':
                changed += '\nfi\n'
            elif label == 'here-doc wrapper':
                changed += '\nCAPTURE\n'
            if name.endswith(".html"):
                # Preserve each unchanged character's original HTML spelling:
                # these mirrors mix literal shell operators and entities.
                raw = match.group(1)
                pieces = re.findall(r'&(?:#\d+|#x[0-9a-fA-F]+|[A-Za-z]+);|.', raw, re.S)
                offsets = [0]
                for piece in pieces:
                    if len(html.unescape(piece)) != 1:
                        raise SystemExit("unsupported fixture HTML entity")
                    offsets.append(offsets[-1] + len(piece))
                encoded = []
                for tag, a, b, c, d in difflib.SequenceMatcher(None, block, changed, autojunk=False).get_opcodes():
                    encoded.append(raw[offsets[a]:offsets[b]] if tag == 'equal'
                                   else html.escape(changed[c:d], quote=False))
                changed = ''.join(encoded)
            target.write_text(original[:match.start(1)] + changed + original[match.end(1):])
            positive = label == "harmless comment"
            check(f"{name}: {label}", positive)
            target.write_text(original)
            if positive:
                positives += 1
            else:
                count += 1
        if name.endswith(".html"):
            target.write_text(original + '\n<p>doc/world-simulator/viewer-auto-focus-capture.md</p>\n')
            check(f"{name}: retired reference", False)
            target.write_text(original)
            count += 1
    check("restored baseline", True)
print(f"manual profile contract: {positives} positive fixtures and {count} independent negative fixtures passed")
PYTHON

echo "site checks contract: passed"
