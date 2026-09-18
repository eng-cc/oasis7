#!/usr/bin/env python3
"""Regression contract for the changed-scope system-design traceability gate.

The fixtures intentionally use small temporary repositories.  They exercise the
range selector separately from the document contract so target-only and
worktree-overlay behavior cannot be hidden by the repository's own corpus.
"""

from __future__ import annotations

import importlib.util
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parent.parent
CHECKER = ROOT / "scripts/system-design-traceability-check.py"
DESIGN = "doc/system/sample.design.md"
PRODUCT = "doc/product/sample.prd.md"
TEST_SOURCE = "doc/tests/sample.test.py"
MANUAL_SOURCE = "doc/manual/sample.md"


PRODUCT_TEXT = """# Sample product requirement

<a id="req-sample"></a>
## REQ-SAMPLE-001

The product requirement has a stable fragment for the system design to consume.
"""


DESIGN_HEADER = """# Sample system design

## 2. 上游约束与相关角色

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [REQ-SAMPLE-001](../product/sample.prd.md#req-sample) | 说明系统必须承接的结果与适用条件。 | [DES-SAMPLE-001](#des-sample-001) | `repository_health_engineer` | 不覆盖未声明的入口。 |

### DES-SAMPLE-001

系统设计条款说明边界、拒绝条件和可观察结果。

## 11. 验证设计与可追溯性

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [REQ-SAMPLE-001](../product/sample.prd.md#req-sample) | [DES-SAMPLE-001](#des-sample-001) | 验证该设计义务的拒绝与成功边界。 | [sample test](../tests/sample.test.py) | GitHub task evidence | 不覆盖生产发布。 |
"""


DEMAND_ROW = "| [REQ-SAMPLE-001](../product/sample.prd.md#req-sample) | 说明系统必须承接的结果与适用条件。 | [DES-SAMPLE-001](#des-sample-001) | `repository_health_engineer` | 不覆盖未声明的入口。 |"
VALIDATION_ROW = "| [REQ-SAMPLE-001](../product/sample.prd.md#req-sample) | [DES-SAMPLE-001](#des-sample-001) | 验证该设计义务的拒绝与成功边界。 | [sample test](../tests/sample.test.py) | GitHub task evidence | 不覆盖生产发布。 |"


def load_checker():
    if not CHECKER.is_file():
        raise AssertionError(
            "system-design-traceability: checker missing; RED must be caused by the unimplemented checker"
        )
    spec = importlib.util.spec_from_file_location("system_design_traceability_check", CHECKER)
    if spec is None or spec.loader is None:
        raise AssertionError("system-design-traceability: checker has no import loader")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def run_git(root: Path, *args: str, check: bool = True) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *args],
        check=False,
        capture_output=True,
        text=True,
    )
    if check and result.returncode != 0:
        raise AssertionError(result.stderr or result.stdout)
    return result.stdout.strip()


def make_repo(*, design_text: str | None = DESIGN_HEADER, include_test: bool = True) -> tuple[Path, str, str]:
    root = Path(tempfile.mkdtemp(prefix="system-design-traceability-"))
    (root / "doc/product").mkdir(parents=True)
    (root / "doc/system").mkdir(parents=True)
    if include_test:
        (root / "doc/tests").mkdir(parents=True)
    (root / "doc/manual").mkdir(parents=True)
    (root / PRODUCT).write_text(PRODUCT_TEXT, encoding="utf-8")
    if design_text is not None:
        (root / DESIGN).write_text(design_text, encoding="utf-8")
    if include_test:
        (root / TEST_SOURCE).write_text("# test source\n", encoding="utf-8")
    (root / MANUAL_SOURCE).write_text("# manual case\n", encoding="utf-8")
    run_git(root, "init", "-q", "-b", "main")
    run_git(root, "config", "user.email", "test@example.invalid")
    run_git(root, "config", "user.name", "Traceability Test")
    run_git(root, "add", ".")
    run_git(root, "commit", "-qm", "fixture")
    base = run_git(root, "rev-parse", "HEAD")
    return root, base, base


def commit(root: Path, message: str) -> str:
    run_git(root, "add", ".")
    run_git(root, "commit", "-qm", message)
    return run_git(root, "rev-parse", "HEAD")


def assert_no_errors(module, path: Path) -> None:
    errors = module.check_system_design(path)
    assert errors == [], "unexpected traceability errors: " + repr(errors)


def assert_code(module, path: Path, code: str) -> None:
    errors = module.check_system_design(path)
    assert any(f"system-design-traceability: {code}:" in error for error in errors), errors
    assert all("repair" in error.lower() or "add " in error.lower() for error in errors), errors


def scenario_valid_new_design() -> None:
    module = load_checker()
    root, base, _head = make_repo(design_text=None)
    try:
        (root / DESIGN).write_text(DESIGN_HEADER, encoding="utf-8")
        head = commit(root, "add valid system design")
        paths = module.changed_system_design_paths(root, base, head, False)
        assert [path.relative_to(root).as_posix() for path in paths] == [DESIGN], paths
        assert_no_errors(module, root / DESIGN)
    finally:
        shutil.rmtree(root)


def scenario_committed_range_reads_trusted_head_content() -> None:
    root, base, _head = make_repo(design_text=None)
    try:
        run_git(root, "checkout", "-qb", "source")
        (root / DESIGN).write_text(DESIGN_HEADER, encoding="utf-8")
        head = commit(root, "add source design")
        run_git(root, "checkout", "-q", "main")
        result = subprocess.run(
            [
                sys.executable,
                str(CHECKER),
                "--repo-root",
                str(root),
                "--base",
                base,
                "--head",
                head,
            ],
            check=False,
            capture_output=True,
            text=True,
        )
        output = result.stdout + result.stderr
        assert result.returncode == 0, output
        assert "checked 1 changed/new system designs" in output, output
    finally:
        shutil.rmtree(root)


def scenario_committed_target_symlink_does_not_redirect_selected_head() -> None:
    root, base, _head = make_repo()
    try:
        (root / "doc/product/alternate.prd.md").write_text("# alternate product\n", encoding="utf-8")
        (root / DESIGN).write_text(DESIGN_HEADER + "\nSubstantive committed design revision.\n", encoding="utf-8")
        head = commit(root, "revise design with alternate target")
        product = root / PRODUCT
        product.unlink()
        product.symlink_to("alternate.prd.md")
        result = subprocess.run(
            [
                sys.executable,
                str(CHECKER),
                "--repo-root",
                str(root),
                "--base",
                base,
                "--head",
                head,
            ],
            check=False,
            capture_output=True,
            text=True,
        )
        output = result.stdout + result.stderr
        assert result.returncode == 0, output
        assert "checked 1 changed/new system designs" in output, output
    finally:
        shutil.rmtree(root)


def scenario_worktree_symlinks_are_contained() -> None:
    module = load_checker()
    root, base, _head = make_repo()
    outside = Path(tempfile.mkdtemp(prefix="system-design-traceability-outside-"))
    try:
        outside_target = outside / "alternate.prd.md"
        outside_target.write_text(PRODUCT_TEXT, encoding="utf-8")
        product = root / PRODUCT
        product.unlink()
        product.symlink_to(outside_target)
        (root / DESIGN).write_text(DESIGN_HEADER + "\nSubstantive worktree design revision.\n", encoding="utf-8")
        paths = module.changed_system_design_paths(root, base, base, True)
        assert [path.relative_to(root).as_posix() for path in paths] == [DESIGN], paths
        result = subprocess.run(
            [
                sys.executable,
                str(CHECKER),
                "--repo-root",
                str(root),
                "--base",
                base,
                "--head",
                base,
                "--worktree",
            ],
            check=False,
            capture_output=True,
            text=True,
        )
        output = result.stdout + result.stderr
        assert result.returncode == 1, output
        assert "reference escapes the repository" in output, output
    finally:
        shutil.rmtree(root)
        shutil.rmtree(outside)


def scenario_missing_demand_allocation() -> None:
    module = load_checker()
    root, _base, _head = make_repo(design_text=DESIGN_HEADER.replace("### 2.1 需求承接与相关角色", "### 2.1 需求承接与相关角色"))
    try:
        (root / DESIGN).write_text(DESIGN_HEADER.replace("### 2.1 需求承接与相关角色", "### 2.1 需求承接与相关角色"), encoding="utf-8")
        # Remove the complete allocation table while retaining the validation table.
        start = DESIGN_HEADER.index("### 2.1")
        end = DESIGN_HEADER.index("### DES-SAMPLE-001")
        (root / DESIGN).write_text(DESIGN_HEADER[:start] + DESIGN_HEADER[end:], encoding="utf-8")
        assert_code(module, root / DESIGN, "trace-upstream-missing")
    finally:
        shutil.rmtree(root)


def scenario_missing_validation_mapping() -> None:
    module = load_checker()
    root, _base, _head = make_repo()
    try:
        marker = "## 11. 验证设计与可追溯性"
        (root / DESIGN).write_text(DESIGN_HEADER[: DESIGN_HEADER.index(marker)] + marker + "\n", encoding="utf-8")
        assert_code(module, root / DESIGN, "trace-validation-missing")
    finally:
        shutil.rmtree(root)


def scenario_unresolved_upstream_fragment() -> None:
    module = load_checker()
    root, _base, _head = make_repo(
        design_text=DESIGN_HEADER.replace("#req-sample)", "#missing-upstream)")
    )
    try:
        assert_code(module, root / DESIGN, "trace-ref-unresolved")
    finally:
        shutil.rmtree(root)


def scenario_required_row_shape_and_cells() -> None:
    module = load_checker()
    cases = (
        (
            DESIGN_HEADER.replace(DEMAND_ROW, "| [REQ-SAMPLE-001](../product/sample.prd.md#req-sample) | obligation | [DES-SAMPLE-001](#des-sample-001) |"),
            "trace-demand-row-invalid",
        ),
        (
            DESIGN_HEADER.replace(DEMAND_ROW, DEMAND_ROW.replace("`repository_health_engineer`", "")),
            "trace-demand-row-invalid",
        ),
        (
            DESIGN_HEADER.replace(VALIDATION_ROW, "| [REQ-SAMPLE-001](../product/sample.prd.md#req-sample) | [DES-SAMPLE-001](#des-sample-001) | obligation | [sample test](../tests/sample.test.py) |"),
            "trace-validation-row-invalid",
        ),
        (
            DESIGN_HEADER.replace(VALIDATION_ROW, VALIDATION_ROW.replace("GitHub task evidence", "")),
            "trace-validation-row-invalid",
        ),
    )
    for text, code in cases:
        root, _base, _head = make_repo(design_text=text)
        try:
            assert_code(module, root / DESIGN, code)
        finally:
            shutil.rmtree(root)


def scenario_validation_source_must_be_test_or_manual() -> None:
    module = load_checker()
    for method in (
        "[self](#des-sample-001)",
        "[product](../product/sample.prd.md#req-sample)",
    ):
        root, _base, _head = make_repo(
            design_text=DESIGN_HEADER.replace("[sample test](../tests/sample.test.py)", method)
        )
        try:
            assert_code(module, root / DESIGN, "trace-validation-source-invalid")
        finally:
            shutil.rmtree(root)

    root, _base, _head = make_repo(
        design_text=DESIGN_HEADER.replace(
            "[sample test](../tests/sample.test.py)",
            "[manual case](../manual/sample.md)",
        )
    )
    try:
        assert_no_errors(module, root / DESIGN)
    finally:
        shutil.rmtree(root)


def scenario_anchor_and_heading_fragment_collision_is_ambiguous() -> None:
    module = load_checker()
    root, _base, _head = make_repo(
        design_text=DESIGN_HEADER.replace(
            "### DES-SAMPLE-001",
            '<a id="des-sample-001"></a>\n### DES-SAMPLE-001',
            1,
        )
    )
    try:
        assert_code(module, root / DESIGN, "trace-ref-ambiguous")
    finally:
        shutil.rmtree(root)


def scenario_fenced_code_trace_decoy_is_ignored() -> None:
    module = load_checker()
    decoy = """```markdown\n<a id=\"des-sample-001\"></a>\n### 2.1 需求承接与分配表\n| upstream | obligation | design | owner | excluded |\n| --- | --- | --- | --- | --- |\n| [bad](missing.md#bad) | bad | [bad](#des-sample-001) | bad | bad |\n```\n\n"""
    root, _base, _head = make_repo(design_text=decoy + DESIGN_HEADER)
    try:
        assert_no_errors(module, root / DESIGN)
    finally:
        shutil.rmtree(root)


def na_design(*, disposition: str) -> str:
    return DESIGN_HEADER.replace(
        "[sample test](../tests/sample.test.py)",
        disposition,
    )


def scenario_complete_na() -> None:
    module = load_checker()
    root, _base, _head = make_repo(
        design_text=na_design(
            disposition="N/A: reason=该条款不涉及运行时技术义务；scope=本次文档治理变更；owner_role=repository_health_engineer；evidence_ref=https://github.com/eng-cc/oasis7/issues/3671#issuecomment-5636906114；re-evaluate=下次改变跨组件行为时。"
        )
    )
    try:
        assert_no_errors(module, root / DESIGN)
    finally:
        shutil.rmtree(root)


def scenario_na_accepts_readable_repository_path_fragment() -> None:
    module = load_checker()
    root, _base, _head = make_repo(
        design_text=na_design(
            disposition="N/A: reason=该条款不涉及运行时技术义务；scope=本次文档治理变更；owner_role=repository_health_engineer；evidence_ref=doc/product/sample.prd.md#req-sample；re-evaluate=下次改变跨组件行为时。"
        )
    )
    try:
        assert_no_errors(module, root / DESIGN)
    finally:
        shutil.rmtree(root)


def scenario_na_rejects_unreadable_or_noncanonical_evidence_locator() -> None:
    module = load_checker()
    for locator in (
        "garbage",
        "../tests/sample.test.py",
        "../tests/missing.test.py#sample",
        "../../outside.md#sample",
        "https://github.com/example/oasis7/issues/3671#issuecomment-5636906114",
        "https://github.com/eng-cc/oasis7/issues/0#issuecomment-5636906114",
        "https://github.com/eng-cc/oasis7/issues/3671#comment-5636906114",
    ):
        root, _base, _head = make_repo(
            design_text=na_design(
                disposition=f"N/A: reason=范围外；scope=本次文档治理变更；owner_role=repository_health_engineer；evidence_ref={locator}；re-evaluate=下次改变跨组件行为时。"
            )
        )
        try:
            assert_code(module, root / DESIGN, "trace-na-incomplete")
        finally:
            shutil.rmtree(root)


def scenario_incomplete_na() -> None:
    module = load_checker()
    root, _base, _head = make_repo(
        design_text=na_design(disposition="N/A: reason=范围外。")
    )
    try:
        assert_code(module, root / DESIGN, "trace-na-incomplete")
    finally:
        shutil.rmtree(root)


def scenario_target_only_design_change_is_excluded() -> None:
    module = load_checker()
    root, common, _head = make_repo(design_text=None)
    try:
        run_git(root, "checkout", "-qb", "feature")
        feature = run_git(root, "rev-parse", "HEAD")
        run_git(root, "checkout", "-q", "main")
        (root / DESIGN).write_text(DESIGN_HEADER, encoding="utf-8")
        target = commit(root, "target-only design change")
        paths = module.changed_system_design_paths(root, target, feature, False)
        assert paths == [], paths
        assert common != target
    finally:
        shutil.rmtree(root)


def scenario_whitespace_and_comment_only_source_change_is_excluded() -> None:
    module = load_checker()
    root, base, _head = make_repo()
    try:
        (root / DESIGN).write_text("<!-- changed comment -->\n" + DESIGN_HEADER.replace("\n", "  \n"), encoding="utf-8")
        head = commit(root, "non-semantic design formatting")
        assert module.changed_system_design_paths(root, base, head, False) == []
    finally:
        shutil.rmtree(root)


def scenario_untracked_worktree_design_is_included() -> None:
    module = load_checker()
    root, base, _head = make_repo(design_text=None)
    try:
        (root / DESIGN).write_text(DESIGN_HEADER, encoding="utf-8")
        paths = module.changed_system_design_paths(root, base, base, True)
        assert [path.relative_to(root).as_posix() for path in paths] == [DESIGN], paths
    finally:
        shutil.rmtree(root)


def scenario_staged_worktree_design_is_included() -> None:
    module = load_checker()
    root, base, _head = make_repo(design_text=None)
    try:
        (root / DESIGN).write_text(DESIGN_HEADER, encoding="utf-8")
        run_git(root, "add", DESIGN)
        paths = module.changed_system_design_paths(root, base, base, True)
        assert [path.relative_to(root).as_posix() for path in paths] == [DESIGN], paths
    finally:
        shutil.rmtree(root)


def scenario_staged_tracked_worktree_design_uses_index_content() -> None:
    module = load_checker()
    root, base, _head = make_repo()
    try:
        staged_text = DESIGN_HEADER.replace(VALIDATION_ROW, "")
        (root / DESIGN).write_text(staged_text, encoding="utf-8")
        run_git(root, "add", DESIGN)
        run_git(root, "restore", "--source=HEAD", "--worktree", "--", DESIGN)
        paths = module.changed_system_design_paths(root, base, base, True)
        assert [path.relative_to(root).as_posix() for path in paths] == [DESIGN], paths
        result = subprocess.run(
            [
                sys.executable,
                str(CHECKER),
                "--repo-root",
                str(root),
                "--base",
                base,
                "--head",
                base,
                "--worktree",
            ],
            check=False,
            capture_output=True,
            text=True,
        )
        output = result.stdout + result.stderr
        assert result.returncode == 1, output
        assert "trace-validation-missing" in output, output
    finally:
        shutil.rmtree(root)


def scenario_unstaged_worktree_design_is_included() -> None:
    module = load_checker()
    root, base, _head = make_repo()
    try:
        (root / DESIGN).write_text(DESIGN_HEADER + "\nAdditional substantive behavior.\n", encoding="utf-8")
        paths = module.changed_system_design_paths(root, base, base, True)
        assert [path.relative_to(root).as_posix() for path in paths] == [DESIGN], paths
    finally:
        shutil.rmtree(root)


def scenario_duplicate_validation_rows_fail_cardinality() -> None:
    module = load_checker()
    root, _base, _head = make_repo(
        design_text=DESIGN_HEADER.replace(VALIDATION_ROW, VALIDATION_ROW + "\n" + VALIDATION_ROW)
    )
    try:
        assert_code(module, root / DESIGN, "trace-slot-cardinality")
    finally:
        shutil.rmtree(root)


def scenario_committed_validation_symlink_sources_fail_closed() -> None:
    for label in ("dangling", "external"):
        root, base, _head = make_repo()
        outside = Path(tempfile.mkdtemp(prefix="system-design-traceability-outside-"))
        try:
            source = root / TEST_SOURCE
            source.unlink()
            if label == "dangling":
                source.symlink_to("missing.test.py")
            else:
                outside_target = outside / "external.test.py"
                outside_target.write_text("# external source\n", encoding="utf-8")
                source.symlink_to(outside_target)
            (root / DESIGN).write_text(
                DESIGN_HEADER + f"\nCommitted {label} validation source revision.\n",
                encoding="utf-8",
            )
            head = commit(root, f"add committed {label} validation source")
            result = subprocess.run(
                [
                    sys.executable,
                    str(CHECKER),
                    "--repo-root",
                    str(root),
                    "--base",
                    base,
                    "--head",
                    head,
                ],
                check=False,
                capture_output=True,
                text=True,
            )
            output = result.stdout + result.stderr
            assert result.returncode == 1, output
            assert "trace-validation-source-invalid" in output, output
        finally:
            shutil.rmtree(root)
            shutil.rmtree(outside)


def scenario_untouched_legacy_design_is_excluded() -> None:
    module = load_checker()
    root, base, _head = make_repo(design_text="# legacy design without current trace contract\n")
    try:
        (root / "README.md").write_text("unrelated change\n", encoding="utf-8")
        head = commit(root, "unrelated change")
        assert module.changed_system_design_paths(root, base, head, False) == []
    finally:
        shutil.rmtree(root)


def run_checker(root: Path, base: str, head: str, *, worktree: bool = False) -> tuple[int, str]:
    command = [
            sys.executable,
            str(CHECKER),
            "--repo-root",
            str(root),
            "--base",
            base,
            "--head",
            head,
        ]
    if worktree:
        command.append("--worktree")
    result = subprocess.run(
        command,
        check=False,
        capture_output=True,
        text=True,
    )
    return result.returncode, result.stdout + result.stderr


def scenario_authority_only_prompt_control_change_rechecks_current_consumer() -> None:
    root, base, _head = make_repo()
    try:
        (root / PRODUCT).write_text(PRODUCT_TEXT.replace("req-sample", "req-renamed"), encoding="utf-8")
        head = commit(root, "change prompt control authority anchor")
        code, output = run_checker(root, base, head)
        assert code == 1, output
        assert "trace-ref-unresolved" in output, output
    finally:
        shutil.rmtree(root)


def scenario_authority_only_root_prd_prompt_control_change_rechecks_consumer() -> None:
    root, _initial_base, _head = make_repo()
    try:
        authority = root / "doc/world-simulator/prd.md"
        authority.parent.mkdir(parents=True)
        authority.write_text(PRODUCT_TEXT, encoding="utf-8")
        (root / DESIGN).write_text(
            DESIGN_HEADER.replace("../product/sample.prd.md", "../world-simulator/prd.md"),
            encoding="utf-8",
        )
        fixture_base = commit(root, "add root professional PromptControl authority")
        authority.write_text(PRODUCT_TEXT.replace("req-sample", "req-renamed"), encoding="utf-8")
        head = commit(root, "change root PromptControl authority anchor")
        code, output = run_checker(root, fixture_base, head)
        assert code == 1, output
        assert "trace-ref-unresolved" in output, output
    finally:
        shutil.rmtree(root)


def scenario_authority_rename_and_delete_fail_closed() -> None:
    for action in ("rename", "delete"):
        root, base, _head = make_repo()
        try:
            if action == "rename":
                run_git(root, "mv", PRODUCT, "doc/product/renamed.prd.md")
            else:
                (root / PRODUCT).unlink()
            head = commit(root, f"{action} authority endpoint")
            code, output = run_checker(root, base, head)
            assert code == 1, output
            assert "reference target is missing" in output, output
        finally:
            shutil.rmtree(root)


def scenario_worktree_authority_only_change_rechecks_current_consumer() -> None:
    root, base, _head = make_repo()
    try:
        (root / PRODUCT).write_text(PRODUCT_TEXT.replace("req-sample", "req-renamed"), encoding="utf-8")
        code, output = run_checker(root, base, base, worktree=True)
        assert code == 1, output
        assert "trace-ref-unresolved" in output, output
    finally:
        shutil.rmtree(root)


def scenario_untouched_legacy_consumer_is_excluded_from_authority_change() -> None:
    legacy = "# legacy design\n\n[old requirement](../product/sample.prd.md#req-sample)\n"
    root, base, _head = make_repo(design_text=legacy)
    try:
        (root / PRODUCT).write_text(PRODUCT_TEXT.replace("req-sample", "req-renamed"), encoding="utf-8")
        head = commit(root, "change authority with legacy consumer")
        code, output = run_checker(root, base, head)
        assert code == 0, output
        assert "checked 0" in output, output
    finally:
        shutil.rmtree(root)


def scenario_partial_or_malformed_range_is_rejected() -> None:
    module = load_checker()
    root, base, _head = make_repo(design_text=None)
    try:
        for bad_base, bad_head in (("", base), (base, ""), ("HEAD", base), (base, "not-an-oid")):
            try:
                module.changed_system_design_paths(root, bad_base, bad_head, False)
            except ValueError as error:
                assert "commit OID" in str(error) or "invalid" in str(error), error
            else:
                raise AssertionError((bad_base, bad_head))
    finally:
        shutil.rmtree(root)


def main() -> None:
    scenarios = (
        scenario_valid_new_design,
        scenario_committed_range_reads_trusted_head_content,
        scenario_committed_target_symlink_does_not_redirect_selected_head,
        scenario_worktree_symlinks_are_contained,
        scenario_missing_demand_allocation,
        scenario_missing_validation_mapping,
        scenario_unresolved_upstream_fragment,
        scenario_required_row_shape_and_cells,
        scenario_validation_source_must_be_test_or_manual,
        scenario_anchor_and_heading_fragment_collision_is_ambiguous,
        scenario_fenced_code_trace_decoy_is_ignored,
        scenario_complete_na,
        scenario_na_accepts_readable_repository_path_fragment,
        scenario_na_rejects_unreadable_or_noncanonical_evidence_locator,
        scenario_incomplete_na,
        scenario_target_only_design_change_is_excluded,
        scenario_whitespace_and_comment_only_source_change_is_excluded,
        scenario_untracked_worktree_design_is_included,
        scenario_staged_worktree_design_is_included,
        scenario_staged_tracked_worktree_design_uses_index_content,
        scenario_unstaged_worktree_design_is_included,
        scenario_duplicate_validation_rows_fail_cardinality,
        scenario_committed_validation_symlink_sources_fail_closed,
        scenario_untouched_legacy_design_is_excluded,
        scenario_authority_only_prompt_control_change_rechecks_current_consumer,
        scenario_authority_only_root_prd_prompt_control_change_rechecks_consumer,
        scenario_authority_rename_and_delete_fail_closed,
        scenario_worktree_authority_only_change_rechecks_current_consumer,
        scenario_untouched_legacy_consumer_is_excluded_from_authority_change,
        scenario_partial_or_malformed_range_is_rejected,
    )
    for scenario in scenarios:
        scenario()
    print(f"system-design-traceability-check.test: OK ({len(scenarios)} scenarios)")


if __name__ == "__main__":
    main()
