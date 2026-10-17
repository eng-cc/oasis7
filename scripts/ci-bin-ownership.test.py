#!/usr/bin/env python3
"""Executable ownership and lightweight service dependency contracts."""
from pathlib import Path
import re
import tomllib
import unittest

ROOT = Path(__file__).resolve().parent.parent
OWNERS = {
    'oasis7_newapi_bridge_service': ['oasis7_newapi_bridge_service'],
    'oasis7_provider_local_bridge': ['oasis7_provider_local_bridge'],
    'oasis7_provider_tools': ['oasis7_llm_agent_demo', 'oasis7_llm_provider_probe', 'oasis7_provider_parity_bench'],
    'oasis7_launcher_backend': ['oasis7_game_launcher', 'oasis7_web_launcher'],
    'oasis7_client_cli': ['oasis7_pure_api_client', 'oasis7_chain_transfer_submit_client'],
    'oasis7_viewer_runtime': ['oasis7_viewer_live', 'oasis7_viewer_server', 'oasis7_viewer_demo'],
    'oasis7_ops_tools': ['oasis7_generate_world_artifact', 'oasis7_governance_registry_audit',
                        'oasis7_governance_registry_import', 'oasis7_init_demo',
                        'oasis7_liveops_grant_cli', 'oasis7_rollback_strict_audit_evidence',
                        'oasis7_runtime_module_routing_perf', 'oasis7_testnet_faucet',
                        'oasis7_world_repair_rebuild'],
}


def manifest(package):
    return tomllib.loads((ROOT / 'crates' / package / 'Cargo.toml').read_text())


class Ownership(unittest.TestCase):
    def test_core_is_library_and_original_executable_names_survive_once(self):
        core = manifest('oasis7')
        self.assertIs(core['package']['autobins'], False)
        self.assertFalse(core.get('bin', []))
        self.assertFalse(list((ROOT / 'crates/oasis7/src/bin').glob('**/*.rs')))
        seen = set()
        for package, names in OWNERS.items():
            directory = ROOT / 'crates' / package
            data = manifest(package)
            declared = {item['name']: item for item in data.get('bin', [])}
            if (directory / 'src/main.rs').exists():
                declared.setdefault(package, {'path': 'src/main.rs'})
            if data['package'].get('autobins', True):
                for source in (directory / 'src/bin').glob('*.rs'):
                    declared.setdefault(source.stem, {'path': str(source.relative_to(directory))})
            self.assertEqual(set(declared), set(names), package)
            for name, target in declared.items():
                self.assertNotIn(name, seen)
                seen.add(name)
                self.assertTrue((directory / target['path']).is_file(), name)

    def test_newapi_service_has_no_transitive_world_dependency(self):
        visited = set()
        pending = ['oasis7_newapi_bridge_service']
        while pending:
            package = pending.pop()
            if package in visited:
                continue
            visited.add(package)
            data = manifest(package)
            tables = [data.get('dependencies', {})]
            tables += [target.get('dependencies', {}) for target in data.get('target', {}).values()]
            for table in tables:
                for name, dependency in table.items():
                    if isinstance(dependency, dict) and 'path' in dependency:
                        pending.append(dependency.get('package', name))
        self.assertNotIn('oasis7', visited)
        self.assertNotIn('oasis7_node', visited)
        self.assertNotIn('oasis7_wasm_executor', visited)

    def test_client_production_does_not_enable_world_execution_features(self):
        data = manifest('oasis7_client_cli')
        self.assertEqual(data['features']['default'], [])
        self.assertIs(data['dependencies']['oasis7']['default-features'], False)
        self.assertFalse(data['dependencies']['oasis7'].get('features', []))

    def test_no_cross_package_source_borrowing_from_binary_directories(self):
        for package in ['oasis7', *OWNERS, 'oasis7_hosted_access', 'oasis7_letai_credit_adapter']:
            directory = ROOT / 'crates' / package
            for source in (directory / 'src').rglob('*.rs'):
                for relative in re.findall(r'#\[path\s*=\s*"([^"]+)"\]', source.read_text()):
                    target = (source.parent / relative).resolve()
                    if '/src/bin/' in str(target):
                        self.assertTrue(target.is_relative_to(directory), f'{source}: {relative}')


if __name__ == '__main__':
    unittest.main()
