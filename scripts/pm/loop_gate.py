"""Live task loop admission shared by PR, review, packet and CI boundaries."""
import base64
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess


def live_binding(task):
    repo = task.get('repository')
    number = task.get('issue_number')
    if not repo or not number:
        raise ValueError('loop admission requires live task Issue identity')
    issue = json.loads(subprocess.check_output(['gh', 'api', f'repos/{repo}/issues/{number}'], text=True))
    body = issue.get('body', '')
    if re.findall(r'^task_uid:[^\r\n]*', body, re.MULTILINE) != ['task_uid: ' + str(task['task_uid'])]:
        raise ValueError('live Issue task UID mismatch')
    matches = re.findall(r'^- loop_binding_b64: `([^`]+)`$', body, re.MULTILINE)
    if 'loop_binding_b64:' not in body:
        if task.get('loop_binding') is not None: raise ValueError('live loop binding disappeared')
        pages = json.loads(subprocess.check_output(['gh', 'api', f'repos/{repo}/issues/{number}/comments', '--paginate', '--slurp'], text=True))
        if not isinstance(pages, list): raise ValueError('loop lineage readback unavailable')
        comments = [item for page in pages for item in (page if isinstance(page, list) else [page])]
        if any('oasis7-loop-binding-history' in str(item.get('body', '')) for item in comments if isinstance(item, dict)):
            raise ValueError('live loop binding deleted after immutable binding history')
        return None
    if len(matches) != 1: raise ValueError('malformed live loop binding')
    return json.loads(base64.urlsafe_b64decode(matches[0] + '=' * (-len(matches[0]) % 4)))


def admission(root, task, base, head, tool_root=None, reader=None):
    # Packet/review callers supply their frozen ancestor scope base, not live integration base.
    binding = (reader or live_binding)(task)
    if binding is None:
        return {'status': 'legacy', 'blockers': []}
    if task.get('loop_binding') != binding:
        raise ValueError('loop cache differs from live Issue; refresh canonical task first')
    path = Path(__file__).with_name('loop.py')
    spec = importlib.util.spec_from_file_location('loop_facade_gate', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    configured = tool_root or os.environ.get('OASIS7_LOOP_TOOL_ROOT')
    if configured:
        subprocess.run(['git', '-C', str(root), 'fetch', '--no-tags', 'origin', 'main:refs/remotes/origin/main'], check=True, capture_output=True)
    result = module.validate_task(Path(root), task, Path(configured) if configured else None, base, head)
    if result['status'] != 'passed': raise ValueError('loop admission: ' + '; '.join(result['blockers']))
    return result


def mapped_admission(root, uid, base, head):
    task = json.loads((Path(root) / '.pm/github-project-sync/tasks.json').read_text())['tasks'][uid]
    return admission(root, task, base, head)
