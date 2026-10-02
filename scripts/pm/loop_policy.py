"""Deterministic, immutable loop policy. No task mutations or scheduling.

Admission callers must validate_tool_root before executing trusted helpers.
validate_scope can also run on candidate code in offline tests; that is not
effective-policy activation or native filesystem isolation evidence.
"""
from __future__ import annotations

import fnmatch
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys

from loop_contracts import coordination_ref_errors, consumed_clause_ref_errors

SCHEMA = "oasis7.loop-task/v1"
POLICY_PATH = "scripts/pm/loop-policy.v1.json"
LOOPS = {"product", "system", "code"}
OID = re.compile(r"[0-9a-f]{40}\Z")
UID = re.compile(r"task_[0-9a-f]{32}\Z")
DIGEST = re.compile(r"sha256:[0-9a-f]{64}\Z")
CORPUS_MODULE = "scripts/document_corpus.py"
TRUSTED_IMPORT_FILES = (
    CORPUS_MODULE,
    "scripts/product-doc-content-check.py",
    "scripts/product_doc_markdown.py",
)


def result(blockers, **fields):
    return {"status": "blocked" if blockers else "passed", "blockers": blockers, **fields}


def git(root, *args):
    process = subprocess.run(["git", "-C", str(root), *args], capture_output=True)
    if process.returncode:
        raise ValueError("git read failed: " + process.stderr.decode("utf-8", "replace").strip())
    return process.stdout


def safe_path(value):
    return (isinstance(value, str) and bool(value) and "\\" not in value
            and not any(ord(c) < 32 for c in value) and not value.startswith("/")
            and not any(part in {"", ".", ".."} for part in value.split("/"))
            and not re.match(r"^[A-Za-z]:", value))


def validate_binding(binding):
    errors = []
    if not isinstance(binding, dict):
        return result(["loop_binding must be an object"])
    if binding.get("schema") != SCHEMA:
        errors.append("unsupported loop binding schema")
    for key in ("change_id", "owner_role", "manual_request_ref", "request_key", "target_delivery"):
        if not isinstance(binding.get(key), str) or not binding[key].strip():
            errors.append(f"{key} must be non-empty text")
    for key, regex in (("task_uid", UID), ("policy_commit", OID), ("policy_digest", DIGEST)):
        if not isinstance(binding.get(key), str) or not regex.fullmatch(binding[key]):
            errors.append(f"invalid {key}")
    if binding.get("loop") not in LOOPS:
        errors.append("invalid loop")
    if type(binding.get("bootstrap_epoch")) is not int or binding["bootstrap_epoch"] < 1:
        errors.append("bootstrap_epoch must be a positive integer")
    for key in ("write_scope", "out_of_scope", "acceptance_refs", "dependencies", "input_contracts"):
        if not isinstance(binding.get(key), list):
            errors.append(f"{key} must be a list")
    for key in ("write_scope", "acceptance_refs"):
        if not binding.get(key):
            errors.append(f"{key} must not be empty")
    for key in ("write_scope", "out_of_scope"):
        if isinstance(binding.get(key), list) and any(not safe_path(p) for p in binding[key]):
            errors.append(f"{key} contains unsafe path pattern")
    if isinstance(binding.get("acceptance_refs"), list):
        for acceptance in binding["acceptance_refs"]:
            if isinstance(acceptance, str) and acceptance.strip():
                continue
            if isinstance(acceptance, dict):
                errors.extend(consumed_clause_ref_errors(acceptance, require_identity=False))
                continue
            errors.append("invalid acceptance reference")
    if "coordination_ref" in binding and binding.get("coordination_ref") is not None:
        errors.extend(coordination_ref_errors(binding.get("coordination_ref")))
    if "consumed_clause_refs" in binding:
        refs = binding.get("consumed_clause_refs")
        if not isinstance(refs, list):
            errors.append("consumed_clause_refs must be a list")
        else:
            for reference in refs:
                errors.extend(consumed_clause_ref_errors(reference, require_identity=False))
    deps = binding.get("dependencies")
    if isinstance(deps, list):
        if any(not isinstance(d, str) or not UID.fullmatch(d) for d in deps):
            errors.append("dependencies must be task UIDs")
        elif len(set(deps)) != len(deps) or binding.get("task_uid") in deps:
            errors.append("duplicate or cyclic task dependency")
    if isinstance(binding.get("input_contracts"), list):
        for item in binding["input_contracts"]:
            if not isinstance(item, dict) or not item.get("contract_id") or type(item.get("revision")) is not int or item["revision"] < 1 or not isinstance(item.get("publication_ref"), dict) or not isinstance(item.get("contract_digest"),str) or not DIGEST.fullmatch(item["contract_digest"]):
                errors.append("invalid immutable input contract reference")
                continue
            ref=item["publication_ref"]
            clauses=item.get("consumed_clauses")
            clause_refs = item.get("consumed_clause_refs")
            has_clauses = isinstance(clauses, list) and bool(clauses)
            has_clause_refs = isinstance(clause_refs, list) and bool(clause_refs)
            if not has_clauses and not has_clause_refs:
                errors.append("immutable input contract needs consumed clauses or refs")
            if not isinstance(item["contract_id"],str) or any(type(ref.get(k)) is not int or ref[k]<1 for k in ("issue_number","comment_id")) or (clauses is not None and (not isinstance(clauses,list) or any(not isinstance(c,str) or not c.strip() for c in clauses))) or (clause_refs is not None and (not isinstance(clause_refs,list) or any(consumed_clause_ref_errors(c, require_identity=True) for c in clause_refs))):
                errors.append("invalid contract publication identity/clauses")
    obligations = binding.get("delivery_obligations", [])
    if not isinstance(obligations, list) or any(not isinstance(o, dict) or not o.get("id") or not isinstance(o.get("task_uid"), str) or not UID.fullmatch(o["task_uid"]) or type(o.get("issue_number")) is not int or o["issue_number"]<1 for o in obligations):
        errors.append("invalid delivery obligations")
    return result(errors)


def validate_dependencies(binding, bindings):
    """Require a complete selected dependency closure; never infer missing nodes."""
    errors = list(validate_binding(binding)["blockers"])
    visiting, visited = set(), set()
    def walk(uid):
        if uid in visiting:
            errors.append("cyclic dependency: " + uid)
            return
        if uid in visited:
            return
        item = binding if uid == binding.get("task_uid") else bindings.get(uid)
        if not isinstance(item, dict):
            errors.append("dependency authority missing: " + str(uid))
            return
        errors.extend(validate_binding(item)["blockers"])
        visiting.add(uid)
        for dependency in item.get("dependencies", []):
            if isinstance(dependency, str):
                walk(dependency)
        visiting.remove(uid)
        visited.add(uid)
    if not errors:
        walk(binding["task_uid"])
    return result(errors)


def validate_tool_root(tool_root, target_repo_root, binding):
    """Validate explicit effective checkout; caller must refresh origin first.

    No candidate checkout may bootstrap its own admission. This local check
    cannot establish that a remote tracking ref is fresh; live gate callers
    retain their existing refresh/readback responsibility.
    """
    errors = list(validate_binding(binding)["blockers"])
    if errors:
        return result(errors)
    try:
        commit = binding["policy_commit"]
        target = Path(target_repo_root).resolve()
        tool = Path(tool_root).resolve()
        def common(root):
            return Path(git(root, "rev-parse", "--path-format=absolute", "--git-common-dir").decode().strip()).resolve()
        if common(tool) != common(target):
            errors.append("tool and target Git common-directory mismatch")
        if git(tool, "rev-parse", "HEAD").decode().strip() != commit:
            errors.append("tool_root is not the pinned effective policy commit")
        git(target, "merge-base", "--is-ancestor", commit, "refs/remotes/origin/main")
        git(tool, "diff", "--exit-code", commit, "--", "scripts/pm", *TRUSTED_IMPORT_FILES, "scripts/prepare-task-pr.sh", "scripts/plan-rust-required-scope.py")
        # Untracked import shadows are executable authority too.
        untracked = git(tool, "ls-files", "--others", "--", "scripts/pm", *TRUSTED_IMPORT_FILES, ":(exclude)**/__pycache__/**")
        if untracked.strip():
            errors.append("untracked files in trusted helper directory")
        for raw in git(tool,"ls-files","-z","--","scripts/pm").split(b"\0"):
            if raw:
                path=tool / raw.decode("utf-8")
                if path.is_symlink() or not path.resolve().is_relative_to(tool):
                    errors.append("trusted helper path escapes tool_root")
        existing_imports = tuple(
            path for path in TRUSTED_IMPORT_FILES
            if git(tool, "ls-tree", "-r", "--name-only", commit, "--", path).strip()
        )
        errors.extend(_trusted_file_errors(tool, commit, existing_imports))
        load_policy(tool, binding)
    except (ValueError, OSError, KeyError, json.JSONDecodeError) as exc:
        errors.append(str(exc))
    return result(errors, tool_root=str(tool_root), target_repo_root=str(target_repo_root))


def _trusted_file_errors(tool_root, commit, relative_paths):
    """Verify exact imported repository modules without following shadow paths."""
    errors = []
    tool = Path(tool_root).resolve()
    for relative in relative_paths:
        try:
            tree = git(tool, "ls-tree", "-z", commit, "--", relative)
            records = [item for item in tree.split(b"\0") if item]
            if len(records) != 1:
                errors.append("trusted module missing or ambiguous: " + relative)
                continue
            metadata, raw_path = records[0].split(b"\t", 1)
            mode, object_type, _oid = metadata.decode("ascii").split()
            if raw_path.decode("utf-8", "strict") != relative or mode != "100644" or object_type != "blob":
                errors.append("trusted module has unsafe Git mode: " + relative)
                continue
            path = tool / relative
            if path.is_symlink() or not path.resolve().is_relative_to(tool) or not path.is_file():
                errors.append("trusted module path escapes tool_root: " + relative)
                continue
            expected = git(tool, "show", commit + ":" + relative)
            if path.read_bytes() != expected:
                errors.append("effective helper bytes differ: " + relative)
            if git(tool, "ls-files", "--others", "--", relative).strip():
                errors.append("untracked trusted module shadow: " + relative)
        except (ValueError, OSError, UnicodeError, subprocess.CalledProcessError) as exc:
            errors.append("trusted module unavailable: " + relative + ": " + str(exc))
    return errors


def load_trusted_corpus_module(tool_root, target_repo_root, binding):
    """Execute only the corpus parser bytes from the pinned effective helper commit.

    This intentionally avoids normal import resolution: target ``sys.path``,
    ``sys.modules`` entries, and a candidate ``scripts/document_corpus.py``
    cannot select or shadow admission code.
    """
    blockers = list(validate_binding(binding)["blockers"])
    if blockers:
        raise ValueError("invalid loop binding: " + "; ".join(blockers))
    tool = Path(tool_root).resolve()
    target = Path(target_repo_root).resolve()
    commit = binding["policy_commit"]
    if git(tool, "rev-parse", "HEAD").decode().strip() != commit:
        raise ValueError("tool root is not the pinned effective policy commit")
    if git(tool, "rev-parse", "--path-format=absolute", "--git-common-dir").decode().strip() != git(target, "rev-parse", "--path-format=absolute", "--git-common-dir").decode().strip():
        raise ValueError("tool and target Git common-directory mismatch")
    git(target, "merge-base", "--is-ancestor", commit, "refs/remotes/origin/main")
    errors = _trusted_file_errors(tool, commit, (CORPUS_MODULE,))
    if errors:
        raise ValueError("; ".join(errors))
    source = git(tool, "show", commit + ":" + CORPUS_MODULE)
    module_name = "_oasis7_effective_document_corpus_" + commit
    import types
    module = types.ModuleType(module_name)
    module.__file__ = str(tool / CORPUS_MODULE)
    module.__package__ = ""
    # Replace even a correctly named/pre-seeded entry; its origin is not proof
    # that its module object contains the pinned source bytes.
    sys.modules[module_name] = module
    exec(compile(source, module.__file__, "exec"), module.__dict__)
    return module


def load_policy(tool_root, binding):
    raw = git(tool_root, "show", binding["policy_commit"] + ":" + POLICY_PATH)
    if "sha256:" + hashlib.sha256(raw).hexdigest() != binding["policy_digest"]:
        raise ValueError("effective policy digest mismatch")
    policy = json.loads(raw)
    if policy.get("schema") != "oasis7.loop-policy/v1" or not isinstance(policy.get("rules"), list) or not policy["rules"]:
        raise ValueError("invalid effective policy")
    for rule in policy["rules"]:
        if not isinstance(rule, dict) or not safe_path(rule.get("pattern")) or rule.get("loop") not in LOOPS:
            raise ValueError("invalid classification rule")
    if not isinstance(policy.get("denied"), list) or any(not safe_path(p) for p in policy["denied"]):
        raise ValueError("invalid denied paths")
    if not isinstance(policy.get("document_extensions"),list) or any(not isinstance(x,str) or not x.startswith(".") for x in policy["document_extensions"]):
        raise ValueError("invalid document type policy")
    if not isinstance(policy.get("mixed_documents", []), list) or any(not safe_path(p) for p in policy.get("mixed_documents", [])):
        raise ValueError("invalid mixed document policy")
    return policy


def path_matches(path, pattern):
    """Match a whole POSIX path; only a standalone ** crosses components."""
    parts = path.split("/")
    positions = {0}
    for component in pattern.split("/"):
        if component == "**":
            positions = set(range(min(positions), len(parts) + 1))
        else:
            positions = {i + 1 for i in positions if i < len(parts)
                         and fnmatch.fnmatchcase(parts[i], component)}
        if not positions:
            return False
    return len(parts) in positions


def classify_path(path, policy):
    if not safe_path(path) or any(path_matches(path, p) for p in policy["denied"]):
        return None
    if path in policy.get("mixed_documents", []):
        return None
    for rule in policy["rules"]:
        if path_matches(path, rule["pattern"]):
            if rule["loop"] in {"product","system"} and PurePosixPath(path).suffix.lower() not in policy["document_extensions"]:
                return None
            return rule["loop"]
    return None


def _corpus_locations(core, model):
    """Index source records by their exact endpoint without trusting path names."""
    indexed = {}
    sources = set(model.objects_by_path)
    sources.update(model.semantic_entries_by_path)
    sources.update(model.path_to_bundle)
    sources.update(model.evidence_entries_by_path)
    sources.update(model.path_to_group)
    for source in sorted(sources):
        for location in core.locate(model, source):
            if location.kind != "static-control":
                indexed.setdefault(location.record_path, []).append(location)
    return indexed


def _corpus_owner(core, policy, model, source):
    """Apply narrow v3 source/evidence ownership before legacy path policy."""
    controls = {
        core.CORPUS_ROOT,
        core.EVIDENCE_ROOT,
        core.REGISTRY_PATH,
        core.LEGACY_SEMANTIC_ROOT,
    }
    if source in controls or source in model.metadata_paths:
        return "code"
    if source.startswith("doc/testing/evidence/"):
        if source == core.EVIDENCE_ROOT:
            return "code"
        if PurePosixPath(source).suffix.lower() in core.EVIDENCE_SUFFIXES:
            return "system"
        return None
    return classify_path(source, policy)


def _declared_path(path, patterns):
    return any(path_matches(path, pattern) for pattern in patterns)


def _check_scope_membership(errors, binding, required, changed_path):
    for path in sorted(required):
        if not _declared_path(path, binding["write_scope"]) or _declared_path(path, binding["out_of_scope"]):
            errors.append(f"outside declared write scope: {changed_path} requires {path}")


def _corpus_scope_errors(core, policy, root, binding, base, head, changed_paths):
    """Resolve every changed doc path against immutable base/head corpus models."""
    base_view = core.GitCorpusView(root, base)
    head_view = core.GitCorpusView(root, head)
    base_model = core.load_corpus(base_view)
    head_model = core.load_corpus(head_view)
    snapshots = ((base_view, base_model, set(base_view.list_doc_paths())),
                 (head_view, head_model, set(head_view.list_doc_paths())))
    endpoint_indexes = [_corpus_locations(core, model) for _, model, _paths in snapshots]
    errors = []
    resolved = []

    def endpoint_locations(path, model, endpoint_index):
        if path in endpoint_index:
            return endpoint_index[path]
        if path in model.metadata_paths or path in {
            core.CORPUS_ROOT, core.EVIDENCE_ROOT, core.REGISTRY_PATH,
            core.LEGACY_SEMANTIC_ROOT,
        }:
            return [core.RecordLocation("static-control", path, path, (path,))]
        return []

    for path in changed_paths:
        if not path.startswith("doc/"):
            continue
        required = {path}
        owners = []
        object_sources = set()
        # A changed shard endpoint inherits from every exact source/group
        # location in both immutable snapshots. Static controls remain code.
        matched_endpoint = False
        for (_view, model, view_paths), index in zip(snapshots, endpoint_indexes):
            locations = endpoint_locations(path, model, index)
            if locations:
                matched_endpoint = True
            for location in locations:
                if location.kind == "static-control":
                    owners.append("code")
                    continue
                owners.extend(_corpus_owner(core, policy, model, member) if member in view_paths else None
                              for member in location.member_paths)
                object_sources.update(location.member_paths)
                required.update(location.member_paths)
                # A group/bundle scope includes every member's own endpoint
                # locations as well as the atomic aggregate record.
                for member in location.member_paths:
                    for member_location in core.locate(model, member):
                        if member_location.kind != "static-control":
                            required.add(member_location.record_path)

        # Ordinary sources need their prospective exact record even when the
        # record is newly added, already missing, or being deleted.
        source_found = False
        for (_view, model, view_paths), index in zip(snapshots, endpoint_indexes):
            is_metadata = path in model.metadata_paths
            is_sidecar_endpoint = path in index
            if is_metadata and not is_sidecar_endpoint:
                matched_endpoint = True
                owners.append("code")
                continue
            if is_metadata:
                # Its ownership was inherited from the exact record members
                # above; never reclassify a valid shard as a control file.
                continue
            owner = _corpus_owner(core, policy, model, path) if path in view_paths else None
            if owner is None:
                continue
            source_found = True
            owners.append(owner)
            locations = core.locate(model, path)
            for location in locations:
                if location.kind != "static-control":
                    required.add(location.record_path)
                    required.update(location.member_paths)
                    owners.extend(_corpus_owner(core, policy, model, member) if member in view_paths else None
                                  for member in location.member_paths)
                    object_sources.update(location.member_paths)
            if owner in {"product", "system"}:
                object_sources.add(path)
            if path.startswith("doc/testing/evidence/") and path != core.EVIDENCE_ROOT:
                required.add(core.record_path("evidence", path))
            elif owner in {"product", "system"}:
                required.add(core.record_path("object", path))

        # New files beneath the corpus metadata root must be known records or
        # fixed controls; the shared loader rejects unknown metadata above.
        if path.startswith("doc/.governance/document-corpus/") and not matched_endpoint:
            errors.append(f"unregistered corpus metadata endpoint: {path}")
        concrete_owners = {owner for owner in owners if owner is not None}
        if not source_found and not matched_endpoint:
            concrete_owners.add(classify_path(path, policy))
        if len(concrete_owners) != 1 or None in concrete_owners or any(owner is None for owner in owners):
            errors.append(f"corpus source or endpoint ownership is unknown or crosses loops: {path}")
            resolved.append({"path": path, "loop": None})
        elif next(iter(concrete_owners)) != binding["loop"]:
            errors.append(f"loop ownership mismatch: {path}")
            resolved.append({"path": path, "loop": next(iter(concrete_owners))})
        else:
            owner = next(iter(concrete_owners))
            resolved.append({"path": path, "loop": owner})
            for view, _model, _view_paths in snapshots:
                mode = view.file_mode(path)
                if mode == "000000":
                    continue
                allowed = {"100644", "100755"} if owner == "code" else {"100644"}
                if mode not in allowed:
                    errors.append(f"unsupported asset mode {mode}: {path}@{view.snapshot_id}")
        for source in sorted(object_sources):
            if source.startswith("doc/testing/evidence/") and source != core.EVIDENCE_ROOT:
                continue
            for view, model, _view_paths in snapshots:
                mode = view.file_mode(source)
                record = model.objects_by_path.get(source)
                if mode == "000000":
                    if record is not None:
                        errors.append(f"generated object has no source: {source}@{view.snapshot_id}")
                    continue
                try:
                    expected_record = core.expected_object(view, source)
                except (ValueError, OSError) as exc:
                    errors.append(f"cannot derive object endpoint for {source}@{view.snapshot_id}: {exc}")
                    continue
                if record != expected_record:
                    errors.append(f"generated object mismatch: {source}@{view.snapshot_id}")
        _check_scope_membership(errors, binding, required, path)

    return errors, resolved


def scope_context(root, integration_base, head):
    """Bind task-owned diff separately from current integration composition."""
    if not all(isinstance(value, str) and OID.fullmatch(value) for value in (integration_base, head)):
        raise ValueError('scope context requires immutable integration/head OIDs')
    bases = git(root, 'merge-base', '--all', integration_base, head).decode().splitlines()
    if len(bases) != 1:
        raise ValueError('scope comparison requires one unambiguous merge-base')
    tree = git(root, 'merge-tree', '--write-tree', integration_base, head).decode().splitlines()[0]
    return {'scope_base_oid': bases[0], 'integration_base_oid': integration_base, 'source_head_oid': head, 'integration_tree_oid': tree}


def validate_scope(tool_root, target_repo_root, binding, base, head):
    errors = list(validate_binding(binding)["blockers"])
    paths = []
    if errors:
        return result(errors, execution_scope="execution_scope_unverified")
    try:
        if not isinstance(base, str) or not OID.fullmatch(base) or not isinstance(head, str) or not OID.fullmatch(head):
            raise ValueError("scope requires immutable base/head OIDs")
        git(target_repo_root, "merge-base", "--is-ancestor", base, head)
        policy = load_policy(tool_root, binding)
        # --no-renames checks both endpoints as deletion/addition even when Git
        # rename similarity heuristics would miss a move or copy.
        changed = git(target_repo_root, "diff", "--no-renames", "--name-only", "-z", base, head).split(b"\0")
        changed_paths = []
        for raw in changed:
            if not raw:
                continue
            path = raw.decode("utf-8", "strict")
            changed_paths.append(path)
            if path in policy.get("mixed_documents", []):
                errors.append(f"mixed document requires separately authorized semantic split/migration: {path}")
            if not path.startswith("doc/"):
                owner = classify_path(path, policy)
                if owner != binding["loop"]:
                    errors.append(f"loop ownership mismatch or unknown path: {path}")
                paths.append({"path": path, "loop": owner})
            if not any(path_matches(path, p) for p in binding["write_scope"]) or any(path_matches(path, p) for p in binding["out_of_scope"]):
                errors.append(f"outside declared write scope: {path}")
            if not path.startswith("doc/"):
                for revision in (base, head):
                    entry = git(target_repo_root, "ls-tree", "-z", revision, "--", path)
                    if not entry:
                        continue
                    mode = entry.split(b" ", 1)[0].decode()
                    allowed = {"100644", "100755"} if owner == "code" else {"100644"}
                    if mode not in allowed:
                        errors.append(f"unsupported asset mode {mode}: {path}@{revision}")
        if any(path.startswith("doc/") for path in changed_paths):
            core = load_trusted_corpus_module(tool_root, target_repo_root, binding)
            corpus_errors, corpus_paths = _corpus_scope_errors(
                core, policy, target_repo_root, binding, base, head, changed_paths,
            )
            errors.extend(corpus_errors)
            paths.extend(corpus_paths)
    except (ValueError, OSError, UnicodeError, KeyError, json.JSONDecodeError) as exc:
        errors.append(str(exc))
    return result(errors, paths=paths, execution_scope="execution_scope_unverified", policy_commit=binding.get("policy_commit"))
