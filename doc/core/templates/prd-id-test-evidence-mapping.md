# Core PRD-ID 到测试证据映射模板

审计轮次: 4

## 目的
- 为 `TASK-CORE-004` 提供仓库级统一追溯格式。
- 建立 `PRD-ID -> 任务ID -> 测试层级 -> 命令 -> 证据路径 -> 结论` 的最小可审计链路。

## 使用说明
- 每个 `PRD-ID` 至少对应 1 条可复现验证记录。
- 同一 `任务ID` 可对应多条 `命令`（required + full 或多条专项验证）。
- 仅在证据可追溯时填写 `pass`；证据不足时必须填写 `blocked` 并补充原因。

## 字段定义
| 字段 | 含义 | 填写规则 |
| --- | --- | --- |
| PRD-ID | 需求追踪主键 | 必填，格式 `PRD-<MODULE>-<NUM>` |
| 任务ID | 项目执行任务主键 | 必填，格式 `TASK-<MODULE>-<NUM>` |
| 测试层级 | 验证层级 | core 治理默认 `test_tier_required`；模块专项按 `required/full` 填写 |
| 命令 | 可复现验证命令 | 必填，可直接执行 |
| 证据路径 | 结果文件或日志路径 | 必填，需指向仓库内可访问路径 |
| 结论 | 当前验证结论 | 仅允许 `pass/fail/blocked` |

## 模板
| PRD-ID | 任务ID | 测试层级 | 命令 | 证据路径 | 结论 |
| --- | --- | --- | --- | --- | --- |
| PRD-XXX | TASK-XXX | test_tier_required | `./scripts/doc-governance-check.sh` | GitHub task issue evidence comments | blocked |
| PRD-XXX | TASK-XXX | required | `env -u RUSTC_WRAPPER cargo test -p <crate> --features test_tier_required` | `output/<module>/required-*.log` | blocked |
| PRD-XXX | TASK-XXX | full | `env -u RUSTC_WRAPPER cargo test -p <crate> --features test_tier_full` | `output/<module>/full-*.log` | blocked |

## 填写约束
- `命令` 需与 `测试层级` 一致，不可出现“层级写 required，命令却是 lint”的情况。
- `证据路径` 不可留空；如证据在 CI，需落盘到仓库路径后再引用。
- `结论 = fail/blocked` 时，必须在对应 GitHub task issue evidence comments 记录整改项（负责人 + 截止时间）或延期备注。

## 最小审查清单
- 是否覆盖所有相关 `PRD-ID`。
- 是否覆盖所有关联 `任务ID`。
- `测试层级` 与 `命令` 是否一致。
- `证据路径` 是否真实存在且可打开。
- `结论` 是否可由证据直接支持。
