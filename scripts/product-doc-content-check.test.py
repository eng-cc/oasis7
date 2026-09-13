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
- 设计判定：`paired-design`
- 配对产品设计：[`sample.design.md`](sample.design.md)

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

## 6. 专业 owner、authority 与测试层级追踪

| REQ / AC | 专业 owner | 专业权威 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| [REQ-SAMPLE-001](#req-sample-001) / [AC-SAMPLE-001](#ac-sample-001) | `producer_system_designer` | [gameplay authority](../../game/prd.md#authority) | 当前入口的可观察结果与恢复边界证据 | `test_tier_required`；`test_tier_full` 覆盖跨入口与恢复复核 |

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
- Last reviewed：2026-09-10

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

ROOT_TEXT = """# 世界规则与核心玩法 PRD

## 文档身份
- 产品模块：世界规则与核心玩法
- 产品模块 slug：`world-rules-core-gameplay`
- 产品层唯一 PRD：`doc/product/world-rules-core-gameplay/prd.md`
- 产品模块总入口：`doc/product/README.md`
- Product PRD-ID：`PRD-PRODUCT-001`
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：`2026-09-10`
- 后继文档：`无`
- 下层专业域：[`gameplay`](../../game/prd.md)

## 1. 产品承诺
## 2. 范围
## 3. 权威与冲突处理
## 4. 路线图
## 5. Done：成功标准与验收
### 5.1 验收追踪
## 6. Non-Goals
"""


def run_git(root: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True, text=True)


def make_repo() -> tuple[Path, str, str]:
    root = Path(tempfile.mkdtemp(prefix="oasis7-product-content-fixture-"))
    (root / "doc/product/world-rules-core-gameplay").mkdir(parents=True)
    (root / "doc/game").mkdir(parents=True)
    (root / "doc/game/prd.md").write_text("# Gameplay\n<a id=\"authority\"></a>\n## Authority\n", encoding="utf-8")
    (root / "doc/product/world-rules-core-gameplay/prd.md").write_text(ROOT_TEXT, encoding="utf-8")
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


def invoke_full_corpus(root: Path, *extra: str) -> subprocess.CompletedProcess[str]:
    command = ["python3", str(CHECKER), "--repo-root", str(root), "--full-corpus", *extra]
    return subprocess.run(command, check=False, capture_output=True, text=True)


REQ_BLOCK = TOPIC_TEXT[
    TOPIC_TEXT.index('<a id="req-sample-001"></a>') : TOPIC_TEXT.index('<a id="ac-sample-001"></a>')
]
AC_BLOCK = TOPIC_TEXT[
    TOPIC_TEXT.index('<a id="ac-sample-001"></a>') : TOPIC_TEXT.index("### 5.2", TOPIC_TEXT.index('<a id="ac-sample-001"></a>'))
]
TRACE_BLOCK = TOPIC_TEXT[
    TOPIC_TEXT.index("## 6. 专业 owner、authority 与测试层级追踪") : TOPIC_TEXT.index("## 7. 设计取舍与未决问题")
]


def remove_fixture_traceability(text: str, *, requirement: bool = False, acceptance: bool = False) -> str:
    if requirement:
        text = text.replace(REQ_BLOCK, "")
    if acceptance:
        text = text.replace(AC_BLOCK, "")
    if requirement and acceptance:
        text = text.replace(TRACE_BLOCK, "")
    return text


def isolate_topic_full_corpus(root: Path) -> None:
    (root / "doc/product/world-rules-core-gameplay/legacy.prd.md").unlink()


LIFECYCLE_CLOSURE = """

## 生命周期闭合
- 接收 authority：[`gameplay authority`](../../game/prd.md#authority)
- 剩余语义：保留历史验收含义与仍需可达的引用。
- 稳定引用：[`gameplay authority`](../../game/prd.md#authority)
- 删除条件：接收 authority 可达、活跃引用修复且无未决阻塞。
"""


def lifecycle_topic_text(lifecycle: str) -> str:
    text = TOPIC_TEXT.replace("生命周期：`active`", f"生命周期：`{lifecycle}`")
    return text + LIFECYCLE_CLOSURE


def scenario_full_corpus_requires_active_topic_requirement() -> None:
    root, _base, _head = make_repo()
    try:
        isolate_topic_full_corpus(root)
        updated = remove_fixture_traceability(TOPIC_TEXT, requirement=True, acceptance=True)
        updated += "\n本主题验收仍受当前入口证据范围约束。\n"
        (root / TOPIC).write_text(updated, encoding="utf-8")
        result = invoke_full_corpus(root)
        output = result.stdout + result.stderr
        assert result.returncode == 1, output
        assert f"active-topic-missing-requirement: {TOPIC}" in output, output
    finally:
        shutil.rmtree(root)


def scenario_full_corpus_requires_active_topic_acceptance() -> None:
    root, _base, _head = make_repo()
    try:
        isolate_topic_full_corpus(root)
        (root / TOPIC).write_text(
            remove_fixture_traceability(TOPIC_TEXT, acceptance=True), encoding="utf-8"
        )
        result = invoke_full_corpus(root)
        output = result.stdout + result.stderr
        assert result.returncode == 1, output
        assert f"active-topic-missing-acceptance: {TOPIC}" in output, output
    finally:
        shutil.rmtree(root)


def scenario_full_corpus_exempts_non_active_topic_cardinality() -> None:
    root, _base, _head = make_repo()
    try:
        isolate_topic_full_corpus(root)
        for lifecycle in ("superseded", "retired"):
            (root / f"doc/product/world-rules-core-gameplay/{lifecycle}.prd.md").write_text(
                remove_fixture_traceability(
                    lifecycle_topic_text(lifecycle), requirement=True, acceptance=True
                )
                + "\n本主题验收仍受当前入口证据范围约束。\n",
                encoding="utf-8",
            )
        result = invoke_full_corpus(root)
        output = result.stdout + result.stderr
        assert result.returncode == 0, output
        assert "active-topic-missing-requirement" not in output, output
        assert "active-topic-missing-acceptance" not in output, output
    finally:
        shutil.rmtree(root)


def scenario_full_corpus_includes_unchanged_legacy_and_sorts_diagnostics() -> None:
    root, _base, _head = make_repo()
    try:
        early = root / "doc/product/world-rules-core-gameplay/aaa-legacy.prd.md"
        late = root / "doc/product/world-rules-core-gameplay/zzz-legacy.prd.md"
        early.write_text("broken early legacy\n", encoding="utf-8")
        late.write_text("broken late legacy\n", encoding="utf-8")
        result = invoke_full_corpus(root)
        output = result.stdout + result.stderr
        assert result.returncode == 1, output
        assert str(early.relative_to(root)) in output, output
        assert "doc/product/world-rules-core-gameplay/legacy.prd.md" in output, output
        assert str(late.relative_to(root)) in output, output
        assert "missing-metadata" in output, output
        assert output.index(str(early.relative_to(root))) < output.index(
            "doc/product/world-rules-core-gameplay/legacy.prd.md"
        ) < output.index(str(late.relative_to(root))), output
    finally:
        shutil.rmtree(root)


def scenario_full_corpus_accepts_retired_topics_and_reports_lifecycle_errors() -> None:
    root, _base, _head = make_repo()
    retired = root / "doc/product/world-rules-core-gameplay/retired.prd.md"
    try:
        (root / "doc/product/world-rules-core-gameplay/legacy.prd.md").unlink()
        retired.write_text(lifecycle_topic_text("retired"), encoding="utf-8")
        result = invoke_full_corpus(root)
        output = result.stdout + result.stderr
        assert result.returncode == 0, output
        assert "product-doc-content:" in output and "full-corpus" in output, output

        retired.write_text(
            lifecycle_topic_text("retired").replace(
                "- 专业域权威：[`gameplay authority`](../../game/prd.md#authority)\n", ""
            ),
            encoding="utf-8",
        )
        result = invoke_full_corpus(root)
        output = result.stdout + result.stderr
        assert result.returncode == 1, output
        assert "retired.prd.md" in output and "missing-metadata" in output, output
    finally:
        shutil.rmtree(root)


def scenario_full_corpus_requires_exact_lifecycle_enum() -> None:
    root, _base, _head = make_repo()
    try:
        (root / "doc/product/world-rules-core-gameplay/legacy.prd.md").unlink()
        (root / TOPIC).write_text(
            TOPIC_TEXT.replace("生命周期：`active`", "生命周期：`active-ish`"),
            encoding="utf-8",
        )
        result = invoke_full_corpus(root)
        output = result.stdout + result.stderr
        assert result.returncode == 1, output
        assert f"invalid-lifecycle: {TOPIC}: `active-ish`" in output, output
    finally:
        shutil.rmtree(root)


def scenario_full_corpus_counts_module_root() -> None:
    root, _base, _head = make_repo()
    try:
        (root / "doc/product/world-rules-core-gameplay/legacy.prd.md").unlink()
        result = invoke_full_corpus(root)
        output = result.stdout + result.stderr
        assert result.returncode == 0, output
        assert "full-corpus checked 3 current-tree product documents" in output, output
    finally:
        shutil.rmtree(root)


def scenario_full_corpus_rejects_product_symlinks() -> None:
    for target_exists in (True, False):
        root, _base, _head = make_repo()
        try:
            (root / "doc/product/world-rules-core-gameplay/legacy.prd.md").unlink()
            target = root / "external.prd.md"
            if target_exists:
                target.write_text(TOPIC_TEXT, encoding="utf-8")
            link = root / "doc/product/world-rules-core-gameplay/linked.prd.md"
            link.symlink_to(target)
            result = invoke_full_corpus(root)
            output = result.stdout + result.stderr
            relative = link.relative_to(root).as_posix()
            assert result.returncode == 1, output
            assert f"symlink-not-allowed: {relative}" in output, output
        finally:
            shutil.rmtree(root)


def scenario_full_corpus_requires_lifecycle_closure() -> None:
    for marker, code in (
        ("- 接收 authority：[`gameplay authority`](../../game/prd.md#authority)\n", "lifecycle-missing-receiving-authority"),
        ("- 剩余语义：保留历史验收含义与仍需可达的引用。\n", "lifecycle-missing-remaining-semantics"),
        ("- 稳定引用：[`gameplay authority`](../../game/prd.md#authority)\n", "lifecycle-missing-stable-reference"),
        ("- 删除条件：接收 authority 可达、活跃引用修复且无未决阻塞。\n", "lifecycle-missing-deletion-condition"),
    ):
        root, _base, _head = make_repo()
        try:
            (root / "doc/product/world-rules-core-gameplay/legacy.prd.md").unlink()
            retired = lifecycle_topic_text("retired").replace(marker, "")
            (root / "doc/product/world-rules-core-gameplay/retired.prd.md").write_text(
                retired, encoding="utf-8"
            )
            result = invoke_full_corpus(root)
            output = result.stdout + result.stderr
            assert result.returncode == 1, output
            assert f"{code}: doc/product/world-rules-core-gameplay/retired.prd.md" in output, output
        finally:
            shutil.rmtree(root)


def scenario_full_corpus_rejects_lifecycle_closure_placeholders_and_unlinked_fields() -> None:
    cases = (
        (
            LIFECYCLE_CLOSURE.replace(
                "- 剩余语义：保留历史验收含义与仍需可达的引用。",
                "- 剩余语义：TBD",
            ),
            "lifecycle-placeholder",
        ),
        (
            LIFECYCLE_CLOSURE.replace(
                "- 接收 authority：[`gameplay authority`](../../game/prd.md#authority)",
                "- 接收 authority：gameplay authority",
            ),
            "lifecycle-missing-receiving-authority-link",
        ),
        (
            LIFECYCLE_CLOSURE.replace(
                "- 稳定引用：[`gameplay authority`](../../game/prd.md#authority)",
                "- 稳定引用：[`gameplay authority`](../../game/prd.md#missing)",
            ),
            "lifecycle-invalid-stable-reference-link",
        ),
    )
    for closure, code in cases:
        root, _base, _head = make_repo()
        try:
            (root / "doc/product/world-rules-core-gameplay/legacy.prd.md").unlink()
            (root / "doc/product/world-rules-core-gameplay/retired.prd.md").write_text(
                lifecycle_topic_text("retired").replace(LIFECYCLE_CLOSURE, closure),
                encoding="utf-8",
            )
            result = invoke_full_corpus(root)
            output = result.stdout + result.stderr
            assert result.returncode == 1, output
            assert f"{code}: doc/product/world-rules-core-gameplay/retired.prd.md" in output, output
        finally:
            shutil.rmtree(root)


def scenario_full_corpus_requires_design_decision_or_exemption() -> None:
    root, _base, _head = make_repo()
    try:
        (root / DESIGN).unlink()
        (root / "doc/product/world-rules-core-gameplay/legacy.prd.md").unlink()
        (root / TOPIC).write_text(
            TOPIC_TEXT.replace("- 设计判定：`paired-design`\n", "")
            .replace("- 配对产品设计：[`sample.design.md`](sample.design.md)\n", ""),
            encoding="utf-8",
        )
        result = invoke_full_corpus(root)
        output = result.stdout + result.stderr
        assert result.returncode == 1, output
        assert f"missing-design-or-exemption: {TOPIC}" in output, output
    finally:
        shutil.rmtree(root)


def scenario_full_corpus_accepts_simple_topic_exemption() -> None:
    root, _base, _head = make_repo()
    try:
        (root / DESIGN).unlink()
        (root / "doc/product/world-rules-core-gameplay/legacy.prd.md").unlink()
        exemption = """

## 设计判定
- 设计判定：`simple-topic-exemption`
- 设计适用性理由：这是简单专题，不增加新的信息分层、交互状态编排或策略取舍。
- 当前 GitHub task evidence：[`issue evidence`](https://github.com/eng-cc/oasis7/issues/3680#issuecomment-5652870280)
"""
        (root / TOPIC).write_text(
            TOPIC_TEXT.replace("- 设计判定：`paired-design`\n", "")
            .replace("- 配对产品设计：[`sample.design.md`](sample.design.md)\n", "")
            + exemption,
            encoding="utf-8",
        )
        result = invoke_full_corpus(root)
        output = result.stdout + result.stderr
        assert result.returncode == 0, output
        assert "full-corpus checked 2 current-tree product documents" in output, output
    finally:
        shutil.rmtree(root)


def scenario_full_corpus_requires_bound_repository_task_evidence() -> None:
    cases = (
        """

## 设计判定
- 设计判定：`simple-topic-exemption`
- 设计适用性理由：这是简单专题，不增加新的信息分层、交互状态编排或策略取舍。
- 当前 GitHub task evidence：见下一行。
- [issue evidence](https://github.com/eng-cc/oasis7/issues/3680#issuecomment-5652870280)
""",
        """

## 设计判定
- 设计判定：`simple-topic-exemption`
- 设计适用性理由：这是简单专题，不增加新的信息分层、交互状态编排或策略取舍。
- 当前 GitHub task evidence：[`issue evidence`](https://github.com/example-owner/example-repo/issues/3680#issuecomment-5652870280)
""",
    )
    for exemption in cases:
        root, _base, _head = make_repo()
        try:
            (root / DESIGN).unlink()
            (root / "doc/product/world-rules-core-gameplay/legacy.prd.md").unlink()
            (root / TOPIC).write_text(
                TOPIC_TEXT.replace("- 设计判定：`paired-design`\n", "")
                .replace("- 配对产品设计：[`sample.design.md`](sample.design.md)\n", "")
                + exemption,
                encoding="utf-8",
            )
            result = invoke_full_corpus(root)
            output = result.stdout + result.stderr
            assert result.returncode == 1, output
            assert f"missing-design-exemption-evidence: {TOPIC}" in output, output
        finally:
            shutil.rmtree(root)


def mapping_topic_text() -> str:
    return TOPIC_TEXT.replace(
        "### 5.2 效果与证据范围",
        """<a id="req-sample-002"></a>
### REQ-SAMPLE-002：可恢复的选择
- 要求：玩家必须（MUST）获得一条恢复路径。
- 验收：AC-SAMPLE-002

<a id="ac-sample-002"></a>
### AC-SAMPLE-002：恢复路径
- 覆盖要求：REQ-SAMPLE-002

### 5.2 效果与证据范围""",
    )


def mapping_design_text(*, inconsistent: bool = False, incomplete: bool = False) -> str:
    second_row = ""
    if not incomplete:
        if inconsistent:
            second_row = (
                "| [`REQ-SAMPLE-002`](sample.prd.md#req-sample-002) | "
                "[`AC-SAMPLE-001`](sample.prd.md#ac-sample-001) / "
                "[`AC-SAMPLE-002`](sample.prd.md#ac-sample-002) |\n"
            )
        else:
            second_row = (
                "| [`REQ-SAMPLE-002`](sample.prd.md#req-sample-002) | "
                "[`AC-SAMPLE-002`](sample.prd.md#ac-sample-002) |\n"
            )
    return DESIGN_TEXT.replace(
        "| [`REQ-SAMPLE-001`](sample.prd.md#req-sample-001) | [`AC-SAMPLE-001`](sample.prd.md#ac-sample-001) |\n",
        "| [`REQ-SAMPLE-001`](sample.prd.md#req-sample-001) | [`AC-SAMPLE-001`](sample.prd.md#ac-sample-001) |\n"
        + second_row,
    )


def scenario_full_corpus_requires_complete_prd_consistent_design_mapping() -> None:
    for inconsistent, incomplete, code in (
        (False, True, "design-prd-mapping-incomplete"),
        (True, False, "design-prd-mapping-inconsistent"),
    ):
        root, _base, _head = make_repo()
        try:
            (root / "doc/product/world-rules-core-gameplay/legacy.prd.md").unlink()
            (root / TOPIC).write_text(mapping_topic_text(), encoding="utf-8")
            (root / DESIGN).write_text(
                mapping_design_text(inconsistent=inconsistent, incomplete=incomplete),
                encoding="utf-8",
            )
            result = invoke_full_corpus(root)
            output = result.stdout + result.stderr
            assert result.returncode == 1, output
            assert f"{code}: {DESIGN}" in output, output
        finally:
            shutil.rmtree(root)


def scenario_full_corpus_requires_paired_design_link() -> None:
    root, _base, _head = make_repo()
    try:
        (root / "doc/product/world-rules-core-gameplay/legacy.prd.md").unlink()
        (root / TOPIC).write_text(
            TOPIC_TEXT.replace("- 配对产品设计：[`sample.design.md`](sample.design.md)\n", ""),
            encoding="utf-8",
        )
        result = invoke_full_corpus(root)
        output = result.stdout + result.stderr
        assert result.returncode == 1, output
        assert f"missing-paired-design-link: {TOPIC}" in output, output
    finally:
        shutil.rmtree(root)


def scenario_full_corpus_requires_prd_trace_fragments() -> None:
    root, _base, _head = make_repo()
    try:
        (root / "doc/product/world-rules-core-gameplay/legacy.prd.md").unlink()
        (root / DESIGN).write_text(
            DESIGN_TEXT.replace("sample.prd.md#req-sample-001", "sample.prd.md")
            .replace("sample.prd.md#ac-sample-001", "sample.prd.md"),
            encoding="utf-8",
        )
        result = invoke_full_corpus(root)
        output = result.stdout + result.stderr
        assert result.returncode == 1, output
        assert f"design-missing-prd-trace-fragment: {DESIGN}" in output, output
    finally:
        shutil.rmtree(root)


def scenario_paired_trace_missing_column() -> None:
    scenario(
        "paired-trace-missing-column",
        lambda root: (root / TOPIC).write_text(
            TOPIC_TEXT.replace("| 验证证据 |", "| 说明 |"), encoding="utf-8"
        ),
    )


def scenario_paired_trace_missing_relation() -> None:
    scenario(
        "paired-trace-missing-relation",
        lambda root: (root / TOPIC).write_text(
            TOPIC_TEXT.replace(
                " / [AC-SAMPLE-001](#ac-sample-001)", ""
            ),
            encoding="utf-8",
        ),
    )


def scenario_paired_trace_empty_evidence() -> None:
    scenario(
        "paired-trace-empty-evidence",
        lambda root: (root / TOPIC).write_text(
            TOPIC_TEXT.replace(
                "当前入口的可观察结果与恢复边界证据", ""
            ),
            encoding="utf-8",
        ),
    )


def scenario_paired_trace_requires_strict_test_tier_tokens() -> None:
    scenario(
        "paired-trace-invalid-test-tier",
        lambda root: (root / TOPIC).write_text(
            TOPIC_TEXT.replace("test_tier_required", "required").replace(
                "test_tier_full", "full"
            ),
            encoding="utf-8",
        ),
    )


def scenario_full_corpus_rejects_changed_range_arguments() -> None:
    root, base, head = make_repo()
    try:
        for extra in (
            ("--base", base),
            ("--head", head),
            ("--worktree",),
            ("--base", base, "--head", head),
            ("--base", base, "--head", head, "--worktree"),
        ):
            result = invoke_full_corpus(root, *extra)
            output = result.stdout + result.stderr
            assert result.returncode == 2, output
            assert (
                "cannot be combined" in output
                or "not allowed with" in output
                or "mutually exclusive" in output
            ), output
    finally:
        shutil.rmtree(root)


def scenario_full_corpus_validates_module_root() -> None:
    root, _base, _head = make_repo()
    try:
        root_prd = root / "doc/product/world-rules-core-gameplay/prd.md"
        root_prd.write_text(ROOT_TEXT.replace("- Product PRD-ID：`PRD-PRODUCT-001`\n", ""), encoding="utf-8")
        result = invoke_full_corpus(root)
        output = result.stdout + result.stderr
        assert result.returncode == 1, output
        assert "root-metadata-contract" in output, output
        assert "Product PRD-ID" in output, output
    finally:
        shutil.rmtree(root)


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


def external_requirement_links(root: Path) -> None:
    path = root / DESIGN
    text = DESIGN_TEXT.replace(
        "sample.prd.md#req-sample-001",
        "https://example.invalid/req#req-sample-001",
    ).replace(
        "sample.prd.md#ac-sample-001",
        "https://example.invalid/ac#ac-sample-001",
    )
    path.write_text(text, encoding="utf-8")


def scenario_worktree_overlapping_target_change() -> None:
    root, base, _head = make_repo()
    try:
        run_git(root, "switch", "-c", "target", base)
        (root / DESIGN).write_text(
            DESIGN_TEXT.replace("- Owner role：`producer_system_designer`\n", ""),
            encoding="utf-8",
        )
        run_git(root, "add", ".")
        run_git(root, "commit", "-qm", "target-only invalid design")
        target = subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
        ).strip()

        run_git(root, "switch", "-c", "source", base)
        (root / DESIGN).write_text(
            DESIGN_TEXT + "\nsource change\n",
            encoding="utf-8",
        )
        run_git(root, "add", ".")
        run_git(root, "commit", "-qm", "source valid design")
        source = subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
        ).strip()

        run_git(root, "merge", "--no-ff", "-m", "synthetic merge", "target")
        result = invoke(root, target, source, worktree=True)
        output = result.stdout + result.stderr
        assert result.returncode == 0, output
        assert "product-doc-content: OK (checked 1" in output, output
    finally:
        shutil.rmtree(root)


def scenario_worktree_local_overlay_wins() -> None:
    root, base, _head = make_repo()
    try:
        (root / DESIGN).write_text(DESIGN_TEXT + "\nsource change\n", encoding="utf-8")
        run_git(root, "add", ".")
        run_git(root, "commit", "-qm", "source valid design")
        source = subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
        ).strip()

        (root / DESIGN).write_text(
            DESIGN_TEXT.replace("- Owner role：`producer_system_designer`\n", ""),
            encoding="utf-8",
        )
        result = invoke(root, base, source, worktree=True)
        output = result.stdout + result.stderr
        assert result.returncode != 0, output
        assert "missing-metadata" in output and "Owner role" in output, output

        run_git(root, "add", DESIGN)
        result = invoke(root, base, source, worktree=True)
        output = result.stdout + result.stderr
        assert result.returncode != 0, output
        assert "missing-metadata" in output and "Owner role" in output, output
    finally:
        shutil.rmtree(root)


def scenario_committed_target_content_is_frozen() -> None:
    root, base, _head = make_repo()
    try:
        target = root / "doc/game/prd.md"
        (root / TOPIC).write_text(
            TOPIC_TEXT.replace("../../game/prd.md#authority", "../../game/prd.md#missing", 1)
            + "\ncommitted source change\n",
            encoding="utf-8",
        )
        target.write_text("# Gameplay\n## Authority\n", encoding="utf-8")
        run_git(root, "add", ".")
        run_git(root, "commit", "-qm", "invalid committed target fragment")
        head = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()

        # A live edit must not make an explicit committed-range check pass.
        target.write_text("# Gameplay\n<a id=\"missing\"></a>\n## Authority\n", encoding="utf-8")
        result = invoke(root, base, head)
        output = result.stdout + result.stderr
        assert result.returncode != 0, output
        assert "invalid-fragment" in output, output
    finally:
        shutil.rmtree(root)


def scenario_worktree_target_overlay_remains_valid() -> None:
    root, base, _head = make_repo()
    try:
        target = root / "doc/game/prd.md"
        (root / TOPIC).write_text(
            TOPIC_TEXT.replace("../../game/prd.md#authority", "../../game/prd.md#missing", 1)
            + "\ncommitted source change\n",
            encoding="utf-8",
        )
        target.write_text("# Gameplay\n## Authority\n", encoding="utf-8")
        run_git(root, "add", ".")
        run_git(root, "commit", "-qm", "invalid committed target fragment")
        head = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()

        # Worktree mode intentionally follows the live target overlay.
        target.write_text("# Gameplay\n<a id=\"missing\"></a>\n## Authority\n", encoding="utf-8")
        result = invoke(root, base, head, worktree=True)
        output = result.stdout + result.stderr
        assert result.returncode == 0, output
        assert "product-doc-content: OK (checked 1" in output, output
    finally:
        shutil.rmtree(root)


def scenario_committed_target_existence_is_frozen() -> None:
    root, base, _head = make_repo()
    try:
        target = root / "doc/game/missing.md"
        (root / TOPIC).write_text(
            TOPIC_TEXT.replace("../../game/prd.md#authority", "../../game/missing.md#authority", 1)
            + "\ncommitted source change\n",
            encoding="utf-8",
        )
        run_git(root, "add", ".")
        run_git(root, "commit", "-qm", "missing committed target")
        head = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()

        # A newly created live target must not mask a missing committed target.
        target.write_text("# Missing\n<a id=\"authority\"></a>\n", encoding="utf-8")
        result = invoke(root, base, head)
        output = result.stdout + result.stderr
        assert result.returncode != 0, output
        assert "missing target: doc/game/missing.md" in output, output
    finally:
        shutil.rmtree(root)


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


def list_continuation_unresolved_id(root: Path) -> None:
    continuation = """
- A list item whose continuation remains prose.
    REQ-NONEXISTENT-001 must not be hidden by indentation.
"""
    (root / TOPIC).write_text(TOPIC_TEXT + continuation, encoding="utf-8")


def list_continuation_orphan_anchor(root: Path) -> None:
    continuation = """
- A list item whose continuation remains prose.
    <a id="req-list-orphan-001"></a>
"""
    (root / TOPIC).write_text(TOPIC_TEXT + continuation, encoding="utf-8")


def true_indented_code_example(root: Path) -> None:
    code = """
    REQ-CODE-001 and [missing](missing.md) remain code.
"""
    (root / TOPIC).write_text(TOPIC_TEXT + code, encoding="utf-8")


def legacy_declarations_missing_anchors(root: Path) -> None:
    declarations = """
## 6. 兼容性声明
- REQ-LEGACY-001：兼容旧入口的要求。
- AC-LEGACY-001：兼容旧入口的验收。
| REQ-TABLE-001 | requirement |
| AC-TABLE-001 | acceptance |
"""
    (root / TOPIC).write_text(TOPIC_TEXT + declarations, encoding="utf-8")


def legacy_declarations_with_anchors(root: Path) -> None:
    declarations = """
## 6. 兼容性声明
<a id="req-legacy-001"></a>
- REQ-LEGACY-001：兼容旧入口的要求。
<a id="ac-legacy-001"></a>
- AC-LEGACY-001：兼容旧入口的验收。
<a id="req-table-001"></a>
| REQ-TABLE-001 | requirement |
<a id="ac-table-001"></a>
| AC-TABLE-001 | acceptance |
"""
    (root / TOPIC).write_text(TOPIC_TEXT + declarations, encoding="utf-8")


def inline_code_anchor_is_not_fragment(root: Path) -> None:
    (root / "doc/game/prd.md").write_text(
        "# Gameplay\n`<a id=\"pseudo\"></a>`\n## Authority\n", encoding="utf-8"
    )
    updated = TOPIC_TEXT.replace(
        "../../game/prd.md#authority", "../../game/prd.md#pseudo", 1
    )
    (root / TOPIC).write_text(updated, encoding="utf-8")


def indented_requirement_heading_without_acceptance(root: Path) -> None:
    updated = TOPIC_TEXT.replace(
        "<a id=\"req-sample-001\"></a>\n### REQ-SAMPLE-001",
        "<a id=\"req-sample-001\"></a>\n  ### REQ-SAMPLE-001",
    ).replace("- 验收：AC-SAMPLE-001\n", "")
    (root / TOPIC).write_text(updated, encoding="utf-8")


def standalone_requirement_and_acceptance_anchors(root: Path) -> None:
    (root / TOPIC).write_text(
        TOPIC_TEXT
        + '\n<a id="req-orphan-001"></a>\n<a id="ac-orphan-001"></a>\n',
        encoding="utf-8",
    )


def main() -> None:
    scenario_full_corpus_requires_active_topic_requirement()
    scenario_full_corpus_requires_active_topic_acceptance()
    scenario_full_corpus_exempts_non_active_topic_cardinality()
    scenario_full_corpus_includes_unchanged_legacy_and_sorts_diagnostics()
    scenario_full_corpus_accepts_retired_topics_and_reports_lifecycle_errors()
    scenario_full_corpus_requires_exact_lifecycle_enum()
    scenario_full_corpus_counts_module_root()
    scenario_full_corpus_rejects_product_symlinks()
    scenario_full_corpus_requires_lifecycle_closure()
    scenario_full_corpus_rejects_lifecycle_closure_placeholders_and_unlinked_fields()
    scenario_full_corpus_requires_design_decision_or_exemption()
    scenario_full_corpus_accepts_simple_topic_exemption()
    scenario_full_corpus_requires_bound_repository_task_evidence()
    scenario_full_corpus_requires_complete_prd_consistent_design_mapping()
    scenario_full_corpus_requires_paired_design_link()
    scenario_full_corpus_requires_prd_trace_fragments()
    scenario_full_corpus_validates_module_root()
    scenario_full_corpus_rejects_changed_range_arguments()
    scenario(None, lambda _root: None)
    scenario("missing-anchor", legacy_declarations_missing_anchors)
    scenario(None, legacy_declarations_with_anchors)
    scenario("invalid-fragment", inline_code_anchor_is_not_fragment)
    scenario("missing-metadata", lambda root: (root / TOPIC).write_text(TOPIC_TEXT.replace("- Owner role：`producer_system_designer`\n", ""), encoding="utf-8"))
    scenario("missing-metadata", lambda root: (root / TOPIC).write_text(TOPIC_TEXT.replace("- Last reviewed：2026-09-10\n", ""), encoding="utf-8"))
    scenario("missing-metadata", lambda root: (root / DESIGN).write_text(DESIGN_TEXT.replace("- Last reviewed：2026-09-10\n", ""), encoding="utf-8"))
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
    scenario("unresolved-id-reference", list_continuation_unresolved_id)
    scenario("anchor-without-declaration", list_continuation_orphan_anchor)
    scenario(None, true_indented_code_example)
    scenario("req-missing-acceptance", indented_requirement_heading_without_acceptance)
    scenario("anchor-without-declaration", standalone_requirement_and_acceptance_anchors)
    scenario("duplicate-anchor", lambda root: (root / TOPIC).write_text(TOPIC_TEXT.replace("<a id=\"ac-sample-001\"></a>", "<a id=\"req-sample-001\"></a>\n<a id=\"ac-sample-001\"></a>"), encoding="utf-8"))
    scenario("unresolved-cross-file-id", lambda root: (root / DESIGN).write_text(DESIGN_TEXT.replace("sample.prd.md#req-sample-001", "sample.prd.md#req-missing"), encoding="utf-8"))
    scenario("external-cross-file-id", external_requirement_links)
    scenario_paired_trace_missing_column()
    scenario_paired_trace_missing_relation()
    scenario_paired_trace_empty_evidence()
    scenario_paired_trace_requires_strict_test_tier_tokens()
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
    scenario_worktree_overlapping_target_change()
    scenario_worktree_local_overlay_wins()
    scenario_committed_target_content_is_frozen()
    scenario_worktree_target_overlay_remains_valid()
    scenario_committed_target_existence_is_frozen()
    scenario_checked(lambda root: (root / TOPIC).write_text(
        TOPIC_TEXT.replace("玩家需要知道当前目标", "  玩家需要知道当前目标"), encoding="utf-8"
    ))
    scenario("missing-anchor", lambda root: (root / TOPIC).write_text(
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
