#!/usr/bin/env python3
"""Check the actual no-test-tier executables reject Hosted mock funding."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile


REJECTION = 'Hosted local-mock funding requires a binary built with feature test_tier_required'
TARGETS = ('oasis7_chain_runtime', 'oasis7_game_launcher')


def verify(manifest):
    executables = {}
    for line in Path(manifest).read_text().splitlines():
        if not line.startswith('{'):
            continue
        artifact = json.loads(line)
        name = artifact.get('target', {}).get('name')
        if artifact.get('reason') != 'compiler-artifact' or name not in TARGETS or not artifact.get('executable'):
            continue
        features = set(artifact['features'])
        assert 'wasmtime' in features and not features.intersection({'test_tier_required', 'test_tier_full'}), features
        assert name not in executables, name
        executables[name] = Path(artifact['executable']).resolve(strict=True)
    assert set(executables) == set(TARGETS), executables
    common = [
        '--chain-local-standalone-test', '--local-test-provider-authority', 'authority.json',
        '--local-test-provider-wasm', 'provider.wasm',
        '--local-test-provider-metadata', 'provider.metadata.json',
        '--local-test-provider-finality-block-hash', 'blake3:' + '0' * 64,
        '--local-test-provider-session-mode', 'hosted_public_join',
        '--agent-decision-source', 'provider_backed',
        '--agent-provider-backend', 'provider_local_mock',
        '--agent-provider-contract', 'worldsim_provider_v1',
        '--agent-provider-transport', 'loopback_http',
        '--agent-execution-lane', 'player_parity',
    ]
    for name in TARGETS:
        args = common
        if name == 'oasis7_game_launcher':
            args = ['--deployment-mode', 'hosted_public_join', '--chain-enable', *common,
                    '--agent-provider-url', 'http://127.0.0.1:5841', '--with-llm', '--no-open-browser']
        with tempfile.TemporaryDirectory(prefix='oasis7-production-guard-') as directory:
            process = subprocess.Popen([str(executables[name]), *args], cwd=directory,
                                       env=dict(os.environ, RUST_LOG='error'),
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                       text=True, start_new_session=True)
            try:
                stdout, stderr = process.communicate(timeout=10)
                assert process.returncode == 1 and REJECTION in stdout + stderr, (name, process.returncode, stdout, stderr)
            finally:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait()
        print(name + ': production Hosted mock funding rejected')


if __name__ == '__main__':
    verify(sys.argv[1])
