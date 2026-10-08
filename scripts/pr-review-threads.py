#!/usr/bin/env python3
"""Optionally inspect or explicitly resolve native GitHub review threads."""
import argparse
import json
import subprocess

def gh(*args):
    result = subprocess.run(['gh', *args], text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(result.stderr.strip() or 'GitHub query failed')
    value = json.loads(result.stdout)
    if isinstance(value, dict) and value.get('errors'):
        raise RuntimeError(str(value['errors']))
    return value

def read_threads(owner, repo, number):
    threads, cursor = [], None
    while True:
        query = 'query($owner:String!,$repo:String!,$number:Int!,$cursor:String){repository(owner:$owner,name:$repo){pullRequest(number:$number){reviewThreads(first:100,after:$cursor){nodes{id isResolved isOutdated path line} pageInfo{hasNextPage endCursor}}}}}'
        args = ['api', 'graphql', '-f', 'query=' + query, '-f', 'owner=' + owner, '-f', 'repo=' + repo, '-F', 'number=' + str(number)]
        if cursor:
            args += ['-f', 'cursor=' + cursor]
        data = gh(*args)['data']['repository']['pullRequest']['reviewThreads']
        threads.extend(data['nodes'])
        page = data['pageInfo']
        if not page['hasNextPage']:
            return threads
        if not page['endCursor'] or page['endCursor'] == cursor:
            raise RuntimeError('invalid review-thread pagination')
        cursor = page['endCursor']

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('pr', nargs='?')
    parser.add_argument('--resolve-thread', action='append', default=[])
    parser.add_argument('--resolve-all-unresolved', action='store_true')
    parser.add_argument('--unresolved-only', action='store_true')
    parser.add_argument('--summary', action='store_true')
    parser.add_argument('--json', action='store_true')
    args = parser.parse_args()
    if (args.resolve_all_unresolved or args.resolve_thread) and not args.pr:
        parser.error('resolving review threads requires an explicit PR number or URL')
    if args.resolve_all_unresolved and args.resolve_thread:
        parser.error('choose explicit threads or all unresolved')
    try:
        command = ['pr', 'view'] + ([args.pr] if args.pr else []) + ['--json', 'number,url']
        pr = gh(*command)
        owner, repo = pr['url'].split('/')[3:5]
        threads = read_threads(owner, repo, pr['number'])
        ids = [t['id'] for t in threads if not t['isResolved']] if args.resolve_all_unresolved else args.resolve_thread
        known = {t['id'] for t in threads}
        if set(ids) - known:
            raise RuntimeError('requested thread does not belong to this PR')
        for thread_id in ids:
            gh('api', 'graphql', '-f', 'query=mutation($id:ID!){resolveReviewThread(input:{threadId:$id}){thread{id isResolved}}}', '-f', 'id=' + thread_id)
        if ids:
            threads = read_threads(owner, repo, pr['number'])
            resolved = {t['id'] for t in threads if t['isResolved']}
            if set(ids) - resolved:
                raise RuntimeError('review thread resolution could not be verified')
        total, unresolved = len(threads), sum(not t['isResolved'] for t in threads)
        payload = {'pr': pr, 'summary': {'total_threads': total, 'unresolved_threads': unresolved}, 'threads': [t for t in threads if not args.unresolved_only or not t['isResolved']], 'resolved_now': ids}
        print(json.dumps(payload, indent=2))
    except (RuntimeError, KeyError, TypeError, ValueError) as exc:
        parser.exit(1, str(exc) + '\n')

if __name__ == '__main__':
    main()
