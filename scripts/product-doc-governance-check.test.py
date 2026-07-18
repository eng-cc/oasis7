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
    "doc/world-runtime/prd.md",
    "doc/p2p/prd.md",
    "doc/world-simulator/prd.md",
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


def scenario(expected: str, mutation) -> None:
    root = make_fixture()
    try:
        mutation(root)
        run(root, expected)
    finally:
        shutil.rmtree(root)


def main() -> None:
    root = make_fixture()
    try:
        run(root)
    finally:
        shutil.rmtree(root)

    scenario(
        "entry-contract",
        lambda root: replace(
            root / "doc/product/README.md",
            "| 玩家入口与发行 |",
            "| 第五个产品 | [`doc/product/fifth/prd.md`](fifth/prd.md) | 非法。 |\n| 玩家入口与发行 |",
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
            root / "doc/game/prd.md",
            "doc/product/world-rules-core-gameplay/prd.md",
            "doc/product/missing/prd.md",
        ),
    )
    scenario(
        "authority-contract",
        lambda root: replace(
            root / "doc/product/world-infrastructure/prd.md",
            "`doc/game/prd.md`、`doc/world-runtime/prd.md`、`doc/p2p/prd.md`",
            "`doc/product/world-rules-core-gameplay/prd.md`",
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
    print("product-doc-governance-check.test: OK")


if __name__ == "__main__":
    main()
