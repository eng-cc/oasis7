#!/usr/bin/env python3

import argparse
import json
import secrets
import sys
from pathlib import Path


DEFAULT_BASE_URL = "https://api.letai.run/v1"


def load_json_object(path: Path) -> dict:
    if not path.exists():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"{path} is not valid JSON: {exc}") from exc
    if not isinstance(payload, dict):
        raise RuntimeError(f"{path} root must be a JSON object")
    return payload


def write_json_object(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def resolve_bridge_token(
    auth_routes: dict,
    user: str,
    requested_token: str | None,
    rotate_token: bool,
) -> str:
    existing_tokens = [token for token, label in auth_routes.items() if label == user]
    if requested_token:
        return requested_token
    if existing_tokens and not rotate_token:
        return existing_tokens[0]
    return secrets.token_hex(24)


def upsert_user(args: argparse.Namespace) -> int:
    auth_routes_path = Path(args.auth_routes)
    llm_routes_path = Path(args.llm_routes)
    auth_routes = load_json_object(auth_routes_path)
    llm_routes = load_json_object(llm_routes_path)

    user = args.user.strip()
    if not user:
        raise RuntimeError("--user must not be empty")

    bridge_token = resolve_bridge_token(
        auth_routes,
        user,
        args.bridge_token.strip() if args.bridge_token else None,
        args.rotate_bridge_token,
    )

    previous_owner = auth_routes.get(bridge_token)
    if previous_owner and previous_owner != user:
        raise RuntimeError(
            f"bridge token already belongs to another route label: {previous_owner}"
        )

    auth_routes = {
        token: label
        for token, label in auth_routes.items()
        if label != user or token == bridge_token
    }
    auth_routes[bridge_token] = user

    route_payload = {
        "api_key": args.api_key,
        "model": args.model,
        "base_url": args.base_url,
    }
    if args.system_prompt:
        route_payload["system_prompt"] = args.system_prompt
    if args.max_output_tokens is not None:
        route_payload["max_output_tokens"] = args.max_output_tokens
    if args.temperature is not None:
        route_payload["temperature"] = args.temperature
    if args.response_format_json_object:
        route_payload["response_format_json_object"] = True

    llm_routes[user] = route_payload

    write_json_object(auth_routes_path, auth_routes)
    write_json_object(llm_routes_path, llm_routes)

    result = {
        "user": user,
        "bridge_token": bridge_token,
        "auth_routes_path": str(auth_routes_path),
        "llm_routes_path": str(llm_routes_path),
        "route": route_payload,
        "curl_example": (
            f"curl -sS -H 'Authorization: Bearer {bridge_token}' "
            "https://t2t.oasis7.tech/v1/provider/info"
        ),
    }
    sys.stdout.write(json.dumps(result, ensure_ascii=True, indent=2) + "\n")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Manage per-user bridge bearer token -> LetAI route mappings."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    upsert = subparsers.add_parser(
        "upsert-user",
        help="Create or update one route label and its bridge bearer token.",
    )
    upsert.add_argument("--auth-routes", required=True, help="Path to auth-routes JSON file.")
    upsert.add_argument("--llm-routes", required=True, help="Path to LLM-routes JSON file.")
    upsert.add_argument("--user", required=True, help="Route label / user key.")
    upsert.add_argument("--bridge-token", help="Fixed bridge bearer token to assign.")
    upsert.add_argument(
        "--rotate-bridge-token",
        action="store_true",
        help="Generate a new bridge bearer token even if the user already has one.",
    )
    upsert.add_argument("--api-key", required=True, help="Upstream LetAI token_key.")
    upsert.add_argument("--model", required=True, help="Upstream model name.")
    upsert.add_argument(
        "--base-url",
        default=DEFAULT_BASE_URL,
        help=f"Upstream base URL. Default: {DEFAULT_BASE_URL}",
    )
    upsert.add_argument("--system-prompt", help="Optional route-scoped system prompt.")
    upsert.add_argument(
        "--max-output-tokens",
        type=int,
        help="Optional route-scoped max output tokens override.",
    )
    upsert.add_argument(
        "--temperature",
        type=float,
        help="Optional route-scoped temperature override.",
    )
    upsert.add_argument(
        "--response-format-json-object",
        action="store_true",
        help="Enable route-scoped response_format={type:json_object}.",
    )
    upsert.set_defaults(func=upsert_user)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    try:
        return args.func(args)
    except RuntimeError as exc:
        print(str(exc), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
