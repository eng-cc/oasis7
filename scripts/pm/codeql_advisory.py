"""Bounded exact-source CodeQL explanation, never a check-success override.

Platform code-scanning checks deliberately remain unknown until hosted evidence
establishes their provider/analysis association. No numeric provider IDs are pins.
"""
from __future__ import annotations
import re

SUCCESS = {'SUCCESS', 'NEUTRAL', 'SKIPPED'}
WORKFLOW_PATH = '.github/workflows/codeql.yml'
MAX_ANOMALIES = 12
JOB_PATTERN = re.compile(r'CodeQL / (?:plan|summary|(?:actions|python|javascript|rust)-repo / (?:default|extended))\Z')


def identity(check):
    app = check.get('app_id')
    if app is None:
        app = ((check.get('checkSuite') or {}).get('app') or {}).get('databaseId')
    return str(check.get('name') or check.get('context') or ''), app


def state(check):
    return str(check.get('conclusion') or check.get('state') or check.get('status') or '').upper()


def _complete(payload, key):
    rows = payload.get(key)
    if (not isinstance(rows, list) or type(payload.get('total_count')) is not int
            or payload['total_count'] != len(rows) or len(rows) > 100):
        raise ValueError('bounded provenance readback incomplete: ' + key)
    return rows


def explain_unstable(data, read):
    """Read only scoped anomalous identities, caching each endpoint once.

    ``read`` is a trusted provider API callback, not candidate check metadata.
    Any ambiguity or failure prevents the entire UNSTABLE explanation.
    """
    proof = {'explained': False, 'checks': [], 'reason': 'not an explanatory anomaly'}
    if str(data.get('mergeStateStatus') or '').upper() != 'UNSTABLE':
        return proof
    policy = data.get('policy_discovery') or {}
    required = policy.get('required_status_checks')
    if (policy.get('status') != 'resolved' or not isinstance(required, list)
            or 'code_scanning' in (policy.get('active_rule_types') or [])):
        proof['reason'] = 'live required/rules policy is incomplete or enforces code scanning'
        return proof
    checks = data.get('statusCheckRollup') or data.get('checks') or []
    for item in required:
        name = item.get('context') if isinstance(item, dict) else item
        app = item.get('app_id') if isinstance(item, dict) else None
        matches = [check for check in checks if identity(check)[0] == name
                   and (app is None or identity(check)[1] == app)]
        if len(matches) != 1 or state(matches[0]) not in SUCCESS:
            proof['reason'] = 'required check missing, ambiguous, pending or failed'
            return proof
    anomalies = [check for check in checks if state(check) not in SUCCESS]
    if (not anomalies or len(anomalies) > MAX_ANOMALIES
            or not any(state(check) in {'FAILURE', 'TIMED_OUT', 'CANCELLED', 'ACTION_REQUIRED', 'STARTUP_FAILURE'} for check in anomalies)):
        proof['reason'] = 'no anomalies or bounded anomaly budget exceeded'
        return proof
    names = [identity(check)[0] for check in checks]
    ids = [check.get('databaseId') for check in anomalies]
    if len(set(ids)) != len(ids) or any(names.count(identity(check)[0]) != 1 for check in anomalies):
        proof['reason'] = 'duplicate or conflicting check identity'
        return proof
    for check in anomalies:
        name, app = identity(check)
        if any((item.get('context') if isinstance(item, dict) else item) == name
               and (not isinstance(item, dict) or item.get('app_id') in (None, app)) for item in required):
            proof['reason'] = 'anomalous check is required'
            return proof
    cache = {}

    def get(endpoint):
        if endpoint not in cache:
            cache[endpoint] = read(endpoint)
        value = cache[endpoint]
        if not isinstance(value, dict):
            raise ValueError('malformed provenance readback')
        return value

    try:
        repo, sha, branch = data['repository'], data['headRefOid'], data['headRefName']
        if (not isinstance(repo, str) or not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', repo)
                or not isinstance(sha, str) or not re.fullmatch(r'[0-9a-f]{40}', sha) or not branch):
            raise ValueError('current PR source identity missing')
        provider = get('apps/github-actions')
        owner = provider.get('owner') or {}
        if (type(provider.get('id')) is not int or provider.get('slug') != 'github-actions'
                or owner.get('login') != 'github' or owner.get('type') != 'Organization'):
            raise ValueError('GitHub Actions provider identity unavailable')
        workflow = get(f'repos/{repo}/actions/workflows/codeql.yml')
        if (type(workflow.get('id')) is not int or workflow.get('path') != WORKFLOW_PATH
                or workflow.get('state') != 'active'):
            raise ValueError('trusted CodeQL workflow identity unavailable')
        runs = _complete(get(f'repos/{repo}/actions/workflows/{workflow["id"]}/runs?head_sha={sha}&event=pull_request&per_page=100'), 'workflow_runs')
        for check in anomalies:
            check_id = check.get('databaseId')
            name, app = identity(check)
            if type(check_id) is not int or type(app) is not int or app != provider['id'] or not JOB_PATTERN.fullmatch(name):
                raise ValueError('unknown check or platform provider; hosted association required')
            observed = get(f'repos/{repo}/check-runs/{check_id}')
            if (observed.get('id') != check_id or observed.get('name') != name
                    or observed.get('head_sha') != sha or (observed.get('app') or {}).get('id') != app
                    or state(observed) != state(check)):
                raise ValueError('check source/state changed during readback')
            suite = (observed.get('check_suite') or {}).get('id')
            matches = [run for run in runs if run.get('check_suite_id') == suite]
            if type(suite) is not int or len(matches) != 1:
                raise ValueError('Actions run association missing or conflicting')
            run = matches[0]
            if (run.get('workflow_id') != workflow['id'] or run.get('path') != WORKFLOW_PATH
                    or run.get('head_sha') != sha or run.get('head_branch') != branch
                    or run.get('event') != 'pull_request' or (run.get('repository') or {}).get('full_name') != repo
                    or not any(pr.get('number') == data.get('number') for pr in run.get('pull_requests') or [])
                    or type(run.get('id')) is not int or type(run.get('run_attempt')) is not int
                    or run['run_attempt'] < 1):
                raise ValueError('Actions run does not bind current PR SHA/ref/workflow')
            jobs = _complete(get(f'repos/{repo}/actions/runs/{run["id"]}/attempts/{run["run_attempt"]}/jobs?per_page=100'), 'jobs')
            url = f'https://api.github.com/repos/{repo}/check-runs/{check_id}'
            matching_jobs = [job for job in jobs if job.get('check_run_url') == url]
            if len(matching_jobs) != 1:
                raise ValueError('exact Actions job/check association missing or conflicting')
            job = matching_jobs[0]
            if (job.get('name') != name or job.get('run_id') != run['id']
                    or job.get('run_attempt') != run['run_attempt'] or job.get('head_sha') != sha
                    or state(job) != state(check) or type(job.get('id')) is not int):
                raise ValueError('Actions job source/attempt/state mismatch')
            proof['checks'].append({'check_id': check_id, 'app_id': app, 'workflow_path': WORKFLOW_PATH,
                                    'run_id': run['id'], 'run_attempt': run['run_attempt'], 'job_id': job['id'],
                                    'head_sha': sha, 'head_ref': branch, 'classification': 'advisory_codeql'})
        proof.update(explained=True, reason='all anomalous checks have exact advisory Actions provenance')
    except Exception as exc:
        proof.update(explained=False, reason='provenance capability blocked: ' + str(exc))
    return proof
