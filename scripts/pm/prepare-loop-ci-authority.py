#!/usr/bin/env python3
"""Prepare hosted loop resources before any effective-primary consumer.

This entrypoint is materialized from frozen B, never selected from candidate H.
"""
import argparse
import base64
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys

sys.dont_write_bytecode = True


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()


def sibling(name):
    path = Path(__file__).resolve().parent / (name + '.py')
    if path.is_symlink() or not path.is_file():
        raise ValueError('trusted bootstrap dependency unavailable: ' + name)
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def github(*args, project_token=None):
    # Pre-guard Issue/provenance requests never inherit the broader credential.
    env = {key: value for key, value in os.environ.items() if key != 'OASIS7_PROJECT_READ_TOKEN'}
    if project_token is not None:
        env['GH_TOKEN'] = project_token
    return json.loads(subprocess.check_output(['gh', *args], text=True, env=env))


def project_read_authority(root, repository, base):
    # Authentication is deliberately unwired in rust.yml. A human-configured
    # protected default-main reader surface is required for current loop proof.
    required = {'GITHUB_EVENT_NAME': 'workflow_dispatch', 'INTEGRATION_MODE': 'integration_revalidation',
        'GITHUB_REF': 'refs/heads/main', 'GITHUB_WORKFLOW_SHA': base,
        'GITHUB_WORKFLOW_REF': repository + '/.github/workflows/rust.yml@refs/heads/main'}
    if any(os.environ.get(key) != value for key, value in required.items()):
        raise ValueError('Project-read setup: trusted default-B integration context required')
    run_id = os.environ.get('GITHUB_RUN_ID', '')
    if not re.fullmatch(r'[1-9][0-9]*', run_id):
        raise ValueError('Project-read setup: immutable hosted run identity unavailable')
    run = github('api', f'repos/{repository}/actions/runs/{run_id}')
    if (run.get('event') != 'workflow_dispatch' or run.get('head_branch') != 'main'
            or run.get('head_sha') != base or run.get('path') != '.github/workflows/rust.yml'):
        raise ValueError('Project-read setup: live run/default-B provenance differs')
    repo = github('api', f'repos/{repository}')
    ref = github('api', f'repos/{repository}/git/ref/heads/main')
    if repo.get('default_branch') != 'main' or ref.get('object', {}).get('sha') != base:
        raise ValueError('Project-read setup: live current default-B identity differs')
    surface = 'oasis7-project-read'
    environment = github('api', f'repos/{repository}/environments/{surface}')
    restrictions = environment.get('deployment_branch_policy') or {}
    policies = github('api', f'repos/{repository}/environments/{surface}/deployment-branch-policies')
    rules = policies.get('branch_policies')
    if (restrictions.get('custom_branch_policies') is not True
            or restrictions.get('protected_branches') is not False
            or not isinstance(rules, list) or len(rules) != 1
            or rules[0].get('name') != 'main' or rules[0].get('type') != 'branch'):
        raise ValueError('Project-read setup: protected default-main-only reader surface required')
    # Never inspect/use the broader credential until all provenance guards pass.
    token = os.environ.get('OASIS7_PROJECT_READ_TOKEN')
    if not token:
        raise ValueError('Project-read setup: approved environment-scoped read credential missing; repository token cannot read Projects')
    return token


def raw_task(root, repository, uid, base):
    path = root / '.pm/github-project-sync/tasks.json'
    mapping = json.loads(path.read_text()) if path.exists() else {'version': 1, 'tasks': {}}
    cached = mapping.get('tasks', {}).get(uid, {})
    if cached and cached.get('repository', repository) != repository:
        raise ValueError('cached Task repository differs')
    hits = github('issue', 'list', '-R', repository, '--state', 'all', '--search',
                  uid + ' in:body', '--json', 'number', '--limit', '5')
    if not isinstance(hits, list) or len(hits) >= 5:
        raise ValueError('canonical Task discovery incomplete')
    matches = []
    for hit in hits:
        issue = github('api', f"repos/{repository}/issues/{hit['number']}")
        body = str(issue.get('body') or '').replace('\r\n', '\n')
        if re.findall(r'^task_uid:[^\n]*$', body, re.M) == ['task_uid: ' + uid]:
            expected = f"https://github.com/{repository}/issues/{hit['number']}"
            if issue.get('number') != hit['number'] or issue.get('html_url') != expected:
                raise ValueError('canonical Issue identity differs')
            matches.append((issue, body))
    if len(matches) != 1:
        raise ValueError('canonical Task Issue missing or ambiguous')
    issue, body = matches[0]
    if cached and (cached.get('issue_number') != issue['number'] or cached.get('issue_url') != issue['html_url']):
        raise ValueError('cached Task Issue differs')
    # Parse fields without resolving effective primary. Only B siblings are on
    # this explicit import path; isolated Python excludes candidate directories.
    for name in ('workflow-durable-store', 'task_primary_package', 'task_complete_claim', 'loop_leaf_result'):
        path = Path(__file__).resolve().parent / (name + '.py')
        if path.is_symlink() or not path.is_file():
            raise ValueError('trusted bootstrap dependency unavailable: ' + name)
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    parser = sibling('github-project-task')
    fields = parser.issue_task_fields(body)
    fields.update(parser.strict_issue_scalar_fields(body, ('owner_role',)))
    task = {**cached, **fields, 'task_uid': uid, 'repository': repository,
            'issue_number': issue['number'], 'issue_url': issue['html_url']}
    gate = sibling('loop_gate')
    live = gate.live_binding(task)
    if cached.get('loop_binding') is not None and cached['loop_binding'] != live:
        raise ValueError('cached/live loop binding differs')
    if live != task.get('loop_binding'):
        raise ValueError('canonical/live loop binding differs')
    primary = sibling('task_primary_package')
    raw_record = fields.get(primary.FIELD)
    raw_payload = raw_record.get('payload') if isinstance(raw_record, dict) else None
    raw_proof = raw_payload.get('scope_evidence') if isinstance(raw_payload, dict) else None
    raw_proof = raw_proof if isinstance(raw_proof, dict) else {}
    if raw_proof.get('schema') == primary.LOOP_PROOF_SCHEMA and any(
            not cached.get(key) for key in ('canonical_worktree', 'task_branch')):
        raise ValueError('Project-read setup: canonical raw Task worktree/branch identity unavailable; trusted materialization required, never derive from completion')
    if raw_record is not None and raw_proof.get('schema') != primary.LOOP_PROOF_SCHEMA and any(
            not task.get(key) for key in ('canonical_worktree', 'task_branch', 'project_item_id')):
        raise ValueError('Project-read setup: legacy completion independent canonical Task identity unavailable; trusted materialization required, never derive from completion')
    completion = primary.validate_completion(task)
    proof = (completion or {}).get('payload', {}).get('scope_evidence', {})
    if proof.get('schema') != primary.LOOP_PROOF_SCHEMA:
        # Prove any historical completion through its unchanged current reader;
        # no record means no approval is being claimed by this resource bypass.
        if completion is not None:
            primary.validate_current_completion(root, task)
        return None, task
    project_token = project_read_authority(root, repository, base)
    owner, name = repository.split('/')
    project = github('project', 'view', '1', '--owner', owner, '--format', 'json', project_token=project_token)
    project_id = project.get('id')
    if not project_id or (mapping.get('project', {}).get('id') not in (None, '', project_id)):
        raise ValueError('canonical live Project identity differs')
    query = '''query($owner:String!,$name:String!,$number:Int!) {
      repository(owner:$owner,name:$name) { issue(number:$number) { number url body
        projectItems(first:100) { pageInfo { hasNextPage } nodes { id project { id number }
          fieldValues(first:100) { pageInfo { hasNextPage } nodes {
            ... on ProjectV2ItemFieldTextValue { text field { ... on ProjectV2FieldCommon { name } } }
            ... on ProjectV2ItemFieldSingleSelectValue { name field { ... on ProjectV2FieldCommon { name } } }
          } }
        } }
      } }
    }'''
    response = github('api', 'graphql', '-f', 'query=' + query, '-F', 'owner=' + owner,
                      '-F', 'name=' + name, '-F', 'number=' + str(issue['number']), project_token=project_token)
    if response.get('errors'):
        raise ValueError('canonical live Project query failed')
    selected = response.get('data', {}).get('repository', {}).get('issue', {})
    if (selected.get('number') != issue['number'] or selected.get('url') != issue['html_url']
            or selected.get('body') != issue.get('body')):
        raise ValueError('canonical Issue/Project query identity differs')
    items = selected.get('projectItems', {})
    if items.get('pageInfo', {}).get('hasNextPage') is not False:
        raise ValueError('canonical Project membership query incomplete')
    matches = [item for item in items.get('nodes', []) if item.get('project', {}).get('id') == project_id
               and item.get('project', {}).get('number') == 1]
    if len(matches) != 1 or not matches[0].get('id'):
        raise ValueError('canonical Project-backed Task missing or ambiguous')
    item = matches[0]
    if cached.get('project_item_id') not in (None, '', item['id']):
        raise ValueError('cached/live Project item identity differs')
    values = item.get('fieldValues', {})
    if values.get('pageInfo', {}).get('hasNextPage') is not False:
        raise ValueError('canonical Project fields query incomplete')
    project_fields = {}
    for value in values.get('nodes', []):
        key = value.get('field', {}).get('name')
        if key:
            if key in project_fields:
                raise ValueError('canonical Project field duplicated')
            project_fields[key] = value.get('name') or value.get('text')
    for key, expected in (('Task UID', uid), ('Owner Role', fields.get('owner_role')),
                          ('Canonical Worktree', cached.get('canonical_worktree'))):
        if not expected or project_fields.get(key) != expected:
            raise ValueError('canonical Task/Project identity mismatch: ' + key)
    if fields.get('worktree_hint') != cached.get('canonical_worktree'):
        raise ValueError('canonical Issue/Task worktree identity differs')
    task = {**cached, **fields, 'task_uid': uid, 'repository': repository,
            'issue_number': issue['number'], 'issue_url': issue['html_url'], 'project_item_id': item['id']}
    binding = task.get('loop_binding')
    gate = sibling('loop_gate')
    live = gate.live_binding(task)
    if cached.get('loop_binding') is not None and cached['loop_binding'] != live:
        raise ValueError('cached/live loop binding differs')
    if live != binding:
        raise ValueError('canonical/live loop binding differs')
    if binding is not None:
        # Absence means original epoch 1, as in canonical bootstrap snapshots;
        # never copy a nested binding value into the identity it must satisfy.
        task.setdefault('bootstrap_epoch', 1)
        for key in ('task_uid', 'owner_role', 'bootstrap_epoch'):
            if task.get(key) != binding.get(key):
                raise ValueError('loop/task identity mismatch: ' + key)
        for key, expected in (('Loop', binding.get('loop')), ('Change ID', binding.get('change_id'))):
            if project_fields.get(key) != expected:
                raise ValueError('canonical loop/Project identity mismatch: ' + key)
    mapping['project'] = {**mapping.get('project', {}), 'id': project_id, 'repo': repository}
    return mapping, task


def prepare(root, repository, uid, destination, base):
    mapping, task = raw_task(root, repository, uid, base)
    if mapping is None:
        return ''
    binding = task.get('loop_binding')
    if binding is None:
        tool = ''
    else:
        commit = binding.get('policy_commit', '')
        if not isinstance(commit, str) or not re.fullmatch(r'[0-9a-f]{40}', commit):
            raise ValueError('immutable policy commit missing')
        subprocess.run(['git', '-C', str(root), 'fetch', '--no-tags', 'origin',
                        'main:refs/remotes/origin/main'], check=True, capture_output=True)
        git(root, 'merge-base', '--is-ancestor', commit, 'refs/remotes/origin/main')
        if destination.exists():
            raise ValueError('trusted policy worktree destination already exists')
        subprocess.run(['git', '-C', str(root), 'worktree', 'add', '--detach', str(destination), commit],
                       check=True, capture_output=True)
        try:
            sibling('loop_recovery')
            loop = sibling('loop')
            policy = loop._trusted_module(destination, root, binding, 'loop_policy')
            blockers = policy.validate_binding(binding)['blockers']
            blockers += policy.validate_tool_root(destination, root, binding)['blockers']
            contracts = loop._trusted_module(destination, root, binding, 'loop_contracts')
            blockers += contracts.validate_contracts(destination, root, binding, purpose='in_flight')['blockers']
            if blockers:
                raise ValueError('trusted loop authority rejected: ' + '; '.join(blockers))
            # Re-read after resource setup, before exposing any effective input.
            if sibling('loop_gate').live_binding(task) != binding:
                raise ValueError('live loop binding moved during bootstrap')
        except BaseException:
            subprocess.run(['git', '-C', str(root), 'worktree', 'remove', '--force', str(destination)],
                           capture_output=True)
            raise
        tool = str(destination.resolve())
    mapping.setdefault('tasks', {})[uid] = task
    path = root / '.pm/github-project-sync/tasks.json'
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(mapping, sort_keys=True) + '\n')
    return tool


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--repo-root', type=Path, required=True)
    parser.add_argument('--repository', required=True)
    parser.add_argument('--task-uid', required=True)
    parser.add_argument('--destination', type=Path, required=True)
    parser.add_argument('--authority-base', required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'task_[0-9a-f]{32}', args.task_uid):
        raise ValueError('canonical Task UID invalid')
    print(prepare(args.repo_root.resolve(), args.repository, args.task_uid, args.destination.resolve(), args.authority_base))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as exc:
        print('prepare-loop-ci-authority: ' + str(exc), file=sys.stderr)
        raise SystemExit(1)
