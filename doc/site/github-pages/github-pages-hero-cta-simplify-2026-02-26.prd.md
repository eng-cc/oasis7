# GitHub Pages 首屏 CTA 收敛与文案校准（2026-02-26）设计文档

- 对应设计文档: `doc/site/github-pages/github-pages-hero-cta-simplify-2026-02-26.design.md`
- 对应项目管理文档: `doc/site/github-pages/github-pages-hero-cta-simplify-2026-02-26.project.md`

审计轮次: 5

## 治理状态（2026-06-18）
- 状态：历史压缩专题，保留原址与互链用于追溯，不再作为默认活跃阅读面。
- 当前入口：`doc/site/project.md`、`doc/site/prd.index.md` 与 `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`。
- 保留原因：本文记录 2026-02-26 首屏 CTA 收敛的原始范围、接口约束与风险，不替代后续首页公开叙事或下载链路真值。

## ROUND-002 主从口径
- 主入口统一指向 `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`，本文仅维护增量。

- 对应标准执行入口: `doc/site/github-pages/github-pages-hero-cta-simplify-2026-02-26.project.md`

## 目标
- 将首页 Hero 区 CTA 从 4 个收敛为 2 个，降低首次访问决策成本。
- 主按钮改为“试玩/观看演示”语义，避免“30 秒进入首局”带来的预期落差。
- 保持“游戏优先、引擎后置”叙事不变，不破坏现有站点交互兼容性。

## 范围
- 范围内
  - 更新 `site/index.html` Hero CTA 结构与中文按钮文案。
  - 更新 `site/en/index.html` Hero CTA 结构与英文按钮文案（同构）。
- 范围外
  - 不调整 section 顺序与内容结构。
  - 不修改 `site/assets/app.js` 交互逻辑。
  - 不改文档中心页与 Rust 代码。

## 接口/数据
- 保持锚点与交互标记兼容：
  - Hero 主按钮继续使用 `href="#demo"`。
  - 次按钮继续指向 `site/doc/cn/index.html`、`../doc/en/index.html`。
  - 不新增或删除 `data-*` 交互字段。
- 文案变更：
  - 中文主 CTA：由“30 秒进入首局”调整为“试玩与演示入口”语义。
  - 英文主 CTA：同步为 “Try Demo Flow” 语义。

## 里程碑
- M0：建档并确认改动范围。
- M1：完成 CN/EN Hero CTA 收敛与主文案重写。
- M2：执行校验、回写项目状态与任务日志。

## 风险
- 风险：删除 Hero 的 GitHub 直链后，技术型用户可能减少首屏外链点击。
  - 缓解：保留“文档中心”入口，并在页面下半部分保留深入阅读入口。
- 风险：CTA 文案过于中性导致点击率下降。
  - 缓解：主按钮强调“试玩/演示”动作，次按钮保持“文档”明确分流。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
