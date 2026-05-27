# oasis7 Simulator：LLM 请求层迁移至 async-openai Responses API（项目管理文档）

- 对应设计文档: `doc/world-simulator/llm/llm-async-openai-responses.design.md`
- 对应需求文档: `doc/world-simulator/llm/llm-async-openai-responses.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] AOR1 输出设计文档（`doc/world-simulator/llm/llm-async-openai-responses.prd.md`）
- [x] AOR2 输出项目管理文档（本文件）
- [x] AOR3 引入 `async-openai` 依赖并清理旧 chat/completions 直连请求结构
- [x] AOR4 迁移客户端请求构造到 Responses API（`instructions + input + tools`）
- [x] AOR5 适配工具注册与函数调用解析（`OutputItem::FunctionCall` -> `module_call`）
- [x] AOR6 保留并验证短超时自动回退重试能力
- [x] AOR7 更新单测（URL 归一化、工具请求、函数调用解析、超时回退）
- [x] AOR8 更新 README / config 示例 / 对外说明
- [x] AOR9 跑测试与检查（`env -u RUSTC_WRAPPER cargo test` / `cargo check`）
- [x] AOR10 回顾并更新项目状态与开发日志（`doc/devlog/README.md`）

## 依赖
- `crates/oasis7/Cargo.toml`
- `crates/oasis7/src/simulator/llm_agent.rs`
- `crates/oasis7/src/simulator/llm_agent/tests.rs`
- `crates/oasis7/src/simulator/mod.rs`
- `crates/oasis7/src/viewer/live.rs`
- `crates/oasis7/src/lib.rs`
- `README.md`
- `config.example.toml`

## 状态
- 当前阶段：AOR10（已完成）
- 下一步：进入下一项 LLM 运行质量优化任务
- 最近更新：完成 AOR10（文档回写与开发日志落盘，2026-02-10）

## 后续优化跟踪（基于 30 tick 实跑）
- [ ] AOR11 降低 Responses 输出解析噪声（目标：`parse_errors` 持续下降并可稳定复现）
- [ ] AOR12 增加动作参数护栏（防止极端参数值进入动作执行层）
- [ ] AOR13 联动 Prompt 预算优化（降低 `llm_input_chars_max` 与 section 裁剪频次）

## 状态（补充）
- 当前阶段：AOR11（待启动）
- 最近更新：补充 30 tick 长跑后续优化项（2026-02-10）
