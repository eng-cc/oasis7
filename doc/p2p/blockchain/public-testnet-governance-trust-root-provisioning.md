# Public-testnet governance trust-root provisioning

The clean-room adapter consumes one structured JSON trust-root artifact. The
artifact is not a caller-selected identity and is not a replacement for the
independent provenance verifier. It is a deployment-bound regular file at:

`/operator/truth/governance-root.json`

The repository fixture is the schema and canonical-digest contract:

`scripts/fixtures/oasis7-governance-root.v1.json`

For the governed fixture deployment, provision the exact fixture bytes as the
code-owned path, then enforce the operator-local ownership contract:

```sh
mkdir -p /operator/truth
install -o "$(id -u)" -g "$(id -g)" -m 0600 \
  scripts/fixtures/oasis7-governance-root.v1.json \
  /operator/truth/governance-root.json
if command -v shasum >/dev/null 2>&1; then
  shasum -a 256 /operator/truth/governance-root.json
else
  sha256sum /operator/truth/governance-root.json
fi
```

The `mkdir` step must create only the governed parent path; do not replace the
artifact with a symlink or copy it from another node. `shasum -a 256` is the
portable macOS/Linux spelling when available, with `sha256sum` retained for
minimal Linux images.

For this fixture the expected file digest is
`f278bc8f060cd6777d68f086fc3131edc5d6b5a6080bde09208ba69a69e3ef66`; the
embedded canonical `root_digest` is
`5abd00f3e90a3e894f110f5a32ecab772e23e97ad7ec2cc9d675ae65282ae8ab`.

The adapter pins both the file SHA-256 and the semantic `root_digest` computed
by the repository provenance helper. It also rejects a symlink in the file or
any ancestor. `owner_scope=operator-local` means the account executing the
adapter must own the regular file; the numeric UID is deployment-local and is
never supplied by a plan or authority envelope. The fixture's public-key
values are non-secret test material and must not be treated as production
signing credentials.
