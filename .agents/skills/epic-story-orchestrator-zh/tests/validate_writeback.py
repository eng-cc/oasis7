#!/usr/bin/env python3
import json
from pathlib import Path

p = Path('.agents/skills/epic-story-orchestrator-zh/tests/fixtures/writeback.sample.json')
data = json.loads(p.read_text(encoding='utf-8'))

required = ['story_slug', 'task_mode', 'canon_scope', 'writeback_targets', 'artifact_refs', 'consistency']
missing = [k for k in required if k not in data]
if missing:
    raise SystemExit(f'missing required keys: {missing}')

if not isinstance(data['writeback_targets'], list) or not data['writeback_targets']:
    raise SystemExit('writeback_targets must be a non-empty list')

for i, target in enumerate(data['writeback_targets']):
    if 'path' not in target or 'mode' not in target:
        raise SystemExit(f'writeback_targets[{i}] missing path/mode')
    if target['mode'] not in {'overwrite', 'append', 'create'}:
        raise SystemExit(f'writeback_targets[{i}] invalid mode: {target["mode"]}')

for key in ['world_rules', 'characters', 'timeline', 'plot_beats']:
    if key not in data['artifact_refs'] or not isinstance(data['artifact_refs'][key], list):
        raise SystemExit(f'artifact_refs.{key} must be list')

if data.get('consistency', {}).get('status') not in {'pass', 'fail'}:
    raise SystemExit('consistency.status must be pass/fail')

print('validate_writeback: OK')
