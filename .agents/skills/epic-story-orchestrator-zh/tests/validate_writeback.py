#!/usr/bin/env python3
import json
from pathlib import Path
import re

p = Path('.agents/skills/epic-story-orchestrator-zh/tests/fixtures/writeback.sample.json')
data = json.loads(p.read_text(encoding='utf-8'))

required = [
    'story_slug',
    'task_mode',
    'canon_scope',
    'gameplay_canon_binding',
    'writeback_targets',
    'artifact_refs',
    'consistency',
]
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

for key in ['world_rules', 'characters', 'timeline', 'plot_beats', 'gameplay_prds']:
    if key not in data['artifact_refs'] or not isinstance(data['artifact_refs'][key], list):
        raise SystemExit(f'artifact_refs.{key} must be list')

binding = data['gameplay_canon_binding']
binding_required = [
    'gameplay_prd_refs',
    'player_leverage',
    'world_change_due_to_player',
    'control_feeling_guarantee',
    'small_player_lane_stage',
    'release_claim_boundary',
]
binding_missing = [k for k in binding_required if k not in binding]
if binding_missing:
    raise SystemExit(f'gameplay_canon_binding missing required keys: {binding_missing}')

if not isinstance(binding['gameplay_prd_refs'], list) or not binding['gameplay_prd_refs']:
    raise SystemExit('gameplay_canon_binding.gameplay_prd_refs must be a non-empty list')

story_slug = data['story_slug']
if not isinstance(story_slug, str) or not re.fullmatch(r'[a-z0-9]+(?:-[a-z0-9]+)*', story_slug):
    raise SystemExit(f'invalid story_slug: {story_slug}')

canonical_root = f'doc/game/lore/{story_slug}/'
append_only_paths = {f'{canonical_root}canon-log.md'}

for i, target in enumerate(data['writeback_targets']):
    path = target['path']
    if not isinstance(path, str) or path.startswith('/') or '..' in Path(path).parts:
        raise SystemExit(f'writeback_targets[{i}] invalid path: {path}')
    if not path.startswith(canonical_root):
        raise SystemExit(
            f'writeback_targets[{i}] path must stay under {canonical_root}: {path}'
        )
    if target['mode'] == 'append' and path not in append_only_paths:
        raise SystemExit(f'writeback_targets[{i}] append only allowed for canon-log.md: {path}')
    if path in append_only_paths and target['mode'] != 'append':
        raise SystemExit(f'writeback_targets[{i}] canon-log.md must use append mode: {path}')

for ref in binding['gameplay_prd_refs']:
    if not isinstance(ref, str) or not ref.startswith('PRD-GAME-'):
        raise SystemExit(f'invalid gameplay PRD ref: {ref}')

allowed_lane_stages = {'none', 'local_operator', 'regional_specialist', 'limited_scope_regional_influence'}
if binding['small_player_lane_stage'] not in allowed_lane_stages:
    raise SystemExit(f'invalid small_player_lane_stage: {binding["small_player_lane_stage"]}')

boundary = binding['release_claim_boundary'].lower()
blocked_claims = ['closed beta', 'play now', 'live now']
if any(claim in boundary for claim in blocked_claims):
    raise SystemExit(f'release_claim_boundary contains blocked claim: {binding["release_claim_boundary"]}')

if data.get('consistency', {}).get('status') not in {'pass', 'fail'}:
    raise SystemExit('consistency.status must be pass/fail')

print('validate_writeback: OK')
