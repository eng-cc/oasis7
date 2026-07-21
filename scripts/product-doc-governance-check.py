#!/usr/bin/env python3
"""Validate the closed four-module product documentation overlay."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import re
import sys


@dataclass(frozen=True)
class ProductModule:
    slug: str
    name: str
    prd_id: str
    authorities: tuple[str, ...]

    @property
    def path(self) -> str:
        return f"doc/product/{self.slug}/prd.md"


MODULES = (
    ProductModule(
        "world-rules-core-gameplay",
        "世界规则与核心玩法",
        "PRD-PRODUCT-001",
        ("doc/game/prd.md", "doc/world-runtime/prd.md", "doc/world-simulator/prd.md", "doc/p2p/prd.md"),
    ),
    ProductModule("world-infrastructure", "大世界基础设施", "PRD-PRODUCT-002", ("doc/game/prd.md", "doc/world-runtime/prd.md", "doc/p2p/prd.md")),
    ProductModule("agents-world-simulation", "智能体与世界模拟", "PRD-PRODUCT-003", ("doc/world-simulator/prd.md",)),
    ProductModule("player-entry-distribution", "玩家入口与发行", "PRD-PRODUCT-004", ("README.md", "doc/world-simulator/prd.md")),
)
LIFECYCLES = {"proposed", "draft", "active", "superseded", "retired"}
RESERVED_PRODUCT_IDENTITY_METADATA = (
    "产品模块",
    "产品模块 slug",
    "产品层唯一 PRD",
    "产品模块总入口",
    "Product PRD-ID",
)
PRODUCT_IDENTITY_EXAMPLE_PATHS = frozenset(
    {"doc/engineering/doc-governance/doc-structure-standard.design.md"}
)
REQUIRED_HEADINGS = (
    "## 文档身份",
    "## 1. 产品承诺",
    "## 2. 范围",
    "## 3. 权威与冲突处理",
    "## 4. 路线图",
    "## 5. Done：成功标准与验收",
    "### 5.1 验收追踪",
    "## 6. Non-Goals",
)


def metadata(text: str, label: str) -> str | None:
    match = re.search(rf"^- {re.escape(label)}：(?:`([^`]+)`|(.+))$", text, re.MULTILINE)
    if not match:
        return None
    return (match.group(1) or match.group(2)).strip()


def reserved_product_identity_metadata_lines(text: str) -> list[tuple[int, str]]:
    """Return exact reserved metadata declarations, never semantic references."""
    labels = "|".join(re.escape(label) for label in RESERVED_PRODUCT_IDENTITY_METADATA)
    pattern = re.compile(rf"^- ({labels})：", re.MULTILINE)
    return [
        (text.count("\n", 0, match.start()) + 1, match.group(1))
        for match in pattern.finditer(text)
    ]


def fail(errors: list[str], code: str, detail: str) -> None:
    errors.append(f"product-doc-governance: {code}: {detail}")


def markdown_targets(root: Path, source: Path, text: str) -> set[str]:
    targets: set[str] = set()
    for raw_target in re.findall(r"\[[^]]+\]\(([^)]+)\)", text):
        target = raw_target.split("#", 1)[0].strip()
        if not target or "://" in target:
            continue
        resolved = (source.parent / target).resolve()
        try:
            targets.add(resolved.relative_to(root).as_posix())
        except ValueError:
            continue
    return targets


def active_topic_targets(root: Path, module_path: Path, text: str) -> set[str]:
    """Return topic PRDs declared by the module's dedicated active-topic section."""
    section = re.search(
        r"^### 活跃产品专题\s*$([\s\S]*?)(?=^#{1,3}\s|\Z)", text, re.MULTILINE
    )
    if not section:
        return set()
    return {
        target
        for target in markdown_targets(root, module_path, section.group(1))
        if target.endswith(".prd.md") and target != module_path.relative_to(root).as_posix()
    }


def check(root: Path) -> list[str]:
    errors: list[str] = []
    for path in sorted(root.joinpath("doc").glob("**/*.md")):
        relative_path = path.relative_to(root).as_posix()
        if relative_path.startswith("doc/product/") or relative_path in PRODUCT_IDENTITY_EXAMPLE_PATHS:
            continue
        for line, label in reserved_product_identity_metadata_lines(path.read_text(encoding="utf-8")):
            fail(
                errors,
                "identity-metadata-location",
                f"{relative_path}:{line}: {label} is reserved for doc/product/**",
            )
    landing_path = root / "doc/product/README.md"
    if not landing_path.is_file():
        return ["product-doc-governance: entry-missing: doc/product/README.md"]
    landing = landing_path.read_text(encoding="utf-8")
    row_re = re.compile(r"^\|\s*([^|]+?)\s*\|\s*\[[^]]+\]\(([^)]+)\)\s*\|", re.MULTILINE)
    rows = [(name.strip(), target.strip()) for name, target in row_re.findall(landing)]
    expected_rows = [(module.name, f"{module.slug}/prd.md") for module in MODULES]
    if rows != expected_rows:
        fail(errors, "entry-contract", f"expected exact four-row manifest {expected_rows!r}, got {rows!r}")
    if len({target for _, target in rows}) != len(rows):
        fail(errors, "entry-duplicate", "product entry targets must be unique")

    expected_paths = {module.path for module in MODULES}
    product_root = root / "doc/product"
    actual_paths = {
        path.relative_to(root).as_posix()
        for pattern in ("**/prd.md", "**/*.prd.md")
        for path in product_root.glob(pattern)
        if path.is_file()
    }
    declared_topics: set[str] = set()

    seen_ids: set[str] = set()
    for module in MODULES:
        path = root / module.path
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        expected_metadata = {
            "产品模块": module.name,
            "产品模块 slug": module.slug,
            "产品层唯一 PRD": module.path,
            "产品模块总入口": "doc/product/README.md",
            "Product PRD-ID": module.prd_id,
            "生命周期": "active",
            "Owner role": "producer_system_designer",
        }
        for label, expected in expected_metadata.items():
            actual = metadata(text, label)
            if actual != expected:
                fail(errors, "metadata-contract", f"{module.path}: {label} expected {expected!r}, got {actual!r}")
        prd_id = metadata(text, "Product PRD-ID")
        if prd_id:
            if prd_id in seen_ids:
                fail(errors, "metadata-duplicate-id", prd_id)
            seen_ids.add(prd_id)
            if "xxx" in prd_id.lower() or not re.fullmatch(r"PRD-PRODUCT-\d{3}", prd_id):
                fail(errors, "metadata-placeholder-id", f"{module.path}: {prd_id}")
        lifecycle = metadata(text, "生命周期")
        if lifecycle not in LIFECYCLES:
            fail(errors, "lifecycle-contract", f"{module.path}: {lifecycle!r}")
        if lifecycle == "active" and metadata(text, "后继文档") != "无":
            fail(errors, "lifecycle-successor", f"{module.path}: active PRD must use successor `无`")
        if not metadata(text, "Last reviewed"):
            fail(errors, "metadata-last-reviewed", module.path)
        for heading in REQUIRED_HEADINGS:
            if heading not in text:
                fail(errors, "section-contract", f"{module.path}: missing heading prefix {heading!r}")
        declared_match = re.search(r"^- 下层专业域：(.+)$", text, re.MULTILINE)
        declared_paths = tuple(
            target
            for target in module.authorities
            if target in markdown_targets(root, path, declared_match.group(1) if declared_match else "")
        )
        if declared_paths != module.authorities:
            fail(errors, "authority-contract", f"{module.path}: expected {module.authorities!r}, got {declared_paths!r}")
        for authority in module.authorities:
            authority_path = root / authority
            if not authority_path.is_file():
                fail(errors, "authority-missing", f"{module.path}: {authority}")
            elif module.path not in markdown_targets(root, authority_path, authority_path.read_text(encoding="utf-8")):
                fail(errors, "authority-backlink", f"{authority} must link {module.path}")

        topics = active_topic_targets(root, path, text)
        for topic in topics:
            topic_path = root / topic
            if topic_path.parent != path.parent:
                fail(errors, "topic-module-boundary", f"{module.path}: {topic}")
            declared_topics.add(topic)

        success_ids = re.findall(r"^- (SC-\d+)：", text, re.MULTILINE)
        trace_rows = re.findall(r"^\|\s*(SC-\d+)\s*\|\s*([^|]+)\|\s*([^|]+)\|\s*([^|]+)\|\s*([^|]+)\|\s*([^|]+)\|$", text, re.MULTILINE)
        trace_ids = [row[0] for row in trace_rows]
        if not success_ids or success_ids != trace_ids or len(trace_ids) != len(set(trace_ids)):
            fail(errors, "traceability-contract", f"{module.path}: success={success_ids!r}, trace={trace_ids!r}")
        for sc_id, owner, prd_ids, authority_docs, evidence, tier in trace_rows:
            prd_refs = re.findall(r"PRD-[A-Z][A-Z0-9_]*(?:-[A-Z0-9]+)+(?:/[A-Z0-9]+)*", prd_ids)
            if not owner.strip() or not prd_refs:
                fail(errors, "traceability-owner-prd", f"{module.path}: {sc_id}")
            row_paths = re.findall(r"`([^`]+\.md)`", authority_docs)
            if not row_paths or any(not (root / row_path).is_file() for row_path in row_paths):
                fail(errors, "traceability-authority", f"{module.path}: {sc_id}")
            else:
                authority_texts = [(root / row_path).read_text(encoding="utf-8") for row_path in row_paths]
                for prd_ref in prd_refs:
                    if not any(prd_ref in authority_text for authority_text in authority_texts):
                        fail(errors, "traceability-prd-resolution", f"{module.path}: {sc_id} missing {prd_ref} in {row_paths!r}")
            if not evidence.strip() or tier.strip() not in {"test_tier_required", "test_tier_full"}:
                fail(errors, "traceability-evidence-tier", f"{module.path}: {sc_id}")

    expected_inventory = expected_paths | declared_topics
    if actual_paths != expected_inventory:
        fail(
            errors,
            "topic-index-contract",
            f"expected roots plus declared topics {sorted(expected_inventory)!r}, got {sorted(actual_paths)!r}",
        )
    if len(declared_topics) != sum(
        len(active_topic_targets(root, root / module.path, (root / module.path).read_text(encoding="utf-8")))
        for module in MODULES
        if (root / module.path).is_file()
    ):
        fail(errors, "topic-duplicate-declaration", "an active product topic may have one module owner")

    for topic in declared_topics:
        topic_path = root / topic
        if not topic_path.is_file():
            continue
        topic_text = topic_path.read_text(encoding="utf-8")
        module_root = topic_path.parent.joinpath("prd.md").relative_to(root).as_posix()
        if module_root not in markdown_targets(root, topic_path, topic_text):
            fail(
                errors,
                "topic-module-backlink",
                f"{topic} must link its owning module PRD {module_root}",
            )
        for suffix in (".design.md", ".project.md"):
            paired_path = topic_path.with_name(topic_path.name.removesuffix(".prd.md") + suffix)
            if paired_path.is_file() and topic not in paired_path.read_text(encoding="utf-8"):
                fail(errors, "topic-pair-backlink", f"{paired_path.relative_to(root)} must reference {topic}")
        if metadata(topic_text, "产品层唯一 PRD") not in {None, module_root}:
            fail(errors, "topic-module-authority", f"{topic}: 产品层唯一 PRD must name its module root")

    paired_files = [
        path
        for suffix in ("*.design.md", "*.project.md")
        for path in (root / "doc/product").glob(f"**/{suffix}")
        if path.is_file()
    ]
    for paired_path in paired_files:
        topic_path = paired_path.with_name(
            paired_path.name.removesuffix(".design.md").removesuffix(".project.md") + ".prd.md"
        )
        topic = topic_path.relative_to(root).as_posix()
        if topic not in declared_topics:
            fail(errors, "topic-pair-orphan", f"{paired_path.relative_to(root)} has no declared {topic}")
    if any((root / "doc/product").glob("**/archive")):
        fail(errors, "lifecycle-archive", "doc/product/**/archive is forbidden")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args()
    errors = check(args.repo_root.resolve())
    if errors:
        print("\n".join(errors))
        return 1
    print("product-doc-governance: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
