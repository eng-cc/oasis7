# Pre-commit remediation 历史路由

- 对应设计路由：`doc/scripts/precommit/precommit-remediation-playbook.design.md`
- 对应项目路由：`doc/scripts/precommit/precommit-remediation-playbook.project.md`

审计轮次: 4

本文件仅为历史路径兼容叶。当前契约已合并到 `doc/scripts/precommit/pre-commit.prd.md#失败修复`。

## 目标
将旧链接导向单一当前操作契约，不在此处重复修复步骤。

## 范围
- 范围内：历史路径兼容与 successor 导航。
- 范围外：定义当前 pre-commit、CI tier 或修复命令。

## 接口 / 数据
- 当前契约：`doc/scripts/precommit/pre-commit.prd.md`
- 可执行入口：`scripts/fix-precommit.sh`

## 里程碑
- 收口完成：当前操作契约与完成历史已并入 pre-commit 主文档链。

## 风险
- 删除本路径会破坏历史审计链接；因此保留最小导航叶，但不维护第二份契约。
