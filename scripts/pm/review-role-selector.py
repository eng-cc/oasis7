#!/usr/bin/env python3
"""Select the minimum repo-owned review roles from an explicit risk class."""
from __future__ import annotations

import argparse
import json
import re
import sys

ROLE_RE = re.compile(r"[a-z][a-z0-9_]*")
DOMAIN_SPECIALIST_ROLES = {
    "producer_system_designer", "gameplay_designer",
    "game_visual_interaction_designer", "runtime_engineer",
    "blockchain_ops_engineer", "wasm_platform_engineer",
    "agent_engineer", "viewer_engineer",
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--change-class", required=True,
                        choices=("mechanical-doc", "workflow-doc", "domain-semantic-doc",
                                 "external-messaging", "unknown", "mixed"))
    parser.add_argument("--domain-role")
    parser.add_argument("--verification-affected", action="store_true")
    parser.add_argument("--changed-path-list",
                        help="semicolon-delimited paths; explicit risk classes require doc-only scope")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    if args.changed_path_list is not None:
        paths = [path.strip() for path in args.changed_path_list.split(";") if path.strip()]
        non_docs = [path for path in paths if not (
            path.startswith("doc/") or path.endswith(".md") or path == "README" or path.startswith("README.")
        )]
        if non_docs:
            print("review-role-selector: explicit change class requires documentation-only paths: "
                  + ",".join(non_docs), file=sys.stderr)
            return 2
    if args.change_class in {"unknown", "mixed"}:
        print("review-role-selector: manual role selection required for unknown or mixed risk", file=sys.stderr)
        return 2
    roles = ["repository_health_engineer"]
    if args.change_class in {"mechanical-doc", "workflow-doc"}:
        roles.append("qa_engineer")
    elif args.change_class == "domain-semantic-doc":
        if (not args.domain_role or not ROLE_RE.fullmatch(args.domain_role)
                or args.domain_role not in DOMAIN_SPECIALIST_ROLES):
            print("review-role-selector: domain role must be one canonical domain specialist", file=sys.stderr)
            return 2
        roles.append(args.domain_role)
        if args.verification_affected:
            roles.append("qa_engineer")
    elif args.change_class == "external-messaging":
        roles.append("liveops_community")
        if args.verification_affected:
            roles.append("qa_engineer")
    payload = {"change_class": args.change_class, "roles": roles,
               "verification_affected": args.verification_affected}
    print(json.dumps(payload, sort_keys=True) if args.json else ",".join(roles))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
