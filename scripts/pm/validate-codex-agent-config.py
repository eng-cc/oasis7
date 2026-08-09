#!/usr/bin/env python3
"""Validate Codex specialist adapters with a complete TOML parser and native load probe."""

from __future__ import annotations

import argparse
from collections import deque
import importlib.util
import json
import os
import queue
import stat
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path
from typing import Any

try:
    import tomllib
except ModuleNotFoundError as error:  # pragma: no cover - selected by the shell wrapper
    reexec_guard = "OASIS7_CODEX_AGENT_CONFIG_TOMLLIB_REEXEC"
    if os.environ.get(reexec_guard) == "1":
        raise SystemExit(
            "validate-codex-agent-config: Python 3.11+ with stdlib tomllib is required; "
            "the repository interpreter fallback did not provide tomllib"
        ) from error
    finder = Path(__file__).with_name("find-python-with-module.sh")
    finder_env = os.environ.copy()
    finder_env["PATH"] = finder_env.get("PATH", "") + os.pathsep
    try:
        result = subprocess.run(
            [str(finder), "tomllib"],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=finder_env,
        )
    except OSError as finder_error:
        raise SystemExit(
            "validate-codex-agent-config: Python 3.11+ with stdlib tomllib is required; "
            f"cannot run repository interpreter finder {finder}: {finder_error}"
        ) from error
    interpreter = result.stdout.strip()
    if result.returncode != 0 or not interpreter:
        detail = result.stderr.strip()
        suffix = f": {detail}" if detail else ""
        raise SystemExit(
            "validate-codex-agent-config: Python 3.11+ with stdlib tomllib is required; "
            f"repository interpreter finder could not locate one{suffix}"
        ) from error
    reexec_env = os.environ.copy()
    reexec_env[reexec_guard] = "1"
    os.execve(interpreter, [interpreter, str(Path(__file__).resolve()), *sys.argv[1:]], reexec_env)


EXPECTED_ROLES = {
    "producer_system_designer",
    "gameplay_designer",
    "game_visual_interaction_designer",
    "runtime_engineer",
    "blockchain_ops_engineer",
    "wasm_platform_engineer",
    "agent_engineer",
    "viewer_engineer",
    "qa_engineer",
    "repository_health_engineer",
    "liveops_community",
}

MANDATORY_INSTRUCTION_MARKERS = (
    "AGENTS.md",
    "doc/engineering/workflow/source-of-truth.md",
    "the dispatched slice contract",
    "explicit write scope",
    "Treat third_party as read-only",
    "Do not commit, push, create a PR, merge, or create a second task truth",
    "Return:",
    "uncertainty",
    "residual risk",
)
TERMINAL_RESPONSE_DRAIN_SECONDS = 0.5

def fail(message: str) -> None:
    raise SystemExit(f"validate-codex-agent-config: {message}")


def load_toml(path: Path) -> dict[str, Any]:
    try:
        with path.open("rb") as handle:
            value = tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as error:
        fail(f"invalid TOML in {path}: {error}")
    if not isinstance(value, dict):
        fail(f"{path} must contain a TOML table")
    return value


def require_regular_file(path: Path, label: str) -> None:
    try:
        mode = path.lstat().st_mode
    except OSError as error:
        fail(f"cannot stat {label} {path}: {error}")
    if not stat.S_ISREG(mode):
        fail(f"{label} {path} must be a regular file and not a symlink")


def load_renderer() -> Any:
    path = Path(__file__).with_name("render-codex-agent-config.py")
    spec = importlib.util.spec_from_file_location("oasis7_codex_agent_renderer", path)
    if spec is None or spec.loader is None:
        fail(f"cannot load deterministic adapter renderer: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def read_response(
    process: subprocess.Popen[str],
    output: queue.Queue[str | None],
    request_id: int,
    timeout_seconds: float,
    stderr_tail: deque[str],
) -> dict[str, Any]:
    deadline = time.monotonic() + timeout_seconds
    terminal_drain_deadline: float | None = None
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            if terminal_drain_deadline is not None and process.poll() is not None:
                try:
                    line = output.get_nowait()
                except queue.Empty:
                    detail = "".join(stderr_tail).strip()
                    suffix = f"; stderr tail: {detail[-2000:]}" if detail else ""
                    fail(f"Codex app-server exited before response {request_id}{suffix}")
            else:
                detail = "".join(stderr_tail).strip()
                suffix = f"; stderr tail: {detail[-2000:]}" if detail else ""
                fail(
                    f"Codex app-server timed out after {timeout_seconds:g}s waiting "
                    f"for response {request_id}{suffix}"
                )
        else:
            try:
                line = output.get(timeout=min(remaining, 0.1))
            except queue.Empty:
                if process.poll() is None:
                    continue
                if terminal_drain_deadline is None:
                    terminal_drain_deadline = min(
                        deadline, time.monotonic() + TERMINAL_RESPONSE_DRAIN_SECONDS
                    )
                if time.monotonic() < terminal_drain_deadline:
                    continue
                try:
                    line = output.get_nowait()
                except queue.Empty:
                    detail = "".join(stderr_tail).strip()
                    suffix = f"; stderr tail: {detail[-2000:]}" if detail else ""
                    fail(f"Codex app-server exited before response {request_id}{suffix}")
        if line is None:
            detail = "".join(stderr_tail).strip()
            suffix = f"; stderr tail: {detail[-2000:]}" if detail else ""
            fail(f"Codex app-server exited before response {request_id}{suffix}")
        try:
            payload = json.loads(line)
        except json.JSONDecodeError as error:
            fail(f"Codex app-server emitted invalid JSON: {error}")
        if payload.get("id") == request_id:
            return payload


def native_config_probe(root: Path, expected_roles: set[str]) -> dict[str, Any]:
    command = ["codex", "app-server", "--strict-config", "--listen", "stdio://"]
    try:
        timeout_seconds = float(
            os.environ.get("CODEX_AGENT_CONFIG_PROBE_TIMEOUT_SECONDS", "15")
        )
    except ValueError:
        fail("CODEX_AGENT_CONFIG_PROBE_TIMEOUT_SECONDS must be numeric")
    if not 1 <= timeout_seconds <= 300:
        fail("CODEX_AGENT_CONFIG_PROBE_TIMEOUT_SECONDS must be between 1 and 300")

    reader_threads: list[threading.Thread] = []
    probe_home = tempfile.TemporaryDirectory(prefix="oasis7-codex-probe-")
    probe_env = os.environ.copy()
    probe_env["CODEX_HOME"] = probe_home.name
    try:
        process = subprocess.Popen(
            command,
            cwd=root,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=probe_env,
        )
    except FileNotFoundError:
        fail("Codex CLI is unavailable; native strict-load capability cannot be verified")
    try:
        if process.stdin is None or process.stdout is None or process.stderr is None:
            fail("Codex app-server pipes were not created")
        output: queue.Queue[str | None] = queue.Queue()
        stderr_tail: deque[str] = deque(maxlen=200)

        def read_stdout() -> None:
            assert process.stdout is not None
            for line in process.stdout:
                output.put(line)
            output.put(None)

        def read_stderr() -> None:
            assert process.stderr is not None
            for line in process.stderr:
                stderr_tail.append(line)

        reader_threads = [
            threading.Thread(target=read_stdout, daemon=True),
            threading.Thread(target=read_stderr, daemon=True),
        ]
        for thread in reader_threads:
            thread.start()

        def send(payload: dict[str, Any]) -> None:
            assert process.stdin is not None
            process.stdin.write(json.dumps(payload) + "\n")
            process.stdin.flush()

        send(
            {
                "id": 1,
                "method": "initialize",
                "params": {
                    "clientInfo": {"name": "oasis7-workflow-eval", "version": "1"},
                    "capabilities": {"experimentalApi": True},
                },
            }
        )
        initialized = read_response(
            process, output, 1, timeout_seconds, stderr_tail
        )
        if "error" in initialized:
            fail(f"Codex app-server initialize failed: {initialized['error']}")
        send({"method": "initialized", "params": {}})
        send(
            {
                "id": 2,
                "method": "config/read",
                "params": {"cwd": str(root), "includeLayers": True},
            }
        )
        response = read_response(
            process, output, 2, timeout_seconds, stderr_tail
        )
        if "error" in response:
            fail(f"Codex strict config/read failed: {response['error']}")
    finally:
        if process.poll() is None:
            process.terminate()
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=2)
        for thread in reader_threads:
            thread.join(timeout=1)
        probe_home.cleanup()

    result = response.get("result")
    if not isinstance(result, dict):
        fail("Codex config/read returned no result")
    layers = result.get("layers")
    if not isinstance(layers, list):
        fail("Codex config/read did not expose config layers")
    project_folder = str((root / ".codex").resolve())
    project_layer = None
    for layer in layers:
        name = layer.get("name") if isinstance(layer, dict) else None
        if not isinstance(name, dict):
            continue
        if name.get("type") == "project" and name.get("dotCodexFolder") == project_folder:
            project_layer = layer.get("config")
            break
    if not isinstance(project_layer, dict):
        fail("Codex native config/read did not load this worktree's .codex layer")
    native_agents = project_layer.get("agents")
    if not isinstance(native_agents, dict) or set(native_agents) != expected_roles:
        fail("Codex native config/read did not load the exact specialist role registry")
    if "model" in project_layer or "model_reasoning_effort" in project_layer:
        fail("project config must not pin the root TPM model or reasoning effort")
    return {
        "status": "registry_strict_loaded",
        "command": "codex app-server --strict-config + config/read (registry layer only)",
        "transport": "cross-platform reader threads with bounded stderr capture",
        "timeout_seconds": timeout_seconds,
        "registered_roles": sorted(native_agents),
        "adapter_native_parse": "not_run",
        "representative_role_activation": "not_run",
        "activation_capability": "requires a dispatch surface with a named-role selector; current Desktop spawn_agent schema has none",
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument(
        "--skip-native-probe",
        action="store_true",
        help="Skip Codex app-server registration probing for isolated negative fixtures only.",
    )
    args = parser.parse_args()
    root = args.root.resolve()
    renderer = load_renderer()
    config_path = root / ".codex/config.toml"
    require_regular_file(config_path, "project config")
    config = load_toml(config_path)

    if "model" in config or "model_reasoning_effort" in config:
        fail(".codex/config.toml must not pin root-level model or model_reasoning_effort")
    agents = config.get("agents")
    if not isinstance(agents, dict) or set(agents) != EXPECTED_ROLES:
        fail(".codex/config.toml must register exactly the eleven specialist roles and not tpm")
    adapter_files = {path.stem for path in (root / ".codex/agents").glob("*.toml")}
    if adapter_files != EXPECTED_ROLES:
        fail(".codex/agents must contain exactly one adapter for each registered specialist role")

    adapter_results = []
    for role in sorted(EXPECTED_ROLES):
        role_card_path = root / f".agents/roles/{role}.md"
        try:
            role_card_mode = role_card_path.lstat().st_mode
        except OSError as error:
            fail(f"cannot read role card {role_card_path}: {error}")
        if not stat.S_ISREG(role_card_mode):
            fail(f"{role_card_path} must be a regular file and not a symlink")
        binding = renderer.projection(role_card_path, role)
        entry = agents.get(role)
        if not isinstance(entry, dict):
            fail(f"agents.{role} must be a table")
        if set(entry) != {"description", "config_file"}:
            fail(f"agents.{role} must contain exactly description and config_file")
        if entry.get("description") != binding["registry_description"]:
            fail(f"agents.{role}.description does not match its role-card Registry Description")
        expected_relative_path = f"agents/{role}.toml"
        if entry.get("config_file") != expected_relative_path:
            fail(f"agents.{role}.config_file must be exactly {expected_relative_path}")
        adapter_path = root / ".codex" / expected_relative_path
        try:
            adapter_mode = adapter_path.lstat().st_mode
        except OSError as error:
            fail(f"cannot stat adapter {adapter_path}: {error}")
        if not stat.S_ISREG(adapter_mode):
            fail(f"{adapter_path} must be a regular file and not a symlink")
        adapter = load_toml(adapter_path)
        required_adapter_keys = {
            "developer_instructions",
            "model",
            "model_reasoning_effort",
        }
        if set(adapter) != required_adapter_keys:
            fail(
                f"{adapter_path} must contain exactly developer_instructions, model, "
                "and model_reasoning_effort"
            )
        if adapter.get("model") != "gpt-5.6-terra":
            fail(f'{adapter_path} model must be exactly "gpt-5.6-terra"')
        if adapter.get("model_reasoning_effort") != "medium":
            fail(f'{adapter_path} model_reasoning_effort must be exactly "medium"')
        instructions = adapter.get("developer_instructions")
        if not isinstance(instructions, str) or not instructions.strip():
            fail(f"{adapter_path} must define non-blank developer_instructions")
        instructions = instructions.strip()
        role_markers = (
            f"oasis7 {role} bounded specialist subagent",
            f".agents/roles/{role}.md",
        )
        missing = [
            marker
            for marker in MANDATORY_INSTRUCTION_MARKERS + role_markers
            if marker not in instructions
        ]
        if missing:
            fail(f"{adapter_path} missing mandatory instruction markers: {missing}")
        expected_instructions = renderer.instructions(role, binding)
        if instructions != expected_instructions:
            fail(f"{adapter_path} is not the deterministic rendering of {role_card_path.relative_to(root)}")
        adapter_results.append(
            {"role": role, "path": str(adapter_path.relative_to(root)), "status": "ok"}
        )

    source = (root / "doc/engineering/workflow/source-of-truth.md").read_text(
        encoding="utf-8"
    )
    capability_markers = (
        "Adapter registration is not proof of adapter activation.",
        "current Desktop `spawn_agent` schema has no named-role selector",
        "role activation: message-assigned; adapter inactive on this surface",
        "full-thread/full-history fork does not activate a registered adapter by itself",
    )
    missing_capability_markers = [m for m in capability_markers if m not in source]
    if missing_capability_markers:
        fail(f"source-of-truth missing capability boundaries: {missing_capability_markers}")

    role_card_markers = {
        "agent_engineer": (
            "世界内 Agent 使用的推理模型/provider 行为",
            "仓库 `.codex/config.toml`、专业 role adapter、Codex subagent runtime、live dispatch",
        ),
        "repository_health_engineer": (
            "仓库 Codex 配置、专业 role adapter 投影与 validation contract",
            "live subagent role selection、dispatch、并发调度与结果合流",
        ),
        "tpm": (
            "live subagent role selection、dispatch、并发/顺序调度与结果集成",
        ),
    }
    for role, markers in role_card_markers.items():
        role_card = (root / f".agents/roles/{role}.md").read_text(encoding="utf-8")
        missing_role_card_markers = [m for m in markers if m not in role_card]
        if missing_role_card_markers:
            fail(
                f".agents/roles/{role}.md missing Codex responsibility boundary: "
                f"{missing_role_card_markers}"
            )

    native_probe = (
        {
            "status": "skipped_for_isolated_fixture",
            "adapter_native_parse": "not_run",
            "representative_role_activation": "not_run",
            "activation_capability": "fixture-only skip; production workflow eval must run native probing",
        }
        if args.skip_native_probe
        else native_config_probe(root, EXPECTED_ROLES)
    )
    print(
        json.dumps(
            {
                "status": "ok",
                "parser": f"stdlib tomllib via {sys.executable}",
                "runtime_policy": (
                    "root/default unpinned; specialist adapter model/reasoning is intended "
                    "configuration for adapter-backed named-role activation only; message-assigned "
                    "adapter-inactive fallback uses user-selected or parent-inherited runtime; "
                    "static validation does not prove activation, model availability, or observed "
                    "actual runtime"
                ),
                "adapters": adapter_results,
                "native_probe": native_probe,
            },
            ensure_ascii=False,
        )
    )


if __name__ == "__main__":
    main()
