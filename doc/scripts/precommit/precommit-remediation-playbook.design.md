# Fix Pre-commit（显式本地修复与诊断脚本）设计

- 对应需求文档: `doc/scripts/precommit/precommit-remediation-playbook.prd.md`
- 对应项目管理文档: `doc/scripts/precommit/precommit-remediation-playbook.project.md`

## 1. 设计定位
定义操作者显式触发的本地修复与诊断流程；普通 commit / pre-commit 保持静默 no-op。

## 2. 设计结构
- 显式修复层：定义操作者按需运行的格式化、测试与治理诊断，不接入普通提交门禁。
- 修复辅助层：提供失败后的 remediation/playbook 脚本入口。
- 口径对齐层：区分显式本地诊断与 authoritative CI required、frozen-head readiness gates。
- 维护回写层：沉淀脚本更新与失败签名。

## 3. 关键接口 / 入口
- legacy pre-commit 静默 no-op 兼容入口
- remediation/playbook 工具
- required 门禁矩阵
- 失败签名与维护说明

## 4. 约束与边界
- 显式本地诊断不得被描述或接线为普通提交门禁。
- CI required 与 frozen-head readiness 保持 authoritative，不由修复脚本替代。
- 修复脚本必须服务可重复故障。
- 不在本专题扩展新的 CI 平台。

## 5. 设计演进计划
- 先冻结普通 pre-commit no-op 与显式诊断的边界。
- 再补修复/对齐链路。
- 最后固化维护说明。
