# task_c7a8defc7c0f4f4c8f86660b50df08a5 Execution Log

- task_uid: task_c7a8defc7c0f4f4c8f86660b50df08a5
- title: wasm executor real compiled cache and hot-path cleanup
- owner_role: wasm_platform_engineer
- worktree_hint: /home/scc/worktrees/oasis7-world-runtime-wasm-executor-real-compiled-cache

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->
## 2026-04-17 09:50:38 CST / wasm_platform_engineer
- 完成内容: 将 `oasis7_wasm_executor` 磁盘 compiled cache 从原始 `.wasm` 字节回盘改为 Wasmtime `Module::serialize()` 产物回盘，命中路径切到 `Module::deserialize_file()`；同步把缓存文件扩展名改为 `.cwasm`，并为 executor perf probe 补齐 `Arc` 引入，恢复 ignored probe 的可编译性。
- 完成内容: 补充 `wasm_executor_disk_cache_persists_serialized_compiled_artifact` 回归，保留并验证磁盘缓存命中、损坏恢复、watchdog/perf probe；更新 `doc/world-runtime/project.md` 与 `doc/world-runtime/wasm/wasm-executor.{prd,project}.md`，使文档口径与实现一致。
- 完成内容: 已验证 `env -u RUSTC_WRAPPER cargo test -p oasis7_wasm_executor --features wasmtime -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7_wasm_executor --features wasmtime perf_probe_executor_call_and_watchdog_overhead -- --ignored --nocapture`、`./scripts/doc-governance-check.sh`、`git diff --check` 全部通过。
- 遗留事项: 若后续继续做 wasm 热路径优化，下一优先级应落在 executor 实例/导出缓存与 runtime 路由候选索引，而不是继续优化 `Arc<[u8]>` 复制路径。
