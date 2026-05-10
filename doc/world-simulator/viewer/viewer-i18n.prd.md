# Viewer UI 多语言支持设计（中文 / 英文）

- 对应设计文档: `doc/world-simulator/viewer/viewer-i18n.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-i18n.project.md`

审计轮次: 5

## 1. Executive Summary
- 为 `oasis7_viewer` UI 系统引入可扩展的多语言机制，首批支持 `zh-CN` 与 `en-US`。
- 消除当前 UI 文案硬编码分散在多个模块的问题，统一文本键与翻译入口。
- 在不改变核心模拟与渲染逻辑的前提下，实现文案可切换、可回退、可测试。
- 保持现有 headless UI 测试能力，并补充多语言断言，避免后续文案回归。
- 提供界面内语言切换选项（中文 / English），不引入命令行参数切换入口。

## 2. User Experience & Functionality
- **范围内**
  - `crates/oasis7_viewer/src` 下 UI 文案集中治理，覆盖：
    - `main.rs`（状态栏、面板标题、按钮文案）
    - `ui_text.rs`（摘要、详情、事件文本模板）
    - `diagnosis.rs`（诊断结论文案）
    - `timeline_controls.rs`（时间轴文案）
    - `panel_layout.rs`（顶部折叠区文案）
  - `event_click_list.rs` / `selection_linking.rs` / `world_overlay.rs`（交互提示文案）
  - 增加界面内语言选择入口（Top Controls 区域），并提供默认语言与回退策略。
  - 为中文、英文建立同构词条集（同一 key 在两种语言均有翻译）。
  - 新增/更新 UI 单元测试，覆盖中英两套关键文案输出。
- **范围外**
  - 暂不引入“按用户系统语言自动切换”逻辑（首版默认中文，手动切换）。
  - 暂不引入第三方 i18n 依赖（如 ICU/Fluent），先用项目内轻量实现。
  - 暂不翻译协议字段、事件枚举 `Debug` 字符串等底层调试输出（仍可保留英文枚举名）。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 1) 语言模型
- 新增 `UiLocale`：
  - `ZhCn`
  - `EnUs`
- 新增 `UiI18n`（Bevy Resource）：
  - `locale: UiLocale`
  - 提供 `text(key)` 与 `format(key, args)` 两类查询。

### 2) 文本键设计
- 新增统一文本 key 枚举（示例）：
  - `StatusConnecting` / `StatusConnected` / `StatusError`
  - `PanelTopControls`
  - `TimelineSeek` / `TimelineNowTargetMax`
  - `DiagnosisWaitingWorldData` / `DiagnosisDisconnected`
  - `SelectionNone` / `DetailsClickToInspect`
  - `EventsEmpty` / `EventsFocused`
- 约束：
  - UI 中不直接写最终显示文案，只能引用 key。
  - 动态文案通过模板参数渲染，禁止拼接跨语言固定片段。

### 3) 词条数据结构
- 采用内置静态词典（Rust 常量或静态映射）：
  - `catalog_zh_cn`
  - `catalog_en_us`
- 每个 key 必须在两套词典中都存在；缺失时按回退链处理：
  1. 当前语言
  2. `en-US`
  3. key 名称（仅用于开发期告警）

### 4) 语言选择策略
- 通过 UI 顶部控制区新增语言切换控件（示例：`Language: 中文 | English`）。
- 默认语言为 `zh-CN`，用户点击后可即时切换到 `en-US`，并触发 UI 文案刷新。
- 可选持久化：将最近选择写入本地配置（如 viewer 本地配置文件），下次启动沿用上次选择。
- 若持久化值未知或损坏：记录 warning，并回退到 `zh-CN`。

### 5) 模板参数规范
- 动态模板统一使用命名参数（如 `{agent_id}`、`{time}`、`{reason}`）。
- 中文与英文模板必须保持参数集合一致，避免单侧缺参。
- 关键诊断/错误文案要求“短句 + 可定位信息”，避免过长导致 UI 折行不可读。

### 6) 测试数据与验收
- 单元测试覆盖：
  - 语言切换交互（点击切换、切换后文案更新）
  - 默认语言与持久化恢复（无配置/有配置/非法配置）
  - 翻译查询与回退逻辑
  - 关键模块在 `zh-CN` 与 `en-US` 下的文本输出
- 回归断言以“关键片段包含”优先，降低因标点微调导致的脆弱失败。

## 5. Risks & Roadmap
- **M1：文本盘点与 key 清单**
  - 汇总现有硬编码文案，形成 key 对照表（旧文案 -> 新 key）。
- **M2：i18n 基础设施落地**
  - 新增 `UiLocale`/`UiI18n`/词典与默认语言逻辑。
- **M3：模块迁移与测试补齐**
  - 将 `main.rs`、`ui_text.rs`、`diagnosis.rs`、`timeline_controls.rs` 等迁移到 key 查询。
  - 在 Top Controls 增加语言切换控件并完成中英文关键场景测试。
- **M4：收敛与文档更新**
  - 清理残留硬编码文案，更新可视化文档与项目管理文档状态。

### Technical Risks
- **词条漂移风险**：新增 UI 功能若未同步补 key，会引入“部分文案未翻译”。
  - 缓解：在测试中增加“词条完整性检查”（两套词典 key 集一致）。
- **模板参数错配**：中英文模板参数名不一致会导致运行时格式化错误。
  - 缓解：对模板 key 增加参数校验测试。
- **布局可读性风险**：中英文本长度差异可能导致按钮挤压、面板换行异常。
  - 缓解：对关键按钮区与右侧面板保留最小宽度/换行策略回归测试。
- **默认语言预期不一致**：多人协作时可能误以为会跟随系统语言。
  - 缓解：首版统一默认中文（`zh-CN`）并提供显式 UI 切换控件。

## 备注
- 本文档仅定义“中英双语首版”方案，后续新增语言（如 `ja-JP`）可复用同一 key 与词典机制扩展。

## 增量补充（2026-02-07）：中文字体渲染
- 问题：UI 默认字体 `DejaVuSans.ttf` 不包含完整 CJK 字形，中文文案会显示为方块字（tofu）。
- 方案：在 viewer 资产目录引入 `ms-yahei.ttf`，并统一将 UI/3D 标签字体切换为 `fonts/ms-yahei.ttf`。
- 约束：保持“仅界面切换语言，不新增命令行参数”的既有策略不变。
- 验收：通过 `software_safe` Web 闭环确认中文文本可读，并在状态、日志与截图中留证。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
