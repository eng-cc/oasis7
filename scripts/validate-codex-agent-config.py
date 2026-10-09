#!/usr/bin/env python3
"""Validate repository Codex configuration syntax, paths, and basic fields."""
import argparse
import pathlib
try:
    import tomllib
except ModuleNotFoundError:
    raise SystemExit("Python 3.11 or newer with tomllib is required; select it with scripts/find-python-with-module.sh tomllib")

def validate(root):
    directory = root / '.codex'
    data = tomllib.loads((directory / 'config.toml').read_text())
    for name, entry in data.get('agents', {}).items():
        if not isinstance(entry, dict) or not isinstance(entry.get('description'), str):
            raise ValueError('invalid agent entry: ' + name)
        path = (directory / entry['config_file']).resolve()
        if not path.is_relative_to(directory.resolve()) or not path.is_file():
            raise ValueError('invalid adapter path: ' + name)
        config = tomllib.loads(path.read_text())
        for field in ('model', 'model_reasoning_effort', 'developer_instructions'):
            if field in config and not isinstance(config[field], str):
                raise ValueError('invalid ' + field + ': ' + name)
    return len(data.get('agents', {}))

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo-root', type=pathlib.Path, default=pathlib.Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    try:
        print('validated agent configurations:', validate(args.repo_root))
    except (OSError, KeyError, TypeError, ValueError) as exc:
        parser.exit(1, str(exc) + '\n')
