#!/usr/bin/env python3
"""Regression fixtures for product-doc-governance-check.py."""

from __future__ import annotations

from pathlib import Path
import shutil
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parent.parent
CHECKER = ROOT / "scripts/product-doc-governance-check.py"
COPY_PATHS = (
    "README.md",
    "doc/product",
    "doc/game/prd.md",
    "doc/game/gameplay/gameplay-top-level-design.prd.md",
    "doc/core/prd.md",
    "doc/engineering/doc-governance/doc-structure-standard.design.md",
    "doc/world-runtime/prd.md",
    "doc/p2p/prd.md",
    "doc/testing/prd.md",
    "doc/world-simulator/prd.md",
    "doc/world-simulator/llm/provider-agent-experience-parity.prd.md",
    "doc/game/gameplay/gameplay-agent-claim-economy-contract.prd.md",
)


def make_fixture() -> Path:
    root = Path(tempfile.mkdtemp(prefix="oasis7-product-doc-fixture-"))
    for relative in COPY_PATHS:
        source = ROOT / relative
        target = root / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        if source.is_dir():
            shutil.copytree(source, target)
        else:
            shutil.copy2(source, target)
    return root


def run(root: Path, expected: str | None = None) -> None:
    result = subprocess.run(
        ["python3", str(CHECKER), "--repo-root", str(root)],
        check=False,
        capture_output=True,
        text=True,
    )
    output = result.stdout + result.stderr
    if expected is None:
        assert result.returncode == 0, output
        assert "product-doc-governance: OK" in output, output
    else:
        assert result.returncode != 0, output
        assert expected in output, output


def replace(path: Path, before: str, after: str) -> None:
    text = path.read_text(encoding="utf-8")
    assert before in text, f"fixture source missing {before!r} in {path}"
    path.write_text(text.replace(before, after, 1), encoding="utf-8")


def scenario(expected: str | None, mutation) -> None:
    root = make_fixture()
    try:
        mutation(root)
        run(root, expected)
    finally:
        shutil.rmtree(root)


def assert_access_mode_consolidated(root: Path) -> None:
    module = root / "doc/product/player-entry-distribution"
    dated_basename = "player-access-mode-contract-2026-03-19"
    assert not list(module.glob(f"{dated_basename}.*.md")), "dated access-mode companion set must remain absent"
    assert dated_basename not in (module / "prd.md").read_text(encoding="utf-8"), (
        "module PRD must not restore a dated active-topic link"
    )


def main() -> None:
    root = make_fixture()
    try:
        run(root)
        assert_access_mode_consolidated(root)
    finally:
        shutil.rmtree(root)

    scenario(
        "retired-project-ledger",
        lambda root: (
            root.joinpath("doc/product/world-infrastructure/reintroduced.project.md").write_text(
                "# forbidden ledger\n", encoding="utf-8"
            )
        ),
    )

    scenario(
        "entry-contract",
        lambda root: replace(
            root / "doc/product/README.md",
            "| 玩家入口与发行 |",
            "| 第五个产品 | [`doc/product/fifth/prd.md`](fifth/prd.md) | 非法。 |\n| 玩家入口与发行 |",
        ),
    )
    scenario(
        "topic-index-contract",
        lambda root: (
            (root / "doc/product/untracked/nested").mkdir(parents=True),
            shutil.copy2(
                root / "doc/product/world-rules-core-gameplay/prd.md",
                root / "doc/product/untracked/nested/untracked.prd.md",
            ),
        ),
    )
    scenario(
        "entry-contract",
        lambda root: replace(
            root / "doc/product/README.md",
            "(world-rules-core-gameplay/prd.md)",
            "(../game/prd.md)",
        ),
    )
    scenario(
        "metadata-placeholder-id",
        lambda root: replace(
            root / "doc/product/world-infrastructure/prd.md",
            "PRD-PRODUCT-002",
            "PRD-PRODUCT-xxx",
        ),
    )
    scenario(
        "lifecycle-contract",
        lambda root: replace(
            root / "doc/product/agents-world-simulation/prd.md",
            "- 生命周期：`active`",
            "- 生命周期：`unknown`",
        ),
    )
    scenario(
        "authority-backlink",
        lambda root: replace(
            root / "doc/world-simulator/prd.md",
            "../product/agents-world-simulation/prd.md",
            "../product/missing/prd.md",
        ),
    )
    scenario(
        "authority-backlink",
        lambda root: replace(
            root / "doc/world-simulator/prd.md",
            "[`doc/product/agents-world-simulation/prd.md`](../product/agents-world-simulation/prd.md)",
            "`doc/product/agents-world-simulation/prd.md`",
        ),
    )
    for authority in (
        "doc/world-runtime/prd.md",
        "doc/world-simulator/prd.md",
        "doc/p2p/prd.md",
    ):
        scenario(
            "authority-backlink",
            lambda root, authority=authority: replace(
                root / authority,
                "[`doc/product/world-rules-core-gameplay/prd.md`](../product/world-rules-core-gameplay/prd.md)",
                "`doc/product/world-rules-core-gameplay/prd.md`",
            ),
        )
    scenario(
        "authority-contract",
        lambda root: replace(
            root / "doc/product/world-infrastructure/prd.md",
            "[`doc/game/prd.md`](../../game/prd.md)",
            "`doc/game/prd.md`",
        ),
    )
    scenario(
        "traceability-contract",
        lambda root: replace(
            root / "doc/product/player-entry-distribution/prd.md",
            "| SC-4 | qa_engineer |",
            "| SC-9 | qa_engineer |",
        ),
    )
    scenario(
        "traceability-prd-resolution",
        lambda root: replace(
            root / "doc/product/world-infrastructure/prd.md",
            "PRD-GAME-016",
            "PRD-GAME-999",
        ),
    )
    scenario(
        "identity-metadata-location",
        lambda root: replace(
            root / "doc/core/prd.md",
            "# core PRD",
            "# core PRD\n\n- 产品模块：`不允许在此声明`",
        ),
    )
    scenario(
        "identity-metadata-location",
        lambda root: replace(
            root / "doc/product/agents-world-simulation/provider-agent-experience-continuity.prd.md",
            "# Agent/provider 体验连续性",
            "# Agent/provider 体验连续性\n\n- Product PRD-ID：`PRD-PRODUCT-003`",
        ),
    )
    scenario(
        "topic-professional-authority",
        lambda root: [
            replace(
                root / "doc/product/agents-world-simulation/provider-agent-experience-continuity.prd.md",
                "../../world-simulator/llm/provider-agent-experience-parity.prd.md",
                "../../world-simulator/llm/"
                + "llm-provider-agent-experience-parity"
                + "-2026-03-12.prd.md",
            )
            for _ in range(2)
        ],
    )
    scenario(
        None,
        lambda root: replace(
            root / "doc/core/prd.md",
            "# core PRD",
            "# core PRD\n\nProduct PRD-ID is reserved for module PRDs; this is a reference, not metadata.",
        ),
    )
    scenario(
        None,
        lambda root: replace(
            root / "doc/core/prd.md",
            "# core PRD",
            "# core PRD\n\n```markdown\n- 产品模块：`示例，不是声明`\n```\n\n~~~markdown\n- Product PRD-ID：`PRD-PRODUCT-999`\n~~~",
        ),
    )
    scenario(
        None,
        lambda root: replace(
            root / "doc/engineering/doc-governance/doc-structure-standard.design.md",
            "### 3.4 产品组合层的薄覆盖例外",
            "### 3.4 产品组合层的薄覆盖例外\n\n- Product PRD-ID：`PRD-PRODUCT-001`",
        ),
    )
    print("product-doc-governance-check.test: OK")


if __name__ == "__main__":
    main()
