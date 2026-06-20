#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shlex
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT_DIR = Path(__file__).resolve().parents[1]
WINDOWS_GOVERNED_BUNDLE = (
    r"C:\oasis7-deploy\config\public-testnet-governed-bootstrap-bundle-2026-06-06.windows.json"
)


def die(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def shell_join(args: list[str]) -> str:
    return " ".join(shlex.quote(arg) for arg in args)


def read_buildinfo(path: Path) -> dict[str, str]:
    info: dict[str, str] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        if not raw or raw.startswith("#"):
            continue
        key, sep, value = raw.partition("=")
        if sep:
            info[key.strip()] = value.strip()
    return info


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_sha256sums(package_dir: Path, sums_path: Path) -> list[str]:
    verified: list[str] = []
    for raw in sums_path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line:
            continue
        expected, _, name = line.partition("  ")
        if not name:
            parts = line.split(maxsplit=1)
            if len(parts) != 2:
                die(f"cannot parse checksum line in {sums_path}: {raw}")
            expected, name = parts
        rel_name = name.lstrip("*")
        target = package_dir / rel_name
        if not target.is_file():
            die(f"checksum target missing: {target}")
        actual = sha256_file(target)
        if actual.lower() != expected.lower():
            die(f"checksum mismatch for {target}: expected {expected}, got {actual}")
        verified.append(rel_name)
    return verified


def find_platform_dir(package_dir: Path, platform: str) -> Path:
    buildinfo = package_dir / f"{platform}-BUILDINFO"
    if buildinfo.is_file():
        return package_dir
    matches = sorted(path.parent for path in package_dir.rglob(f"{platform}-BUILDINFO"))
    if not matches:
        die(f"cannot find {platform}-BUILDINFO under {package_dir}")
    if len(matches) > 1:
        die(f"multiple {platform}-BUILDINFO files under {package_dir}: {matches}")
    return matches[0]


def platform_asset(platform_dir: Path, platform: str) -> Path:
    names = {
        "linux-x64": "oasis7-linux-x64-bundle.tar.gz",
        "windows-x64": "oasis7-windows-x64.exe",
        "macos-x64": "oasis7-macos-x64.dmg",
    }
    name = names.get(platform)
    if not name:
        die(f"unsupported platform: {platform}")
    asset = platform_dir / name
    if not asset.is_file():
        die(f"missing {platform} asset: {asset}")
    return asset


def require_verified_files(platform: str, platform_dir: Path, buildinfo: Path, asset: Path, verified: list[str]) -> None:
    verified_set = set(verified)
    required = [
        buildinfo.relative_to(platform_dir).as_posix(),
        asset.relative_to(platform_dir).as_posix(),
    ]
    for rel_name in required:
        if rel_name not in verified_set:
            die(f"{platform} checksum file does not cover required artifact: {rel_name}")


def require_same_build(platform_infos: dict[str, dict[str, str]]) -> dict[str, str]:
    required = ("package_version", "commit", "run_id")
    first_platform = next(iter(platform_infos))
    expected = {key: platform_infos[first_platform].get(key, "") for key in required}
    for key, value in expected.items():
        if not value:
            die(f"{first_platform} BUILDINFO missing {key}")
    for platform, info in platform_infos.items():
        for key, expected_value in expected.items():
            actual = info.get(key, "")
            if actual != expected_value:
                die(
                    f"{platform} BUILDINFO {key}={actual!r} does not match "
                    f"{first_platform} {key}={expected_value!r}"
                )
    return expected


def load_manifest(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        die("manifest must be a JSON object")
    nodes = data.get("nodes")
    if not isinstance(nodes, list) or not nodes:
        die("manifest must contain a non-empty nodes array")
    return data


def artifact_ref(platform: str, version: str, asset_name: str, runtime_name: str) -> str:
    return f"testnet-package-{platform}-{version}/{asset_name}!/bin/{runtime_name}"


def linux_command(
    node: dict[str, Any],
    linux_asset: Path,
    version: str,
    commit: str,
    run_id: str,
    readiness_policy: str,
    bundle_tar: str | None = None,
    script_path: str | None = None,
) -> list[str]:
    node_root = str(node.get("node_root") or "")
    if not node_root:
        die(f"linux node {node.get('name', '<unnamed>')} missing node_root")
    command = [
        script_path or str(ROOT_DIR / "scripts" / "p2p-public-testnet-package-node-upgrade.sh"),
        "--node-root",
        node_root,
        "--bundle-tar",
        bundle_tar or str(linux_asset),
        "--package-version",
        version,
        "--commit",
        commit,
        "--run-id",
        run_id,
        "--artifact-ref",
        artifact_ref("linux-x64", version, linux_asset.name, "oasis7_chain_runtime"),
    ]
    service = str(node.get("systemd_service") or "")
    if node.get("restart", False):
        if not service:
            die(f"linux node {node.get('name', '<unnamed>')} has restart=true but no systemd_service")
        command.extend(["--systemd-service", service, "--restart-service"])
        status_url = str(node.get("status_url") or "")
        if readiness_policy == "strict-ready" and not status_url:
            die(f"linux node {node.get('name', '<unnamed>')} uses strict-ready but has no status_url")
        if readiness_policy == "strict-ready":
            command.extend(["--post-restart-status-url", status_url])
            timeout_secs = str(node.get("post_restart_timeout_secs") or 120)
            command.extend(["--post-restart-timeout-secs", timeout_secs])
    return command


def linux_plan_commands(
    node: dict[str, Any],
    linux_asset: Path,
    version: str,
    commit: str,
    run_id: str,
    readiness_policy: str,
) -> list[str]:
    host = str(node.get("host") or "")
    if not host:
        return [shell_join(linux_command(node, linux_asset, version, commit, run_id, readiness_policy))]
    user = str(node.get("user") or "root")
    remote_bundle = str(node.get("remote_bundle") or linux_asset.name)
    remote_script = str(node.get("remote_script") or "./scripts/p2p-public-testnet-package-node-upgrade.sh")
    remote_command = linux_command(
        node,
        linux_asset,
        version,
        commit,
        run_id,
        readiness_policy,
        bundle_tar=remote_bundle,
        script_path=remote_script,
    )
    return [
        shell_join(["scp", str(linux_asset), f"{user}@{host}:{remote_bundle}"]),
        shell_join(["ssh", f"{user}@{host}", shell_join(remote_command)]),
    ]


def windows_script(
    node: dict[str, Any],
    installer_name: str,
    version: str,
    commit: str,
    run_id: str,
) -> str:
    deploy_root = str(node.get("deploy_root") or r"C:\oasis7-deploy")
    task_name = str(node.get("scheduled_task") or "Oasis7Observer")
    status_url = str(node.get("status_url") or "")
    install_root = str(node.get("install_root") or "$env:LOCALAPPDATA\\Programs\\oasis7")
    installer_path = str(node.get("installer_path") or f"$env:USERPROFILE\\{installer_name}")
    governed_bundle_path = str(node.get("governed_bundle_path") or WINDOWS_GOVERNED_BUNDLE)
    ref = artifact_ref("windows-x64", version, installer_name, "oasis7_chain_runtime.exe")
    status_block = (
        f"Invoke-RestMethod -Uri '{status_url}' -TimeoutSec 8 | "
        "Select-Object running,last_error,readiness,consensus | ConvertTo-Json -Compress -Depth 8"
        if status_url
        else "Write-Output 'status_check=skipped'"
    )
    return f"""$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Set-JsonProperty {{
  param(
    [Parameter(Mandatory = $true)] [object] $Object,
    [Parameter(Mandatory = $true)] [string] $Name,
    [Parameter(Mandatory = $true)] $Value
  )
  if ($Object.PSObject.Properties.Name -contains $Name) {{
    $Object.$Name = $Value
  }} else {{
    $Object | Add-Member -NotePropertyName $Name -NotePropertyValue $Value
  }}
}}

$version = '{version}'
$commit = '{commit}'
$runId = '{run_id}'
$artifactRef = '{ref}'
$installRoot = "{install_root}"
$runtime = Join-Path $installRoot 'bin\\oasis7_chain_runtime.exe'
$installer = "{installer_path}"
$deployRoot = '{deploy_root}'
$taskName = '{task_name}'
$bundlePath = '{governed_bundle_path}'

$bundle = Get-Item $bundlePath -ErrorAction Stop
$json = Get-Content $bundle.FullName -Raw | ConvertFrom-Json
if ($null -eq $json.runtime_build) {{
  throw "governed bundle missing runtime_build: $($bundle.FullName)"
}}

$oldHash = if (Test-Path $runtime) {{
  (Get-FileHash $runtime -Algorithm SHA256).Hash.ToLowerInvariant()
}} else {{
  'missing'
}}
Write-Output "old_runtime_sha256=$oldHash"

Stop-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
Get-Process oasis7_chain_runtime -ErrorAction SilentlyContinue |
  Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 3

$install = Start-Process -FilePath $installer -ArgumentList '/S' -Wait -PassThru
Write-Output "installer_exit_code=$($install.ExitCode)"
if ($install.ExitCode -ne 0) {{
  throw "installer failed with exit code $($install.ExitCode)"
}}
if (!(Test-Path $runtime)) {{
  throw "runtime missing after install: $runtime"
}}

$hash = (Get-FileHash $runtime -Algorithm SHA256).Hash.ToLowerInvariant()
$size = (Get-Item $runtime).Length
Write-Output "new_runtime_sha256=$hash"
Write-Output "new_runtime_size=$size"

Set-JsonProperty $json.runtime_build 'git_commit' $commit
Set-JsonProperty $json.runtime_build 'kind' 'file'
Set-JsonProperty $json.runtime_build 'path' $runtime
Set-JsonProperty $json.runtime_build 'resolved_path' $runtime
Set-JsonProperty $json.runtime_build 'ref' $artifactRef
Set-JsonProperty $json.runtime_build 'sha256' $hash
Set-JsonProperty $json.runtime_build 'size_bytes' $size
Set-JsonProperty $json.runtime_build 'updated_by' "windows package upgrade $version (run $runId, commit $commit)"
Set-JsonProperty $json 'git_commit' $commit
Set-JsonProperty $json 'updated_by' "windows package upgrade $version (run $runId, commit $commit)"
$jsonText = $json | ConvertTo-Json -Depth 100
[System.IO.File]::WriteAllText(
  $bundle.FullName,
  $jsonText + [Environment]::NewLine,
  [System.Text.UTF8Encoding]::new($false)
)
$updated = 1

Set-Content -Encoding UTF8 (Join-Path $deployRoot 'CURRENT_VERSION') $version
@(
  'workflow=Testnet Packages',
  "run_id=$runId",
  'repository=eng-cc/oasis7',
  "commit=$commit",
  "package_version=$version",
  'platform=windows-x64',
  "runtime_sha256=$hash",
  "runtime_size=$size"
) | Set-Content -Encoding UTF8 (Join-Path $deployRoot 'DEPLOYED_BUILDINFO')
Write-Output "updated_bundle_count=$updated"

Start-ScheduledTask -TaskName $taskName
Start-Sleep -Seconds 10
Get-Process oasis7_chain_runtime -ErrorAction SilentlyContinue |
  Select-Object -First 1 Id,Path |
  ConvertTo-Json -Compress
Get-ScheduledTask -TaskName $taskName |
  Select-Object TaskName,State |
  ConvertTo-Json -Compress
try {{
  {status_block}
}} catch {{
  Write-Output "STATUS_ERROR=$($_.Exception.Message)"
}}
"""


def write_windows_plan(
    out_dir: Path,
    node: dict[str, Any],
    windows_asset: Path,
    version: str,
    commit: str,
    run_id: str,
) -> tuple[Path, list[str]]:
    name = str(node.get("name") or "windows-node")
    safe_name = "".join(ch if ch.isalnum() or ch in "._-" else "-" for ch in name)
    script_path = out_dir / f"{safe_name}-windows-upgrade.ps1"
    host = str(node.get("host") or "")
    user = str(node.get("user") or "Administrator")
    remote_script = str(node.get("remote_script") or f"{safe_name}-windows-upgrade.ps1")
    remote_installer = str(node.get("remote_installer") or windows_asset.name)
    script_node = dict(node)
    if host and not script_node.get("installer_path"):
        script_node["installer_path"] = remote_installer
    script_text = windows_script(script_node, windows_asset.name, version, commit, run_id)
    script_path.write_text(script_text, encoding="utf-8")
    # Rewrite without BOM explicitly; Windows PowerShell accepts this and the runtime JSON writer also uses no-BOM.
    script_path.write_bytes(script_text.encode("utf-8"))
    commands: list[str] = []
    if host:
        commands.append(shell_join(["scp", str(windows_asset), f"{user}@{host}:{remote_installer}"]))
        commands.append(shell_join(["scp", str(script_path), f"{user}@{host}:{remote_script}"]))
        commands.append(
            shell_join(
                [
                    "ssh",
                    f"{user}@{host}",
                    f"powershell -NoProfile -ExecutionPolicy Bypass -File {remote_script}",
                ]
            )
        )
    else:
        commands.append(
            shell_join(
                [
                    "powershell",
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    str(script_path),
                ]
            )
        )
    return script_path, commands


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Plan standardized public testnet package version replacements. The default mode is "
            "plan-only and does not mutate nodes. Mutation requires either --apply-local for local "
            "linux-x64 entries or deliberate execution of the generated operator commands/scripts. "
            "The script never reads or stores credentials; remote execution commands are rendered "
            "for the operator's SSH transport."
        )
    )
    parser.add_argument("--manifest", required=True, type=Path, help="JSON node rollout manifest")
    parser.add_argument("--package-dir", required=True, type=Path, help="Downloaded GitHub artifact directory")
    parser.add_argument("--out-dir", type=Path, default=Path(".tmp/testnet-package-rollout"))
    parser.add_argument(
        "--apply-local",
        action="store_true",
        help="Mutate local linux-x64 nodes without a host; omitted means plan-only.",
    )
    parser.add_argument(
        "--readiness-policy",
        choices=("rpc-running", "strict-ready", "degraded-ok"),
        default="rpc-running",
        help=(
            "Post-restart health policy for generated plans. rpc-running keeps replacement "
            "separate from network recovery, strict-ready passes status_url into the Linux "
            "primitive, and degraded-ok records an operator-tolerated degraded rollout."
        ),
    )
    parser.add_argument("--json", action="store_true", help="Print machine-readable rollout plan")
    args = parser.parse_args()

    manifest = load_manifest(args.manifest)
    package_dir = args.package_dir.resolve()
    out_dir = args.out_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    platforms = sorted({str(node.get("platform") or "") for node in manifest["nodes"]})
    if "" in platforms:
        die("all nodes must declare platform")

    platform_dirs: dict[str, Path] = {}
    platform_infos: dict[str, dict[str, str]] = {}
    platform_assets: dict[str, Path] = {}
    verified_files: dict[str, list[str]] = {}
    for platform in platforms:
        platform_dir = find_platform_dir(package_dir, platform)
        platform_dirs[platform] = platform_dir
        buildinfo = platform_dir / f"{platform}-BUILDINFO"
        sums = platform_dir / f"{platform}-SHA256SUMS"
        if not sums.is_file():
            die(f"missing {platform} checksum file: {sums}")
        verified_files[platform] = verify_sha256sums(platform_dir, sums)
        platform_infos[platform] = read_buildinfo(buildinfo)
        platform_assets[platform] = platform_asset(platform_dir, platform)
        require_verified_files(platform, platform_dir, buildinfo, platform_assets[platform], verified_files[platform])

    build = require_same_build(platform_infos)
    version = build["package_version"]
    commit = build["commit"]
    run_id = build["run_id"]

    plan: dict[str, Any] = {
        "package_version": version,
        "commit": commit,
        "run_id": run_id,
        "out_dir": str(out_dir),
        "readiness_policy": args.readiness_policy,
        "verified_files": verified_files,
        "nodes": [],
    }

    for raw_node in manifest["nodes"]:
        if not isinstance(raw_node, dict):
            die("each node manifest entry must be an object")
        node = raw_node
        platform = str(node.get("platform"))
        name = str(node.get("name") or platform)
        node_plan: dict[str, Any] = {
            "name": name,
            "platform": platform,
            "host": node.get("host"),
            "commands": [],
            "applied": False,
        }
        if platform == "linux-x64":
            command = linux_command(
                node,
                platform_assets[platform],
                version,
                commit,
                run_id,
                args.readiness_policy,
            )
            node_plan["commands"].extend(
                linux_plan_commands(node, platform_assets[platform], version, commit, run_id, args.readiness_policy)
            )
            if args.apply_local and not node.get("host"):
                applied = subprocess.run(
                    command,
                    check=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                )
                node_plan["apply_output"] = applied.stdout.strip().splitlines()
                node_plan["applied"] = True
        elif platform == "windows-x64":
            script_path, commands = write_windows_plan(
                out_dir,
                node,
                platform_assets[platform],
                version,
                commit,
                run_id,
            )
            node_plan["windows_script"] = str(script_path)
            node_plan["governed_bundle_path"] = str(node.get("governed_bundle_path") or WINDOWS_GOVERNED_BUNDLE)
            node_plan["commands"].extend(commands)
        elif platform == "macos-x64":
            node_plan["note"] = (
                "macos-x64 packages are installer artifacts only; this rollout helper verifies "
                "the artifact but does not replace a running observer unless a platform-specific "
                "operator script is supplied."
            )
        else:
            die(f"unsupported platform in node {name}: {platform}")
        plan["nodes"].append(node_plan)

    plan_path = out_dir / "rollout-plan.json"
    plan_path.write_text(json.dumps(plan, ensure_ascii=True, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if args.json:
        print(json.dumps(plan, ensure_ascii=False, indent=2, sort_keys=True))
    else:
        print(f"package_version={version}")
        print(f"commit={commit}")
        print(f"run_id={run_id}")
        print(f"rollout_plan={plan_path}")
        for node in plan["nodes"]:
            print(f"node={node['name']} platform={node['platform']} applied={str(node['applied']).lower()}")
            for command in node["commands"]:
                print(f"  {command}")
            if "windows_script" in node:
                print(f"  windows_script={node['windows_script']}")
            if "note" in node:
                print(f"  note={node['note']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
