# Harden launcher provider configuration contracts

PR URL: https://github.com/eng-cc/oasis7/pull/482

Task: .pm/tasks/task_169255fb26a2410a9c9edfaa839fc466.yaml
Task UID: task_169255fb26a2410a9c9edfaa839fc466

Summary:
- Add tested web launcher agent-provider schema/config/args coverage.
- Make provider-backed web config fail closed for invalid provider subfields, transport URL policy, and invalid agent_decision_source.
- Clarify trusted_local_only as an internal local-playtest escape hatch and share HTTP base URL parser coverage.
