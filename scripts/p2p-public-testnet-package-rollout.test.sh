#!/usr/bin/env bash
set -euo pipefail

if ! command -v shasum >/dev/null 2>&1; then
  shasum() {
    [[ "${1:-}" == "-a" && "${2:-}" == "256" ]] || {
      echo "test shasum shim supports only -a 256" >&2
      return 2
    }
    shift 2
    sha256sum "$@"
  }
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oasis7-package-rollout-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

package_dir="$TMP_DIR/package"
bundle_src="$TMP_DIR/bundle/oasis7-linux-x64"
node_root="$TMP_DIR/node"
out_dir="$TMP_DIR/out"
package_version="0.0.0+testnet.89.419e119bc897"
commit="419e119bc897efaa34750bee04c63470d1156699"
run_id="27605906795"

mkdir -p "$package_dir/windows" "$bundle_src/bin" "$node_root/releases/old/bin" "$node_root/config/doc/testing/evidence"
printf 'runtime-v2\n' >"$bundle_src/bin/oasis7_chain_runtime"
chmod +x "$bundle_src/bin/oasis7_chain_runtime"
tar -czf "$package_dir/oasis7-linux-x64-bundle.tar.gz" -C "$TMP_DIR/bundle" oasis7-linux-x64
printf 'fake windows installer\n' >"$package_dir/windows/oasis7-windows-x64.exe"

windows_bundle="$package_dir/windows/public-testnet-governed-bootstrap-bundle-2026-06-06.windows.json"
windows_genesis="$package_dir/windows/public-testnet-governed-bootstrap-genesis-2026-06-06.windows.json"
windows_manifest="$package_dir/windows/public-testnet-governed-bootstrap-manifest-2026-06-06.windows.json"
windows_bootstrap="$package_dir/windows/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.windows.txt"
windows_evidence_dir="$package_dir/windows/doc/testing/evidence"
windows_world_dir="$package_dir/windows/generated-world/world"
windows_sidecar_dir="$package_dir/windows/generated-world/generated-scenario-world"
windows_provenance="$package_dir/windows/generated-world/world-generation-provenance.json"
mkdir -p "$windows_evidence_dir" "$windows_world_dir" "$windows_sidecar_dir/chunks"
for evidence_name in \
  governance-public-signers.json \
  liveops-public-signers.json \
  signer-truth-binding.md \
  genesis-validator-registry.json \
  governed-bootstrap-topology.md; do
  printf 'windows governed evidence: %s\n' "$evidence_name" >"$windows_evidence_dir/$evidence_name"
done
printf '{"height":17,"world_id":"windows-fixture"}\n' >"$windows_world_dir/snapshot.json"
printf '{"events":["bootstrap"]}\n' >"$windows_world_dir/journal.json"
printf '{"scenario_id":"asteroid_fragment_bootstrap"}\n' >"$windows_sidecar_dir/snapshot.json"
printf '{"entries":[1,2]}\n' >"$windows_sidecar_dir/journal.json"
printf '{"chunk":"alpha"}\n' >"$windows_sidecar_dir/chunks/0001.json"
printf '{"scenario_id":"asteroid_fragment_bootstrap","seed":7}\n' >"$windows_provenance"
printf '/ip4/192.0.2.10/tcp/6831/p2p/12D3KooWWindowsFixture\n' >"$windows_bootstrap"
cat >"$windows_genesis" <<'EOF'
{
  "schema_version": "test",
  "operator_note": "historical build note mentioned /Users/build/oasis7 but is not a path-bearing field",
  "governance_bootstrap_refs": {
    "governance_public_manifest_ref": "doc/testing/evidence/governance-public-signers.json",
    "liveops_public_manifest_ref": "doc/testing/evidence/liveops-public-signers.json",
    "binding_notes_ref": "doc/testing/evidence/signer-truth-binding.md",
    "genesis_validator_registry_ref": "doc/testing/evidence/genesis-validator-registry.json",
    "topology_ref": "doc/testing/evidence/governed-bootstrap-topology.md"
  }
}
EOF

python3 - \
  "$package_dir/windows" \
  "$windows_bundle" \
  "$windows_manifest" \
  "$windows_genesis" \
  "$windows_bootstrap" \
  "$windows_world_dir" \
  "$windows_sidecar_dir" \
  "$windows_provenance" \
  "$windows_evidence_dir/genesis-validator-registry.json" \
  "$windows_evidence_dir/governed-bootstrap-topology.md" \
  "$package_dir/windows/oasis7-windows-x64.exe" <<'PY'
from pathlib import Path
import hashlib
import json
import sys

(
    platform_dir_raw,
    bundle_raw,
    manifest_raw,
    genesis_raw,
    bootstrap_raw,
    world_raw,
    sidecar_raw,
    provenance_raw,
    governance_raw,
    topology_raw,
    installer_raw,
) = sys.argv[1:]
platform_dir = Path(platform_dir_raw)
build_root = r"D:\\a\\oasis7\\oasis7\\public-testnet-stage"


def file_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tree_metadata(path: Path) -> dict[str, object]:
    digest = hashlib.sha256()
    files = sorted(item for item in path.rglob("*") if item.is_file())
    total_bytes = 0
    for item in files:
        relative = item.relative_to(path).as_posix()
        payload = item.read_bytes()
        total_bytes += len(payload)
        digest.update(relative.encode("utf-8"))
        digest.update(b"\0")
        digest.update(hashlib.sha256(payload).hexdigest().encode("ascii"))
        digest.update(b"\n")
    return {
        "kind": "directory",
        "sha256_tree": digest.hexdigest(),
        "file_count": len(files),
        "total_bytes": total_bytes,
    }


def file_metadata(path: Path, ref: str) -> dict[str, object]:
    return {
        "ref": ref,
        "resolved_path": build_root + "\\" + ref.replace("/", "\\"),
        "kind": "file",
        "sha256": file_sha256(path),
        "size_bytes": path.stat().st_size,
    }


def directory_metadata(path: Path, ref: str) -> dict[str, object]:
    result = {
        "ref": ref,
        "resolved_path": build_root + "\\" + ref.replace("/", "\\"),
    }
    result.update(tree_metadata(path))
    return result


bundle_path = Path(bundle_raw)
manifest_path = Path(manifest_raw)
genesis_path = Path(genesis_raw)
bootstrap_path = Path(bootstrap_raw)
world_path = Path(world_raw)
sidecar_path = Path(sidecar_raw)
provenance_path = Path(provenance_raw)
governance_path = Path(governance_raw)
topology_path = Path(topology_raw)
installer_path = Path(installer_raw)

bundle = {
    "schema_version": "oasis7.release_candidate_bundle.v1",
    "candidate_id": "windows-rollout-fixture",
    "track": "public_testnet",
    "runtime_build": file_metadata(installer_path, "oasis7-windows-x64.exe"),
    "world_snapshot": directory_metadata(world_path, "generated-world/world"),
    "generated_world_sidecar": directory_metadata(
        sidecar_path, "generated-world/generated-scenario-world"
    ),
    "world_generation_provenance": file_metadata(
        provenance_path, "generated-world/world-generation-provenance.json"
    ),
    "governance_manifest": file_metadata(
        governance_path, "doc/testing/evidence/genesis-validator-registry.json"
    ),
    "network_manifest": {},
    "evidence_refs": [
        file_metadata(
            topology_path, "doc/testing/evidence/governed-bootstrap-topology.md"
        )
    ],
}

manifest = {
    "schema_version": "oasis7.network_tier_manifest.v1",
    "tier": "public_testnet",
    "network_id": "oasis7-public-testnet-windows-fixture",
    "chain_id": "oasis7-public-testnet-windows-fixture",
    "runtime_refs": {
        "release_candidate_bundle_ref": build_root
        + r"\public-testnet-governed-bootstrap-bundle-2026-06-06.windows.json",
        "genesis_ref": build_root
        + r"\public-testnet-governed-bootstrap-genesis-2026-06-06.windows.json",
        "bootstrap_peer_ref": build_root
        + r"\public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.windows.txt",
        "generated_world_sidecar_ref": build_root
        + r"\generated-world\generated-scenario-world",
        "world_generation_provenance_ref": build_root
        + r"\generated-world\world-generation-provenance.json",
    },
}
manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
# network_manifest metadata depends on the completed manifest bytes.
bundle["network_manifest"] = file_metadata(
    manifest_path,
    "public-testnet-governed-bootstrap-manifest-2026-06-06.windows.json",
)
bundle_path.write_text(json.dumps(bundle, indent=2) + "\n", encoding="utf-8")
PY

cat >"$package_dir/linux-x64-BUILDINFO" <<EOF
workflow=Testnet Packages
run_id=$run_id
run_number=89
repository=eng-cc/oasis7
requested_ref=$commit
commit=$commit
build_profile=release
package_scope=all_existing
platform=linux-x64
package_version=$package_version
published=false
EOF

cat >"$package_dir/windows/windows-x64-BUILDINFO" <<EOF
workflow=Testnet Packages
run_id=$run_id
run_number=89
repository=eng-cc/oasis7
requested_ref=$commit
commit=$commit
build_profile=release
package_scope=all_existing
platform=windows-x64
package_version=$package_version
published=false
EOF

(
  cd "$package_dir"
  shasum -a 256 oasis7-linux-x64-bundle.tar.gz linux-x64-BUILDINFO >linux-x64-SHA256SUMS
  cd "$package_dir/windows"
  shasum -a 256 \
    oasis7-windows-x64.exe \
    windows-x64-BUILDINFO \
    public-testnet-governed-bootstrap-bundle-2026-06-06.windows.json \
    public-testnet-governed-bootstrap-genesis-2026-06-06.windows.json \
    public-testnet-governed-bootstrap-manifest-2026-06-06.windows.json \
    public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.windows.txt \
    generated-world/world/snapshot.json \
    generated-world/world/journal.json \
    generated-world/generated-scenario-world/snapshot.json \
    generated-world/generated-scenario-world/journal.json \
    generated-world/generated-scenario-world/chunks/0001.json \
    generated-world/world-generation-provenance.json \
    doc/testing/evidence/governance-public-signers.json \
    doc/testing/evidence/liveops-public-signers.json \
    doc/testing/evidence/signer-truth-binding.md \
    doc/testing/evidence/genesis-validator-registry.json \
    doc/testing/evidence/governed-bootstrap-topology.md \
    >windows-x64-SHA256SUMS
)

# Windows-only: package fixture setup above is shared with the POSIX harness, but
# the native behavior harness must run before any POSIX symlink/readlink assertion.
windows_powershell_behavior_success_marker='OASIS7_WINDOWS_POWERSHELL_ROLLOUT_BEHAVIOR_COMPLETE=true'

require_windows_powershell_behavior_success() {
  local status="$1"
  local output_path="$2"
  local marker_path="$3"

  if [[ "$status" -ne 0 ]]; then
    echo "Windows PowerShell rollout behavior harness failed with exit status $status" >&2
    cat "$output_path" >&2
    return 1
  fi
  if [[ ! -f "$marker_path" ]]; then
    echo "Windows PowerShell rollout behavior harness did not write completion marker: $marker_path" >&2
    cat "$output_path" >&2
    return 1
  fi
  if ! printf '%s' "$windows_powershell_behavior_success_marker" | cmp -s - "$marker_path"; then
    echo "Windows PowerShell rollout behavior harness wrote an invalid completion marker: $marker_path" >&2
    cat "$output_path" >&2
    return 1
  fi
}

verify_windows_powershell_behavior_boundary_fails_closed() {
  local output_path="$TMP_DIR/windows-powershell-boundary-negative-self-check.log"
  local marker_path="$TMP_DIR/windows-powershell-boundary-negative-self-check.marker"

  printf 'ParserError: simulated fixture parser failure\n' >"$output_path"
  rm -f "$marker_path"
  if require_windows_powershell_behavior_success 0 "$output_path" "$marker_path" >/dev/null 2>&1; then
    echo "Windows PowerShell behavior boundary accepted missing marker file" >&2
    return 1
  fi
  printf 'OASIS7_WINDOWS_POWERSHELL_ROLLOUT_BEHAVIOR_COMPLETE=false' >"$marker_path"
  if require_windows_powershell_behavior_success 0 "$output_path" "$marker_path" >/dev/null 2>&1; then
    echo "Windows PowerShell behavior boundary accepted wrong marker file content" >&2
    return 1
  fi
  printf '%s' "$windows_powershell_behavior_success_marker" >"$marker_path"
  if require_windows_powershell_behavior_success 17 "$output_path" "$marker_path" >/dev/null 2>&1; then
    echo "Windows PowerShell behavior boundary accepted nonzero PowerShell status" >&2
    return 1
  fi
}

run_windows_powershell_behavior_harness() {
windows_powershell="$(command -v powershell.exe || command -v powershell || command -v pwsh || true)"
if [[ -z "$windows_powershell" ]]; then
  echo "OASIS7_WINDOWS_POWERSHELL_BEHAVIOR_TEST=1 requires Windows PowerShell" >&2
  exit 1
fi
if ! command -v cygpath >/dev/null 2>&1; then
  echo "OASIS7_WINDOWS_POWERSHELL_BEHAVIOR_TEST=1 requires Git Bash cygpath" >&2
  exit 1
fi
local powershell_output="$TMP_DIR/windows-powershell-behavior.log"
local powershell_completion_marker
local powershell_fixture_script
local powershell_fixture_script_windows
local powershell_status
if ! awk '
  /^[[:space:]]*"\$windows_powershell" -NoProfile -ExecutionPolicy Bypass -File "\$powershell_fixture_script_windows"/ {
    found = 1
  }
  END { exit !found }
' "${BASH_SOURCE[0]}"; then
  echo "Windows PowerShell behavior harness must invoke its fixture with -File" >&2
  exit 1
fi
powershell_completion_marker="$(mktemp "$TMP_DIR/windows-powershell-behavior-completion-marker.XXXXXX")"
rm -f "$powershell_completion_marker"
powershell_fixture_script="$(mktemp "$TMP_DIR/windows-powershell-behavior-fixture.XXXXXX.ps1")"
cat >"$powershell_fixture_script" <<'PS'
$ErrorActionPreference = 'Stop'

function Assert-FixtureNoReparseAncestor([string] $Path) {
$probe = [System.IO.Path]::GetFullPath($Path)
while (![string]::IsNullOrWhiteSpace($probe)) {
  $item = Get-Item -LiteralPath $probe -Force -ErrorAction SilentlyContinue
  if ($null -ne $item -and
      (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0)) {
    throw "fixture safe-root precondition failed: reparse-point component=$probe configured_base=$Path"
  }
  $parent = [System.IO.Path]::GetDirectoryName($probe)
  if ([string]::IsNullOrWhiteSpace($parent) -or
      $parent.Equals($probe, [System.StringComparison]::OrdinalIgnoreCase)) {
    break
  }
  $probe = $parent
}
}

function Assert-FixturePathUnderSafeBase([string] $Path, [string] $Label) {
$safeRoot = [System.IO.Path]::GetFullPath($fixtureSafeRootBase).TrimEnd('\')
$candidate = [System.IO.Path]::GetFullPath($Path).TrimEnd('\')
if (!$candidate.StartsWith($safeRoot + '\', [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "fixture generated mutable path escapes verified safe base: label=$Label path=$Path canonical=$candidate safe_base=$safeRoot"
}
return $candidate
}

$fixtureWorkspaceRoot = [System.IO.Path]::GetFullPath($env:OASIS7_FIXTURE_WORKSPACE_ROOT)
if (!(Test-Path -LiteralPath $fixtureWorkspaceRoot -PathType Container)) {
  throw "fixture workspace root does not exist: $fixtureWorkspaceRoot"
}
$fixtureDriveRoot = [System.IO.Path]::GetPathRoot($fixtureWorkspaceRoot)
Assert-FixtureNoReparseAncestor $fixtureDriveRoot
$fixtureRunId = [Guid]::NewGuid().ToString('N')
$fixtureRoot = Join-Path $fixtureDriveRoot ("o7fx-" + $fixtureRunId)
if (Test-Path -LiteralPath $fixtureRoot) {
  throw "fixture unique drive-root path collision: $fixtureRoot"
}
New-Item -ItemType Directory -Path $fixtureRoot -Force | Out-Null
Assert-FixtureNoReparseAncestor $fixtureRoot
$fixtureSafeRootBase = $fixtureRoot
$fixtureTasks = [System.Collections.Generic.List[string]]::new()

function Get-Sha256([string] $Path) {
(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Assert-Equal([object] $Actual, [object] $Expected, [string] $Label) {
if ($Actual -ne $Expected) { throw "${Label}: expected=[$Expected] actual=[$Actual]" }
}

function Get-FixtureCanonicalExistingPath([string] $Path) {
$item = Get-Item -LiteralPath $Path -Force -ErrorAction Stop
return [System.IO.Path]::GetFullPath($item.FullName).TrimEnd('\')
}

function Assert-FixtureSameExistingPath([string] $Actual, [string] $Expected, [string] $Label) {
$actualCanonical = Get-FixtureCanonicalExistingPath $Actual
$expectedCanonical = Get-FixtureCanonicalExistingPath $Expected
if (!$actualCanonical.Equals($expectedCanonical, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "${Label}: expected=[$Expected] expected_canonical=[$expectedCanonical] actual=[$Actual] actual_canonical=[$actualCanonical]"
}
}

function Assert-Utf8NoBom([string] $Path) {
$bytes = [System.IO.File]::ReadAllBytes($Path)
if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
  throw "fixture JSON must be UTF-8 without BOM: $Path"
}
}

function Invoke-FixtureRolloutExpectingFailure([string] $Rollout) {
$previousErrorActionPreference = $ErrorActionPreference
try {
  $ErrorActionPreference = 'Continue'
  $output = @(& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $Rollout 2>&1)
  $exitCode = $LASTEXITCODE
} finally {
  $ErrorActionPreference = $previousErrorActionPreference
}
return [PSCustomObject]@{ ExitCode = $exitCode; Output = @($output) }
}

function Write-FixtureExecutables {
param([string] $Directory)
$runtimeSource = @'
using System;
using System.Net;
using System.Text;
public static class FixtureRuntime {
public static void Main(string[] args) {
  int port = Int32.Parse(args[1]);
  var listener = new HttpListener();
  listener.Prefixes.Add("http://127.0.0.1:" + port + "/");
  listener.Start();
  while (true) {
    var context = listener.GetContext();
    byte[] payload = Encoding.UTF8.GetBytes("{\"running\":true}");
    context.Response.ContentType = "application/json";
    context.Response.OutputStream.Write(payload, 0, payload.Length);
    context.Response.Close();
  }
}
}
'@
$installerSource = @'
using System;
using System.IO;
public static class FixtureInstaller {
public static void Main() {
  string installRoot = Environment.GetEnvironmentVariable("OASIS7_FIXTURE_INSTALL_ROOT");
  string runtime = Environment.GetEnvironmentVariable("OASIS7_FIXTURE_RUNTIME_TEMPLATE");
  string target = Path.Combine(installRoot, "bin", "oasis7_chain_runtime.exe");
  Directory.CreateDirectory(Path.GetDirectoryName(target));
  File.Copy(runtime, target, true);
}
}
'@
$runtime = Join-Path $Directory 'oasis7_chain_runtime.exe'
$installer = Join-Path $Directory 'fixture-installer.exe'
Add-Type -TypeDefinition $runtimeSource -OutputAssembly $runtime -OutputType ConsoleApplication
Add-Type -TypeDefinition $installerSource -OutputAssembly $installer -OutputType ConsoleApplication
return @{ Runtime = $runtime; Installer = $installer }
}

$executables = Write-FixtureExecutables -Directory $fixtureRoot

function New-RolloutFixture {
param([string] $Name, [switch] $ActiveConfigJunction, [switch] $RollbackRootJunction)
$root = Join-Path $fixtureRoot $Name
$deployReal = Join-Path $root 'deploy-real'
$deploy = if ($ActiveConfigJunction) { Join-Path $root 'deploy-link' } else { $deployReal }
$installRoot = Join-Path $root 'install'
$rollbackReal = Join-Path $deployReal 'backups/rollback-real'
$rollbackRoot = if ($RollbackRootJunction) { Join-Path $deploy 'backups/rollback-link' } else { $rollbackReal }
New-Item -ItemType Directory -Path $deployReal, $installRoot, $rollbackReal -Force | Out-Null
if ($ActiveConfigJunction) { New-Item -ItemType Junction -Path $deploy -Target $deployReal | Out-Null }
if ($RollbackRootJunction) { New-Item -ItemType Junction -Path $rollbackRoot -Target $rollbackReal | Out-Null }
if ($ActiveConfigJunction) {
  $deployJunction = Get-Item -LiteralPath $deploy -Force -ErrorAction Stop
  if (($deployJunction.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -eq 0) {
    throw "fixture active deploy root is not a reparse point: $deploy"
  }
}
if ($RollbackRootJunction) {
  $rollbackJunction = Get-Item -LiteralPath $rollbackRoot -Force -ErrorAction Stop
  if (($rollbackJunction.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -eq 0) {
    throw "fixture rollback root is not a reparse point: $rollbackRoot"
  }
}
$taskName = "Oasis7Fixture2269_" + [Guid]::NewGuid().ToString('N')
$port = Get-Random -Minimum 20000 -Maximum 45000
$config = Join-Path $deploy 'config'
$activeBundle = Join-Path $config 'public-testnet-governed-bootstrap-bundle-2026-06-06.windows.json'
$activeGenesis = Join-Path $config 'public-testnet-governed-bootstrap-genesis-2026-06-06.windows.json'
$activeManifest = Join-Path $config 'public-testnet-governed-bootstrap-manifest-2026-06-06.windows.json'
$activeBootstrap = Join-Path $config 'public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.windows.txt'
New-Item -ItemType Directory -Path $config, (Join-Path $installRoot 'bin'), (Join-Path $rollbackRoot 'runtime'), (Join-Path $rollbackRoot 'config') -Force | Out-Null
Copy-Item -LiteralPath $executables.Runtime -Destination (Join-Path $installRoot 'bin/oasis7_chain_runtime.exe') -Force
Copy-Item -LiteralPath $executables.Runtime -Destination (Join-Path $rollbackRoot 'runtime/oasis7_chain_runtime.exe') -Force
Set-Content -LiteralPath (Join-Path $deploy 'CURRENT_VERSION') -Value 'known-good-version' -NoNewline
Set-Content -LiteralPath (Join-Path $deploy 'DEPLOYED_BUILDINFO') -Value 'known-good-buildinfo' -NoNewline
Copy-Item -LiteralPath (Join-Path $deploy 'CURRENT_VERSION') -Destination (Join-Path $rollbackRoot 'CURRENT_VERSION') -Force
Copy-Item -LiteralPath (Join-Path $deploy 'DEPLOYED_BUILDINFO') -Destination (Join-Path $rollbackRoot 'DEPLOYED_BUILDINFO') -Force
$runtimeBackupHash = Get-Sha256 (Join-Path $rollbackRoot 'runtime/oasis7_chain_runtime.exe')
$backupManifestPath = Join-Path $rollbackRoot 'backup-manifest.json'
[System.IO.File]::WriteAllText(
  $backupManifestPath,
  (@{
  schema_version = 'oasis7.windows_observer_backup.v1'
  runtime_path = 'runtime\oasis7_chain_runtime.exe'
  runtime_sha256 = $runtimeBackupHash
  } | ConvertTo-Json),
  [System.Text.UTF8Encoding]::new($false)
)
$action = New-ScheduledTaskAction -Execute (Join-Path $installRoot 'bin/oasis7_chain_runtime.exe') -Argument "--port $port"
Register-ScheduledTask -TaskName $taskName -Action $action -Force | Out-Null
$fixtureTasks.Add($taskName)
$manifest = @{
  nodes = @(@{
    name = 'windows-fixture'; platform = 'windows-x64'; deploy_root = $deploy; install_root = $installRoot
    remote_installer = (Join-Path $deploy 'placeholder/oasis7-windows-x64.exe'); scheduled_task = $taskName
    status_url = "http://127.0.0.1:$port/v1/chain/status"; rollback_backup_root = $rollbackRoot; rollback_unlock_timeout_secs = 5
    governed_bundle_path = $activeBundle; governed_genesis_path = $activeGenesis; governed_manifest_path = $activeManifest; governed_bootstrap_path = $activeBootstrap
  })
}
$manifestPath = Join-Path $root 'manifest.json'
$outDir = Join-Path $root 'out'
[System.IO.File]::WriteAllText(
  $manifestPath,
  ($manifest | ConvertTo-Json -Depth 8),
  [System.Text.UTF8Encoding]::new($false)
)
Assert-Utf8NoBom $backupManifestPath
Assert-Utf8NoBom $manifestPath
& python $env:OASIS7_FIXTURE_ROLLOUT_PY --manifest $manifestPath --package-dir $env:OASIS7_FIXTURE_PACKAGE_DIR --out-dir $outDir --json | Out-Null
if ($LASTEXITCODE -ne 0) { throw "fixture rollout generation failed: $Name" }
$rollout = Join-Path $outDir 'windows-fixture-windows-upgrade.ps1'
$scriptText = [IO.File]::ReadAllText($rollout)
$pathGuardStart = $scriptText.IndexOf('function Test-NodeLocalPath')
$pathGuardEnd = $scriptText.IndexOf('function Preserve-AttemptDiagnostics', $pathGuardStart)
if ($pathGuardStart -lt 0 -or $pathGuardEnd -le $pathGuardStart) {
  throw 'fixture could not isolate generated node-local physical path guards'
}
$pathGuardDefinitions = [scriptblock]::Create($scriptText.Substring($pathGuardStart, $pathGuardEnd - $pathGuardStart))
. $pathGuardDefinitions
$deployRoot = $deploy
if ($ActiveConfigJunction -or $RollbackRootJunction) {
  $expectedReparseAncestor = if ($ActiveConfigJunction) { $deploy } else { $rollbackRoot }
  $nestedReparseProbe = Join-Path $expectedReparseAncestor 'nested-parent-traversal/leaf.txt'
  $ancestorRejected = $false
  try {
    Assert-NodeLocalPhysicalPath -Path $nestedReparseProbe -Label 'fixture parent traversal probe' | Out-Null
  } catch {
    $reportedReparseMatch = [regex]::Match($_.Exception.Message, 'reparse-point component:\s*(?<path>.+?)\s*$')
    if (!$reportedReparseMatch.Success) {
      throw "fixture parent traversal did not identify its reparse ancestor: expected=$expectedReparseAncestor error=$($_.Exception.Message)"
    }
    $reportedReparseAncestor = $reportedReparseMatch.Groups['path'].Value
    $reportedReparseItem = Get-Item -LiteralPath $reportedReparseAncestor -Force -ErrorAction Stop
    if (($reportedReparseItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -eq 0) {
      throw "fixture parent traversal reported a non-reparse path: $reportedReparseAncestor"
    }
    Assert-FixtureSameExistingPath $reportedReparseAncestor $expectedReparseAncestor 'fixture parent traversal rejected the wrong reparse ancestor'
    $ancestorRejected = $true
  }
  if (!$ancestorRejected) {
    throw "fixture parent traversal accepted nested path below reparse ancestor: $nestedReparseProbe"
  }
}
$requiredGeneratedAssignments = @{
  deployRoot = $deploy
  installRoot = $installRoot
  rollbackBackupRoot = $rollbackRoot
  activeBundlePath = $activeBundle
  activeGenesisPath = $activeGenesis
  activeManifestPath = $activeManifest
  activeBootstrapPath = $activeBootstrap
}
foreach ($requiredGeneratedAssignment in $requiredGeneratedAssignments.GetEnumerator()) {
  $assignmentPattern = '\$' + [regex]::Escape($requiredGeneratedAssignment.Key) + ' = \[Environment\]::ExpandEnvironmentVariables\(''([^'']+)''\)'
  $assignmentMatch = [regex]::Match($scriptText, $assignmentPattern)
  if (!$assignmentMatch.Success -or $assignmentMatch.Groups[1].Value -ne $requiredGeneratedAssignment.Value) {
    throw "fixture generated mutable destination diverged: variable=$($requiredGeneratedAssignment.Key) expected=$($requiredGeneratedAssignment.Value) actual=$($assignmentMatch.Groups[1].Value)"
  }
  Assert-FixturePathUnderSafeBase $assignmentMatch.Groups[1].Value $requiredGeneratedAssignment.Key | Out-Null
}
$stagingRootAssignmentPattern = '\$stagingRoot = \[Environment\]::ExpandEnvironmentVariables\(''([^'']+)''\)'
$representativeStagingAssignment = '$stagingRoot = [Environment]::ExpandEnvironmentVariables(''C:\oasis7-deploy\staging\package-rollout\manual'')'
$representativeStagingMatch = [regex]::Match($representativeStagingAssignment, $stagingRootAssignmentPattern)
if (!$representativeStagingMatch.Success -or $representativeStagingMatch.Groups[1].Value -ne 'C:\oasis7-deploy\staging\package-rollout\manual') {
  throw 'fixture staging-root extraction regex does not match the generated PowerShell assignment contract'
}
$stagingMatch = [regex]::Match($scriptText, $stagingRootAssignmentPattern)
if (!$stagingMatch.Success -or [string]::IsNullOrWhiteSpace($stagingMatch.Groups[1].Value)) {
  throw "fixture could not locate generated staging root: $Name"
}
$staging = $stagingMatch.Groups[1].Value
Assert-FixturePathUnderSafeBase $staging 'stagingRoot' | Out-Null
foreach ($stagedAssignmentName in @('installer', 'bundlePath', 'genesisPath', 'manifestPath', 'bootstrapPath')) {
  $stagedAssignmentPattern = '\$' + [regex]::Escape($stagedAssignmentName) + ' = \[Environment\]::ExpandEnvironmentVariables\(''([^'']+)''\)'
  $stagedAssignmentMatches = [regex]::Matches($scriptText, $stagedAssignmentPattern)
  if ($stagedAssignmentMatches.Count -eq 0) {
    throw "fixture generated script omitted guarded staged assignment: $stagedAssignmentName"
  }
  foreach ($stagedAssignmentMatch in $stagedAssignmentMatches) {
    Assert-FixturePathUnderSafeBase $stagedAssignmentMatch.Groups[1].Value $stagedAssignmentName | Out-Null
  }
}
[Console]::Error.WriteLine("fixture_generated_staging_root name=$Name path=$staging safe_base=$fixtureSafeRootBase package_input=$env:OASIS7_FIXTURE_PACKAGE_DIR")
New-Item -ItemType Directory -Path (Join-Path $staging 'config') -Force | Out-Null
Copy-Item -LiteralPath $executables.Installer -Destination (Join-Path $staging 'oasis7-windows-x64.exe') -Force
$packageWindows = Join-Path $env:OASIS7_FIXTURE_PACKAGE_DIR 'windows'
Get-ChildItem -LiteralPath $packageWindows -Recurse -File | Where-Object { $_.Name -notin @('oasis7-windows-x64.exe', 'windows-x64-BUILDINFO', 'windows-x64-SHA256SUMS') } | ForEach-Object {
  $relative = $_.FullName.Substring($packageWindows.Length).TrimStart('\\')
  $destination = Join-Path (Join-Path $staging 'config') $relative
  New-Item -ItemType Directory -Path (Split-Path $destination -Parent) -Force | Out-Null
  Copy-Item -LiteralPath $_.FullName -Destination $destination -Force
}
# Mirror every staged governed file into the active config and known-good
# backup. The generated rollout promotes this complete recursive closure,
# not only the three transformed JSON documents.
$stagingConfig = Join-Path $staging 'config'
$activeGovernedFiles = @(
  Get-ChildItem -LiteralPath $stagingConfig -Recurse -File | ForEach-Object {
    $relative = $_.FullName.Substring($stagingConfig.Length).TrimStart('\\')
    $activePath = Join-Path $config $relative
    $backupPath = Join-Path (Join-Path $rollbackRoot 'config') $relative
    New-Item -ItemType Directory -Path (Split-Path $activePath -Parent), (Split-Path $backupPath -Parent) -Force | Out-Null
    Set-Content -LiteralPath $activePath -Value ("known-good-$relative") -NoNewline
    Copy-Item -LiteralPath $activePath -Destination $backupPath -Force
    $activePath
  }
)
foreach ($requiredActivePath in @($activeBundle, $activeGenesis, $activeManifest, $activeBootstrap)) {
  $requiredActiveLeaf = [System.IO.Path]::GetFileName($requiredActivePath)
  $projectedActivePaths = @(
    $activeGovernedFiles | Where-Object {
      [System.IO.Path]::GetFileName($_) -eq $requiredActiveLeaf
    }
  )
  if ($projectedActivePaths.Count -eq 0) {
    throw "fixture staged governed closure omitted required active target: $requiredActivePath"
  }
  if ($projectedActivePaths.Count -ne 1) {
    throw "fixture staged governed closure has ambiguous required active target: $requiredActiveLeaf"
  }
  if (!$ActiveConfigJunction) {
    $projectedActivePath = $projectedActivePaths[0]
    if (!(Test-Path -LiteralPath $requiredActivePath -PathType Leaf)) {
      throw "fixture logical active target is not reachable through its configured path: required=$requiredActivePath required_length=$($requiredActivePath.Length) projected=$projectedActivePath projected_length=$($projectedActivePath.Length) safe_base=$fixtureSafeRootBase"
    }
    Assert-Equal (Get-Sha256 $projectedActivePath) (Get-Sha256 $requiredActivePath) "fixture active target projection diverged for $requiredActiveLeaf"
  }
}
$atomicPromotionSuffixBudget = 48
$windowsLegacyPathStringBudget = 259
$governedPathBudget = $windowsLegacyPathStringBudget - $atomicPromotionSuffixBudget
if ($governedPathBudget -le 0) {
  throw 'fixture derived governed path budget must be positive'
}
$governedPathCandidates = @(
  Get-ChildItem -LiteralPath $stagingConfig -Recurse -File | ForEach-Object { $_.FullName }
  $activeGovernedFiles
  Get-ChildItem -LiteralPath (Join-Path $rollbackRoot 'config') -Recurse -File | ForEach-Object { $_.FullName }
)
$longestGovernedPath = @($governedPathCandidates | Sort-Object Length -Descending | Select-Object -First 1)[0]
if ([string]::IsNullOrWhiteSpace($longestGovernedPath)) {
  throw 'fixture governed path budget check found no governed files'
}
[Console]::Error.WriteLine("fixture_governed_path_budget longest_length=$($longestGovernedPath.Length) budget=$governedPathBudget path=$longestGovernedPath safe_base=$fixtureSafeRootBase")
if ($longestGovernedPath.Length -gt $governedPathBudget) {
  throw "fixture governed path exceeds Windows PowerShell 5.1 safety budget: length=$($longestGovernedPath.Length) budget=$governedPathBudget path=$longestGovernedPath"
}
if (($longestGovernedPath.Length + $atomicPromotionSuffixBudget) -gt $windowsLegacyPathStringBudget) {
  throw "fixture governed path leaves insufficient atomic promotion headroom: length=$($longestGovernedPath.Length) suffix_budget=$atomicPromotionSuffixBudget max=$windowsLegacyPathStringBudget path=$longestGovernedPath"
}
return @{ Root=$root; Deploy=$deploy; Install=$installRoot; Rollback=$rollbackRoot; Staging=$staging; Task=$taskName; Port=$port; Rollout=$rollout; Active=@($activeGovernedFiles + (Join-Path $deploy 'CURRENT_VERSION') + (Join-Path $deploy 'DEPLOYED_BUILDINFO')); Runtime=(Join-Path $installRoot 'bin/oasis7_chain_runtime.exe') }
}

function Get-FixtureSnapshot($Fixture) {
$files = @($Fixture.Runtime) + $Fixture.Active
@($files | ForEach-Object { "$($_)=$(Get-Sha256 $_)" }) -join "`n"
}

function Assert-OriginalTaskAction($Fixture) {
$action = @((Get-ScheduledTask -TaskName $Fixture.Task).Actions)[0]
Assert-Equal $action.Execute $Fixture.Runtime "scheduled task execute changed for $($Fixture.Task)"
Assert-Equal $action.Arguments "--port $($Fixture.Port)" "scheduled task arguments changed for $($Fixture.Task)"
}

function Remove-RolloutFixture($Fixture) {
if ($null -eq $Fixture) { return }
Unregister-ScheduledTask -TaskName $Fixture.Task -Confirm:$false -ErrorAction SilentlyContinue
Get-Process oasis7_chain_runtime -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$($Fixture.Install)*" } | Stop-Process -Force -ErrorAction SilentlyContinue
}

try {
# Both cases use a live, uniquely named task. A reparse ancestor must reject
# before Stop-ScheduledTask: the task remains running and all persisted state
# is byte-identical.
foreach ($case in @(@{Name='active-config-junction'; Active=$true; Rollback=$false}, @{Name='rollback-root-junction'; Active=$false; Rollback=$true})) {
  $fixture = New-RolloutFixture -Name $case.Name -ActiveConfigJunction:$case.Active -RollbackRootJunction:$case.Rollback
  try {
    Start-ScheduledTask -TaskName $fixture.Task
    Start-Sleep -Seconds 1
    $before = Get-FixtureSnapshot $fixture
    $invocation = Invoke-FixtureRolloutExpectingFailure $fixture.Rollout
    if ($invocation.ExitCode -eq 0) { throw "junction fixture unexpectedly succeeded: $($case.Name)" }
    $junctionOutput = $invocation.Output -join "`n"
    [Console]::Error.WriteLine("fixture_expected_reparse case=$($case.Name) staging_root=$($fixture.Staging) output=$junctionOutput")
    if ($junctionOutput -notmatch 'reparse') { throw "junction fixture missing reparse rejection: $($case.Name) output=$junctionOutput" }
    Assert-Equal (Get-FixtureSnapshot $fixture) $before "junction fixture mutated persisted state: $($case.Name)"
    Assert-OriginalTaskAction $fixture
    if ((Get-ScheduledTask -TaskName $fixture.Task).State -ne 'Running') { throw "junction fixture reached Stop-ScheduledTask: $($case.Name)" }
  } finally { Remove-RolloutFixture $fixture }
}

$phases = @('installer','governed_copy','bundle_move','genesis_move','manifest_move','current_version_write','deployed_buildinfo_write','task_action_restore')
foreach ($phase in $phases) {
  $fixture = New-RolloutFixture -Name "phase-$phase"
  try {
    $before = Get-FixtureSnapshot $fixture
    for ($attempt = 1; $attempt -le 2; $attempt++) {
      $env:OASIS7_FIXTURE_INSTALL_ROOT = $fixture.Install
      $env:OASIS7_FIXTURE_RUNTIME_TEMPLATE = $executables.Runtime
      $env:OASIS7_ROLLOUT_FAIL_PHASE = $phase
      # The generated production interface currently consumes the explicit
      # INJECT spelling; retain FAIL_PHASE as the fixture contract alias.
      $env:OASIS7_ROLLOUT_INJECT_FAILURE_PHASE = $phase
      $invocation = Invoke-FixtureRolloutExpectingFailure $fixture.Rollout
      if ($invocation.ExitCode -eq 0) { throw "injected phase unexpectedly succeeded: phase=$phase attempt=$attempt" }
      Assert-Equal (Get-FixtureSnapshot $fixture) $before "rollback failed to restore known-good state: phase=$phase attempt=$attempt"
      Assert-OriginalTaskAction $fixture
      $diagnostics = (Get-ChildItem -LiteralPath $fixture.Deploy -Recurse -File | ForEach-Object { Get-Content -LiteralPath $_.FullName -Raw -ErrorAction SilentlyContinue }) -join "`n"
      foreach ($field in @('failure_phase=', 'rollback_path=', 'rollback_error=', 'rollback_exit_code=', 'rollback_required=true')) {
        if ($diagnostics -notmatch [regex]::Escape($field)) { throw "missing stable rollback diagnostic: phase=$phase field=$field" }
      }
    }
  } finally {
    Remove-Item Env:OASIS7_ROLLOUT_FAIL_PHASE -ErrorAction SilentlyContinue
    Remove-Item Env:OASIS7_ROLLOUT_INJECT_FAILURE_PHASE -ErrorAction SilentlyContinue
    Remove-Item Env:OASIS7_FIXTURE_INSTALL_ROOT -ErrorAction SilentlyContinue
    Remove-Item Env:OASIS7_FIXTURE_RUNTIME_TEMPLATE -ErrorAction SilentlyContinue
    Remove-RolloutFixture $fixture
  }
}
[System.IO.File]::WriteAllText(
  $env:OASIS7_FIXTURE_COMPLETION_MARKER,
  'OASIS7_WINDOWS_POWERSHELL_ROLLOUT_BEHAVIOR_COMPLETE=true',
  [System.Text.ASCIIEncoding]::new()
)
} finally {
foreach ($taskName in $fixtureTasks) { Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue }
Remove-Item -LiteralPath $fixtureRoot -Recurse -Force -ErrorAction SilentlyContinue
}
PS

if [[ "$(od -An -v -t x1 -N 3 "$powershell_fixture_script")" == *"ef bb bf"* ]]; then
  echo "Windows PowerShell behavior fixture must be UTF-8 without a BOM: $powershell_fixture_script" >&2
  exit 1
fi
powershell_fixture_script_windows="$(cygpath -w "$powershell_fixture_script")"
if [[ "$powershell_fixture_script_windows" != *.ps1 ]]; then
  echo "Windows PowerShell behavior fixture did not retain its .ps1 path: $powershell_fixture_script_windows" >&2
  exit 1
fi

set +e
OASIS7_FIXTURE_WORKSPACE_ROOT="$(cygpath -w "$ROOT_DIR")" \
  OASIS7_FIXTURE_PACKAGE_DIR="$(cygpath -w "$package_dir")" \
  OASIS7_FIXTURE_ROLLOUT_PY="$(cygpath -w "$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py")" \
  OASIS7_FIXTURE_COMPLETION_MARKER="$(cygpath -w "$powershell_completion_marker")" \
  "$windows_powershell" -NoProfile -ExecutionPolicy Bypass -File "$powershell_fixture_script_windows" >"$powershell_output" 2>&1
powershell_status=$?
set -e
cat "$powershell_output"
require_windows_powershell_behavior_success \
  "$powershell_status" "$powershell_output" "$powershell_completion_marker"
}
verify_windows_powershell_behavior_boundary_fails_closed
python3 - "${BASH_SOURCE[0]}" <<'PY'
from pathlib import Path
import re
import sys

source = Path(sys.argv[1]).read_text(encoding="utf-8")
fixture_match = re.search(
    r"cat >\"\$powershell_fixture_script\" <<'PS'\n(?P<body>.*?)\nPS$",
    source,
    re.MULTILINE | re.DOTALL,
)
assert fixture_match, "could not isolate native Windows PowerShell behavior fixture"
fixture = fixture_match.group("body")
assert "function Get-FixtureCanonicalExistingPath" in fixture
assert "Get-Item -LiteralPath $Path -Force -ErrorAction Stop" in fixture
assert "$item.FullName" in fixture
assert "[System.StringComparison]::OrdinalIgnoreCase" in fixture
assert "reportedReparseItem.Attributes" in fixture
assert "[System.IO.FileAttributes]::ReparsePoint" in fixture
assert "Assert-FixtureSameExistingPath $reportedReparseAncestor $expectedReparseAncestor" in fixture
assert "-notmatch [regex]::Escape($expectedReparseAncestor)" not in fixture, (
    "fixture must not compare raw short/long Windows path spellings"
)
assert "function Assert-FixtureNoReparseAncestor" in fixture
assert "function Assert-FixturePathUnderSafeBase" in fixture
assert "fixture safe-root precondition failed: reparse-point component=" in fixture
assert "fixture generated mutable path escapes verified safe base:" in fixture
assert "Assert-FixturePathUnderSafeBase $staging 'stagingRoot'" in fixture
assert "fixture_generated_staging_root name=$Name path=$staging" in fixture
assert "fixture_expected_reparse case=$($case.Name) staging_root=$($fixture.Staging)" in fixture
assert "function Invoke-FixtureRolloutExpectingFailure" in fixture
assert "$ErrorActionPreference = 'Continue'" in fixture
assert "$ErrorActionPreference = $previousErrorActionPreference" in fixture
assert fixture.count("Invoke-FixtureRolloutExpectingFailure $fixture.Rollout") == 2
safe_root_check = fixture.index("Assert-FixtureNoReparseAncestor $fixtureDriveRoot\n")
fixture_root_check = fixture.index("Assert-FixtureNoReparseAncestor $fixtureRoot\n")
intentional_junction = fixture.index("New-Item -ItemType Junction")
assert safe_root_check < fixture_root_check < intentional_junction, (
    "native fixture must verify its workspace root before constructing intentional junctions"
)
assert not re.search(r'^OASIS7_FIXTURE_(?:ROOT_DIR|SAFE_ROOT_BASE)=', source, re.MULTILINE)
assert re.search(
    r'^OASIS7_FIXTURE_WORKSPACE_ROOT="\$\(cygpath -w "\$ROOT_DIR"\)" \\$',
    source,
    re.MULTILINE,
)
assert 'Join-Path $fixtureDriveRoot ("o7fx-" + $fixtureRunId)' in fixture
assert "fixture unique drive-root path collision:" in fixture
assert "fixture_governed_path_budget longest_length=" in fixture
assert "$atomicPromotionSuffixBudget = 48" in fixture
assert "$windowsLegacyPathStringBudget = 259" in fixture
assert "$governedPathBudget = $windowsLegacyPathStringBudget - $atomicPromotionSuffixBudget" in fixture
assert "$governedPathBudget = 200" not in fixture
assert "$longestGovernedPath.Length -gt $governedPathBudget" in fixture
assert "$longestGovernedPath.Length + $atomicPromotionSuffixBudget" in fixture
assert "required_length=$($requiredActivePath.Length)" in fixture
staging_descendant_check = fixture.index("Assert-FixturePathUnderSafeBase $staging 'stagingRoot'")
generated_script_execution = fixture.index("Invoke-FixtureRolloutExpectingFailure $fixture.Rollout")
assert staging_descendant_check < generated_script_execution, (
    "generated staging root must be proven beneath the safe base before native execution"
)
PY
if [[ "${OASIS7_WINDOWS_POWERSHELL_BEHAVIOR_TEST:-0}" == "1" ]]; then
  run_windows_powershell_behavior_harness
  exit 0
fi

printf 'runtime-v1\n' >"$node_root/releases/old/bin/oasis7_chain_runtime"
chmod +x "$node_root/releases/old/bin/oasis7_chain_runtime"
ln -s "$node_root/releases/old" "$node_root/current"
rollback_backup_fixture="$TMP_DIR/authorized-task-2269-backup"
mkdir -p "$rollback_backup_fixture/runtime" "$rollback_backup_fixture/config"
printf 'authorized-known-good-runtime\n' \
  >"$rollback_backup_fixture/runtime/oasis7_chain_runtime.exe"
cat >"$rollback_backup_fixture/backup-manifest.json" <<'EOF'
{
  "schema_version": "oasis7.windows_observer_backup.v1",
  "runtime": {
    "relative_path": "runtime/oasis7_chain_runtime.exe",
    "sha256": "6069ce6697cf15afc54e127fc748eb8de067cf7f69199125958ab59adf81b69d"
  },
  "provenance": {
    "deployed_buildinfo": "DEPLOYED_BUILDINFO"
  }
}
EOF
printf 'package_version=0.0.0+testnet.157.c62f1c78333f\n' \
  >"$rollback_backup_fixture/DEPLOYED_BUILDINFO"
test -f "$rollback_backup_fixture/runtime/oasis7_chain_runtime.exe"
test ! -e "$rollback_backup_fixture/bin/oasis7_chain_runtime.exe"
actual_rollback_backup_fixture="$TMP_DIR/authorized-task-2269-actual-backup"
mkdir -p "$actual_rollback_backup_fixture/runtime"
cp \
  "$rollback_backup_fixture/runtime/oasis7_chain_runtime.exe" \
  "$actual_rollback_backup_fixture/runtime/oasis7_chain_runtime.exe"
cat >"$actual_rollback_backup_fixture/backup-manifest.json" <<'EOF'
{
  "schema_version": "oasis7.windows_observer_backup.v1",
  "runtime_path": "runtime\\oasis7_chain_runtime.exe",
  "runtime_sha256": "6069ce6697cf15afc54e127fc748eb8de067cf7f69199125958ab59adf81b69d"
}
EOF
test -f "$actual_rollback_backup_fixture/runtime/oasis7_chain_runtime.exe"
test ! -e "$actual_rollback_backup_fixture/bin/oasis7_chain_runtime.exe"
legacy_rollback_backup_fixture="$TMP_DIR/authorized-task-2269-legacy-rooted-backup"
mkdir -p "$legacy_rollback_backup_fixture/runtime"
cp \
  "$rollback_backup_fixture/runtime/oasis7_chain_runtime.exe" \
  "$legacy_rollback_backup_fixture/runtime/oasis7_chain_runtime.exe"
cat >"$legacy_rollback_backup_fixture/backup-manifest.json" <<'EOF'
{
  "schema_version": "oasis7.windows_observer_backup.v1",
  "runtime_path": "C:\\Users\\Observer\\AppData\\Local\\Programs\\oasis7\\bin\\oasis7_chain_runtime.exe",
  "runtime_sha256": "6069ce6697cf15afc54e127fc748eb8de067cf7f69199125958ab59adf81b69d"
}
EOF
test -f "$legacy_rollback_backup_fixture/runtime/oasis7_chain_runtime.exe"
test ! -e "$legacy_rollback_backup_fixture/bin/oasis7_chain_runtime.exe"
cat >"$node_root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json" <<'EOF'
{
  "schema_version": "oasis7.release_candidate_bundle.v1",
  "runtime_build": {
    "path": "old",
    "ref": "old",
    "resolved_path": "old",
    "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
    "size_bytes": 1
  }
}
EOF

cat >"$TMP_DIR/manifest.json" <<EOF
{
  "nodes": [
    {
      "name": "local-linux",
      "platform": "linux-x64",
      "node_root": "$node_root",
      "restart": false,
      "status_url": "http://127.0.0.1:6632/v1/chain/status"
    },
    {
      "name": "remote-linux",
      "platform": "linux-x64",
      "host": "198.51.100.44",
      "user": "root",
      "node_root": "/opt/oasis7/p2p-testnet",
      "remote_bundle": "/tmp/oasis7-linux-x64-bundle.tar.gz",
      "remote_script": "/opt/oasis7/oasis7/scripts/p2p-public-testnet-package-node-upgrade.sh",
      "restart": true,
      "systemd_service": "oasis7-testnet-storage.service",
      "status_url": "http://127.0.0.1:6632/v1/chain/status"
    },
    {
      "name": "windows-observer",
      "platform": "windows-x64",
      "host": "192.0.2.33",
      "user": "Administrator",
      "deploy_root": "C:\\\\oasis7-deploy",
      "install_root": "C:\\\\Users\\\\Observer\\\\AppData\\\\Local\\\\Programs\\\\oasis7",
      "remote_installer": "C:/oasis7-deploy/oasis7-windows-x64.exe",
      "scheduled_task": "Oasis7Observer",
      "status_url": "http://127.0.0.1:5121/v1/chain/status",
      "rollback_backup_root": "C:\\\\oasis7-deploy\\\\backups\\\\task-2269-fixture",
      "rollback_unlock_timeout_secs": 30
    }
  ]
}
EOF

"$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
  --manifest "$TMP_DIR/manifest.json" \
  --package-dir "$package_dir" \
  --out-dir "$TMP_DIR/plan-only-out" \
  --json >"$TMP_DIR/plan-only.json"

python3 - \
  "$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
  "$TMP_DIR/powershell-injection.ps1" <<'PY'
import importlib.util
from pathlib import Path
import re
import sys

module_path = Path(sys.argv[1])
output_path = Path(sys.argv[2])
spec = importlib.util.spec_from_file_location("package_rollout", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(module)
injected = r"C:\safe'; Write-Output INJECTED; #"
node = {
    key: injected
    for key in (
        "deploy_root",
        "staging_root",
        "install_root",
        "installer_path",
        "configured_installer_path",
        "scheduled_task",
        "status_url",
        "governed_bundle_path",
        "governed_genesis_path",
        "governed_manifest_path",
        "governed_bootstrap_path",
        "active_governed_bundle_path",
        "active_governed_genesis_path",
        "active_governed_manifest_path",
        "active_governed_bootstrap_path",
        "rollback_backup_root",
    )
}
text = module.windows_script(
    node,
    injected,
    injected,
    injected,
    injected,
    {injected: "a" * 64},
    "rpc-running",
)
output_path.write_text(text, encoding="utf-8")
assignments = (
    "$version",
    "$commit",
    "$runId",
    "$artifactRef",
    "$installRoot",
    "$installer",
    "$deployRoot",
    "$stagingRoot",
    "$taskName",
    "$bundlePath",
    "$genesisPath",
    "$manifestPath",
    "$bootstrapPath",
    "$activeBundlePath",
    "$activeGenesisPath",
    "$activeManifestPath",
    "$activeBootstrapPath",
    "$rollbackBackupRoot",
    "$statusUrl",
)
for variable in assignments:
    lines = [line for line in text.splitlines() if line.startswith(variable + " =")]
    assert lines, f"missing generated assignment for {variable}"
    for line in lines:
        assert "''; Write-Output INJECTED; #" in line, (
            f"manifest value was not encoded as one PowerShell single-quoted literal: {line}"
        )
        assert not re.search(r"(?<!')'; Write-Output INJECTED", line), (
            f"manifest value escaped its PowerShell literal: {line}"
        )
integrity_lines = [line for line in text.splitlines() if "INJECTED" in line and " = " in line]
assert integrity_lines
assert all("''; Write-Output INJECTED; #" in line for line in integrity_lines)
PY

node_root_abs=$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).resolve())' "$node_root")
plan_current_target=$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).resolve())' "$(readlink "$node_root/current")")
test "$plan_current_target" = "$node_root_abs/releases/old"
jq -e '
  (.nodes[] | select(.name == "local-linux") | .applied == false)
  and (.nodes[] | select(.name == "remote-linux") | .commands[0] | startswith("scp "))
  and (.nodes[] | select(.name == "remote-linux") | .commands[1] | startswith("ssh root@198.51.100.44 "))
  and (.nodes[] | select(.name == "remote-linux") | .commands[1] | contains("--bundle-tar /tmp/oasis7-linux-x64-bundle.tar.gz"))
  and (.nodes[] | select(.name == "windows-observer") | any(.commands[]; contains("staging_parent_ready=")))
  and (.nodes[] | select(.name == "windows-observer") | .governed_bundle_path | endswith("public-testnet-governed-bootstrap-bundle-2026-06-06.windows.json"))' \
  "$TMP_DIR/plan-only.json" >/dev/null

missing_remote_rollback_manifest="$TMP_DIR/missing-remote-rollback-backup-root.json"
jq '(.nodes[] | select(.name == "windows-observer")) |= del(.rollback_backup_root)' \
  "$TMP_DIR/manifest.json" >"$missing_remote_rollback_manifest"
if "$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
  --manifest "$missing_remote_rollback_manifest" \
  --package-dir "$package_dir" \
  --out-dir "$TMP_DIR/missing-remote-rollback-backup-root-out" \
  >"$TMP_DIR/missing-remote-rollback-backup-root.stdout" \
  2>"$TMP_DIR/missing-remote-rollback-backup-root.stderr"; then
  echo "expected remote Windows node without rollback_backup_root to fail plan generation" >&2
  exit 1
fi
grep -q "windows-observer" "$TMP_DIR/missing-remote-rollback-backup-root.stderr"
grep -q "rollback_backup_root" "$TMP_DIR/missing-remote-rollback-backup-root.stderr"

package_contract_failed=0
if ! python3 - "$TMP_DIR/plan-only.json" <<'PY'
from pathlib import Path, PurePosixPath
import base64
import json
import re
import shlex
import sys

plan = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
windows = next(node for node in plan["nodes"] if node["name"] == "windows-observer")
commands = windows["commands"]


def decode_remote_powershell(command_index: int) -> str:
    command = commands[command_index]
    assert " -Command " not in command and "'\"'\"'" not in command, (
        "remote PowerShell must not embed raw statements through POSIX shell quoting: "
        f"command_index={command_index}"
    )
    tokens = shlex.split(command)
    ssh_index = tokens.index("ssh")
    remote = tokens[ssh_index + 2 :]
    expected_prefix = [
        "powershell.exe",
        "-NoProfile",
        "-NonInteractive",
        "-EncodedCommand",
    ]
    assert remote[:4] == expected_prefix and len(remote) == 5, (
        "remote PowerShell command must use powershell.exe -NoProfile "
        "-NonInteractive -EncodedCommand: "
        f"command_index={command_index} remote_argv={remote[:4]}"
    )
    encoded = remote[4]
    payload = base64.b64decode(encoded, validate=True)
    assert len(payload) % 2 == 0, (
        f"EncodedCommand payload is not UTF-16LE aligned: command_index={command_index}"
    )
    statement = payload.decode("utf-16le")
    assert not statement.startswith("\ufeff"), (
        f"EncodedCommand statement must not contain a BOM: command_index={command_index}"
    )
    assert base64.b64encode(statement.encode("utf-16le")).decode("ascii") == encoded, (
        f"EncodedCommand failed exact UTF-16LE/base64 round-trip: command_index={command_index}"
    )
    return statement


assert (len(commands) - 1) % 3 == 0
for parent_index in range(0, len(commands) - 1, 3):
    scp_index = parent_index + 1
    hash_index = parent_index + 2
    scp_tokens = shlex.split(commands[scp_index])
    remote_path = scp_tokens[-1].split(":", 1)[1]
    parent_path = PurePosixPath(remote_path).parent.as_posix()

    parent_statement = decode_remote_powershell(parent_index)
    assert "staging_parent_ready=" in parent_statement
    assert remote_path in parent_statement and parent_path in parent_statement
    assert "New-Item" in parent_statement

    hash_statement = decode_remote_powershell(hash_index)
    assert "staging_transfer_ack=" in hash_statement
    assert remote_path in hash_statement
    assert "Get-FileHash" in hash_statement and "SHA256" in hash_statement

apply_statement = decode_remote_powershell(len(commands) - 1)
assert re.search(r"task-2269-windows-upgrade\.ps1", apply_statement, re.IGNORECASE), (
    "decoded apply invocation omits the generated Windows upgrade script path"
)

staged_script_paths = {
    shlex.split(command)[-1].split(":", 1)[1]
    for command in commands
    if command.startswith("scp ")
    and shlex.split(command)[-1].lower().endswith("-windows-upgrade.ps1")
}
assert len(staged_script_paths) == 1, (
    "decoded apply contract requires exactly one staged Windows upgrade script: "
    f"paths={sorted(staged_script_paths)}"
)
staged_script_path = next(iter(staged_script_paths))
assert staged_script_path in apply_statement, (
    "decoded apply invocation must pass the exact staged upgrade script path: "
    f"path={staged_script_path!r}"
)

staged_script_transfers = [
    shlex.split(command)
    for command in commands
    if command.startswith("scp ")
    and shlex.split(command)[-1].split(":", 1)[1] == staged_script_path
]
assert len(staged_script_transfers) == 1, (
    "generated Windows upgrade script must have exactly one local transfer source"
)
generated_script_path = Path(staged_script_transfers[0][-2])
generated_script = generated_script_path.read_text(encoding="utf-8")

# Audit the emitted PowerShell rather than the Python template: a single
# backslash in a Python string can collapse a quoted path separator to ''.
empty_path_separator_findings = []
empty_trim_argument = re.compile(
    r"\.Trim(?P<direction>Start|End)\(\s*(?P<quote>['\"])(?P=quote)\s*\)"
)
empty_replace_old_value = re.compile(
    r"\.Replace\(\s*(?P<quote>['\"])(?P=quote)\s*,"
)
empty_boundary_concatenation = re.compile(r"(?:\+\s*''|''\s*\+)")
for line_number, line in enumerate(generated_script.splitlines(), start=1):
    for match in empty_trim_argument.finditer(line):
        empty_path_separator_findings.append(
            (line_number, f"Trim{match.group('direction')}", line.strip())
        )
    for _match in empty_replace_old_value.finditer(line):
        empty_path_separator_findings.append(
            (line_number, "Replace-old-value", line.strip())
        )
    if (
        empty_boundary_concatenation.search(line)
        and re.search(r"(?i)(?:path|root|prefix|suffix|StartsWith|EndsWith)", line)
    ):
        empty_path_separator_findings.append(
            (line_number, "path-boundary-concatenation", line.strip())
        )

# An empty replacement value is legitimate and must not be mistaken for an
# empty path-separator argument.
assert ".Replace('-', '')" in generated_script
assert not empty_path_separator_findings, (
    "generated PowerShell contains empty path-separator literals caused by "
    "Python escape collapse; legitimate empty Replace new-values are allowed: "
    f"findings={empty_path_separator_findings}"
)

tree_metadata_match = re.search(
    r"function Get-TreeMetadata\s*\{(?P<body>.*?)\n\}",
    generated_script,
    re.DOTALL,
)
assert tree_metadata_match, "generated script omits Get-TreeMetadata"
tree_metadata = tree_metadata_match.group("body")
assert ".TrimStart('')" not in tree_metadata, (
    "Python template escape collapse produced an empty TrimStart path separator"
)
assert ".Replace('', '/')" not in tree_metadata, (
    "Python template escape collapse produced an empty Replace path separator"
)
safe_relative_path_expression = (
    r"$file.FullName.Substring($rootItem.FullName.Length)"
    r".TrimStart('\').Replace('\', '/')"
)
assert safe_relative_path_expression in tree_metadata, (
    "Get-TreeMetadata must normalize Windows relative paths with non-empty "
    "literal backslash arguments: "
    f"expected={safe_relative_path_expression!r}"
)

normalized_apply = re.sub(r"\s+", " ", apply_statement).strip()
controlled_process = re.search(
    r"(?i)(?:&\s*)?powershell(?:\.exe)?\s+"
    r"-NoProfile\s+-NonInteractive\s+"
    r"-ExecutionPolicy\s+Bypass\s+-File\s+"
    r"(?P<path_quote>['\"]?)"
    + re.escape(staged_script_path)
    + r"(?P=path_quote)(?:\s|;|$)",
    normalized_apply,
)
assert controlled_process, (
    "decoded apply invocation must launch the staged script through an explicit "
    "powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "
    f"process boundary: statement={apply_statement!r}"
)

tail = normalized_apply[controlled_process.end() :]
assert re.search(r"(?i)(?:;|\|)\s*exit\s+\$LASTEXITCODE\s*$", tail), (
    "decoded apply invocation must wait synchronously and propagate the child "
    "PowerShell exit code exactly through SSH"
)

apply_tokens = shlex.split(commands[-1])
apply_ssh_index = apply_tokens.index("ssh")
apply_target = apply_tokens[apply_ssh_index + 1]
expected_target = "Administrator@192.0.2.33"
assert apply_target == expected_target, (
    "generated apply SSH destination must be the exact user@host target with no "
    f"suffix or trailing colon: expected={expected_target!r} actual={apply_target!r}"
)
assert not apply_target.endswith(":")
PY
then
  package_contract_failed=1
fi

if ! python3 - "$TMP_DIR/plan-only.json" <<'PY'
from pathlib import Path, PurePosixPath
import json
import re
import shlex
import sys

plan = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
windows = next(node for node in plan["nodes"] if node["name"] == "windows-observer")
commands = windows["commands"]
transfer_commands = [command for command in commands if command.startswith("scp ")]
assert transfer_commands, "Windows rollout fixture generated no transfers"

active_paths = (
    "C:/oasis7-deploy/config",
    "C:/oasis7-deploy/oasis7-windows-x64.exe",
    "C:/Users/Observer/AppData/Local/Programs/oasis7",
)
staging_roots = set()
transferred_names = set()
for command in transfer_commands:
    destination = shlex.split(command)[-1]
    remote_target, separator, remote_path = destination.partition(":")
    assert separator and remote_target == "Administrator@192.0.2.33", (
        f"Windows transfer target is not the exact observer host: {destination}"
    )
    normalized = PurePosixPath(remote_path).as_posix()
    assert not any(
        normalized == active or normalized.startswith(active + "/")
        for active in active_paths
    ), f"pre-apply transfer mutates an active deployment path: {normalized}"
    match = re.match(
        r"(?i)^(C:/oasis7-deploy/(?:\.staging|staging|attempts)/([^/]+))/(?:.+)$",
        normalized,
    )
    assert match, (
        "every pre-apply transfer must use an attempt-specific independent staging root: "
        f"path={normalized}"
    )
    attempt_id = match.group(2)
    assert attempt_id.lower() not in {"current", "config", "runtime", "bin"}
    staging_roots.add(match.group(1).lower())
    transferred_names.add(PurePosixPath(normalized).name)

assert len(staging_roots) == 1, (
    "one rollout plan must stage its complete closure under exactly one attempt root: "
    f"roots={sorted(staging_roots)}"
)
required_names = {
    "oasis7-windows-x64.exe",
    "public-testnet-governed-bootstrap-bundle-2026-06-06.windows.json",
    "public-testnet-governed-bootstrap-genesis-2026-06-06.windows.json",
    "public-testnet-governed-bootstrap-manifest-2026-06-06.windows.json",
    "public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.windows.txt",
    "windows-observer-windows-upgrade.ps1",
}
missing = sorted(required_names - transferred_names)
assert not missing, f"fresh Windows staging closure omits required files: {missing}"
PY
then
  package_contract_failed=1
fi

if ! python3 - "$package_dir/windows/windows-x64-SHA256SUMS" <<'PY'
from pathlib import Path, PurePosixPath
import sys

sums = Path(sys.argv[1])
covered = {
    PurePosixPath(line.split(maxsplit=1)[1].lstrip("*")).as_posix()
    for line in sums.read_text(encoding="utf-8").splitlines()
    if line.strip()
}
required = {
    "oasis7-windows-x64.exe",
    "public-testnet-governed-bootstrap-bundle-2026-06-06.windows.json",
    "public-testnet-governed-bootstrap-genesis-2026-06-06.windows.json",
    "public-testnet-governed-bootstrap-manifest-2026-06-06.windows.json",
    "public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.windows.txt",
    "generated-world/world/snapshot.json",
    "generated-world/world/journal.json",
    "generated-world/generated-scenario-world/snapshot.json",
    "generated-world/generated-scenario-world/journal.json",
    "generated-world/generated-scenario-world/chunks/0001.json",
    "generated-world/world-generation-provenance.json",
    "doc/testing/evidence/governance-public-signers.json",
    "doc/testing/evidence/liveops-public-signers.json",
    "doc/testing/evidence/signer-truth-binding.md",
    "doc/testing/evidence/genesis-validator-registry.json",
    "doc/testing/evidence/governed-bootstrap-topology.md",
}
missing = sorted(required - covered)
assert not missing, f"Windows runtime truth fixture lacks checksum coverage: {missing}"
PY
then
  package_contract_failed=1
fi

if ! python3 - "$TMP_DIR/plan-only.json" <<'PY'
from pathlib import PurePosixPath
from pathlib import Path
import json
import shlex
import sys

plan = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
windows = next(node for node in plan["nodes"] if node["name"] == "windows-observer")
commands = windows["commands"]
transfer_indices = [index for index, command in enumerate(commands) if command.startswith("scp ")]
assert transfer_indices, "Windows rollout fixture generated no transfers"
for transfer_index in transfer_indices:
    transfer = shlex.split(commands[transfer_index])
    remote_path = PurePosixPath(transfer[-1].split(":", 1)[1]).as_posix()
    parent = PurePosixPath(remote_path).parent.as_posix()
    parent_preflight = commands[:transfer_index]
    assert any(
        parent in command
        and ("New-Item -ItemType Directory" in command or "mkdir" in command)
        and "staging_parent_ready=" in command
        for command in parent_preflight
    ), f"staging parent not created before transfer: parent={parent} path={remote_path}"
PY
then
  package_contract_failed=1
fi

if ! python3 - "$TMP_DIR/plan-only.json" <<'PY'
from pathlib import Path, PurePosixPath
import hashlib
import json
import shlex
import sys

plan = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
windows = next(node for node in plan["nodes"] if node["name"] == "windows-observer")
commands = windows["commands"]
apply_index = len(commands) - 1
assert "ssh " in commands[apply_index] and "powershell" in commands[apply_index]
transfer_indices = [index for index, command in enumerate(commands) if command.startswith("scp ")]
for transfer_index in transfer_indices:
    transfer = shlex.split(commands[transfer_index])
    source = Path(transfer[1])
    remote_path = PurePosixPath(transfer[-1].split(":", 1)[1]).as_posix()
    expected_sha256 = hashlib.sha256(source.read_bytes()).hexdigest()
    closure_commands = commands[transfer_index + 1 : apply_index]
    assert any(
        remote_path in command
        and expected_sha256 in command
        and "Get-FileHash" in command
        and "staging_transfer_ack=" in command
        and "throw" in command
        for command in closure_commands
    ), (
        "transfer lacks explicit success acknowledgment and remote hash closure before apply: "
        f"path={remote_path} sha256={expected_sha256}"
    )
assert all(
    token not in command
    for command in commands[:apply_index]
    for token in ("Stop-ScheduledTask", "Set-ScheduledTask", "Start-Process")
), "apply/mutation command appeared before every staged transfer was acknowledged"
PY
then
  package_contract_failed=1
fi

if ! python3 - "$TMP_DIR/plan-only.json" "$TMP_DIR/plan-only-out/windows-observer-windows-upgrade.ps1" <<'PY'
from pathlib import Path
import base64
import json
import shlex
import sys

plan = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
script = Path(sys.argv[2]).read_text(encoding="utf-8")
windows = next(node for node in plan["nodes"] if node["name"] == "windows-observer")
commands = windows["commands"]
apply_index = len(commands) - 1

transfer_indices = [index for index, command in enumerate(commands) if command.startswith("scp ")]
hash_ack_indices = [
    index for index, command in enumerate(commands) if "staging_transfer_ack=" in command
]
assert transfer_indices and len(hash_ack_indices) == len(transfer_indices), (
    "apply must be preceded by one remote SHA acknowledgment per staged transfer"
)
assert max(hash_ack_indices) < apply_index, (
    "remote apply appeared before the complete staged SHA closure"
)

apply_tokens = shlex.split(commands[apply_index])
ssh_index = apply_tokens.index("ssh")
remote_argv = apply_tokens[ssh_index + 2 :]
assert remote_argv[:4] == [
    "powershell.exe", "-NoProfile", "-NonInteractive", "-EncodedCommand"
]
apply_statement = base64.b64decode(remote_argv[4], validate=True).decode("utf-16le")
assert any(token in apply_statement.lower() for token in ("/staging/", "/.staging/", "/attempts/")), (
    "remote apply must execute the upgrade script from the attempt-specific staging root"
)

closure_marker = "staged_sha_closure_complete=true"
promotion_begin = "promotion_begin=true"
promotion_complete = "promotion_complete=true"
for token in (closure_marker, promotion_begin, promotion_complete):
    assert token in script, f"remote apply omits transactional promotion evidence: {token}"
assert script.index(closure_marker) < script.index(promotion_begin) < script.index(promotion_complete), (
    "active deployment promotion must occur only after the full staged SHA closure"
)
pre_promotion = script[: script.index(promotion_begin)]
for mutation in ("Stop-ScheduledTask", "Start-Process"):
    assert mutation not in pre_promotion, (
        f"active mutation {mutation} occurred before staged closure and promotion"
    )
PY
then
  package_contract_failed=1
fi

if ! python3 - "$TMP_DIR/plan-only-out/windows-observer-windows-upgrade.ps1" <<'PY'
from pathlib import Path
import re
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
rollback = text[text.index("rollback_begin=true"):]

runtime_decision_marker = "rollback_runtime_restore_required="
component_restored_marker = "rollback_component_restored="
component_unchanged_marker = "rollback_component_unchanged="
for token in (
    runtime_decision_marker,
    component_restored_marker,
    component_unchanged_marker,
):
    assert token in rollback, f"rollback omits changed-component evidence: {token}"

runtime_decision_index = rollback.index(runtime_decision_marker)
unlock_deadline_index = rollback.index("$rollbackFileUnlockDeadline")
exclusive_open_index = rollback.index("[System.IO.FileShare]::None", unlock_deadline_index)
runtime_copy_match = re.search(
    r"Copy-Item\s+-LiteralPath\s+\$rollbackRuntimeSourceAtRestore\s+"
    r"-Destination\s+\$runtime\s+-Force",
    rollback,
)
assert runtime_copy_match, "rollback runtime restore copy is missing"
runtime_copy_index = runtime_copy_match.start()
assert runtime_decision_index < unlock_deadline_index < exclusive_open_index < runtime_copy_index, (
    "runtime hash decision must guard any lock probe and runtime replacement"
)
runtime_guard = rollback[runtime_decision_index:runtime_copy_index]
assert re.search(
    r"if\s*\(\s*\$rollbackRuntimeRestoreRequired\s*\)",
    runtime_guard,
    re.IGNORECASE,
), (
    "runtime unlock/replacement must be conditional on installed and backup SHA mismatch; "
    "an equal-hash runtime lock is irrelevant to config-only rollback"
)
for token in ("Get-FileHash", "$rollbackRuntimeSource", "$runtime"):
    assert token in rollback[:unlock_deadline_index], (
        f"runtime restore decision must close installed and backup SHA before lock polling: {token}"
    )

config_loop_index = rollback.index("foreach ($rollbackConfigTarget in $rollbackConfigTargets)")
config_loop = rollback[config_loop_index:]
for token in ("Get-FileHash", "$rollbackConfigSource", "$rollbackConfigTarget"):
    assert token in config_loop, (
        f"rollback must compare each config component before replacement: {token}"
    )
assert re.search(
    r"if\s*\([^)]*(?:Sha256|Hash)[^)]*-ne[^)]*(?:Sha256|Hash)[^)]*\).*?"
    r"Copy-Item\s+-LiteralPath\s+\$rollbackConfigSource\s+"
    r"-Destination\s+\$rollbackConfigTarget\s+-Force",
    config_loop,
    re.IGNORECASE | re.DOTALL,
), "rollback must restore each config only when its installed SHA differs from backup"
PY
then
  package_contract_failed=1
fi

if ! python3 - "$TMP_DIR/plan-only-out/windows-observer-windows-upgrade.ps1" <<'PY'
from pathlib import Path
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
try_start = text.index("try {\nInvoke-RolloutFailureInjection -Phase 'installer'")
catch_start = text.index("} catch {\n  $postStopFailure = $_", try_start)
mutation_boundary = text[try_start:catch_start]
phases = (
    "installer",
    "governed_copy",
    "bundle_move",
    "genesis_move",
    "manifest_move",
    "current_version_write",
    "deployed_buildinfo_write",
    "task_action_restore",
)
for phase in phases:
    token = f"Invoke-RolloutFailureInjection -Phase '{phase}'"
    assert token in mutation_boundary, (
        f"post-stop mutation lacks rollback-boundary failure injection: {phase}"
    )
assert "if (!$script:rollbackInvoked)" in text[catch_start:], (
    "post-stop catch must guard against duplicate rollback"
)
rollback_function = text[
    text.index("function Invoke-KnownGoodRollback"):
    text.index("function Invoke-RolloutFailureInjection")
]
assert "$script:rollbackInvoked = $true" in rollback_function
assert "rollback_already_invoked=true" in rollback_function
assert "Invoke-KnownGoodRollback" in text[catch_start:], (
    "post-stop catch does not invoke known-good rollback"
)
PY
then
  package_contract_failed=1
fi

for missing_runtime_truth in generated_world_sidecar world_generation_provenance; do
  missing_package="$TMP_DIR/missing-$missing_runtime_truth-package"
  cp -R "$package_dir" "$missing_package"
  missing_windows="$missing_package/windows"
  missing_bundle="$missing_windows/$(basename "$windows_bundle")"
  missing_manifest="$missing_windows/$(basename "$windows_manifest")"
  if [[ "$missing_runtime_truth" == "generated_world_sidecar" ]]; then
    jq 'del(.runtime_refs.generated_world_sidecar_ref)' \
      "$missing_manifest" >"$missing_manifest.tmp"
    mv "$missing_manifest.tmp" "$missing_manifest"
    jq 'del(.generated_world_sidecar)' "$missing_bundle" >"$missing_bundle.tmp"
    mv "$missing_bundle.tmp" "$missing_bundle"
    rm -rf "$missing_windows/generated-world/generated-scenario-world"
  else
    jq 'del(.runtime_refs.world_generation_provenance_ref)' \
      "$missing_manifest" >"$missing_manifest.tmp"
    mv "$missing_manifest.tmp" "$missing_manifest"
    jq 'del(.world_generation_provenance)' "$missing_bundle" >"$missing_bundle.tmp"
    mv "$missing_bundle.tmp" "$missing_bundle"
    rm "$missing_windows/generated-world/world-generation-provenance.json"
  fi
  python3 - "$missing_bundle" "$missing_manifest" <<'PY'
from pathlib import Path
import hashlib
import json
import sys

bundle_path = Path(sys.argv[1])
manifest_path = Path(sys.argv[2])
bundle = json.loads(bundle_path.read_text(encoding="utf-8"))
manifest_meta = bundle["network_manifest"]
manifest_meta["sha256"] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
manifest_meta["size_bytes"] = manifest_path.stat().st_size
bundle_path.write_text(json.dumps(bundle, indent=2) + "\n", encoding="utf-8")
PY
  (
    cd "$missing_windows"
    find . -type f ! -name windows-x64-SHA256SUMS -print \
      | LC_ALL=C sort \
      | sed 's#^./##' \
      | while IFS= read -r path; do shasum -a 256 "$path"; done \
      >windows-x64-SHA256SUMS
  )
  if "$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
    --manifest "$TMP_DIR/manifest.json" \
    --package-dir "$missing_package" \
    --out-dir "$TMP_DIR/missing-$missing_runtime_truth-out" \
    >"$TMP_DIR/missing-$missing_runtime_truth.stdout" \
    2>"$TMP_DIR/missing-$missing_runtime_truth.stderr"; then
    echo "expected public_testnet Windows source missing $missing_runtime_truth to fail" >&2
    package_contract_failed=1
  elif ! grep -q "$missing_runtime_truth" \
    "$TMP_DIR/missing-$missing_runtime_truth.stderr"; then
    echo "missing-$missing_runtime_truth rejection did not identify the missing contract" >&2
    cat "$TMP_DIR/missing-$missing_runtime_truth.stderr" >&2
    package_contract_failed=1
  fi
done

for escaped_field in generated_world_sidecar world_generation_provenance; do
  for escaped_ref_kind in traversal absolute; do
    escaped_package="$TMP_DIR/escaped-$escaped_field-$escaped_ref_kind-package"
    cp -R "$package_dir" "$escaped_package"
    escaped_windows="$escaped_package/windows"
    escaped_bundle="$escaped_windows/$(basename "$windows_bundle")"
    if [[ "$escaped_field" == "generated_world_sidecar" ]]; then
      escaped_source="$TMP_DIR/escaped-$escaped_field-$escaped_ref_kind"
      mkdir -p "$escaped_source"
      printf 'escaped sidecar fixture\n' >"$escaped_source/snapshot.json"
    else
      escaped_source="$TMP_DIR/escaped-$escaped_field-$escaped_ref_kind.json"
      printf '{"escaped":true}\n' >"$escaped_source"
    fi
    if [[ "$escaped_ref_kind" == "traversal" ]]; then
      escaped_ref="../$(basename "$escaped_source")"
      # Make the traversal resolve outside windows/ while retaining an existing source.
      cp -R "$escaped_source" "$escaped_package/$(basename "$escaped_source")"
    else
      escaped_ref="$escaped_source"
    fi
    python3 - "$escaped_bundle" "$escaped_field" "$escaped_ref" <<'PY'
from pathlib import Path
import json
import sys

bundle_path = Path(sys.argv[1])
field = sys.argv[2]
raw_ref = sys.argv[3]
bundle = json.loads(bundle_path.read_text(encoding="utf-8"))
bundle[field]["ref"] = raw_ref
bundle_path.write_text(json.dumps(bundle, indent=2) + "\n", encoding="utf-8")
PY
    (
      cd "$escaped_windows"
      find . -type f ! -name windows-x64-SHA256SUMS -print \
        | LC_ALL=C sort \
        | sed 's#^./##' \
        | while IFS= read -r path; do shasum -a 256 "$path"; done \
        >windows-x64-SHA256SUMS
    )
    escaped_out="$TMP_DIR/escaped-$escaped_field-$escaped_ref_kind-out"
    if "$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
      --manifest "$TMP_DIR/manifest.json" \
      --package-dir "$escaped_package" \
      --out-dir "$escaped_out" \
      >"$escaped_out.stdout" 2>"$escaped_out.stderr"; then
      echo "expected escaped Windows $escaped_field ref to fail before plan transfer generation" >&2
      package_contract_failed=1
    elif ! grep -q 'Windows runtime truth ref escapes platform closure' "$escaped_out.stderr"; then
      echo "escaped Windows $escaped_field ref did not fail with the platform-closure diagnostic" >&2
      cat "$escaped_out.stderr" >&2
      package_contract_failed=1
    fi
    if [[ -e "$escaped_out/windows-observer-windows-upgrade.ps1" \
      || -e "$escaped_out/rollout-plan.json" ]]; then
      echo "escaped Windows $escaped_field ref generated a transfer/apply plan before rejection" >&2
      package_contract_failed=1
    fi
  done
done

# A runtime-truth directory may not smuggle a host-owned member into the
# staged closure through either kind of symlink.  Both must fail before the
# rollout helper emits a plan, PowerShell script, or transfer command.
for symlink_kind in file directory; do
  symlink_package="$TMP_DIR/runtime-truth-$symlink_kind-symlink-package"
  cp -R "$package_dir" "$symlink_package"
  symlink_windows="$symlink_package/windows"
  symlink_sidecar="$symlink_windows/generated-world/generated-scenario-world"
  external_root="$TMP_DIR/runtime-truth-$symlink_kind-external"
  mkdir -p "$external_root"
  if [[ "$symlink_kind" == "file" ]]; then
    printf 'outside platform root\n' >"$external_root/outside.json"
    ln -s "$external_root/outside.json" "$symlink_sidecar/outside-file-link.json"
  else
    mkdir -p "$external_root/outside-dir"
    printf 'outside platform root\n' >"$external_root/outside-dir/outside.json"
    ln -s "$external_root/outside-dir" "$symlink_sidecar/outside-directory-link"
  fi

  symlink_out="$TMP_DIR/runtime-truth-$symlink_kind-symlink-out"
  if "$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
    --manifest "$TMP_DIR/manifest.json" \
    --package-dir "$symlink_package" \
    --out-dir "$symlink_out" \
    >"$symlink_out.stdout" 2>"$symlink_out.stderr"; then
    echo "expected runtime-truth $symlink_kind symlink to fail before rollout generation" >&2
    package_contract_failed=1
  elif ! grep -q 'Windows runtime truth tree contains symlink for generated_world_sidecar' \
    "$symlink_out.stderr"; then
    echo "runtime-truth $symlink_kind symlink did not produce the stable containment diagnostic" >&2
    cat "$symlink_out.stderr" >&2
    package_contract_failed=1
  fi
  if [[ -e "$symlink_out/windows-observer-windows-upgrade.ps1" \
    || -e "$symlink_out/rollout-plan.json" ]]; then
    echo "runtime-truth $symlink_kind symlink generated rollout output before rejection" >&2
    package_contract_failed=1
  fi
done

# Top-level governed truth is parsed before the recursive artifact graph, so it
# needs the same no-symlink contract as every nested runtime-truth member.
for governed_name in \
  public-testnet-governed-bootstrap-bundle-2026-06-06.windows.json \
  public-testnet-governed-bootstrap-genesis-2026-06-06.windows.json \
  public-testnet-governed-bootstrap-manifest-2026-06-06.windows.json \
  public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.windows.txt; do
  symlink_package="$TMP_DIR/governed-$(basename "$governed_name")-symlink-package"
  cp -R "$package_dir" "$symlink_package"
  governed_path="$symlink_package/windows/$governed_name"
  mv "$governed_path" "$governed_path.real"
  ln -s "$(basename "$governed_path").real" "$governed_path"
  symlink_out="$TMP_DIR/governed-$(basename "$governed_name")-symlink-out"
  if "$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
    --manifest "$TMP_DIR/manifest.json" \
    --package-dir "$symlink_package" \
    --out-dir "$symlink_out" \
    >"$symlink_out.stdout" 2>"$symlink_out.stderr"; then
    echo "expected top-level governed symlink to fail before rollout generation: $governed_name" >&2
    package_contract_failed=1
  elif ! grep -q 'Windows governed bootstrap artifact contains symlink component' \
    "$symlink_out.stderr"; then
    echo "top-level governed symlink did not produce stable rejection: $governed_name" >&2
    cat "$symlink_out.stderr" >&2
    package_contract_failed=1
  fi
  if [[ -e "$symlink_out/windows-observer-windows-upgrade.ps1" \
    || -e "$symlink_out/rollout-plan.json" ]]; then
    echo "top-level governed symlink generated rollout output: $governed_name" >&2
    package_contract_failed=1
  fi
done

ancestor_package="$TMP_DIR/governed-ancestor-symlink-package"
cp -R "$package_dir" "$ancestor_package"
mv "$ancestor_package/windows" "$ancestor_package/windows-real"
ln -s windows-real "$ancestor_package/windows"
ancestor_out="$TMP_DIR/governed-ancestor-symlink-out"
if "$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
  --manifest "$TMP_DIR/manifest.json" \
  --package-dir "$ancestor_package" \
  --out-dir "$ancestor_out" \
  >"$ancestor_out.stdout" 2>"$ancestor_out.stderr"; then
  echo "expected governed ancestor symlink to fail before rollout generation" >&2
  package_contract_failed=1
elif ! grep -Eq 'Windows governed bootstrap artifact contains symlink component|platform package path contains symlink component' \
  "$ancestor_out.stderr"; then
  echo "governed ancestor symlink did not produce stable rejection" >&2
  cat "$ancestor_out.stderr" >&2
  package_contract_failed=1
fi
if [[ -e "$ancestor_out/windows-observer-windows-upgrade.ps1" \
  || -e "$ancestor_out/rollout-plan.json" ]]; then
  echo "governed ancestor symlink generated rollout output" >&2
  package_contract_failed=1
fi

"$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
  --manifest "$TMP_DIR/manifest.json" \
  --package-dir "$package_dir" \
  --out-dir "$out_dir" \
  --apply-local \
  --json >"$TMP_DIR/plan.json"

test "$(readlink "$node_root_abs/current")" = "$node_root_abs/releases/$package_version"
grep -q "^package_version=$package_version$" "$node_root_abs/DEPLOYED_BUILDINFO"
jq -e \
  --arg commit "$commit" \
  --arg version "$package_version" \
  '.commit == $commit
    and .package_version == $version
    and .readiness_policy == "rpc-running"
    and (.nodes[] | select(.name == "local-linux") | .applied == true)
    and (.nodes[] | select(.name == "windows-observer") | .windows_script | endswith("windows-observer-windows-upgrade.ps1"))' \
  "$TMP_DIR/plan.json" >/dev/null

windows_script="$out_dir/windows-observer-windows-upgrade.ps1"
test -f "$windows_script"
if ! python3 - "$windows_script" <<'PY'
from pathlib import Path
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
preflight_index = text.index("foreach ($entry in $expectedGovernedSha256.GetEnumerator())")
attempt_id_index = text.find("$attemptId")
assert 0 <= attempt_id_index < preflight_index, (
    "staging/preflight diagnostics must be initialized before checksum preflight"
)
for token in ("$attemptStdoutPath", "$attemptStderrPath", "$attemptExitMarkerPath"):
    assert text.find(token) < preflight_index, (
        f"unique staging/preflight diagnostic initialized too late: {token}"
    )
preflight_diagnostic_tokens = (
    "failure_phase=staging_preflight",
    "staging_preflight_failed_path=$($entry.Key)",
    "staging_preflight_error=$($_.Exception.Message)",
    "AppendAllText($attemptStderrPath",
    "AppendAllText($attemptExitMarkerPath",
    "rollback_required=true",
)
for token in preflight_diagnostic_tokens:
    assert token in text, (
        "staging/preflight failure must preserve exact failed path/error in unique attempt "
        f"diagnostics: missing={token}"
    )
PY
then
  package_contract_failed=1
fi

if ! python3 - \
  "$windows_script" \
  "$rollback_backup_fixture" \
  "$actual_rollback_backup_fixture" \
  "$legacy_rollback_backup_fixture" <<'PY'
from pathlib import Path, PureWindowsPath
import hashlib
import json
import re
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
backup_root = Path(sys.argv[2])
actual_backup_root = Path(sys.argv[3])
legacy_backup_root = Path(sys.argv[4])
backup_manifest = json.loads(
    (backup_root / "backup-manifest.json").read_text(encoding="utf-8")
)
actual_backup_manifest = json.loads(
    (actual_backup_root / "backup-manifest.json").read_text(encoding="utf-8")
)
legacy_backup_manifest = json.loads(
    (legacy_backup_root / "backup-manifest.json").read_text(encoding="utf-8")
)
assert (backup_root / "runtime/oasis7_chain_runtime.exe").is_file()
assert not (backup_root / "bin/oasis7_chain_runtime.exe").exists()
assert backup_manifest["runtime"]["relative_path"] == (
    "runtime/oasis7_chain_runtime.exe"
)
assert backup_manifest["runtime"]["sha256"] == hashlib.sha256(
    (backup_root / "runtime/oasis7_chain_runtime.exe").read_bytes()
).hexdigest()
assert actual_backup_manifest["runtime_path"] == r"runtime\oasis7_chain_runtime.exe"
assert actual_backup_manifest["runtime_sha256"] == hashlib.sha256(
    (actual_backup_root / "runtime/oasis7_chain_runtime.exe").read_bytes()
).hexdigest()
assert not (actual_backup_root / "bin/oasis7_chain_runtime.exe").exists()
legacy_runtime_path = PureWindowsPath(legacy_backup_manifest["runtime_path"])
assert legacy_runtime_path.is_absolute()
assert legacy_runtime_path.parts[0].lower() != "runtime"
assert legacy_backup_manifest["runtime_sha256"] == hashlib.sha256(
    (legacy_backup_root / "runtime/oasis7_chain_runtime.exe").read_bytes()
).hexdigest()
assert not (legacy_backup_root / "bin/oasis7_chain_runtime.exe").exists()
assert "$rollbackBackupRoot = [Environment]::ExpandEnvironmentVariables('C:\\oasis7-deploy\\backups\\task-2269-fixture')" in text, (
    "Windows rollback omits the configured known-good backup root"
)
resolver_match = re.search(
    r"function Resolve-RollbackRuntimeSource\s*\{(?P<body>.*?)\n\}",
    text,
    re.DOTALL,
)
assert resolver_match, (
    "Windows rollback must resolve authorized runtime/ backup layout through "
    "manifest/provenance before deterministic fallback candidates"
)
resolver = resolver_match.group("body")
counted_candidates = set(re.findall(r"\$(\w+)\.Count", resolver))
indexed_candidates = set(re.findall(r"\$(\w+)\[0\]", resolver))
pipeline_candidates = sorted(counted_candidates & indexed_candidates)
assert {"metadataPaths", "legacyConfinedCandidates", "fallbackCandidates"}.issubset(
    pipeline_candidates
), "resolver candidate-pipeline fixture no longer covers all known Count/index consumers"
unsafe_scalar_pipelines = []
for variable in pipeline_candidates:
    assignment = re.search(
        rf"\${variable}\s*=\s*(?P<expression>.*?)"
        rf"(?=\n\s*if\s*\(\${variable}\.Count)",
        resolver,
        re.DOTALL,
    )
    assert assignment, f"could not isolate candidate pipeline assignment: {variable}"
    expression = assignment.group("expression").strip()
    if "|" in expression and not (
        expression.startswith("@(") and expression.endswith(")")
    ):
        unsafe_scalar_pipelines.append(variable)
assert not unsafe_scalar_pipelines, (
    "PowerShell resolver candidate pipelines consumed by .Count/[0] must wrap "
    "the complete pipeline in @(...), so one candidate remains a full path: "
    f"unsafe={unsafe_scalar_pipelines}"
)
assert "runtime_path" in resolver and "runtime_sha256" in resolver, (
    "Windows rollback resolver must accept authorized top-level "
    "runtime_path/runtime_sha256 manifest schema"
)
for token in ("GetFullPath", "StartsWith", "Get-FileHash"):
    assert token in resolver, (
        "top-level rollback runtime metadata must retain path confinement and SHA closure: "
        f"missing={token}"
    )
legacy_rooted_tokens = (
    "legacy rooted runtime_path",
    "confined backup runtime candidate missing",
    "confined backup runtime candidate ambiguous",
    "confined backup runtime sha256 mismatch",
)
missing_legacy_rooted_tokens = [
    token for token in legacy_rooted_tokens if token not in resolver
]
assert not missing_legacy_rooted_tokens, (
    "Windows rollback resolver must map an escaped legacy runtime_path only to "
    "a unique confined SHA-matching backup candidate: "
    f"missing={missing_legacy_rooted_tokens}"
)
manifest_index = resolver.find("backup-manifest.json")
runtime_candidate_index = resolver.find(r"runtime\oasis7_chain_runtime.exe")
bin_candidate_index = resolver.find(r"bin\oasis7_chain_runtime.exe")
assert 0 <= manifest_index < runtime_candidate_index < bin_candidate_index, (
    "rollback runtime resolution order must be backup manifest/provenance, "
    "runtime/oasis7_chain_runtime.exe, then bin/oasis7_chain_runtime.exe"
)
assert "ConvertFrom-Json" in resolver, (
    "rollback runtime resolver must consume available backup manifest provenance"
)
assert re.search(r"runtime.*(relative_path|path)", resolver, re.IGNORECASE | re.DOTALL), (
    "rollback runtime resolver must accept a manifest-provenance runtime path"
)
assert "Get-FileHash" in resolver, (
    "rollback runtime resolver must close an available provenance SHA before restore"
)
assert "rollback runtime ambiguous" in resolver, (
    "rollback runtime resolver must reject multiple valid runtime candidates"
)
assert "known-good rollback runtime missing" in resolver, (
    "rollback runtime resolver must reject missing manifest and fallback candidates"
)
assert text.count("Resolve-RollbackRuntimeSource") >= 3, (
    "rollback preflight and restoration must use the same runtime resolver"
)
timeout_match = re.search(r"\$rollbackUnlockTimeoutSeconds\s*=\s*(\d+)", text)
assert timeout_match, "Windows rollback must define a bounded process-exit/file-unlock timeout"
timeout_seconds = int(timeout_match.group(1))
assert timeout_seconds == 30, (
    "Windows rollback ignored rollback_unlock_timeout_secs fixture: "
    f"expected=30 actual={timeout_seconds}"
)
rollback_index = text.index("rollback_begin=true")
rollback = text[rollback_index:]
PY
then
  package_contract_failed=1
fi

if ! python3 - "$windows_script" <<'PY'
from pathlib import Path
import re
import sys

data = Path(sys.argv[1]).read_bytes()
assert not data.startswith(b"\xef\xbb\xbf"), "PowerShell script must be UTF-8 without BOM"
text = data.decode("utf-8")
assert "Set-JsonProperty $json.runtime_build 'sha256' $hash" in text
assert "$installer = [Environment]::ExpandEnvironmentVariables('C:/oasis7-deploy/oasis7-windows-x64.exe')" in text
assert 'throw "governed bundle missing runtime_build' in text
assert "[System.Text.UTF8Encoding]::new($false)" in text
assert "Start-ScheduledTask -TaskName $taskName" in text
assert "public-testnet-governed-bootstrap-bundle-2026-06-06.windows.json" in text
assert "-Filter '*bundle*.json'" not in text
governed_keys = (
    "governance_public_manifest_ref",
    "liveops_public_manifest_ref",
    "binding_notes_ref",
    "genesis_validator_registry_ref",
    "topology_ref",
)
for key in governed_keys:
    assert key in text, f"Windows deployment omits governed ref localization for {key}"
assert "governance_bootstrap_refs" in text
assert "doc\\testing\\evidence" in text
localization_index = text.index("governance_bootstrap_refs")
missing_source_index = text.index("genesis governance ref source missing")
collision_index = text.index("genesis governance refs collide at localized target")
genesis_lookup_index = text.index("$genesis = Get-Item $genesisPath")
stop_index = text.index("Stop-ScheduledTask -TaskName $taskName")
install_index = text.index("$install = Start-Process")
start_index = text.index("Start-ScheduledTask -TaskName $taskName")
assert localization_index < start_index, "governed refs must be localized before observer start"
for label, index in (
    ("genesis lookup", genesis_lookup_index),
    ("missing-source preflight", missing_source_index),
    ("basename-collision preflight", collision_index),
):
    assert index < stop_index, f"{label} must complete before Stop-ScheduledTask"
    assert index < install_index, f"{label} must complete before installer mutation"
assert text.count("[System.Text.UTF8Encoding]::new($false)") >= 2, (
    "bundle and localized genesis must both be emitted as UTF-8 without BOM"
)

bundle_path_sections = (
    "runtime_build",
    "world_snapshot",
    "generated_world_sidecar",
    "world_generation_provenance",
    "governance_manifest",
    "network_manifest",
    "evidence_refs",
)
manifest_runtime_refs = (
    "release_candidate_bundle_ref",
    "genesis_ref",
    "bootstrap_peer_ref",
    "generated_world_sidecar_ref",
    "world_generation_provenance_ref",
)
for field in bundle_path_sections + manifest_runtime_refs:
    assert field in text, f"Windows deployment omits path localization for {field}"
assert "public-testnet-governed-bootstrap-manifest-2026-06-06.windows.json" in text
assert "public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.windows.txt" in text
assert "generated-world\\world" in text
assert "generated-world\\generated-scenario-world" in text
assert "build-host absolute path survived Windows localization" in text
assert "D:\\a\\oasis7" not in text, "generated PowerShell embeds a build-host path"

tree_contract_tokens = (
    "sha256_tree",
    "file_count",
    "total_bytes",
    "Get-ChildItem",
    "-Recurse",
    "world_snapshot tree integrity mismatch",
    "generated_world_sidecar tree integrity mismatch",
)
for token in tree_contract_tokens:
    assert token in text, f"Windows deployment omits tree integrity contract token: {token}"
path_localization_index = min(text.index(field) for field in bundle_path_sections)
tree_integrity_index = min(
    text.index("world_snapshot tree integrity mismatch"),
    text.index("generated_world_sidecar tree integrity mismatch"),
)
assert path_localization_index < stop_index
assert tree_integrity_index < stop_index
assert path_localization_index < install_index
assert tree_integrity_index < install_index

timeout_match = re.search(r"\$verificationTimeoutSeconds\s*=\s*(\d+)", text)
assert timeout_match, "rpc-running verification must define a bounded startup timeout"
timeout_seconds = int(timeout_match.group(1))
assert 60 <= timeout_seconds <= 300, (
    "rpc-running verification timeout must allow normal startup without becoming unbounded: "
    f"{timeout_seconds}s"
)
assert "$verificationDeadline = (Get-Date).AddSeconds($verificationTimeoutSeconds)" in text
assert "while ((Get-Date) -lt $verificationDeadline)" in text
assert re.search(
    r"\$processRunning\s*=.*Get-Process\s+oasis7_chain_runtime",
    text,
), "rpc-running process evidence must come from the observer process"
assert re.search(
    r"\$statusRunning\s*=\s*\$status\.running\s*-eq\s*\$true",
    text,
), "rpc-running status evidence must require status running=true"
assert re.search(
    r"if\s*\(\s*\$processRunning\s+-and\s+\$statusRunning\s*\)",
    text,
), "rpc-running verification may succeed only when process and RPC status are both running"
assert "STATUS_ERROR=" not in text, "RPC failures must not be caught and printed as success"
deadline_failure_match = re.search(
    r"if\s*\(\s*!\$verified\s*\)\s*\{(?P<body>.*?)\}",
    text,
    re.DOTALL,
)
assert deadline_failure_match, "rpc-running verification must check failure after its poll deadline"
assert "throw " in deadline_failure_match.group("body"), (
    "rpc-running verification must exit nonzero after the poll deadline"
)
PY
then
  package_contract_failed=1
fi

if ! python3 - "$windows_script" <<'PY'
from pathlib import Path
import re
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
task_start = text.index("Start-ScheduledTask -TaskName $taskName")
startup_poll = text[task_start:text.index("if (!$verified)", task_start)]

# Fixture: Get-ScheduledTaskInfo can report a fresh LastRunTime with 0x41301
# while the wrapper is still running and has not yet emitted this attempt's
# terminal exit marker.  That is nonterminal and must keep polling, not roll
# back a healthy observer.
fresh_running_without_marker = {
    "last_run_time_is_fresh": True,
    "last_task_result": 267009,  # 0x41301: task is currently running
    "exact_attempt_exit_marker": None,
    "expects_terminal_failure": False,
    "expects_rollback": False,
}
terminal_child_failure = {
    "last_run_time_is_fresh": True,
    "last_task_result": 1,
    "exact_attempt_exit_marker": 255,
    "expects_terminal_failure": True,
    "expects_rollback": True,
}
assert fresh_running_without_marker["last_task_result"] == 267009
assert fresh_running_without_marker["exact_attempt_exit_marker"] is None
assert not fresh_running_without_marker["expects_rollback"]
assert terminal_child_failure["exact_attempt_exit_marker"] == 255
assert terminal_child_failure["expects_rollback"]

assert re.search(
    r"\$schedulerRunningResult\s*=\s*267009\b", startup_poll
), (
    "fresh LastTaskResult=267009 (0x41301 task running) needs an explicit "
    "nonterminal scheduler classification"
)
assert re.search(
    r"\$schedulerReportsRunning\s*=\s*\$lastTaskResult\s*-eq\s*\$schedulerRunningResult",
    startup_poll,
), "startup poll must classify 0x41301 from the fresh scheduler observation"
assert re.search(
    r"\$terminalExitCode\s*=\s*if\s*\(\$null\s*-ne\s*\$attemptChildExitCode\)\s*\{\s*\$attemptChildExitCode\s*\}\s*else\s*\{\s*\$null\s*\}",
    startup_poll,
), (
    "without the exact current-attempt exit marker, scheduler status alone "
    "must not manufacture a terminal child exit code"
)
terminal_guard = re.search(
    r"if\s*\((?P<condition>.*?)\)\s*\{\s*Preserve-AttemptDiagnostics",
    startup_poll,
    re.DOTALL,
)
assert terminal_guard, "startup poll must retain one marker-bound terminal failure guard"
condition = terminal_guard.group("condition")
assert "$attemptChildExitCode" in condition, (
    "terminal child failure and rollback must require the exact current-attempt exit marker"
)
assert "$newTaskResult" not in condition, (
    "a fresh scheduler result, including 0x41301, must not independently request rollback"
)
assert "$schedulerReportsRunning" in startup_poll, (
    "the running scheduler result must remain nonterminal while the attempt marker is absent"
)
PY
then
  package_contract_failed=1
fi

if ! python3 - "$windows_script" <<'PY'
from pathlib import Path
import re
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
assert "$installRoot = [Environment]::ExpandEnvironmentVariables('C:\\Users\\Observer\\AppData\\Local\\Programs\\oasis7')" in text, (
    "configured node-local install root must survive plan generation"
)

assert "$localizedPreview -match" not in text, (
    "node-local validation must inspect structured path-bearing fields, not arbitrary JSON text"
)
assert "function Test-NodeLocalPath" in text, (
    "Windows rollout must provide one explicit node-local path classifier"
)
classifier_match = re.search(
    r"function Test-NodeLocalPath\s*\{(?P<body>.*?)\n\}", text, re.DOTALL
)
assert classifier_match, "could not isolate Test-NodeLocalPath contract"
classifier = classifier_match.group("body")
for token in ("IsPathRooted", "$deployRoot", "$installRoot"):
    assert token in classifier, f"path classifier omits allowed-root boundary: {token}"
assert re.search(r"(?i)allowed.*root|root.*allowed", classifier), (
    "path classifier must accept only configured node-local roots, including "
    r"C:\oasis7-deploy and C:\Users\... install roots"
)
classifier_fixtures = {
    r"C:\oasis7-deploy\config\genesis.json": True,
    r"C:\Users\Observer\AppData\Local\Programs\oasis7\bin\oasis7_chain_runtime.exe": True,
    r"D:\a\oasis7\oasis7\public-testnet-stage\genesis.json": False,
    r"E:\foreign-node\genesis.json": False,
    r"/Users/build/oasis7/genesis.json": False,
    r"/home/build/oasis7/genesis.json": False,
}
assert set(classifier_fixtures.values()) == {False, True}
assert "operator_note" not in classifier, (
    "arbitrary non-path JSON text must remain outside node-local path classification"
)

physical_contract = re.search(
    r"function Assert-NodeLocalPhysicalPath\s*\{\s*param\((?P<params>.*?)\)\s*if",
    text,
    re.DOTALL,
)
assert physical_contract, "could not isolate Assert-NodeLocalPhysicalPath parameter contract"
physical_params = physical_contract.group("params")
assert re.findall(r"\[string\]\s+\$(\w+)", physical_params) == ["Path", "Label"], (
    "physical path guard must expose exactly one Path and one Label parameter"
)
assert physical_params.count("Mandatory = $true") == 2
assert "ParameterSetName" not in physical_params
physical_guard = text[
    physical_contract.start():text.index("function Preserve-AttemptDiagnostics")
]
assert "Split-Path -LiteralPath $probe -Parent" not in physical_guard, (
    "PowerShell 5.1 places LiteralPath and Parent in incompatible Split-Path parameter sets"
)
assert "$parent = [System.IO.Path]::GetDirectoryName($probe)" in physical_guard, (
    "physical path guard must use wildcard-safe .NET lexical parent traversal"
)
reparse_check = physical_guard.index("[System.IO.FileAttributes]::ReparsePoint")
parent_step = physical_guard.index("[System.IO.Path]::GetDirectoryName($probe)")
probe_advance = physical_guard.index("$probe = $parent")
assert reparse_check < parent_step < probe_advance, (
    "physical path guard must inspect each path before walking to its parent"
)
assert "$parent.Equals($probe, [System.StringComparison]::OrdinalIgnoreCase)" in physical_guard, (
    "physical path guard must retain explicit root/self-parent termination"
)

# Keep the explicit string casts at object/array call boundaries as independent
# enforcement of the function's declared string contract. They are not the
# cause or fix for the Split-Path parameter-set failure above.
normalized_calls = re.sub(r"`\r?\n\s*", " ", text)
physical_calls = [
    line.strip()
    for line in normalized_calls.splitlines()
    if line.lstrip().startswith("Assert-NodeLocalPhysicalPath ")
]
assert physical_calls, "generated rollout never invokes its physical path guard"
for call in physical_calls:
    assert len(re.findall(r"(?<!\w)-Path\b", call)) == 1, (
        f"physical path guard call must bind Path exactly once: {call}"
    )
    assert len(re.findall(r"(?<!\w)-Label\b", call)) == 1, (
        f"physical path guard call must bind Label exactly once: {call}"
    )
    assert not re.search(r"-(?:Path|Label)\s+\$\w+(?:\.|\[)", call), (
        "PowerShell 5.1-incompatible ungrouped member/index argument in physical "
        f"path guard call: {call}"
    )
assert any(
    "-Path ([string]$physicalPreflightTarget.path)" in call
    and "-Label ([string]$physicalPreflightTarget.label)" in call
    for call in physical_calls
), "native preflight fixture must retain the grouped member-expression call form"
assert any(
    "-Path ([string]$metadataPaths[0])" in call for call in physical_calls
), "native rollback fixture must retain the grouped index-expression call form"
assert any(
    "-Path $logRoot -Label 'active deploy log root'" in call
    for call in physical_calls
), "native fixture must retain the scalar-variable/literal call form"
PY
then
  package_contract_failed=1
fi

if ! python3 - "$windows_script" <<'PY'
from pathlib import Path
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")

structured_path_fields = (
    "runtime_build.path",
    "runtime_build.resolved_path",
    "world_snapshot.resolved_path",
    "generated_world_sidecar.resolved_path",
    "world_generation_provenance.resolved_path",
    "governance_manifest.resolved_path",
    "network_manifest.resolved_path",
    "evidence_refs",
    "runtime_refs.release_candidate_bundle_ref",
    "runtime_refs.genesis_ref",
    "runtime_refs.bootstrap_peer_ref",
    "runtime_refs.generated_world_sidecar_ref",
    "runtime_refs.world_generation_provenance_ref",
    "governance_bootstrap_refs",
)
for field in structured_path_fields:
    assert field in text, f"structured node-local validation omits staged path field: {field}"
assert "staged target path is not node-local" in text, (
    "all staged target path fields need one fail-closed post-rewrite diagnostic"
)
rewrite_end = text.index("$manifestText = $manifestJson | ConvertTo-Json")
path_validation_index = text.index("staged target path is not node-local")
stop_index = text.index("Stop-ScheduledTask -TaskName $taskName")
assert rewrite_end <= path_validation_index < stop_index, (
    "structured staged-path validation must run after rewrite and before restart/install mutation"
)
PY
then
  package_contract_failed=1
fi

if ! python3 - "$windows_script" <<'PY'
from pathlib import Path
import re
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")
stop_index = text.index("Stop-ScheduledTask -TaskName $taskName")

task_start_index = text.index("Start-ScheduledTask -TaskName $taskName")
assert "Get-ScheduledTaskInfo" in text[task_start_index:], (
    "Windows startup poll does not inspect terminal scheduled-task child results"
)
task_info_index = text.index("Get-ScheduledTaskInfo", task_start_index)
task_result_index = text.index("LastTaskResult", task_info_index)
terminal_child_failure_index = text.index("terminal task child exited nonzero", task_result_index)
poll_sleep_index = text.index("Start-Sleep -Seconds 2", task_result_index)
assert task_info_index < terminal_child_failure_index < poll_sleep_index, (
    "terminal scheduled-task child failure must be detected before another startup poll sleep"
)
verification_tail = text[task_start_index:]
assert "rollback_required=true" in verification_tail
assert "failure_diagnostics=" in verification_tail
assert "Get-Content" in verification_tail and "-Tail" in verification_tail, (
    "terminal child failure must emit bounded existing-log diagnostics"
)
assert "Set-Content" not in verification_tail and "Out-File" not in verification_tail, (
    "startup failure handling must not overwrite existing task logs"
)

attempt_contract = (
    "$attemptId",
    "$attemptStdoutPath",
    "$attemptStderrPath",
    "$attemptExitMarkerPath",
)
for token in attempt_contract:
    assert token in text, f"scheduled-task wrapper omits unique attempt artifact: {token}"
assert re.search(r"attemptId\s*=.*Guid.*NewGuid", text, re.IGNORECASE), (
    "scheduled-task wrapper must generate a unique identifier for every attempt"
)
assert "New-ScheduledTaskAction" in text or "Set-ScheduledTask" in text, (
    "rollout must install an attempt-aware scheduled-task wrapper before restart"
)
wrapper_index = min(
    index for token in ("New-ScheduledTaskAction", "Set-ScheduledTask")
    if (index := text.find(token)) >= 0
)
assert wrapper_index < task_start_index, "attempt wrapper must be installed before task start"
assert "AppendAllText" in text, (
    "attempt stdout/stderr/exit evidence must use append, non-truncating writes"
)
for truncating_token in (
    "[System.IO.File]::WriteAllText($attemptStdoutPath",
    "[System.IO.File]::WriteAllText($attemptStderrPath",
    "[System.IO.File]::WriteAllText($attemptExitMarkerPath",
):
    assert truncating_token not in text, (
        f"attempt evidence must not be truncated: {truncating_token}"
    )

preserve_occurrences = [
    match.start() for match in re.finditer("Preserve-AttemptDiagnostics", text)
]
assert len(preserve_occurrences) >= 3, (
    "attempt preservation contract needs a function plus pre-restart and pre-rollback calls"
)
assert any(index < stop_index for index in preserve_occurrences[1:]), (
    "pre-existing attempt diagnostics must be preserved before restart/install mutation"
)
rollback_index = text.index("rollback_required=true", task_start_index)
assert any(task_start_index < index < rollback_index for index in preserve_occurrences), (
    "current-attempt diagnostics must be preserved before rollback is requested"
)
diagnostic_tail = text[task_start_index:]
for token in attempt_contract[1:]:
    assert token in diagnostic_tail, (
        f"generated terminal diagnostics must reference exact attempt artifact: {token}"
    )
assert "Get-ChildItem -LiteralPath $logRoot" not in diagnostic_tail, (
    "terminal diagnostics must not substitute unrelated newest log files for exact attempt evidence"
)
PY
then
  package_contract_failed=1
fi

if ! python3 - "$windows_script" <<'PY'
from pathlib import Path
import re
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")

exit_capture = re.search(
    r"\$(?P<name>[A-Za-z][A-Za-z0-9]*exit[A-Za-z0-9]*code)\s*="
    r"\s*\$childProcess\.ExitCode",
    text,
    re.IGNORECASE,
)
assert exit_capture, "scheduled-task wrapper must capture the exact Process.ExitCode"
exit_name = exit_capture.group("name")
assert re.search(
    rf"AppendAllText\([^\n]*attemptExitMarkerPath[^\n]*\${re.escape(exit_name)}",
    text,
    re.IGNORECASE,
), "exit marker must record the same unmodified child exit-code variable"
assert re.search(rf"exit\s+\${re.escape(exit_name)}\b", text, re.IGNORECASE), (
    "scheduled-task wrapper must propagate the recorded child exit code unchanged; "
    "the terminal-255 fixture must remain 255"
)
PY
then
  package_contract_failed=1
fi

if ! python3 - "$windows_script" <<'PY'
from pathlib import Path
import re
import sys

text = Path(sys.argv[1]).read_text(encoding="utf-8")

# The scheduled-task wrapper must invoke the action captured from the trusted
# task definition directly.  A second ComSpec /c changes cmd.exe parsing and
# caused the native Windows rollout action to exit 1 before selective rollback.
assert re.search(
    r"\$childExecute\s*=\s*(?:\[string\])?\$originalTaskAction\.Execute", text
), (
    "scheduled-task wrapper must preserve OriginalTaskAction.Execute as its direct child"
)
assert re.search(
    r"\$childArguments\s*=\s*(?:\[string\])?\$originalTaskAction\.Arguments", text
), (
    "scheduled-task wrapper must preserve OriginalTaskAction.Arguments as its direct child"
)
assert not re.search(r"\$env:ComSpec\s+(?:(?:/d|/s)\s+)*/c\b", text, re.IGNORECASE), (
    "scheduled-task wrapper must not launch the trusted task action through nested $env:ComSpec /c"
)

process_start_info = re.search(
    r"\$processStartInfo\s*=\s*\[System\.Diagnostics\.ProcessStartInfo\]::new\(\)"
    r"(?P<body>.*?)\$childProcess\s*=\s*\[System\.Diagnostics\.Process\]::new\(\)",
    text,
    re.DOTALL,
)
assert process_start_info, (
    "scheduled-task wrapper must construct System.Diagnostics.ProcessStartInfo for the trusted action"
)
start_info_body = process_start_info.group("body")
for assignment in (
    "$processStartInfo.FileName = $childExecute",
    "$processStartInfo.Arguments = $childArguments",
    "$processStartInfo.UseShellExecute = $false",
    "$processStartInfo.CreateNoWindow = $true",
):
    assert assignment in start_info_body, (
        "scheduled-task wrapper must directly configure ProcessStartInfo: " + assignment
    )

for async_contract in (
    "add_OutputDataReceived",
    "add_ErrorDataReceived",
    "BeginOutputReadLine()",
    "BeginErrorReadLine()",
    "WaitForExit()",
):
    assert async_contract in text, (
        "scheduled-task wrapper must capture stdout/stderr concurrently without pipe deadlock: "
        + async_contract
    )
assert "ReadToEnd()" not in text, (
    "scheduled-task wrapper must not sequentially drain redirected stdout/stderr pipes"
)
PY
then
  package_contract_failed=1
fi

"$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
  --manifest "$TMP_DIR/manifest.json" \
  --package-dir "$package_dir" \
  --out-dir "$TMP_DIR/repeated-plan-out" \
  --json >"$TMP_DIR/repeated-plan.json"
cmp -s \
  "$TMP_DIR/plan-only-out/windows-observer-windows-upgrade.ps1" \
  "$TMP_DIR/repeated-plan-out/windows-observer-windows-upgrade.ps1"
python3 - "$windows_genesis" "$TMP_DIR/repeated-plan-out/windows-observer-windows-upgrade.ps1" <<'PY'
from pathlib import Path
import sys

for raw_path in sys.argv[1:]:
    path = Path(raw_path)
    assert not path.read_bytes().startswith(b"\xef\xbb\xbf"), f"UTF-8 BOM forbidden: {path}"
PY

if [[ "$package_contract_failed" -ne 0 ]]; then
  exit 1
fi


python3 -W error::SyntaxWarning "$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" --help >/tmp/oasis7-package-rollout-help.out
grep -q "mode is plan-only" /tmp/oasis7-package-rollout-help.out
grep -q "Mutation requires" /tmp/oasis7-package-rollout-help.out
grep -q "never reads or stores credentials" /tmp/oasis7-package-rollout-help.out
grep -q -- "--readiness-policy" /tmp/oasis7-package-rollout-help.out

bad_package_dir="$TMP_DIR/bad-package"
cp -R "$package_dir" "$bad_package_dir"
sed -i.bak "s/^commit=.*/commit=0000000000000000000000000000000000000000/" \
  "$bad_package_dir/windows/windows-x64-BUILDINFO"
rm "$bad_package_dir/windows/windows-x64-BUILDINFO.bak"
(
  cd "$bad_package_dir/windows"
  shasum -a 256 oasis7-windows-x64.exe windows-x64-BUILDINFO >windows-x64-SHA256SUMS
)
if "$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
  --manifest "$TMP_DIR/manifest.json" \
  --package-dir "$bad_package_dir" \
  --out-dir "$TMP_DIR/bad-out" \
  >"$TMP_DIR/bad.out" 2>"$TMP_DIR/bad.err"; then
  echo "expected mismatched BUILDINFO to fail" >&2
  exit 1
fi
grep -q "does not match" "$TMP_DIR/bad.err"

bad_sums_dir="$TMP_DIR/bad-sums-package"
cp -R "$package_dir" "$bad_sums_dir"
(
  cd "$bad_sums_dir"
  shasum -a 256 linux-x64-BUILDINFO >linux-x64-SHA256SUMS
)
if "$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
  --manifest "$TMP_DIR/manifest.json" \
  --package-dir "$bad_sums_dir" \
  --out-dir "$TMP_DIR/bad-sums-out" \
  >"$TMP_DIR/bad-sums.out" 2>"$TMP_DIR/bad-sums.err"; then
  echo "expected missing asset checksum coverage to fail" >&2
  exit 1
fi
grep -q "checksum file does not cover required artifact: oasis7-linux-x64-bundle.tar.gz" "$TMP_DIR/bad-sums.err"

strict_node_root="$TMP_DIR/strict-node"
mkdir -p "$strict_node_root/releases/old/bin" "$strict_node_root/config/doc/testing/evidence"
printf 'runtime-v1\n' >"$strict_node_root/releases/old/bin/oasis7_chain_runtime"
chmod +x "$strict_node_root/releases/old/bin/oasis7_chain_runtime"
ln -s "$strict_node_root/releases/old" "$strict_node_root/current"
cp \
  "$node_root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json" \
  "$strict_node_root/config/doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json"
python3 - "$TMP_DIR/manifest.json" "$strict_node_root" <<'PY'
from pathlib import Path
import json
import sys

manifest = json.loads(Path(sys.argv[1]).read_text())
manifest["nodes"][0]["node_root"] = sys.argv[2]
manifest["nodes"][0]["restart"] = True
manifest["nodes"][0]["systemd_service"] = "oasis7-testnet-storage.service"
Path(sys.argv[1]).write_text(json.dumps(manifest, indent=2) + "\n")
PY
"$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
  --manifest "$TMP_DIR/manifest.json" \
  --package-dir "$package_dir" \
  --out-dir "$TMP_DIR/strict-out" \
  --readiness-policy strict-ready \
  --json >"$TMP_DIR/strict-plan.json"
jq -e '
  .readiness_policy == "strict-ready"
  and (.nodes[] | select(.name == "local-linux") | .commands[0] | contains("--post-restart-status-url"))' \
  "$TMP_DIR/strict-plan.json" >/dev/null

python3 - "$TMP_DIR/manifest.json" <<'PY'
from pathlib import Path
import json
import sys

manifest = json.loads(Path(sys.argv[1]).read_text())
manifest["nodes"][0].pop("status_url", None)
Path(sys.argv[1]).write_text(json.dumps(manifest, indent=2) + "\n")
PY
if "$ROOT_DIR/scripts/p2p-public-testnet-package-rollout.py" \
  --manifest "$TMP_DIR/manifest.json" \
  --package-dir "$package_dir" \
  --out-dir "$TMP_DIR/strict-missing-status-out" \
  --readiness-policy strict-ready \
  >"$TMP_DIR/strict-missing-status.out" 2>"$TMP_DIR/strict-missing-status.err"; then
  echo "expected strict-ready restart without status_url to fail" >&2
  exit 1
fi
grep -q "uses strict-ready but has no status_url" "$TMP_DIR/strict-missing-status.err"

echo "ok: package rollout helper validates artifacts and standardizes linux/windows replacement plans"
