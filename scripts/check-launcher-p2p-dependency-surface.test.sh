#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
checker="$repo_root/scripts/check-launcher-p2p-dependency-surface.sh"
ci_tests="$repo_root/scripts/ci-tests.sh"

required_gate_line='  run bash ./scripts/check-launcher-p2p-dependency-surface.test.sh'
if [[ "$(grep -F -xc -- "$required_gate_line" "$ci_tests" || true)" -ne 1 ]]; then
  echo "scripts/ci-tests.sh must contain exactly one required-gate invocation: $required_gate_line" >&2
  exit 1
fi
if ! awk -v expected="$required_gate_line" '
  $0 == "run_required_gate_checks() {" { in_required_gate=1; next }
  in_required_gate && $0 == expected { found=1 }
  in_required_gate && $0 == "}" { exit found ? 0 : 1 }
  END { if (!found) exit 1 }
' "$ci_tests"; then
  echo "required-gate invocation must be unconditional inside run_required_gate_checks" >&2
  exit 1
fi

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

fake_bin="$tmp_dir/bin"
mkdir -p "$fake_bin"

cat >"$fake_bin/cargo" <<'FAKE_CARGO'
#!/usr/bin/env bash
set -euo pipefail

printf '%s\n' "$*" >>"${FAKE_CARGO_LOG:?}"

if [[ "${1:-}" != "tree" ]]; then
  echo "unexpected fake cargo command: $*" >&2
  exit 1
fi

args=("$@")
spec=""
for ((index = 0; index < ${#args[@]}; index++)); do
  if [[ "${args[index]}" == "-i" && $((index + 1)) -lt ${#args[@]} ]]; then
    spec="${args[index + 1]}"
  fi
done

case "$spec" in
  libp2p|ring@0.16.20|rustls-webpki@0.101.7|hickory-proto@0.24.4)
    printf 'error: package ID specification %s did not match any packages\n' "$spec" >&2
    exit 1
    ;;
  *)
    echo "unexpected forbidden-spec query: $spec" >&2
    exit 1
    ;;
esac
FAKE_CARGO
chmod +x "$fake_bin/cargo"

fake_cargo_log="$tmp_dir/cargo-invocations.log"
run_output="$tmp_dir/checker-output.log"
if ! PATH="$fake_bin:$PATH" \
  FAKE_CARGO_LOG="$fake_cargo_log" \
  bash "$checker" >"$run_output" 2>&1; then
  echo "dependency-surface checker did not preserve its four exclusion checks" >&2
  sed -n '1,120p' "$run_output" >&2
  exit 1
fi

expected_specs=(
  "libp2p"
  "ring@0.16.20"
  "rustls-webpki@0.101.7"
  "hickory-proto@0.24.4"
)

invocation_count=$(awk 'END { print NR + 0 }' "$fake_cargo_log")
if [[ "$invocation_count" -ne "${#expected_specs[@]}" ]]; then
  echo "expected one cargo tree invocation per forbidden spec, got $invocation_count" >&2
  sed -n '1,120p' "$fake_cargo_log" >&2
  exit 1
fi

for spec in "${expected_specs[@]}"; do
  if [[ "$(grep -F -c -- "-i $spec" "$fake_cargo_log" || true)" -ne 1 ]]; then
    echo "expected exactly one cargo tree query for forbidden spec: $spec" >&2
    sed -n '1,120p' "$fake_cargo_log" >&2
    exit 1
  fi

  expected_ok="ok: oasis7_client_launcher dependency closure excludes $spec"
  if [[ "$(grep -F -c -- "$expected_ok" "$run_output" || true)" -ne 1 ]]; then
    echo "missing preserved exclusion result for forbidden spec: $spec" >&2
    sed -n '1,120p' "$run_output" >&2
    exit 1
  fi
done

while IFS= read -r invocation; do
  if [[ "$invocation" != tree* ]]; then
    echo "fake Cargo recorded a non-tree invocation: $invocation" >&2
    exit 1
  fi
  for flag in --locked --offline; do
    case " $invocation " in
      *" $flag "*) ;;
      *)
        echo "missing required Cargo reproducibility flag $flag in invocation: $invocation" >&2
        exit 1
        ;;
    esac
  done
done <"$fake_cargo_log"

echo "ok: all launcher dependency-surface cargo tree invocations are locked and offline"
