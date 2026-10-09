#!/usr/bin/env python3
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
spec = importlib.util.spec_from_file_location("source_plan", Path(__file__).with_name("package-source-plan.py"))
plan = importlib.util.module_from_spec(spec)
spec.loader.exec_module(plan)
with tempfile.TemporaryDirectory() as temporary:
    root=Path(temporary)
    def git(*args): return subprocess.check_output(["git", *args],text=True).strip()
    old=os.getcwd()
    os.chdir(root)
    try:
        git("init", "-q")
        (root / "a").write_text("fixture")
        git("add", ".")
        git("-c", "user.name=Test", "-c", "user.email=test@example.test", "commit", "-qm", "fixture")
        sha=git("rev-parse", "HEAD")
        git("update-ref", "refs/remotes/origin/main", sha)
        git("tag", "v1.2.3")
        git("-c", "user.name=Test", "-c", "user.email=test@example.test", "tag", "-am", "annotated", "v1.2.4")
        for ref in ("v1.2.3", "refs/tags/v1.2.4"):
            result=plan.resolve(ref, True)
            assert result["checkoutSha"]==sha
        git("update-ref", "refs/remotes/origin/v1.2.3", sha)
        try: plan.resolve("v1.2.3")
        except ValueError: pass
        else: raise AssertionError("ambiguous ref accepted")
        assert plan.resolve("refs/heads/main")["checkoutSha"]==sha
        assert plan.resolve(sha)["checkoutSha"]==sha
        strange="literal$(touch${IFS}pwned)"
        git("update-ref", "refs/remotes/origin/" + strange, sha)
        assert plan.resolve(strange)["checkoutSha"] == sha
        assert not (root / "pwned").exists()
        for bad in ("v1.2.3\nextra=x", "v1.2.3;touch pwn", "--help", "v1abc", "refs/tags/v1.2.3\x00"):
            try: plan.resolve(bad, True)
            except ValueError: pass
            else: raise AssertionError(bad)
        git("checkout", "-q", "--orphan", "outside-protected-history")
        git("-c", "user.name=Test", "-c", "user.email=test@example.test", "commit", "-qm", "outside")
        git("tag", "v9.9.9")
        try: plan.resolve("v9.9.9", True)
        except ValueError: pass
        else: raise AssertionError("release outside protected history accepted")
    finally: os.chdir(old)
print("ok: immutable source plan tags, commits, branches, ambiguity, input injection")

import re
repo = Path(__file__).resolve().parent.parent
for path in (repo / ".github/workflows").glob("*.yml"):
    workflow = path.read_text()
    assert "\npermissions:\n" in workflow, path
    for action in re.findall(r"uses: ([^\s]+)", workflow):
        if not action.startswith("./"):
            assert re.fullmatch(r"[\w/-]+@[0-9a-f]{40}", action), (path, action)
for name in ("mainnet-packages", "testnet-packages", "release-packages"):
    workflow = (repo / ".github/workflows" / (name + ".yml")).read_text()
    blocks = re.split(r"(?=^  [\w-]+:\n)", workflow, flags=re.M)
    for block in blocks:
        if "ref: ${{ needs.plan.outputs.checkoutSha }}" in block:
            assert "Verify planned source HEAD" in block
            assert "persist-credentials: false" in block
            assert "plan" in re.search(r"    needs:[^\n]*(?:\n      - [^\n]+)*", block)[0]
    assert "SOURCE_PLAN: ${{ needs.plan.outputs.sourcePlan }}" in workflow
    assert "trusted-v2" in workflow
    if name != "release-packages":
        assert "cache-mode: read" in workflow
        for cache in re.findall(r"uses: Swatinem/rust-cache@[^\n]+\n(.*?)(?=      - |\Z)", workflow, re.S):
            assert "save-if: false" in cache
release = (repo / ".github/workflows/release-packages.yml").read_text()
assert 'test "$object" = "$PLANNED_SHA"' in release
assert "cache-mode: none" in release
assert 'while [[ "$kind" == tag ]]' in release
print("ok: every current workflow action pinned, package SHA consumers and cache policies")

# Execute the trusted workflow publication check against a fake GitHub API.
import json
import shutil
check = re.search(r"      - name: Verify release tag still matches plan\n(.*?)(?=      - name: Publish GitHub release)", release, re.S)[1]
script = check.split("        run: |\n", 1)[1]
script = "\n".join(line[10:] for line in script.splitlines())
with tempfile.TemporaryDirectory() as temporary:
    root=Path(temporary)
    tool=root / "gh"
    tool.write_text("#!/usr/bin/env python3\nimport json,os,sys\nkind=os.environ['MOCK_KIND']\nsha=os.environ['MOCK_SHA']\nprint(json.dumps({'object': {'type': 'commit' if '/git/tags/' in sys.argv[2] else kind, 'sha': sha if kind == 'commit' or '/git/tags/' in sys.argv[2] else 'b'*40}}))\n")
    tool.chmod(0o755)
    (root / "output/release/assets").mkdir(parents=True)
    for platform in ("linux-x64", "macos-x64", "windows-x64"):
        (root / "output/release/assets" / (platform+"-SOURCEPLAN.json")).write_text(json.dumps({"checkoutSha": "a"*40}))
        (root / "output/release/assets" / (platform+"-BUILDINFO")).write_text("commit=" + "a"*40 + "\n")
    for kind in ("commit", "tag"):
        for moved in (False, True):
            environment=dict(os.environ, PATH=str(root)+os.pathsep+os.environ['PATH'],
                PLANNED_REF="refs/tags/v1.2.3", PLANNED_SHA="a"*40,
                GITHUB_REPOSITORY="example/repo", GITHUB_OUTPUT=str(root / "github-output"),
                MOCK_KIND=kind, MOCK_SHA=("c" if moved else "a")*40)
            result=subprocess.run(["bash", "-c", script], env=environment, cwd=root, capture_output=True, text=True)
            assert (result.returncode == 0) != moved, (kind, moved, result.stderr)
    (root / "output/release/assets/linux-x64-SOURCEPLAN.json").write_text(json.dumps({"checkoutSha": "c"*40}))
    environment["MOCK_SHA"]="a"*40
    assert subprocess.run(["bash", "-c", script], env=environment, cwd=root, capture_output=True).returncode != 0
print("ok: publication rejects lightweight/annotated tag drift and mixed artifact SHA")
