"""Live task loop admission shared by PR, review, packet and CI boundaries."""
import base64
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import types


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


def _pinned_module(tool_root, commit, name):
    relative = 'scripts/pm/' + name + '.py'
    entries = subprocess.check_output(
        ['git', '-C', str(tool_root), 'ls-tree', commit, '--', relative], text=True,
    ).splitlines()
    if len(entries) != 1 or '\t' not in entries[0]:
        raise ValueError('effective loop module missing or ambiguous: ' + relative)
    metadata, recorded_path = entries[0].split('\t', 1)
    mode, object_type, _oid = metadata.split()
    if recorded_path != relative or mode != '100644' or object_type != 'blob':
        raise ValueError('effective loop module has unsafe Git mode: ' + relative)
    source = subprocess.check_output(['git', '-C', str(tool_root), 'show', commit + ':' + relative])
    module = types.ModuleType(name)
    module.__file__ = f'{tool_root}/{relative}@{commit}'
    module.__package__ = ''
    sys.modules[name] = module
    exec(compile(source, module.__file__, 'exec'), module.__dict__)
    return module


def admission(root, task, base, head, tool_root=None, reader=None):
    # Packet/review callers supply their frozen ancestor scope base, not live integration base.
    binding = (reader or live_binding)(task)
    if binding is None:
        return {'status': 'legacy', 'blockers': []}
    if task.get('loop_binding') != binding:
        raise ValueError('loop cache differs from live Issue; refresh canonical task first')
    configured = tool_root or os.environ.get('OASIS7_LOOP_TOOL_ROOT')
    if not configured:
        raise ValueError('activation prerequisite: explicit trusted --tool-root required')
    path = Path(__file__).with_name('loop.py')
    spec = importlib.util.spec_from_file_location('loop_facade_gate', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    result = module.validate_task(Path(root), task, Path(configured) if configured else None, base, head)
    if result['status'] != 'passed': raise ValueError('loop admission: ' + '; '.join(result['blockers']))
    return result


def mapped_admission(root, uid, base, head):
    task = json.loads((Path(root) / '.pm/github-project-sync/tasks.json').read_text())['tasks'][uid]
    return admission(root, task, base, head)
