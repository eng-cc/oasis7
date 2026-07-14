#!/usr/bin/env bash
set -euo pipefail

root_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

mkdir -p "$fixture/repo/scripts" "$fixture/bin"
cp "$root_dir/scripts/pre-commit.sh" "$fixture/repo/scripts/pre-commit.sh"

for command in git rustfmt cargo npm; do
  cat >"$fixture/bin/$command" <<EOF
#!/usr/bin/env bash
echo "pre-commit invoked forbidden command: $command \$*" >&2
exit 97
EOF
  chmod +x "$fixture/bin/$command"
done

cat >"$fixture/repo/scripts/ci-tests.sh" <<'EOF'
#!/usr/bin/env bash
echo "pre-commit invoked forbidden validation entrypoint: scripts/ci-tests.sh $*" >&2
exit 98
EOF
chmod +x "$fixture/repo/scripts/ci-tests.sh"

if ! output=$(PATH="$fixture/bin:/usr/bin:/bin" "$fixture/repo/scripts/pre-commit.sh" 2>&1); then
  printf '%s\n' "$output" >&2
  echo "pre-commit must remain a successful no-op for legacy installed hooks" >&2
  exit 1
fi

if [[ -n "$output" ]]; then
  printf 'pre-commit must not produce validation or formatting output, got:\n%s\n' "$output" >&2
  exit 1
fi

echo "pre-commit.test: OK"
