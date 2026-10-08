"""Endpoint facts for Cargo scope; documentation never removes owners.

Loaded by exact trusted sibling paths, including under Python -I. No candidate
repository path is placed on sys.path, and cold imports do not write bytecode.
"""
from __future__ import annotations

import importlib.util
from pathlib import Path, PurePosixPath
import re
import sys

sys.dont_write_bytecode = True
_corpus_path = Path(__file__).resolve().parents[1] / "document_corpus.py"
_spec = importlib.util.spec_from_file_location("cargo_scope_document_corpus", _corpus_path)
if _spec is None or _spec.loader is None:
    raise ImportError(f"trusted corpus module unavailable: {_corpus_path}")
corpus = importlib.util.module_from_spec(_spec)
sys.modules[_spec.name] = corpus
_spec.loader.exec_module(corpus)


def governance_control(path: str, protected: set[str]) -> bool:
    return (path in protected or PurePosixPath(path).name == "AGENTS.md"
            or path.startswith((".agents/", ".codex/", ".github/", ".pm/", ".cargo/", "scripts/pm/", "doc/.governance/", "doc/testing/evidence/", "doc/engineering/workflow/")))


def ordinary_document(view, path: str, protected: set[str]) -> bool:
    if governance_control(path, protected) or view.file_mode(path) != "100644":
        return False
    # Use the corpus kind model, then require the supported text representation.
    if corpus.object_kind(path) == "supporting_artifact":
        return False
    # Registered document domains plus the model's repository landing page
    # are supported. An arbitrary Markdown suffix outside this model is not
    # authority to exempt an otherwise unknown repository asset.
    if not path.startswith("doc/") and path != "README.md":
        return False
    try:
        data = view.read_bytes(path)
        data.decode("utf-8", "strict")
        if b"\0" in data:
            return False
        if path.startswith("doc/"):
            model = corpus.expected_object(view, path)  # trusted registered domain
            if model["authority_layer"] in {"evidence_domain", "controlled_exception"}:
                return False
    except (UnicodeError, corpus.CorpusError):
        return False
    return True


def compile_consumers(repo, revision, paths, packages, owner_for, source_text,
                      strip_comments, resolve_relative, resolve_manifest_relative,
                      build_scripts=None):
    """Collect static Rust input consumers before any document admission.

    Unresolved references remain subject to the existing checker; this query
    does not grant a waiver for arbitrary macro/build-script evaluation.
    """
    consumers: dict[str, set[str]] = {}
    literal = re.compile(r'(?:"([^"\n\\]+)"|r(#{0,255})"([^"\n]+)"\2)')
    manifest = re.compile(r'concat\s*!\s*\(\s*env\s*!\s*\(\s*"CARGO_MANIFEST_DIR"\s*\)\s*,\s*"([^"\\\n]*)"\s*\)')
    for path in paths:
        owner = owner_for(path, packages)
        if not owner or not path.endswith(".rs"):
            continue
        text = strip_comments(source_text(repo, revision, path))
        for match in re.finditer(r'\binclude(?:_bytes|_str)?\s*!\s*\(\s*|#\s*\[\s*path\s*=\s*', text):
            expression = text[match.end():]
            simple = literal.match(expression)
            relative = manifest.match(expression)
            resolved = None
            if simple:
                resolved = resolve_relative(path, simple.group(1) or simple.group(3))
            elif relative:
                resolved = resolve_manifest_relative(repo, packages, owner, relative.group(1))
            if resolved:
                consumers.setdefault(resolved, set()).add(owner)
            else:
                consumers.setdefault("<unresolved>", set()).add(owner)
        if path in (build_scripts or {}).get(owner, set()):
            manifest_path = next((target for _, target in packages[owner] if target.endswith("Cargo.toml")), None)
            for match in re.finditer(r'cargo(?::|::)rerun-if-changed=([^"\n]+)', text):
                resolved = resolve_relative(manifest_path, match.group(1)) if manifest_path else None
                if resolved:
                    consumers.setdefault(resolved, set()).add(owner)
            for match in re.finditer(r'\b(?:read|read_to_string|read_to_end|open)\s*\(\s*', text):
                simple = literal.match(text[match.end():])
                resolved = resolve_relative(manifest_path, simple.group(1) or simple.group(3)) if simple and manifest_path else None
                if resolved:
                    consumers.setdefault(resolved, set()).add(owner)
                else:
                    consumers.setdefault("<unresolved>", set()).add(owner)
    return consumers


def endpoint_facts(view, path: str, packages, consumers, owner_for, protected: set[str]):
    present = view.file_mode(path) != "000000"
    if not present:
        return {"owners": set(), "document": False, "control": False,
                "source": None, "present": False}
    owners = set(consumers.get(path, set()))
    owner = owner_for(path, packages)
    if owner:
        owners.add(owner)
    source = None
    if present and path.startswith(corpus.OBJECTS_ROOT + "/"):
        source = corpus.direct_object_source(view, path)
        source_facts = endpoint_facts(view, source, packages, consumers, owner_for, protected)
        owners.update(source_facts["owners"])
        return {"owners": owners, "document": source_facts["document"],
                "control": source_facts["control"], "source": source, "present": True}
    control = present and governance_control(path, protected)
    if path.startswith("doc/") and not control:
        try:
            model = corpus.expected_object(view, path)
            control = model["authority_layer"] in {"evidence_domain", "controlled_exception"}
        except corpus.CorpusError:
            pass
    document = present and not owners and not consumers.get("<unresolved>") and ordinary_document(view, path, protected)
    return {"owners": owners, "document": document, "control": control,
            "source": source, "present": present}
