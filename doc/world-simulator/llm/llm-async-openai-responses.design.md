# Retired Direct Model Path Design Tombstone

- 对应需求文档: `doc/world-simulator/llm/llm-async-openai-responses.prd.md`
- 对应项目管理文档: `doc/world-simulator/llm/llm-async-openai-responses.project.md`

## 1. 设计定位
该历史设计路径已废弃，仅保留为文档治理 tombstone。游戏端不再维护直连模型客户端、模型 SDK 依赖、模型 endpoint 配置或客户端侧模型鉴权。

## 2. 设计结构
- 当前有效路径：游戏端只通过 provider-backed agent decision flow 接入远端 provider bridge。
- 云端 provider bridge 可自行连接 OpenAI-compatible 上游，但该能力不属于游戏端 runtime 或 viewer binary。
- 本文件不得作为恢复直连客户端、SDK 依赖或本地模型密钥配置的依据。

## 3. 关键接口 / 入口
- 无 active 游戏端接口。
- 历史入口已在 MVP playtest readiness hardening 中移除。

## 4. 约束与边界
- 不在游戏端恢复直连模型客户端。
- 不在游戏端新增模型 API key、base URL、model name 等直连配置。
- provider bridge 的上游实现由 bridge 服务边界承接。

## 5. 设计演进计划
- 无后续游戏端实施计划。
- 如需扩展模型能力，应在 provider bridge 协议与云端服务中演进。
