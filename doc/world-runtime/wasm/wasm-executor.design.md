# oasis7 Runtime：WASM 执行器接入（设计分册）设计

- 对应需求文档: `doc/world-runtime/wasm/wasm-executor.prd.md`
- 对应项目管理文档: `doc/world-runtime/wasm/wasm-executor.project.md`

## 1. 设计定位
定义 WASM 执行器接入设计，统一执行入口、宿主交互、资源限制与错误回传。

## 2. 设计结构
- 执行入口层：定义模块加载、实例化与调用入口。
- 宿主交互层：明确 host function、上下文注入与结果回传。
- 资源约束层：对 fuel、内存和中断语义实施限制。
- 错误观测层：把 trap、超时和权限失败映射为稳定错误。
- ABI/capability 层：以可选兼容字段承载 schema、cap slot、policy hook 与 ModuleContext 元信息。
- 工件完整性层：校验存储 hash 与 compiled-cache wrapper/compatibility domain。

## 3. 关键接口 / 入口
- WASM 执行入口
- host function 接口
- 资源限制配置
- 执行错误映射
- `ModuleManifest.abi_contract`、`ModuleEffectIntent.cap_slot` 与 pure policy hooks
- serialized compiled artifact cache 与持久化工件 hash 校验

## 4. 约束与边界
- 执行器必须可观测、可中断。
- 资源限制优先保证宿主安全。
- 不在本专题扩展新的字节码格式。
- 新 ABI/schema 字段保持 optional/default compatible，不把 agent-os 参考实现升级为 Oasis ABI 替代品。
- pure policy hook 只判定，不产生递归副作用；cap slot 未声明或冲突时 fail closed。
- `max_gas=0` 回退到配置 fuel；epoch 与 memory limiter 保证可抢占和有界资源。
- compiled cache 按 engine/OS/arch 隔离，损坏缓存降级为 miss；storage lifecycle 仍由 module-storage 拥有。

## 5. 设计演进计划
- 先接执行入口。
- 再补宿主交互与资源约束。
- 最后沉淀错误观测与回归。
- 修改 capability、sandbox limits 或 cache 格式时，同步更新 ABI 兼容与损坏恢复测试。
