#!/usr/bin/env python3
"""Fresh local gh admission. Callers must execute this from effective tools."""
import argparse
import base64
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys


def run(*args):
    return subprocess.check_output(args, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--task-uid', required=True)
    parser.add_argument('--base', required=True)
    parser.add_argument('--head', required=True)
    parser.add_argument('--tool-root', type=Path)
    parser.add_argument('--json', action='store_true')
    args = parser.parse_args()
    try:
        root = args.root.resolve()
        mapping = json.loads((root / '.pm/github-project-sync/tasks.json').read_text())
        task = mapping['tasks'][args.task_uid]
        if task.get('task_uid') != args.task_uid: raise ValueError('task UID mismatch')
        repository = task.get('repository') or mapping.get('project', {}).get('repo')
        number = task['issue_number']
        issue = json.loads(run('gh', 'api', f'repos/{repository}/issues/{number}'))
        body = issue.get('body', '')
        if re.findall(r'^task_uid:[^\r\n]*', body, re.MULTILINE) != ['task_uid: ' + args.task_uid]: raise ValueError('live task Issue identity mismatch')
        if 'loop_binding_b64:' not in body:
            if task.get('loop_binding') is not None: raise ValueError('live loop binding disappeared')
            pages = json.loads(run('gh', 'api', f'repos/{repository}/issues/{number}/comments', '--paginate', '--slurp'))
            if not isinstance(pages, list): raise ValueError('lineage readback unavailable')
            comments = [item for page in pages for item in (page if isinstance(page, list) else [page])]
            if any('oasis7-loop-binding-history' in str(item.get('body', '')) for item in comments): raise ValueError('loop binding deleted after history')
            result = {'status': 'legacy', 'blockers': []}
        else:
            if Path(task.get('canonical_worktree') or '').resolve() != root:
                raise ValueError('canonical task worktree mismatch')
            if task.get('task_branch') != run('git', '-C', str(root), 'branch', '--show-current'):
                raise ValueError('canonical task branch mismatch')
            if args.head != run('git', '-C', str(root), 'rev-parse', 'HEAD'):
                raise ValueError('local source HEAD changed before admission')
            matches = re.findall(r'^- loop_binding_b64: `([^`]+)`$', body, re.MULTILINE)
            if len(matches) != 1: raise ValueError('malformed live loop binding')
            binding = json.loads(base64.b64decode(matches[0] + '=' * (-len(matches[0]) % 4), altchars=b'-_', validate=True))
            if binding != task.get('loop_binding'): raise ValueError('live binding differs from task cache')
            helper_root = Path(__file__).resolve().parents[2]
            sys.path.insert(0, str(helper_root / 'scripts/pm'))
            spec = importlib.util.spec_from_file_location('trusted_loop_facade', helper_root / 'scripts/pm/loop.py')
            module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
            active, policy_context = module.resolve_effective_binding(root, task, return_context=True)
            if not isinstance(active, dict): raise ValueError('effective loop binding is unavailable')
            trusted_default_oid = ((policy_context or {}).get('trusted_current_policy') or {}).get('default_branch_oid')
            tool = module.existing_policy_tool_root(
                root, active,
                args.tool_root or os.environ.get('OASIS7_LOOP_TOOL_ROOT') or helper_root,
            )
            commit = active.get('policy_commit', '')
            if not re.fullmatch(r'[0-9a-f]{40}', commit): raise ValueError('missing effective policy commit')
            policy = module._trusted_module_for_binding(
                tool, root, active, 'loop_policy', trusted_default_oid,
            )
            context = policy.scope_context(root, args.base, args.head)
            def validated_admission():
                return module.validate_task(
                    root, {**task, 'repository': repository}, tool,
                    context['scope_base_oid'], args.head,
                    effective_binding=active,
                    trusted_default_oid=trusted_default_oid,
                )
            result = module.pre_mutation_admission(
                'promotion',
                binding=active,
                target_root=root,
                effective_tool_root=tool,
                source_commit=commit,
                effective_tool_commit=commit,
                record_source_commit=(active.get('coordination_ref') or {}).get('source_commit'),
                mutation=validated_admission,
            )
            result['scope_context'] = context
        print(json.dumps(result, sort_keys=True))
        return 0 if result['status'] in ('passed', 'legacy') else 2
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as exc:
        print(json.dumps({'status': 'blocked', 'blockers': [str(exc)]}))
        return 2


if __name__ == '__main__': raise SystemExit(main())
