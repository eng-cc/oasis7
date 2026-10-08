#!/usr/bin/env python3
"""Validate selection and actual Actions results; every expected result is explicit."""
import argparse
import json
from pathlib import Path


def validate(plan, needs, group_jobs):
    if needs.get('select', {}).get('result') != 'success':
        raise ValueError('select did not succeed')
    if not isinstance(plan, dict) or set(plan.get('groups', [])) - set(group_jobs):
        raise ValueError('invalid selected groups')
    chosen = plan.get('groups')
    if not isinstance(chosen, list) or len(chosen) != len(set(chosen)) or 'baseline' not in chosen:
        raise ValueError('missing baseline or duplicate groups')
    matrices = plan.get('matrices')
    if not isinstance(matrices, dict) or set(matrices) != set(group_jobs):
        raise ValueError('incomplete matrix inventory')
    for group, job in group_jobs.items():
        expected = [{'group': group}] if group in chosen else []
        if matrices[group] != {'include': expected}:
            raise ValueError('invalid or empty selected matrix: ' + group)
        result = needs.get(job, {}).get('result')
        allowed = {'success'} if group in chosen else {'skipped', 'success'}
        if result not in allowed:
            raise ValueError(f'{group}: unexpected result {result!r}')
    expected_jobs = {'select', *group_jobs.values()}
    for job, entry in needs.items():
        if job not in expected_jobs:
            if not isinstance(entry, dict) or entry.get('result') != 'success':
                raise ValueError(f'{job}: extra dependency did not explicitly succeed')
    return True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', required=True)
    parser.add_argument('--needs', required=True)
    parser.add_argument('--config', default=str(Path(__file__).with_name('ci-required-scope.json')))
    args = parser.parse_args()
    config = json.loads(Path(args.config).read_text())
    jobs = {group: group.replace('_', '-') for group in config['groups']}
    validate(json.loads(args.plan), json.loads(args.needs), jobs)
    print('All selected CI groups succeeded.')


if __name__ == '__main__':
    main()
