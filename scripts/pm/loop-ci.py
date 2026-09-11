#!/usr/bin/env python3
"""Independent PR loop gate: read live task, execute only effective helpers."""
import argparse
import base64
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile


def run(*args):
    return subprocess.check_output(args, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo-root', type=Path, default=Path.cwd())
    parser.add_argument('--repository', required=True)
    parser.add_argument('--pr-number', required=True, type=int)
    parser.add_argument('--base', required=True)
    parser.add_argument('--head', required=True)
    args = parser.parse_args()
    try:
        pr = json.loads(run('gh', 'api', f'repos/{args.repository}/pulls/{args.pr_number}'))
        if pr['head']['sha'] != args.head: raise ValueError('live PR HEAD differs from CI source HEAD')
        uids = set(re.findall(r'task_[0-9a-f]{32}', pr.get('body') or ''))
        candidate_items = {}
        if len(uids) == 1:
            uid = next(iter(uids))
            refs = set(re.findall(r'(?:Refs|Fixes|Closes)\s+#(\d+)', pr.get('body') or '', re.I))
            if len(refs) > 20: raise ValueError('task reference discovery budget exhausted')
            if refs:
                candidates = sorted(refs)
            else:
                hits = json.loads(run('gh', 'issue', 'list', '-R', args.repository, '--state', 'all', '--search', uid + ' in:body', '--json', 'number', '--limit', '5'))
                if not isinstance(hits, list) or len(hits) >= 5: raise ValueError('task search discovery incomplete')
                candidates = [hit['number'] for hit in hits]
            matches = []
            for candidate in candidates:
                item = json.loads(run('gh', 'api', f'repos/{args.repository}/issues/{candidate}'))
                candidate_items[candidate] = item
                candidate_body = (item.get('body') or '').replace('\r\n', '\n')
                candidate_fields = re.findall(r'^task_uid:[^\n]*$', candidate_body, re.MULTILINE)
                candidate_uids = re.findall(r'^task_uid: (task_[0-9a-f]{32})$', candidate_body, re.MULTILINE)
                if uid in candidate_uids:
                    if candidate_fields != ['task_uid: ' + uid] or candidate_uids != [uid]: raise ValueError('ambiguous canonical Issue UID')
                    matches.append(candidate)
            if len(matches) != 1: raise ValueError('live task Issue is ambiguous or missing')
            number = matches[0]
        else:
            refs = set(re.findall(r'(?:Refs|Fixes|Closes)\s+#(\d+)', pr.get('body') or '', re.I))
            if uids or len(refs) != 1: raise ValueError('PR must identify exactly one canonical task Issue')
            number, uid = int(next(iter(refs))), None
        candidate_issue = candidate_items.get(number) if uids else None
        issue = (candidate_issue
                 if isinstance(candidate_issue, dict)
                 and 'loop_binding_b64:' in (candidate_issue.get('body') or '')
                 else None)
        if issue is None:
            issue = json.loads(run('gh', 'api', f'repos/{args.repository}/issues/{number}'))
        body = (issue.get('body') or '').replace('\r\n', '\n')
        issue_fields = re.findall(r'^task_uid:[^\n]*$', body, re.MULTILINE)
        issue_uids = re.findall(r'^task_uid: (task_[0-9a-f]{32})$', body, re.MULTILINE)
        if len(issue_fields) != 1 or len(issue_uids) != 1: raise ValueError('Issue UID missing or ambiguous')
        if uid is None:
            if len(issue_uids) != 1: raise ValueError('Issue UID missing')
            uid = issue_uids[0]
        if issue_uids != [uid]: raise ValueError('Issue UID mismatch')
        bound_pr = re.findall(r'^- pr_number: `([0-9]+)`$', body, re.MULTILINE)
        if bound_pr != [str(args.pr_number)]:
            raise ValueError('live task Issue does not bind this PR number; refresh task PR identity')
        if 'loop_binding_b64:' not in body:
            pages = json.loads(run('gh', 'api', f'repos/{args.repository}/issues/{number}/comments', '--paginate', '--slurp'))
            if not isinstance(pages, list): raise ValueError('lineage readback unavailable')
            comments = [item for page in pages for item in (page if isinstance(page, list) else [page])]
            if any('oasis7-loop-binding-history' in str(item.get('body', '')) for item in comments if isinstance(item, dict)):
                raise ValueError('loop binding deleted after immutable history')
            print(json.dumps({'status': 'legacy', 'task_uid': uid}))
            return 0
        matches = re.findall(r'^- loop_binding_b64: `([^`]+)`$', body, re.MULTILINE)
        if len(matches) != 1: raise ValueError('malformed live binding')
        binding = json.loads(base64.b64decode(matches[0] + '=' * (-len(matches[0]) % 4), altchars=b'-_', validate=True))
        if binding.get('task_uid') != uid: raise ValueError('loop task UID mismatch')
        commit = binding.get('policy_commit', '')
        if not re.fullmatch(r'[0-9a-f]{40}', commit): raise ValueError('missing immutable effective policy')
        subprocess.run(['git', '-C', str(args.repo_root), 'fetch', '--no-tags', 'origin', 'main:refs/remotes/origin/main'], check=True, capture_output=True)
        subprocess.run(['git', '-C', str(args.repo_root), 'merge-base', '--is-ancestor', commit, 'refs/remotes/origin/main'], check=True)
        with tempfile.TemporaryDirectory(prefix='oasis7-loop-tools-') as tmp:
            tool_root = Path(tmp) / 'tools'
            subprocess.run(['git', '-C', str(args.repo_root), 'worktree', 'add', '--detach', str(tool_root), commit], check=True, capture_output=True)
            try:
                # All Python code below is loaded from the verified effective commit.
                code = r'''
import json
import sys
from pathlib import Path

from loop_ci_content import validate_ci_content
from loop_traceability import GitHubAuthorityReader, _validate_authority_record


def coordinating_record(root, binding, repository):
    reference = binding.get("coordination_ref")
    if reference is None:
        return None
    if not isinstance(reference, dict):
        raise ValueError("bound coordinating reference is not an object")
    readback = GitHubAuthorityReader(root, repository)(reference)
    comment = readback.get("comment")
    try:
        payload = json.loads(comment.get("body") or "") if isinstance(comment, dict) else None
    except (TypeError, json.JSONDecodeError) as exc:
        raise ValueError("coordinating authority body is not valid JSON") from exc
    record = payload.get("record") if isinstance(payload, dict) else None
    if not isinstance(record, dict):
        raise ValueError("coordinating authority comment has no usable record projection")
    blockers = _validate_authority_record(readback, record, binding)
    if blockers:
        raise ValueError("coordinating authority readback blocked: " + "; ".join(blockers))
    return record


t, r, b, base, head, repo = sys.argv[1:]
try:
    binding = json.loads(b)
    record = coordinating_record(Path(r), binding, repo)
    result = validate_ci_content(
        Path(t), Path(r), binding, base, head, repo, record=record,
    )
except (OSError, ValueError, KeyError, TypeError) as exc:
    result = {"status": "blocked", "blockers": [str(exc)]}
print(json.dumps(result))
sys.exit(0 if result["status"] == "passed" else 2)
'''
                result = subprocess.run([sys.executable, '-c', code, str(tool_root), str(args.repo_root.resolve()), json.dumps(binding), args.base, args.head, args.repository], cwd=tool_root / 'scripts/pm')
                return result.returncode
            finally:
                subprocess.run(['git', '-C', str(args.repo_root), 'worktree', 'remove', str(tool_root)], check=True, capture_output=True)
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as exc:
        print(json.dumps({'status': 'blocked', 'blockers': [str(exc)]}))
        return 2


if __name__ == '__main__': raise SystemExit(main())
