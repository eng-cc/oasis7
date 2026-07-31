# Agent 默认模块设计

- 对应需求文档: `doc/world-runtime/module/agent-default-modules.prd.md`
- 当前任务状态与历史变更：GitHub task issue evidence 与 Git history。

## 1. 设计定位
定义 Agent 出厂默认模块的安装方式、模块角色划分、生命周期与治理挂接方式。

## 2. 设计结构
- 默认模块集合：定义最小默认启用模块与对应职责。
- 安装链路：power bootstrap 与 agent default package 分开通过治理事件完成 register / activate，并保证幂等。
- 生命周期：支持首次安装、重装激活、版本替换与停用恢复。

## 3. 关键接口 / 入口
- `install_m1_power_bootstrap_modules`、`install_m1_agent_default_modules` 与 power-first `install_m1_scenario_bootstrap_modules`
- module manifest / module changeset / governance apply
- 事件：模块启用、停用、状态变更

## 4. 约束与边界
- 默认模块必须保持最小可信集合，不直接固化业务扩展。
- 世界初始状态与模块能力边界必须通过治理链路显式落地。
- 重复安装必须幂等，不允许重复 register。
- power 安装保留低速 radiation emission、有限私有储能与移动前 denial；已注册停用模块只能 re-activate。
- 安装使用既有 `manifest.version.saturating_add(1)`，不将它外推为 checked numeric contract。

## 5. 设计演进计划
- 先完成设计补齐与互链回写。
- 再按对应 GitHub task 的任务拆解推进实现与验证。
