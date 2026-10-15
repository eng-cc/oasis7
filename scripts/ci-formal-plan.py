#!/usr/bin/env python3
"""Conservative transition from an event-base selector, without rewriting its plan."""
import argparse
import json
from pathlib import Path

CORE = {'rust_baseline', 'oasis7_required', 'consensus', 'distfs', 'node', 'net'}


def selected(plan, known_groups):
    if 'formal_process' in known_groups:
        return 'formal_process' in plan['groups']
    return plan['scope'] == 'full' or bool(CORE.intersection(plan['groups']))


def aggregate(selected_value, needs):
    if needs.get('select', {}).get('result') != 'success':
        raise ValueError('formal selection failed')
    required = {'oasis7-test-build': 'success', 'formal-process-tests': 'success'} if selected_value else {'formal-process-tests': 'skipped'}
    for job, expected in required.items():
        if needs.get(job, {}).get('result') != expected:
            raise ValueError(f'{job}: expected {expected}')
    return 'passed' if selected_value else 'not_selected'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path)
    parser.add_argument('--config', type=Path)
    parser.add_argument('--github-output', type=Path)
    parser.add_argument('--needs')
    parser.add_argument('--selected', choices=['true', 'false'])
    args = parser.parse_args()
    if args.needs is not None:
        if args.selected is None:
            raise ValueError('missing selection')
        print(aggregate(args.selected == 'true', json.loads(args.needs)))
    else:
        value = selected(json.loads(args.plan.read_text()), json.loads(args.config.read_text())['groups'])
        with args.github_output.open('a') as stream:
            stream.write('run_formal_process=' + str(value).lower() + '\n')


if __name__ == '__main__':
    main()
