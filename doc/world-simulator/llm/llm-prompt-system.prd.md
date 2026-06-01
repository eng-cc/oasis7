# Retired Direct Model Path Document

Paired project: `doc/world-simulator/llm/llm-prompt-system.project.md`.

## 目标
Retire the obsolete game-side direct model path and keep this path only as a governance tombstone.

## 范围
No active implementation, operator workflow, or playtest runbook should depend on this retired document.

## 接口 / 数据
Current gameplay uses provider-backed agent decisions through the remote provider bridge.

## 里程碑
Retired during MVP playtest readiness hardening.

## 风险
Do not reintroduce client-side model configuration from this historical path.
