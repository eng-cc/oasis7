#!/usr/bin/env python3
"""Build once and run native test executables without Cargo on consumers."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
CONFIG = ROOT / 'scripts/ci-formal-process-suites.json'


def output(command):
    return subprocess.check_output(command, text=True, cwd=ROOT).strip()


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def inventory(path, ignored=False, env=None):
    command = [str(path), '--list', '--format', 'terse']
    if ignored:
        command.append('--ignored')
    text = subprocess.check_output(command, text=True, env=env, timeout=120)
    return sorted(line[:-6] for line in text.splitlines() if line.endswith(': test'))


def resolve_suites(config, binaries):
    suites = {name: list(entries) for name, entries in config['suites'].items()}
    assigned = set()
    for entries in suites.values():
        for entry in entries:
            key = (entry['binary'], entry['name'])
            binary = binaries[entry['binary']]
            if key in assigned or entry['name'] not in binary['ignored_tests']:
                raise ValueError(f'duplicate or missing ignored acceptance: {key}')
            assigned.add(key)
    for binary, names in config.get('auxiliary_entries', {}).items():
        for name in names:
            if name not in binaries[binary]['tests'] or (binary, name) in assigned:
                raise ValueError(f'missing or independently scheduled auxiliary: {binary}: {name}')
    for binary, filters in config['legacy_ignored_filters'].items():
        expected = {name for name in binaries[binary]['ignored_tests']
                    if any(part in name for part in filters)}
        actual = {name for owner, name in assigned if owner == binary}
        if expected != actual:
            raise ValueError(f'ignored coverage changed: {binary}: {expected ^ actual}')
    # Preserve the three original formal commands with their binary environment.
    # These are deliberately retained until equivalence to ordinary tests is proven.
    for binary, suite in config['environment_formal_batches'].items():
        ordinary = set(binaries[binary]['tests']) - set(binaries[binary]['ignored_tests'])
        for name in sorted(ordinary):
            if 'formal' in name:
                suites[suite].append(dict(binary=binary, name=name, ignored=False))
    for suite, scripts in config['shell_tests'].items():
        suites[suite].extend(dict(script=script) for script in scripts)
    return suites


def build(bundle):
    bundle.mkdir(parents=True, exist_ok=False)
    (bundle / 'bin').mkdir()
    (bundle / 'lib').mkdir()
    env = dict(os.environ)
    env.pop('RUSTC_WRAPPER', None)
    binaries = {}
    required_features = ['test_tier_required']
    # Separate variants preserve the old snapshot regressions' default features.
    variants = [
        ('viewer', ['build', '-p', 'oasis7', '--bin', 'oasis7_viewer_live', '--features', 'test_tier_required'], required_features),
        ('required', ['test', '-p', 'oasis7', '-p', 'oasis7_chain_runtime', '--tests', '--features', 'test_tier_required', '--no-run'], required_features),
        ('runtime', ['build', '-p', 'oasis7_chain_runtime', '--bin', 'oasis7_chain_runtime', '--features', 'test_tier_required'], required_features),
        ('endpoint', ['build', '-p', 'oasis7_distfs', '--bin', 'oasis7_durability_endpoint', '--release'], []),
        ('distfs-tests', ['test', '-p', 'oasis7_distfs', '--lib', '--no-run'], []),
        ('snapshot-tests', ['test', '-p', 'oasis7', '--lib', '--no-run'], []),
    ]
    for variant, args, features in variants:
        command = ['cargo', *args, '--locked', '--message-format=json-render-diagnostics']
        print('+ ' + ' '.join(command), flush=True)
        with subprocess.Popen(command, stdout=subprocess.PIPE, text=True, env=env, cwd=ROOT) as process:
            for line in process.stdout:
                try:
                    artifact = json.loads(line)
                except ValueError:
                    continue
                if artifact.get('reason') == 'compiler-message':
                    print(artifact['message'].get('rendered', ''), end='', flush=True)
                executable = artifact.get('executable')
                if artifact.get('reason') != 'compiler-artifact' or not executable:
                    continue
                target = artifact['target']
                test = artifact['profile']['test']
                name = target['name']
                logical = variant
                if variant == 'required':
                    if not test:
                        logical = 'required-executable-' + name
                    else:
                        logical = {'oasis7_chain_runtime': 'runtime-tests', 'oasis7': 'oasis7-lib-tests'}.get(name, 'required-test-' + '-'.join(target['kind']) + '-' + name)
                if logical in binaries:
                    raise ValueError(f'duplicate executable: {logical}')
                source = Path(executable)
                if variant == 'endpoint':
                    profile = artifact['profile']
                    if (name != 'oasis7_durability_endpoint' or test
                            or profile['opt_level'] != '3' or profile['debug_assertions']
                            or source.parent.name != 'release'):
                        raise ValueError('expected optimized release endpoint artifact')
                destination = bundle / 'bin' / logical
                shutil.copy2(source, destination)
                # Cargo uses the package directory as test cwd, and exposes its binaries.
                manifest = Path(artifact['manifest_path'])
                binaries[logical] = dict(path=str(destination.relative_to(bundle)), sha256=digest(destination),
                    profile=artifact['profile'], features=artifact['features'], requested_features=features,
                    test=test, target=name, original_executable=str(source), cwd=str(manifest.parent.relative_to(ROOT)),
                    ordinary=variant == 'required' and test)
                if sys.platform == 'linux':
                    linked = subprocess.run(['ldd', str(source)], text=True, capture_output=True)
                    for line in linked.stdout.splitlines():
                        fields = line.split()
                        if '=>' in fields and len(fields) > 2 and fields[2].startswith(str(ROOT / 'target') + '/'):
                            library = Path(fields[2])
                            shutil.copy2(library, bundle / 'lib' / library.name)
            if process.wait() != 0:
                raise RuntimeError(f'build failed: {variant}')
    manifest = dict(schema_version=1, tested_sha=output(['git', 'rev-parse', 'HEAD']),
        source_sha=os.environ.get('SOURCE_SHA', output(['git', 'rev-parse', 'HEAD'])),
        producer_run_id=os.environ.get('GITHUB_RUN_ID', 'local'),
        producer_attempt=os.environ.get('GITHUB_RUN_ATTEMPT', '1'),
        rust_toolchain=os.environ.get('RUST_TOOLCHAIN', 'local'),
        rustc_version=output(['rustc', '--version']),
        target_triple=output(['rustc', '-vV']).split('host: ')[1].splitlines()[0],
        runner_image=os.environ.get('ImageOS', sys.platform), checkout=str(ROOT), binaries=binaries,
        config_sha256=digest(CONFIG))
    list_env = dict(env, LD_LIBRARY_PATH=str(bundle / 'lib') + os.pathsep + env.get('LD_LIBRARY_PATH', ''))
    for logical, binary in binaries.items():
        if binary['test']:
            binary['tests'] = inventory(bundle / binary['path'], env=list_env)
            binary['ignored_tests'] = inventory(bundle / binary['path'], True, list_env)
            binary['test_list_sha256'] = hashlib.sha256(json.dumps(binary['tests']).encode()).hexdigest()
    manifest['suites'] = resolve_suites(json.loads(CONFIG.read_text()), binaries)
    manifest['libraries'] = {str(p.relative_to(bundle)): digest(p) for p in (bundle / 'lib').iterdir()}
    (bundle / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')


def load(bundle):
    manifest = json.loads((bundle / 'manifest.json').read_text())
    expected = dict(tested_sha=output(['git', 'rev-parse', 'HEAD']),
                    source_sha=os.environ.get('SOURCE_SHA', output(['git', 'rev-parse', 'HEAD'])),
                    producer_run_id=os.environ.get('GITHUB_RUN_ID', 'local'), checkout=str(ROOT),
                    config_sha256=digest(CONFIG),
                    rust_toolchain=os.environ.get('RUST_TOOLCHAIN', 'local'),
                    runner_image=os.environ.get('ImageOS', sys.platform))
    for key, value in expected.items():
        if manifest.get(key) != value:
            raise ValueError(f'artifact identity mismatch: {key}')
    for entry in manifest['binaries'].values():
        path = bundle / entry['path']
        if not path.resolve().is_relative_to(bundle.resolve()) or digest(path) != entry['sha256'] or not os.access(path, os.X_OK):
            raise ValueError('missing, invalid or corrupt executable')
    # env!(CARGO_BIN_EXE_*) embeds Cargo's original absolute binary path.
    # Restore only recorded executables under this checkout's target directory.
    for entry in manifest['binaries'].values():
        original = Path(entry['original_executable'])
        if not entry['test'] and original.is_relative_to(ROOT / 'target'):
            original.parent.mkdir(parents=True, exist_ok=True)
            if not original.exists() or digest(original) != entry['sha256']:
                shutil.copy2(bundle / entry['path'], original)
    for name, sha in manifest['libraries'].items():
        path = bundle / name
        if not path.resolve().is_relative_to(bundle.resolve()) or digest(path) != sha:
            raise ValueError('invalid dynamic library')
    env = dict(os.environ)
    env['LD_LIBRARY_PATH'] = str(bundle / 'lib') + os.pathsep + env.get('LD_LIBRARY_PATH', '')
    for variable, logical in {
        'PRE2_VIEWER_BINARY': 'viewer', 'OASIS7_CHAIN_RUNTIME_BIN': 'runtime',
        'OASIS7_FORMAL_ENDPOINT_TEST_BIN': 'endpoint',
        'OASIS7_NODE_LIFECYCLE_TEST_BIN': 'runtime-tests',
        'OASIS7_FORMAL_NODE_TEST_BIN': 'runtime',
    }.items():
        env[variable] = str(bundle / manifest['binaries'][logical]['path'])
    # Validate actual downloaded harnesses; the manifest alone is not coverage proof.
    for binary in manifest['binaries'].values():
        if binary['test']:
            if inventory(bundle / binary['path'], env=env) != binary['tests'] or inventory(bundle / binary['path'], True, env) != binary['ignored_tests']:
                raise ValueError('downloaded test inventory mismatch')
    if resolve_suites(json.loads(CONFIG.read_text()), manifest['binaries']) != manifest['suites']:
        raise ValueError('suite inventory mismatch')
    return manifest, env


def execute(bundle, suite, results):
    from ci_process_runner import run
    manifest, env = load(bundle)
    if suite == 'ordinary':
        entries = [dict(binary=name, batch=True) for name, binary in manifest['binaries'].items() if binary['ordinary'] and set(binary['tests']) - set(binary['ignored_tests'])]
        binary = manifest['binaries']['snapshot-tests']
        for pattern in ['snapshot_progress::', 'snapshot_player_gameplay_execution_state_backfills_from_legacy_fields']:
            names = [name for name in binary['tests'] if pattern in name and name not in binary['ignored_tests']]
            if not names:
                raise ValueError('missing snapshot regression: ' + pattern)
            entries.append(dict(binary='snapshot-tests', filter=pattern, count=len(names)))
        # Original ordinary environment did not contain formal-process child paths.
        for variable in list(env):
            if variable.startswith('OASIS7_FORMAL_') or variable in ('OASIS7_CHAIN_RUNTIME_BIN', 'OASIS7_NODE_LIFECYCLE_TEST_BIN'):
                env.pop(variable)
    else:
        entries = manifest['suites'][suite]
    failed = False
    for entry in entries:
        if 'script' in entry:
            command = ['bash', str(ROOT / 'scripts' / entry['script'])]
            cwd, count = ROOT, None
            name = entry['script']
        else:
            binary = manifest['binaries'][entry['binary']]
            command = [str(bundle / binary['path'])]
            name = entry.get('name', entry['binary'])
            if entry.get('batch'):
                count = len(set(binary['tests']) - set(binary['ignored_tests']))
            elif 'filter' in entry:
                command.append(entry['filter'])
                name, count = entry['filter'], entry['count']
            else:
                command += ['--exact', name]
                if entry['ignored']:
                    command.append('--ignored')
                count = 1
            if suite != 'ordinary':
                command += ['--test-threads=1', '--nocapture']
            cwd = ROOT / binary['cwd']
        test_env = dict(env, CARGO_MANIFEST_DIR=str(cwd))
        result = run(command, cwd, test_env, suite, name, results, expected_count=count)
        failed |= not result['passed']
        if result['cleanup_result'] != 'clean':
            break
    if failed:
        raise RuntimeError(f'{suite} failed; see {results}')


def main():
    global ROOT, CONFIG
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['build', 'run'])
    parser.add_argument('--repo-root', type=Path, default=ROOT)
    parser.add_argument('--bundle', required=True, type=Path)
    parser.add_argument('--suite', default='ordinary')
    parser.add_argument('--results', type=Path)
    args = parser.parse_args()
    ROOT = args.repo_root.resolve()
    CONFIG = ROOT / 'scripts/ci-formal-process-suites.json'
    if args.action == 'build':
        build(args.bundle.resolve())
    else:
        execute(args.bundle.resolve(), args.suite, args.results or ROOT / 'output/formal-process/results.jsonl')


if __name__ == '__main__':
    main()
