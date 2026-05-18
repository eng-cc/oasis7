#!/usr/bin/env python3

import json
import os
import sys
import time
import urllib.error
import urllib.request


DEFAULT_BASE_URL = "https://api.letai.run/v1"
DEFAULT_TIMEOUT_MS = 15000
DEFAULT_MAX_OUTPUT_TOKENS = 256
DEFAULT_TEMPERATURE = 0.0
DEFAULT_USER_AGENT = "curl/8.5.0"


def env_required(*names: str) -> str:
    for name in names:
        value = os.environ.get(name, "").strip()
        if value:
            return value
    raise RuntimeError(f"missing required environment variable: {' or '.join(names)}")


def env_optional(*names: str) -> str:
    for name in names:
        value = os.environ.get(name, "")
        if value.strip():
            return value.strip()
    return ""


def env_int(default: int, *names: str) -> int:
    raw = env_optional(*names)
    if not raw:
        return default
    try:
        return int(raw)
    except ValueError as exc:
        raise RuntimeError(f"invalid integer for {' or '.join(names)}: {raw}") from exc


def env_float(default: float, *names: str) -> float:
    raw = env_optional(*names)
    if not raw:
        return default
    try:
        return float(raw)
    except ValueError as exc:
        raise RuntimeError(f"invalid float for {' or '.join(names)}: {raw}") from exc


def env_bool(default: bool, *names: str) -> bool:
    raw = env_optional(*names)
    if not raw:
        return default
    return raw.lower() in {"1", "true", "yes", "on"}


def normalize_base_url(raw: str) -> str:
    base = raw.strip().rstrip("/")
    for suffix in ("/chat/completions", "/responses"):
        if base.endswith(suffix):
            base = base[: -len(suffix)]
    return base


def load_route_config() -> dict:
    route_label = env_optional("OASIS7_REMOTE_LLM_ROUTE_LABEL")
    routes_path = env_optional("OASIS7_REMOTE_LLM_ROUTES_PATH")
    if not routes_path:
        return load_newapi_bridge_state_route(route_label)
    try:
        with open(routes_path, "r", encoding="utf-8") as handle:
            payload = json.load(handle)
    except OSError as exc:
        raise RuntimeError(f"failed to read OASIS7_REMOTE_LLM_ROUTES_PATH: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise RuntimeError(
            f"OASIS7_REMOTE_LLM_ROUTES_PATH must contain valid JSON: {exc}"
        ) from exc
    if not isinstance(payload, dict):
        raise RuntimeError("OASIS7_REMOTE_LLM_ROUTES_PATH root must be a JSON object")
    lookup_label = route_label or "default"
    route = payload.get(lookup_label)
    if route is None:
        if route_label:
            raise RuntimeError(
                f"route config `{lookup_label}` was not found in OASIS7_REMOTE_LLM_ROUTES_PATH"
            )
        return load_newapi_bridge_state_route(route_label)
    if not isinstance(route, dict):
        raise RuntimeError(f"route config `{lookup_label}` must be a JSON object")
    return route


def load_newapi_bridge_state_route(route_label: str) -> dict:
    state_path = env_optional("OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH")
    if not state_path or not route_label:
        return {}
    try:
        with open(state_path, "r", encoding="utf-8") as handle:
            payload = json.load(handle)
    except OSError as exc:
        raise RuntimeError(
            f"failed to read OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH: {exc}"
        ) from exc
    except json.JSONDecodeError as exc:
        raise RuntimeError(
            "OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH must contain valid JSON: "
            f"{exc}"
        ) from exc
    if not isinstance(payload, dict):
        raise RuntimeError(
            "OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH root must be a JSON object"
        )
    bindings = payload.get("bindings")
    project_bindings = payload.get("project_bindings")
    if not isinstance(bindings, list) or not isinstance(project_bindings, list):
        raise RuntimeError(
            "newapi bridge state must contain `bindings` and `project_bindings` arrays"
        )
    binding = resolve_newapi_binding(bindings, route_label)
    if binding is None:
        raise RuntimeError(
            "no active newapi bridge binding found for "
            f"OASIS7_REMOTE_LLM_ROUTE_LABEL={route_label}"
        )
    bridge_user_id = str(binding.get("bridge_user_id") or "").strip()
    if not bridge_user_id:
        raise RuntimeError(
            f"newapi bridge binding for {route_label} is missing bridge_user_id"
        )
    token_key = resolve_newapi_token_key(project_bindings, bridge_user_id)
    if not token_key:
        raise RuntimeError(
            f"newapi bridge binding for {route_label} does not have a usable token_key"
        )
    return {"api_key": token_key}


def resolve_newapi_binding(bindings: list, route_label: str) -> dict | None:
    normalized_label = route_label.strip()
    by_ref = normalized_label
    by_bridge_user_id = normalized_label
    if ":" in normalized_label:
        prefix, value = normalized_label.split(":", 1)
        value = value.strip()
        if prefix == "newapi_user_ref" and value:
            by_bridge_user_id = ""
            by_ref = value
        elif prefix == "bridge_user_id" and value:
            by_ref = ""
            by_bridge_user_id = value
    for entry in bindings:
        if not isinstance(entry, dict):
            continue
        if str(entry.get("status") or "").strip() != "active":
            continue
        if by_ref and str(entry.get("newapi_user_ref") or "").strip() == by_ref:
            return entry
        if by_bridge_user_id and str(entry.get("bridge_user_id") or "").strip() == by_bridge_user_id:
            return entry
    return None


def resolve_newapi_token_key(project_bindings: list, bridge_user_id: str) -> str:
    selected = None
    for entry in project_bindings:
        if not isinstance(entry, dict):
            continue
        if str(entry.get("bridge_user_id") or "").strip() != bridge_user_id:
            continue
        token_key = str(entry.get("token_key") or "").strip()
        if not token_key:
            continue
        selected = token_key
    return selected or ""


def route_or_env(route: dict, route_key: str, *env_names: str, default: str = "") -> str:
    value = route.get(route_key)
    if isinstance(value, str) and value.strip():
        return value.strip()
    return env_optional(*env_names) or default


def route_or_env_int(route: dict, route_key: str, default: int, *env_names: str) -> int:
    value = route.get(route_key)
    if value is not None:
        try:
            return int(value)
        except (TypeError, ValueError) as exc:
            raise RuntimeError(f"invalid integer for route field {route_key}: {value}") from exc
    return env_int(default, *env_names)


def route_or_env_float(route: dict, route_key: str, default: float, *env_names: str) -> float:
    value = route.get(route_key)
    if value is not None:
        try:
            return float(value)
        except (TypeError, ValueError) as exc:
            raise RuntimeError(f"invalid float for route field {route_key}: {value}") from exc
    return env_float(default, *env_names)


def route_or_env_bool(route: dict, route_key: str, default: bool, *env_names: str) -> bool:
    value = route.get(route_key)
    if isinstance(value, bool):
        return value
    if value is not None:
        return str(value).lower() in {"1", "true", "yes", "on"}
    return env_bool(default, *env_names)


def parse_gateway_call(argv: list[str]) -> tuple[str, int, str]:
    params = ""
    timeout_ms = DEFAULT_TIMEOUT_MS
    agent_id = "letai"
    index = 3
    while index < len(argv):
        flag = argv[index]
        if flag == "--params":
            index += 1
            if index >= len(argv):
                raise RuntimeError("--params requires a value")
            params = argv[index]
        elif flag == "--timeout":
            index += 1
            if index >= len(argv):
                raise RuntimeError("--timeout requires a value")
            # `agent --timeout` comes from the local embedded fallback path, which
            # still passes seconds rather than milliseconds.
            timeout_ms = int(argv[index]) * 1000
        index += 1
    if not params:
        raise RuntimeError("gateway call requires --params")
    payload = json.loads(params)
    prompt = str(payload.get("message", "")).strip()
    if not prompt:
        raise RuntimeError("gateway params missing message")
    agent_id = str(payload.get("agentId", agent_id)).strip() or agent_id
    return prompt, max(timeout_ms, 1000), agent_id


def parse_local_agent(argv: list[str]) -> tuple[str, int, str]:
    prompt = ""
    timeout_ms = DEFAULT_TIMEOUT_MS
    agent_id = "letai"
    index = 1
    while index < len(argv):
        flag = argv[index]
        if flag == "--message":
            index += 1
            if index >= len(argv):
                raise RuntimeError("--message requires a value")
            prompt = argv[index]
        elif flag == "--timeout":
            index += 1
            if index >= len(argv):
                raise RuntimeError("--timeout requires a value")
            # `gateway call agent --timeout` is already passed in milliseconds.
            timeout_ms = int(argv[index])
        elif flag == "--agent":
            index += 1
            if index >= len(argv):
                raise RuntimeError("--agent requires a value")
            agent_id = argv[index].strip() or agent_id
        index += 1
    if not prompt.strip():
        raise RuntimeError("agent invocation missing --message")
    return prompt.strip(), max(timeout_ms, 1000), agent_id


def content_from_choice(choice: dict) -> str:
    message = choice.get("message") or {}
    content = message.get("content")
    if isinstance(content, str):
        return content.strip()
    if isinstance(content, list):
        text_parts: list[str] = []
        for item in content:
            if isinstance(item, dict):
                if isinstance(item.get("text"), str):
                    text_parts.append(item["text"])
                elif item.get("type") == "text":
                    text = item.get("content") or item.get("value")
                    if isinstance(text, str):
                        text_parts.append(text)
        return "".join(text_parts).strip()
    return ""


def make_headers(api_key: str) -> dict[str, str]:
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
        "User-Agent": env_optional("OASIS7_REMOTE_LLM_USER_AGENT") or DEFAULT_USER_AGENT,
    }
    extra_headers_json = env_optional(
        "OASIS7_REMOTE_LLM_EXTRA_HEADERS_JSON", "LETAI_EXTRA_HEADERS_JSON"
    )
    if extra_headers_json:
        try:
            extra_headers = json.loads(extra_headers_json)
        except json.JSONDecodeError as exc:
            raise RuntimeError(
                "OASIS7_REMOTE_LLM_EXTRA_HEADERS_JSON must be valid JSON"
            ) from exc
        if not isinstance(extra_headers, dict):
            raise RuntimeError("OASIS7_REMOTE_LLM_EXTRA_HEADERS_JSON must be a JSON object")
        for key, value in extra_headers.items():
            headers[str(key)] = str(value)
    return headers


def request_completion(prompt: str, timeout_ms: int, agent_id: str) -> dict:
    route = load_route_config()
    base_url = normalize_base_url(
        route_or_env(route, "base_url", "OASIS7_REMOTE_LLM_BASE_URL", "LETAI_BASE_URL", default=DEFAULT_BASE_URL)
    )
    api_key = route_or_env(route, "api_key", "OASIS7_REMOTE_LLM_API_KEY", "LETAI_API_KEY")
    if not api_key:
        raise RuntimeError("missing required remote LLM api key")
    model = route_or_env(route, "model", "OASIS7_REMOTE_LLM_MODEL", "LETAI_MODEL")
    if not model:
        raise RuntimeError("missing required remote LLM model")
    system_prompt = route_or_env(
        route,
        "system_prompt",
        "OASIS7_REMOTE_LLM_SYSTEM_PROMPT", "LETAI_SYSTEM_PROMPT"
    )
    max_output_tokens = route_or_env_int(
        route,
        "max_output_tokens",
        DEFAULT_MAX_OUTPUT_TOKENS,
        "OASIS7_REMOTE_LLM_MAX_OUTPUT_TOKENS",
        "LETAI_MAX_OUTPUT_TOKENS",
    )
    temperature = route_or_env_float(
        route,
        "temperature",
        DEFAULT_TEMPERATURE,
        "OASIS7_REMOTE_LLM_TEMPERATURE",
        "LETAI_TEMPERATURE",
    )
    use_json_object = route_or_env_bool(
        route,
        "response_format_json_object",
        False,
        "OASIS7_REMOTE_LLM_RESPONSE_FORMAT_JSON_OBJECT",
        "LETAI_RESPONSE_FORMAT_JSON_OBJECT",
    )

    messages = []
    if system_prompt:
        messages.append({"role": "system", "content": system_prompt})
    messages.append({"role": "user", "content": prompt})

    body = {
        "model": model,
        "messages": messages,
        "temperature": temperature,
        "stream": True,
        "max_tokens": max_output_tokens,
        "user": f"oasis7-provider:{agent_id}",
    }
    if use_json_object:
        body["response_format"] = {"type": "json_object"}

    request = urllib.request.Request(
        url=f"{base_url}/chat/completions",
        data=json.dumps(body).encode("utf-8"),
        headers=make_headers(api_key),
        method="POST",
    )
    started = time.time()
    try:
        with urllib.request.urlopen(request, timeout=max(timeout_ms, 1000) / 1000.0) as response:
            payload = response.read().decode("utf-8")
            status_code = response.status
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"upstream chat completion returned HTTP {exc.code}: {detail}") from exc
    except urllib.error.URLError as exc:
        raise RuntimeError(f"upstream chat completion request failed: {exc}") from exc

    if status_code < 200 or status_code >= 300:
        raise RuntimeError(f"upstream chat completion returned unexpected HTTP {status_code}")
    decoded, content, usage = decode_completion_payload(payload)
    duration_ms = int((time.time() - started) * 1000)
    return {
        "payloads": [{"text": content}],
        "meta": {
            "durationMs": duration_ms,
            "agentMeta": {
                "provider": "letai",
                "model": str(decoded.get("model") or model),
                "promptTokens": usage.get("prompt_tokens"),
                "usage": {
                    "output": usage.get("completion_tokens"),
                    "total": usage.get("total_tokens"),
                },
            },
        },
    }


def decode_completion_payload(payload: str) -> tuple[dict, str, dict]:
    stripped = payload.strip()
    if not stripped:
        raise RuntimeError("upstream response body was empty")
    if any(
        line.strip().startswith("data:")
        for line in payload.splitlines()
        if line.strip()
    ):
        return decode_sse_completion_payload(stripped)
    decoded = json.loads(payload)
    choices = decoded.get("choices")
    if not isinstance(choices, list) or not choices:
        raise RuntimeError("upstream response missing choices[0]")
    content = content_from_choice(choices[0])
    if not content:
        raise RuntimeError("upstream response missing choices[0].message.content")
    usage = decoded.get("usage") or {}
    return decoded, content, usage


def decode_sse_completion_payload(payload: str) -> tuple[dict, str, dict]:
    text_parts: list[str] = []
    usage: dict = {}
    last_chunk: dict = {}
    for raw_line in payload.splitlines():
        line = raw_line.strip()
        if not line or not line.startswith("data:"):
            continue
        data = line[5:].strip()
        if not data or data == "[DONE]":
            continue
        chunk = json.loads(data)
        last_chunk = chunk
        choices = chunk.get("choices")
        if isinstance(choices, list):
            for choice in choices:
                if not isinstance(choice, dict):
                    continue
                delta = choice.get("delta") or {}
                if isinstance(delta, dict):
                    content = delta.get("content")
                    if isinstance(content, str):
                        text_parts.append(content)
                if not text_parts:
                    message_content = content_from_choice(choice)
                    if message_content:
                        text_parts.append(message_content)
        if isinstance(chunk.get("usage"), dict):
            usage = chunk["usage"]
    content = "".join(text_parts).strip()
    if not content:
        raise RuntimeError("upstream SSE response did not contain assistant content")
    return last_chunk, content, usage


def main() -> int:
    argv = sys.argv[1:]
    if not argv:
        print("usage: letai_provider_cli.py <gateway|agent> ...", file=sys.stderr)
        return 2
    try:
        if argv[:3] == ["gateway", "call", "agent"]:
            prompt, timeout_ms, agent_id = parse_gateway_call(argv)
            result = request_completion(prompt, timeout_ms, agent_id)
            sys.stdout.write(json.dumps({"result": result}, ensure_ascii=True))
            return 0
        if argv[0] == "agent":
            prompt, timeout_ms, agent_id = parse_local_agent(argv)
            result = request_completion(prompt, timeout_ms, agent_id)
            sys.stdout.write(json.dumps(result, ensure_ascii=True))
            return 0
        raise RuntimeError(f"unsupported invocation mode: {' '.join(argv[:3])}")
    except Exception as exc:
        print(str(exc), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
