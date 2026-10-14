#!/usr/bin/env python3
"""Advisory core headings and explicit feature cards, not semantic truth verification.

Scope uses the existing product/system selectors. A selected product file is
scanned in full, including legacy REQ blocks; absence is a finding, not coverage.
No remote links are fetched and no implementation status is inferred from Git.
"""
from __future__ import annotations
import argparse
import importlib.util
from pathlib import Path
import re
import sys
from doc_content_warnings import report_findings
from markdown_it import MarkdownIt


def load_checker(name):
    spec = importlib.util.spec_from_file_location(name.replace('-', '_'), Path(__file__).with_name(name + '.py'))
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


product = load_checker('product-doc-content-check')
system = load_checker('system-design-traceability-check')
STATUS_FIELDS = ('系统设计状态', '实现状态', '系统设计依据', '实现与验证依据', '核对日期与基线', '剩余范围')
DESIGN_STATUS = {'未核实', '未设计', '部分覆盖', '已覆盖', '不适用'}
IMPLEMENTATION_STATUS = {'未核实', '未实现', '部分实现', '已实现待验证', '已验证'}


def finding(path, line, code, message):
    return f'doc-core-content: {code}: {path}:{line}: {message}'


def check_core(path, text):
    lines = product.visible_lines(text)
    headings = '\n'.join(line for _, line in lines if re.match(r'^ {0,3}#{1,6}\s', line))
    # Root product PRDs retain their separate, established root contract.
    if path.endswith('/prd.md'):
        return []
    if path.startswith('doc/product/') and path.endswith('.prd.md'):
        groups = ('文档身份', '问题|目标', '范围|Non-Goals', '体验|决策', '要求', '验收|证据', '未决|取舍|风险')
    elif path.startswith('doc/product/'):
        groups = ('文档身份', '设计命题|问题|目标', '代表性|流程|体验', '机制|设计', '验证|追踪|证据', '决策|取舍|未决|风险')
    else:
        groups = ('问题|目标|范围', '上游|需求承接', '边界|结构|设计条款', '验证', '决策|未决|风险')
    tailoring = [(number, line) for number, line in lines if re.match(r'^\s*-?\s*结构裁剪[:：]', line)]
    if tailoring:
        number, line = tailoring[0]
        values = {key: re.search(key + r'[:：]\s*([^；;]+)', line) for key in ('理由', '范围', '风险', '映射')}
        complete = all(match and match[1].strip() not in {'无', '-', 'N/A', '待补'} for match in values.values())
        mapping = values['映射'][1] if values['映射'] else ''
        mappings = []
        for token in MarkdownIt('commonmark').parse(mapping):
            target, label = None, ''
            for child in token.children or []:
                if child.type == 'link_open':
                    target, label = child.attrGet('href'), ''
                elif child.type == 'link_close':
                    link_path, fragment = product.split_link_target(target or '')
                    if not link_path and fragment and product.fragment_exists(text, fragment):
                        mappings.append(label)
                    target = None
                elif target is not None:
                    label += child.content
        has_valid_mapping = bool(mappings)
        missing = [group for group in groups if not re.search(group, headings, re.I)]
        unmapped = []
        for group in missing:
            match = next((i for i, label in enumerate(mappings) if re.search(group, label, re.I)), None)
            if match is None:
                unmapped.append(group)
            else:
                mappings.pop(match)  # Each missing group needs its own labelled link.
        if complete and not unmapped and has_valid_mapping:
            return []
        return [finding(path, number, 'invalid-structure-tailoring', '裁剪须有具体理由、范围、风险；每个缺失核心组分别用匹配该组的链接标签映射到本文件真实章节。缺映射：' + ', '.join(unmapped))]
    return [finding(path, 1, 'missing-core-section', f'缺少核心章节（可使用等价标题或说明结构裁剪）：{group}') for group in groups if not re.search(group, headings, re.I)]


def check_status(path, text):
    lines = product.visible_lines(text)
    starts = []
    for i, (_, line) in enumerate(lines):
        if re.match(r'^ {0,3}#{2,6}\s+(?:REQ-[A-Z0-9_-]+|(?:\d+(?:\.\d+)*[.、]?\s*)?功能(?:点|条目)?[：: ]|功能状态卡)', line, re.I):
            starts.append(i)
    # Explicit cards outside a recognizable feature heading are still checked.
    if not starts and any(re.match(r'^\s*(?:-\s*)?(系统设计状态|实现状态)[:：]', line) for _, line in lines):
        starts = [0]
    errors = []
    for i in starts:
        number, heading = lines[i]
        level_match = re.match(r'^ {0,3}(#{1,6})\s', heading)
        level = len(level_match[1]) if level_match else 0
        end = next((j for j in range(i+1, len(lines)) if (m := re.match(r'^ {0,3}(#{1,6})\s', lines[j][1])) and len(m[1]) <= level), len(lines))
        block = '\n'.join(line for _, line in lines[i:end])
        fields = {}
        for label in STATUS_FIELDS:
            matches = re.findall(r'^\s*(?:-\s*)?'+re.escape(label)+r'[:：]\s*(.*?)\s*$', block, re.M)
            if len(matches) != 1 or not matches[0]:
                errors.append(finding(path, number, 'missing-status-field', f'{heading.strip()} 需要唯一非空字段：{label}'))
            fields[label] = matches[0].strip('` ') if matches else ''
        for label, allowed in [('系统设计状态', DESIGN_STATUS), ('实现状态', IMPLEMENTATION_STATUS)]:
            value = fields[label]
            if value and value not in allowed:
                errors.append(finding(path, number, 'invalid-status', f'{label}={value!r}；允许：{", ".join(sorted(allowed))}'))
        for status, evidence, positive in [('系统设计状态','系统设计依据',{'部分覆盖','已覆盖'}),('实现状态','实现与验证依据',{'部分实现','已实现待验证','已验证'})]:
            if fields[status] in positive and not product.markdown_links(fields[evidence]):
                errors.append(finding(path, number, 'missing-status-evidence', f'{status}必须关联真实证据链接（链接真实性/内容仍需人工核对）'))
        if fields['系统设计状态'] == '不适用':
            if not all(re.search(word + r'[:：]\s*[^；;\s]+', fields['系统设计依据']) for word in ('理由', '范围')):
                errors.append(finding(path, number, 'not-applicable-missing-reason', '设计不适用须说明具体理由和适用范围；需人工核实没有独立技术义务'))
        if fields['系统设计状态'] == '部分覆盖' or fields['实现状态'] == '部分实现':
            if not fields['剩余范围'] or fields['剩余范围'] in {'无','N/A','待补','-'}:
                errors.append(finding(path, number, 'partial-missing-scope', '部分状态须明确剩余未覆盖范围'))
        if fields['实现状态'] == '已验证':
            evidence = fields['实现与验证依据'] + fields['核对日期与基线']
            if not re.search(r'\d{4}-\d{2}-\d{2}', fields['核对日期与基线']) or not all(word in evidence for word in ('基线','环境','验收')):
                errors.append(finding(path, number, 'verified-missing-boundary', '已验证需要核对日期、基线、环境与验收范围'))
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo-root', type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument('--base')
    parser.add_argument('--head')
    parser.add_argument('--worktree', action='store_true')
    parser.add_argument('--full-corpus', action='store_true', help='Scan all product PRD/design documents, including legacy blocks; professional system designs remain scoped')
    parser.add_argument('--warn-only', action='store_true')
    args = parser.parse_args()
    root = args.repo_root.resolve()
    if args.full_corpus and (args.base or args.head or args.worktree):
        parser.error('--full-corpus cannot be combined with range/worktree')
    try:
        if args.full_corpus:
            product.run_git(root, 'rev-parse', '--verify', 'HEAD^{commit}')
            documents, hard_errors = product.collect_full_corpus(root)
            if hard_errors:
                print('\n'.join(hard_errors))
                return 2
            selected = [(d.path, d.new_text) for d in documents]
        else:
            base = product.resolve_commit(root, args.base, 'base')
            head = product.resolve_commit(root, args.head, 'head')
            documents, _ = product.collect_documents(root, base, head, args.worktree)
            selected = [(d.path, d.new_text) for d in documents]
            checkout = product.run_git(root, 'rev-parse', '--verify', 'HEAD^{commit}').strip()
            for path in system.changed_system_design_paths(root, base, head, args.worktree):
                relative = path.relative_to(root).as_posix()
                text = system.worktree_file_text(root, checkout, relative) if args.worktree else product.git_text(root, head, relative)
                if text is None:
                    raise OSError(f'selected document cannot be read: {relative}')
                selected.append((relative, text))
        errors = []
        for path, text in selected:
            lifecycle = product.metadata_value(product.document_identity_text(text), '生命周期')
            if path.startswith('doc/product/') and lifecycle and lifecycle.strip('` ') in {'superseded', 'retired'}:
                print(f'doc-core-content: skipped active core/status rules: {path}: lifecycle={lifecycle}; identity and successor remain checked by product content checker')
                continue
            core_findings = check_core(path, text)
            errors.extend(core_findings)
            if not core_findings and any('结构裁剪' in line for _, line in product.visible_lines(text)):
                print(f'::notice::doc-core-content: {path}: 结构裁剪映射可定位；理由、适用范围和风险仍需人工复核')
            if path.startswith('doc/product/') and (path.endswith('.prd.md') or path.endswith('/prd.md')):
                errors.extend(check_status(path, text))
        print(f'doc-core-content: checked {len(selected)} documents; selected documents scanned in full, including legacy REQ blocks; unrecognized feature prose and evidence truth require human review')
        return report_findings(errors, args.warn_only) if errors else 0
    except (OSError, ValueError) as exc:
        print(f'doc-core-content: error: {exc}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
