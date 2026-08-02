# Testing templates and examples

This directory contains reusable schemas and deliberately non-live examples. It is a control surface, not an evidence archive.

## Claim boundary

- A template, blank field, sample `pass`, checklist, or example lane never proves that a test ran or passed.
- `*.example.json`, `*.example.txt`, and `*.template.tsv` values are non-live inputs. Addresses under `.example.invalid`, sample genesis, bootstrap peers, manifests, lane rows, account values, and endpoints must be replaced and validated for the current execution window.
- `mainnet`, `public_testnet`, legacy-network, incident, release, proof, playability, and hosted-world filenames describe the target schema only. They do not establish network health, readiness, public availability, incident closure, player validation, settlement, or release approval.
- Completed evidence belongs in the governed evidence/task sink and must bind commands, inputs, outputs, owners, time window, and current authority; do not edit a template in place and cite it as a result.

## Lifecycle

The exact existing template/example set is retained as one reviewed semantic bundle in `doc/.governance/document-semantic-review-overrides.json`. A zero text-reference count is not sufficient for deletion because schemas may be copied by operator or external workflows. Retirement requires a reviewed successor, semantic absorption, repaired callers, domain-owner approval, and QA/repository-health verification.
