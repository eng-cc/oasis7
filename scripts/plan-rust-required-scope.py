#!/usr/bin/env python3
"""Select ordinary CI groups from Git data. No network, task state or candidate code execution."""
import argparse
import fnmatch
import json
import posixpath
import re
import subprocess
try:
    import tomllib
except ModuleNotFoundError:
    raise SystemExit("Python 3.11 or newer is required; use scripts/plan-rust-required-scope.sh to discover it")
from pathlib import Path

PLATFORM_GROUPS = ('windows_rollout', 'macos_package_contract', 'fleet_health')
CONTROL = ('Cargo.toml', 'Cargo.lock', '**/Cargo.toml', '**/Cargo.lock', '**/build.rs',
           '.cargo/**', 'rust-toolchain*', '.github/**', 'scripts/ci-*', 'scripts/build-*', '**/world/artifacts/**',
           'scripts/plan-rust-required-scope*', 'scripts/document_corpus.py')


def git(*args):
    return subprocess.check_output(['git', *args], stderr=subprocess.DEVNULL)


def changed_paths(base, head):
    """Both rename/copy endpoints; Git NUL records preserve whitespace and punctuation."""
    records = git('diff', '--name-status', '--find-renames', '-z', base, head).split(b'\0')
    if records.pop() != b'':
        raise ValueError('incomplete name-status diff')
    result = []
    while records:
        status = records.pop(0).decode('ascii')
        count = 2 if status.startswith(('R', 'C')) else 1
        if len(records) < count:
            raise ValueError('incomplete rename record')
        for _ in range(count):
            result.append(records.pop(0).decode('utf-8', 'surrogateescape'))
    return list(dict.fromkeys(result))


def cargo_graph(ref):
    """Resolve workspace path edges including dev/build/target and renamed dependencies.

    Optional dependencies are included conservatively, so no feature combination is lost.
    Unsupported/ambiguous workspace relationships widen selection rather than reject work.
    """
    records = git('ls-tree', '-r', '-z', ref).split(b'\0')
    entries = []
    tracked_paths = set()
    for record in records:
        if not record:
            continue
        identity, path = record.split(b'\t', 1)
        mode, kind, oid = identity.split()
        tracked_paths.add(path.decode('utf-8', 'surrogateescape'))
        if kind == b'blob' and (path.endswith(b'Cargo.toml') or path.endswith(b'.rs')):
            entries.append((oid, path.decode('utf-8', 'surrogateescape')))
    data = subprocess.check_output(['git', 'cat-file', '--batch'], input=b''.join(oid + b'\n' for oid, _ in entries))
    blobs = {}
    offset = 0
    for _, path in entries:
        end = data.index(b'\n', offset)
        size = int(data[offset:end].split()[2])
        offset = end + 1
        blobs[path] = data[offset:offset + size]
        offset += size + 1
    paths = list(blobs)
    root = tomllib.loads(blobs['Cargo.toml'].decode())
    workspace = root.get('workspace', {})
    members = workspace.get('members', [])
    excludes = workspace.get('exclude', [])
    manifests = {}
    for path in paths:
        if not path.endswith('Cargo.toml'):
            continue
        directory = posixpath.dirname(path)
        if path != 'Cargo.toml' and (not any(fnmatch.fnmatchcase(directory, m.rstrip('/')) for m in members)
                                  or any(fnmatch.fnmatchcase(directory, e.rstrip('/')) for e in excludes)):
            continue
        manifest = tomllib.loads(blobs[path].decode())
        package = manifest.get('package')
        if not package:
            continue
        name = package['name']
        if name in manifests:
            raise ValueError('duplicate package name')
        manifests[name] = (directory, manifest)
    if not manifests:
        raise ValueError('no workspace packages')
    by_directory = {directory: name for name, (directory, _) in manifests.items()}
    reverse = {name: set() for name in manifests}
    workspace_dependencies = workspace.get('dependencies', {})
    for name, (directory, manifest) in manifests.items():
        tables = [manifest, *manifest.get('target', {}).values()]
        for table in tables:
            for kind in ('dependencies', 'dev-dependencies', 'build-dependencies'):
                for alias, dependency in table.get(kind, {}).items():
                    if not isinstance(dependency, dict):
                        continue
                    inherited = dependency.get('workspace') is True
                    if inherited:
                        if alias not in workspace_dependencies:
                            raise ValueError('missing inherited dependency')
                        dependency = workspace_dependencies[alias]
                    if not isinstance(dependency, dict) or 'path' not in dependency:
                        continue
                    target_directory = posixpath.normpath(posixpath.join('' if inherited else directory, dependency['path']))
                    target = by_directory.get(target_directory)
                    if target is None:
                        # External local packages may be shared generators/consumers.
                        raise ValueError('path dependency outside resolved workspace')
                    if dependency.get('package', target) != target:
                        raise ValueError('path dependency identity mismatch')
                    reverse[target].add(name)
    # Embedded documents/schema are compilation inputs, even outside a crate.
    embedded = {}
    for path in paths:
        owners = [n for n, (d, _) in manifests.items()
                  if (d and path.startswith(d + '/')) or (not d and path.startswith(('src/', 'tests/', 'examples/', 'benches/')))]
        if not path.endswith('.rs') or not owners:
            continue
        source = blobs[path].decode('utf-8', 'replace')
        literal = list(re.finditer(r'include_(?:str|bytes)!\s*\(\s*"([^"\n]+)"\s*,?\s*\)', source))
        concatenated = list(re.finditer(r'include_(?:str|bytes)!\s*\(\s*concat!\s*\(\s*env!\s*\(\s*"CARGO_MANIFEST_DIR"\s*\)\s*,\s*"([^"\n]+)"\s*,?\s*\)\s*,?\s*\)', source))
        for match in literal:
            embedded.setdefault(posixpath.normpath(posixpath.join(posixpath.dirname(path), match.group(1))), set()).update(owners)
        for match in concatenated:
            for owner in owners:
                directory = manifests[owner][0]
                embedded.setdefault(posixpath.normpath(directory + '/' + match.group(1).lstrip('/')), set()).add(owner)
        # Tests often read fixtures through manifest-relative or repo-relative literals.
        for token in re.findall(r'"([^"\n]+)"', source):
            if '/' not in token or '\\' in token or len(token) > 512:
                continue
            for owner in owners:
                directory = manifests[owner][0]
                for root in ('', directory, posixpath.dirname(path)):
                    candidate = posixpath.normpath(posixpath.join(root, token.lstrip('/')))
                    if candidate in tracked_paths:
                        embedded.setdefault(candidate, set()).add(owner)
        if len(re.findall(r'include_(?:str|bytes)!', source)) > len(literal) + len(concatenated):
            embedded.setdefault('__unresolved_embed__', set()).update(owners)
    return manifests, reverse, embedded


def reverse_consumers(changed, reverse):
    result = set(changed)
    pending = list(changed)
    while pending:
        for consumer in reverse[pending.pop()]:
            if consumer not in result:
                result.add(consumer)
                pending.append(consumer)
    return result


def validate_config(config):
    groups = config['groups']
    if (not isinstance(groups, list) or len(groups) != len(set(groups))
            or not {'baseline', 'rust_baseline', *PLATFORM_GROUPS}.issubset(groups)):
        raise ValueError('invalid group inventory')
    if set(config['resources']) != set(groups):
        raise ValueError('missing resource mapping')
    for selected in config['package_groups'].values():
        if not selected or not set(selected).issubset(groups):
            raise ValueError('invalid package groups')
    for rule in config['rules']:
        if not rule.get('match') or not set(rule.get('groups', [])).issubset(groups):
            raise ValueError('invalid directory mapping')
    return config


def select(config, base, source_head, test_head=None, additions=()):
    validate_config(config)
    groups = set(config['groups'])
    chosen = {'baseline'}
    reasons = ['baseline always runs']
    paths = []
    affected = set()
    full = False
    try:
        # Selection covers both source diff and the actual tested merge combination.
        paths = changed_paths(base, source_head)
        if test_head and test_head != source_head:
            paths = list(dict.fromkeys(paths + changed_paths(base, test_head)))
        graphs = [cargo_graph(ref) for ref in dict.fromkeys([base, source_head, test_head or source_head])]
        for path in paths:
            if any(fnmatch.fnmatchcase(path, pattern) for pattern in CONTROL):
                full = True
                reasons.append('shared build or CI control changed: ' + path)
            matched = False
            for manifests, reverse, embedded in graphs:
                owners = {name for name, (directory, _) in manifests.items()
                          if directory and path.startswith(directory + '/')}
                owners.update(embedded.get(path, ()))
                if owners:
                    matched = True
                    affected.update(reverse_consumers(owners, reverse))
            for rule in config['rules']:
                if any(fnmatch.fnmatchcase(path, pattern) for pattern in rule['match']):
                    matched = True
                    chosen.update(rule.get('groups', []))
                    full |= rule.get('full', False)
                    reasons.append(rule['reason'] + ': ' + path)
            if any(embedded.get('__unresolved_embed__') for _, _, embedded in graphs) and not any(
                    directory and path.startswith(directory + '/') for manifests, _, _ in graphs
                    for directory, _ in manifests.values()):
                full = True
                reasons.append('unresolved embedded consumer input: ' + path)
            if not matched:
                full = True
                reasons.append('unknown input: ' + path)
        for package in sorted(affected):
            mapped = config['package_groups'].get(package)
            if not mapped:
                full = True
                reasons.append('unmapped consumer: ' + package)
            else:
                chosen.update(mapped)
        if affected:
            chosen.add('rust_baseline')
            reasons.append('changed packages and reverse consumers: ' + ', '.join(sorted(affected)))
    except (OSError, subprocess.CalledProcessError, ValueError, KeyError, TypeError) as error:
        full = True
        reasons.append('uncertain Git/dependency inputs: ' + str(error))
    if full:
        chosen = set(groups)
        reasons.append('ordinary required fallback')
    if 'operational_contracts' in chosen:
        chosen.update(('windows_rollout', 'fleet_health'))
    if 'packaging_contracts' in chosen:
        chosen.add('macos_package_contract')
    if not set(additions).issubset(groups):
        raise ValueError('additional request names unknown group')
    chosen.update(additions)
    if 'operational_contracts' in chosen:
        chosen.update(('windows_rollout', 'fleet_health'))
    if 'packaging_contracts' in chosen:
        chosen.add('macos_package_contract')
    resources = sorted({resource for group in chosen for resource in config['resources'][group]})
    return {'groups': sorted(chosen), 'resources': resources,
            'reasons': list(dict.fromkeys(reasons)), 'scope': 'full' if full else 'targeted'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base-ref', required=True)
    parser.add_argument('--head-ref', required=True, help='source HEAD')
    parser.add_argument('--test-ref', help='actual tested merge/tree commit')
    parser.add_argument('--config', default=str(Path(__file__).with_name('ci-required-scope.json')))
    parser.add_argument('--add-group', action='append', default=[])
    parser.add_argument('--github-output')
    parser.add_argument('--output')
    args = parser.parse_args()
    config = json.loads(Path(args.config).read_text())
    plan = select(config, args.base_ref, args.head_ref, args.test_ref, args.add_group)
    encoded = json.dumps(plan, ensure_ascii=True, separators=(',', ':'))
    if args.output:
        Path(args.output).write_text(encoded + '\n')
    if args.github_output:
        with Path(args.github_output).open('a') as output:
            output.write('plan=' + encoded + '\n')
            for group in config['groups']:
                output.write(f'run_{group}=' + str(group in plan['groups']).lower() + '\n')
    print(encoded)


if __name__ == '__main__':
    main()
