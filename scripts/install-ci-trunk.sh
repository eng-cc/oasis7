#!/usr/bin/env bash
# Install only the inspected Linux x86_64 Trunk release, including cache hits.
set -euo pipefail

version=0.21.14
checksum=f2b4680cd239693a646a2795e4633c625328d7b2a044fbe749fa3a2fe9e7036b
[[ "${TRUNK_VERSION:-$version}" == "$version" ]] || { echo 'unsupported CI Trunk version' >&2; exit 1; }
[[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || { echo 'unsupported CI Trunk platform' >&2; exit 1; }
[[ $# == 2 ]] || { echo 'usage: install-ci-trunk.sh ARCHIVE BIN_DIRECTORY' >&2; exit 1; }
archive=$1
bin_directory=$2
mkdir -p "$(dirname "$archive")"
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
verify_archive() {
  printf '%s  %s\n' "$checksum" "$1" >"$stage/checksum"
  sha256sum -c "$stage/checksum" >/dev/null
}
if [[ ! -f "$archive" ]]; then
  curl --fail --location --silent --show-error --http1.1 --retry 3 --connect-timeout 15 --max-time 120 \
    "https://github.com/trunk-rs/trunk/releases/download/v${version}/trunk-x86_64-unknown-linux-gnu.tar.gz" \
    --output "$stage/archive.tar.gz"
  verify_archive "$stage/archive.tar.gz"
  mv "$stage/archive.tar.gz" "$archive"
fi
# A cache hit is an untrusted download until its original digest is verified.
verify_archive "$archive"
tar -xzf "$archive" -C "$stage" trunk
[[ -f "$stage/trunk" && ! -L "$stage/trunk" ]] || { echo 'Trunk release binary missing' >&2; exit 1; }
chmod +x "$stage/trunk"
[[ "$("$stage/trunk" --version)" == "trunk $version" ]] || { echo 'CI Trunk version mismatch' >&2; exit 1; }
mkdir -p "$bin_directory"
install -m 755 "$stage/trunk" "$bin_directory/trunk"
if [[ -n "${GITHUB_PATH:-}" ]]; then
  printf '%s\n' "$bin_directory" >>"$GITHUB_PATH"
fi
