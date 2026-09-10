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
            tool = (args.tool_root or Path(os.environ.get('OASIS7_LOOP_TOOL_ROOT', Path(__file__).resolve().parents[2]))).resolve()
            commit = binding.get('policy_commit', '')
            if not re.fullmatch(r'[0-9a-f]{40}', commit): raise ValueError('missing effective policy commit')
            subprocess.run(['git', '-C', str(root), 'fetch', '--no-tags', 'origin', 'main:refs/remotes/origin/main'], check=True, capture_output=True)
            subprocess.run(['git', '-C', str(root), 'merge-base', '--is-ancestor', commit, 'refs/remotes/origin/main'], check=True, capture_output=True)
            if run('git', '-C', str(tool), 'rev-parse', 'HEAD') != commit: raise ValueError('effective tool HEAD mismatch')
            if run('git', '-C', str(root), 'rev-parse', '--path-format=absolute', '--git-common-dir') != run('git', '-C', str(tool), 'rev-parse', '--path-format=absolute', '--git-common-dir'): raise ValueError('effective tool repository mismatch')
            names = run('git', '-C', str(tool), 'ls-tree', '-r', '--name-only', commit, '--', 'scripts/pm').splitlines()
            for name in names:
                if (tool / name).is_symlink() or (tool / name).read_bytes() != subprocess.check_output(['git', '-C', str(tool), 'show', commit + ':' + name]): raise ValueError('effective helper bytes mismatch')
            if run('git', '-C', str(tool), 'ls-files', '--others', '--', 'scripts/pm', ':(exclude)**/__pycache__/**'): raise ValueError('untracked effective helper shadow')
            sys.path.insert(0, str(tool / 'scripts/pm'))
            spec = importlib.util.spec_from_file_location('effective_loop', tool / 'scripts/pm/loop.py')
            module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
            from loop_policy import scope_context
            context = scope_context(root, args.base, args.head)
            result = module.validate_task(root, {**task, 'repository': repository}, tool, context['scope_base_oid'], args.head)
            result['scope_context'] = context
        print(json.dumps(result, sort_keys=True))
        return 0 if result['status'] in ('passed', 'legacy') else 2
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as exc:
        print(json.dumps({'status': 'blocked', 'blockers': [str(exc)]}))
        return 2


if __name__ == '__main__': raise SystemExit(main())
