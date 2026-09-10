#!/usr/bin/env python3
"""Regression fixtures for the changed/new product-document content gate."""

from __future__ import annotations

from pathlib import Path
import re
import shutil
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parent.parent
CHECKER = ROOT / "scripts/product-doc-content-check.py"
TOPIC = "doc/product/world-rules-core-gameplay/sample.prd.md"
DESIGN = "doc/product/world-rules-core-gameplay/sample.design.md"


TOPIC_TEXT = """# Sample topic

## 文档身份
- 所属产品模块：世界规则与核心玩法
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`gameplay authority`](../../game/prd.md#authority)
- Last reviewed：2026-09-10

本文从一个玩家的首局工业情境出发，说明当前产品承诺与专业边界。

## 1. 玩家问题与目标
玩家需要知道当前目标、选择和成功含义。

## 2. 范围与 Non-Goals
本主题的范围是一个受支持入口；不定义实现细节。

## 3. 玩家体验与决策
### 3.1 正常路径
玩家比较选择、成本和风险，沿路径继续到下一步。
### 3.2 主要失败与恢复
阻塞时显示失败原因、恢复路径和证据边界。

## 4. 产品要求
<a id="req-sample-001"></a>
### REQ-SAMPLE-001：可读的选择
- 要求：玩家必须（MUST）看到一次可观察的结果。
- 专业权威：[`gameplay authority`](../../game/prd.md#authority)
- 验收：AC-SAMPLE-001

## 5. 验收与证据
<a id="ac-sample-001"></a>
### AC-SAMPLE-001：正向结果
- 覆盖要求：REQ-SAMPLE-001
- 证据范围：当前入口与当前版本。
### 5.2 效果与证据范围
不能由该场景证明真实留存或发行就绪。

## 7. 设计取舍与未决问题
尚未决定：后续入口如何承接，解决触发条件由产品 owner 裁定。
"""

DESIGN_TEXT = """# Sample topic design

## 文档身份
- 配对产品 PRD：[`Sample topic`](sample.prd.md)
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`gameplay authority`](../../game/prd.md#authority)

## 1. 设计命题
玩家在一次选择中理解后果和下一步。
## 2. 代表性片段
目标、选择、后果和恢复路径保持可读。
## 3. 机制与体验关系
失败时给出恢复方式；验证证据回到配对 PRD。
## 7. 追踪
| REQ | AC |
| --- | --- |
| [`REQ-SAMPLE-001`](sample.prd.md#req-sample-001) | [`AC-SAMPLE-001`](sample.prd.md#ac-sample-001) |
"""


def run_git(root: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True, text=True)


def make_repo() -> tuple[Path, str, str]:
    root = Path(tempfile.mkdtemp(prefix="oasis7-product-content-fixture-"))
    (root / "doc/product/world-rules-core-gameplay").mkdir(parents=True)
    (root / "doc/game").mkdir(parents=True)
    (root / "doc/game/prd.md").write_text("# Gameplay\n<a id=\"authority\"></a>\n## Authority\n", encoding="utf-8")
    (root / "doc/product/world-rules-core-gameplay/prd.md").write_text("# Root\n", encoding="utf-8")
    (root / TOPIC).write_text(TOPIC_TEXT, encoding="utf-8")
    (root / DESIGN).write_text(DESIGN_TEXT, encoding="utf-8")
    (root / "doc/product/world-rules-core-gameplay/legacy.prd.md").write_text("broken legacy\n", encoding="utf-8")
    run_git(root, "init", "-q")
    run_git(root, "config", "user.email", "test@example.invalid")
    run_git(root, "config", "user.name", "fixture")
    run_git(root, "add", ".")
    run_git(root, "commit", "-qm", "base")
    base = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
    run_git(root, "commit", "--allow-empty", "-qm", "head")
    head = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
    return root, base, head


def invoke(root: Path, base: str, head: str, *, worktree: bool = False) -> subprocess.CompletedProcess[str]:
    command = ["python3", str(CHECKER), "--repo-root", str(root), "--base", base, "--head", head]
    if worktree:
        command.append("--worktree")
    return subprocess.run(command, check=False, capture_output=True, text=True)


def scenario(expected: str | None, mutate) -> None:
    root, base, _head = make_repo()
    try:
        mutate(root)
        run_git(root, "add", ".")
        run_git(root, "commit", "--allow-empty", "-qm", "mutation")
        head = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
        result = invoke(root, base, head)
        output = result.stdout + result.stderr
        if expected is None:
            assert result.returncode == 0, output
            assert "product-doc-content: OK" in output or "checked 0" in output, output
        else:
            assert result.returncode != 0, output
            assert expected in output, output
    finally:
        shutil.rmtree(root)


def scenario_checked(mutate) -> None:
    root, base, _head = make_repo()
    try:
        mutate(root)
        run_git(root, "add", ".")
        run_git(root, "commit", "--allow-empty", "-qm", "mutation")
        head = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
        result = invoke(root, base, head)
        output = result.stdout + result.stderr
        assert result.returncode == 0, output
        assert "product-doc-content: OK (checked 1" in output, output
    finally:
        shutil.rmtree(root)


def fenced_authority_target(root: Path) -> None:
    (root / "doc/game/prd.md").write_text(
        "# Gameplay\n```md\n<a id=\"authority\"></a>\n## Authority\n```\n", encoding="utf-8"
    )
    (root / TOPIC).write_text(TOPIC_TEXT + "\n补充当前 authority 证据边界。\n", encoding="utf-8")


def pseudo_paired_design_link(root: Path, form: str) -> None:
    path = root / DESIGN
    text = re.sub(
        r"\[([^\]]+)\]\((sample\.prd\.md(?:#[^)]+)?)\)",
        r"`[\1](\2)`",
        DESIGN_TEXT,
    )
    target = "[paired PRD](sample.prd.md)"
    if form == "inline":
        target = f"`{target}`"
    elif form == "indented":
        target = f"    {target}"
    elif form == "comment":
        target = f"<!-- {target} -->"
    elif form == "escaped":
        target = f"\\{target}"
    elif form == "even-escaped":
        target = f"\\\\{target}"
    elif form == "image":
        target = f"!{target}"
    elif form == "quote-fence":
        target = "> ```markdown\n> " + target + "\n> ```"
    elif form == "list-fence":
        target = "- ```markdown\n  " + target + "\n  ```"
    elif form == "quote-indent":
        target = ">     " + target
    elif form == "list-indent":
        target = "-     " + target
    else:
        raise AssertionError(form)
    path.write_text(text + f"\n{target}\n", encoding="utf-8")

def even_escaped_paired_design_link(root: Path) -> None:
    path = root / DESIGN
    target = "[" + chr(96) + "Sample topic" + chr(96) + "](sample.prd.md)"
    text = DESIGN_TEXT.replace(target, r"\\[Sample topic](sample.prd.md)", 1)
    path.write_text(text, encoding="utf-8")


def multiline_comment_before_authority(root: Path) -> None:
    marker = "- 专业域权威：[`gameplay authority`](../../game/prd.md#authority)"
    comment = "<!-- removed from the rendered document\nthis comment spans multiple source lines\n-->\n"
    updated = TOPIC_TEXT.replace(marker, comment + marker, 1)
    (root / TOPIC).write_text(updated + "\n补充当前入口的证据边界。\n", encoding="utf-8")


def multiline_comment_before_cross_file_refs(root: Path) -> None:
    marker = "| [`REQ-SAMPLE-001`](sample.prd.md#req-sample-001) | [`AC-SAMPLE-001`](sample.prd.md#ac-sample-001) |"
    comment = "<!-- trace note\nkept out of the rendered document\n-->\n"
    updated = DESIGN_TEXT.replace(marker, comment + marker, 1)
    (root / DESIGN).write_text(updated + "\n补充当前 trace 的验证边界。\n", encoding="utf-8")


def nested_fenced_examples(root: Path) -> None:
    examples = """
- ```markdown
  REQ-EXAMPLE-001 AC-EXAMPLE-001 [missing](missing.md)
  ```

> ```text
> REQ-EXAMPLE-002 AC-EXAMPLE-002 [also missing](also-missing.md)
> ```
"""
    (root / TOPIC).write_text(TOPIC_TEXT + examples, encoding="utf-8")


def main() -> None:
    scenario(None, lambda _root: None)
    scenario("missing-metadata", lambda root: (root / TOPIC).write_text(TOPIC_TEXT.replace("- Owner role：`producer_system_designer`\n", ""), encoding="utf-8"))
    scenario("missing-normal-path", lambda root: (root / TOPIC).write_text(TOPIC_TEXT.replace("正常路径", "体验").replace("路径", "方向").replace("流程", "方向").replace("循环", "方向").replace("链路", "方向"), encoding="utf-8"))
    scenario("missing-evidence-boundary", lambda root: (root / TOPIC).write_text(TOPIC_TEXT.replace("证据", "范围证明"), encoding="utf-8"))
    scenario("authority-not-link", lambda root: (root / TOPIC).write_text(TOPIC_TEXT.replace("[`gameplay authority`](../../game/prd.md#authority)", "`doc/game` authority", 1), encoding="utf-8"))
    scenario("authority-external-link", lambda root: (root / TOPIC).write_text(
        TOPIC_TEXT.replace("[`gameplay authority`](../../game/prd.md#authority)", "[gameplay authority](https://example.invalid/authority#authority)", 1),
        encoding="utf-8",
    ))
    scenario("authority-link", lambda root: (root / TOPIC).write_text(TOPIC_TEXT.replace("../../game/prd.md#authority", "../../game/missing.md#authority", 1), encoding="utf-8"))
    scenario("invalid-fragment", lambda root: (root / TOPIC).write_text(TOPIC_TEXT.replace("../../game/prd.md#authority", "../../game/prd.md#missing", 1), encoding="utf-8"))
    scenario("missing-metadata", lambda root: (root / TOPIC).write_text(
        TOPIC_TEXT.replace("- Owner role：`producer_system_designer`\n", "```md\n- Owner role：`producer_system_designer`\n```\n"),
        encoding="utf-8",
    ))
    scenario("invalid-fragment", fenced_authority_target)
    scenario(None, multiline_comment_before_authority)
    scenario(None, multiline_comment_before_cross_file_refs)
    scenario(None, nested_fenced_examples)
    scenario("duplicate-anchor", lambda root: (root / TOPIC).write_text(TOPIC_TEXT.replace("<a id=\"ac-sample-001\"></a>", "<a id=\"req-sample-001\"></a>\n<a id=\"ac-sample-001\"></a>"), encoding="utf-8"))
    scenario("unresolved-cross-file-id", lambda root: (root / DESIGN).write_text(DESIGN_TEXT.replace("sample.prd.md#req-sample-001", "sample.prd.md#req-missing"), encoding="utf-8"))
    scenario("req-missing-acceptance", lambda root: (root / TOPIC).write_text(TOPIC_TEXT.replace("- 验收：AC-SAMPLE-001\n", ""), encoding="utf-8"))
    scenario("ac-missing-requirement", lambda root: (root / TOPIC).write_text(TOPIC_TEXT.replace("- 覆盖要求：REQ-SAMPLE-001\n", ""), encoding="utf-8"))
    scenario("unresolved-id-reference", lambda root: (root / TOPIC).write_text(
        TOPIC_TEXT + "\n| REQ-SAMPLE-001 | AC-TYPO |\n", encoding="utf-8"
    ))
    scenario(None, lambda root: (root / TOPIC).write_text(
        TOPIC_TEXT + "\n普通背景链接：[external reference](https://example.invalid/reference)。\n", encoding="utf-8"
    ))
    for pseudo_form in (
        "inline",
        "indented",
        "comment",
        "escaped",
        "image",
        "quote-fence",
        "list-fence",
        "quote-indent",
        "list-indent",
    ):
        scenario(
            "missing-paired-prd-link",
            lambda root, pseudo_form=pseudo_form: pseudo_paired_design_link(root, pseudo_form),
        )
    scenario(None, even_escaped_paired_design_link)
    scenario_checked(lambda root: (root / TOPIC).write_text(
        TOPIC_TEXT.replace("玩家需要知道当前目标", "  玩家需要知道当前目标"), encoding="utf-8"
    ))
    scenario(None, lambda root: (root / TOPIC).write_text(
        TOPIC_TEXT + "\n本文仅说明 REQ-SAMPLE-* / AC-SAMPLE-* 这一组旧编号。\n- AC-LEGACY-001：保留既有验收编号。\n",
        encoding="utf-8",
    ))

    root, base, head = make_repo()
    try:
        (root / TOPIC).write_text(TOPIC_TEXT + "\n```md\n[bad](missing.md#bad) REQ-MISSING AC-MISSING\n```\n", encoding="utf-8")
        result = invoke(root, base, head, worktree=True)
        output = result.stdout + result.stderr
        assert result.returncode == 0, output
        assert "checked 1" in output, output
    finally:
        shutil.rmtree(root)

    root, base, head = make_repo()
    try:
        (root / TOPIC).write_text(TOPIC_TEXT + "\n<!-- comment-only change -->\n", encoding="utf-8")
        result = invoke(root, base, head, worktree=True)
        output = result.stdout + result.stderr
        assert result.returncode == 0, output
        assert "checked 0" in output, output
    finally:
        shutil.rmtree(root)

    root, base, head = make_repo()
    try:
        untracked = root / "doc/product/world-rules-core-gameplay/untracked.prd.md"
        untracked.write_text(TOPIC_TEXT, encoding="utf-8")
        result = invoke(root, base, head, worktree=True)
        output = result.stdout + result.stderr
        assert result.returncode == 0, output
        assert "product-doc-content: OK (checked 1" in output, output
    finally:
        shutil.rmtree(root)

    root, base, _head = make_repo()
    try:
        run_git(root, "switch", "-c", "target", base)
        (root / TOPIC).write_text(TOPIC_TEXT.replace("- Owner role：`producer_system_designer`\n", ""), encoding="utf-8")
        run_git(root, "add", ".")
        run_git(root, "commit", "-qm", "target-only invalid document")
        target = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
        run_git(root, "switch", "-c", "source", base)
        (root / DESIGN).write_text(DESIGN_TEXT + "\n补充当前设计验证边界。\n", encoding="utf-8")
        run_git(root, "add", ".")
        run_git(root, "commit", "-qm", "source document change")
        source = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
        result = invoke(root, target, source)
        output = result.stdout + result.stderr
        assert result.returncode == 0, output
        assert "product-doc-content: OK (checked 1" in output, output
    finally:
        shutil.rmtree(root)

    root, base, _head = make_repo()
    try:
        run_git(root, "switch", "-c", "target", base)
        (root / TOPIC).write_text(
            TOPIC_TEXT.replace("- Owner role：`producer_system_designer`\n", ""),
            encoding="utf-8",
        )
        run_git(root, "add", ".")
        run_git(root, "commit", "-qm", "target-only invalid document")
        target = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
        run_git(root, "switch", "-c", "source", base)
        (root / DESIGN).write_text(DESIGN_TEXT + "\n补充当前设计验证边界。\n", encoding="utf-8")
        run_git(root, "add", ".")
        run_git(root, "commit", "-qm", "source document change")
        source = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
        run_git(root, "merge", "--no-ff", "-m", "synthetic merge", "target")
        result = invoke(root, target, source, worktree=True)
        output = result.stdout + result.stderr
        assert result.returncode == 0, output
        assert "product-doc-content: OK (checked 1" in output, output
    finally:
        shutil.rmtree(root)

    root, base, _head = make_repo()
    try:
        run_git(root, "switch", "-c", "target", base)
        (root / TOPIC).write_text(TOPIC_TEXT + "\n目标分支的合法背景补充。\n", encoding="utf-8")
        run_git(root, "add", ".")
        run_git(root, "commit", "-qm", "target-only valid document")
        target = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
        run_git(root, "switch", "-c", "source", base)
        (root / DESIGN).write_text(DESIGN_TEXT + "\n补充当前设计验证边界。\n", encoding="utf-8")
        run_git(root, "add", ".")
        run_git(root, "commit", "-qm", "source document change")
        source = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
        run_git(root, "merge", "--no-ff", "-m", "synthetic merge", "target")
        (root / TOPIC).write_text(
            TOPIC_TEXT.replace("- Owner role：`producer_system_designer`\n", ""),
            encoding="utf-8",
        )
        run_git(root, "add", TOPIC)
        result = invoke(root, target, source, worktree=True)
        output = result.stdout + result.stderr
        assert result.returncode != 0, output
        assert "missing-metadata" in output and "Owner role" in output, output
    finally:
        shutil.rmtree(root)

    print("product-doc-content-check.test: OK")


if __name__ == "__main__":
    main()
