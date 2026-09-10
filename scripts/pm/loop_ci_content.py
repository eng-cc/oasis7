"""Hosted repository-only checks; full live admission remains a local gh gate."""
import json
import subprocess

from loop_contracts import MARKER, REPOSITORY, contract_digest, validate_contract_record
from loop_policy import scope_context, validate_binding, validate_scope, validate_tool_root


def repository_json(repository, path):
    return json.loads(subprocess.check_output(['gh', 'api', f'repos/{repository}/{path}'], text=True))


def validate_ci_content(tool_root, root, binding, base, head, repository, reader=None):
    blockers = []
    read = reader or repository_json
    context = {}
    try:
        context = scope_context(root, base, head)
        for result in (validate_binding(binding), validate_tool_root(tool_root, root, binding), validate_scope(tool_root, root, binding, context['scope_base_oid'], head)):
            blockers.extend(result['blockers'])
    except (ValueError, OSError) as exc:
        blockers.append(str(exc))
    active, visited = set(), set()
    revisions = {}

    def inspect(reference):
        ref = reference['publication_ref']
        key = (reference['contract_id'], reference['revision'])
        if key in active:
            raise ValueError('cyclic immutable contract references')
        comment = read(repository, f"issues/comments/{ref['comment_id']}")
        if comment.get('id') != ref['comment_id'] or comment.get('issue_url') != f"https://api.github.com/repos/{repository}/issues/{ref['issue_number']}":
            raise ValueError('published contract repository identity mismatch')
        payload = json.loads(comment.get('body') or '')
        contract = payload['contract']
        if payload.get('marker') != MARKER or contract.get('contract_id') != key[0] or contract.get('revision') != key[1]:
            raise ValueError('immutable publication identity mismatch')
        if reference['contract_digest'] != contract_digest(contract) or payload.get('contract_digest') != reference['contract_digest']:
            raise ValueError('immutable published contract digest mismatch')
        if revisions.setdefault(key, reference['contract_digest']) != reference['contract_digest']:
            raise ValueError('conflicting immutable contract revision')
        pr = read(repository, f"pulls/{contract['approval_ref']['pr_number']}")
        if (pr.get('base', {}).get('repo') or {}).get('full_name') != repository:
            raise ValueError('contract approval repository mismatch')
        blockers.extend(validate_contract_record(contract, root, {'number': pr.get('number'), 'merged': pr.get('merged'), 'head': pr.get('head', {}).get('sha'), 'merge_commit': pr.get('merge_commit_sha')}))
        clauses = {clause for item in contract.get('content_refs', []) for clause in item.get('clauses', [])}
        if not reference.get('consumed_clauses') or any(clause not in clauses for clause in reference['consumed_clauses']):
            raise ValueError('unapproved consumed contract clause')
        if binding['target_delivery'] not in contract.get('scope', []):
            raise ValueError('contract content does not cover target delivery')
        if key not in visited:
            active.add(key)
            for upstream in contract['upstream_contracts']:
                inspect(upstream)
            active.remove(key)
            visited.add(key)

    try:
        if repository != REPOSITORY:
            raise ValueError('unsupported canonical repository')
        if not blockers:
            for reference in binding['input_contracts']:
                inspect(reference)
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as exc:
        blockers.append(str(exc))
    return {'status': 'blocked' if blockers else 'passed', 'blockers': blockers,
            'scope_context': context,
            'verification_boundary': 'repository_identity_policy_scope_contract_content',
            'local_live_admission_required': True,
            'not_verified_here': ['Project terminal truth', 'dependency completion', 'contract eligibility and admin provenance', 'merge hold and authorization']}
