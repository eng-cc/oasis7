# GitHub Pages 架构图 SVG 精修（四期增量）设计文档

- 对应设计文档: `doc/site/github-pages/github-pages-architecture-svg-refresh.design.md`
- 对应项目管理文档: `doc/site/github-pages/github-pages-architecture-svg-refresh.project.md`

审计轮次: 5

## ROUND-002 主从口径
- 主入口文档：`doc/site/prd.md`。
- 本文件仅维护架构图 SVG 精修增量内容。

- 对应标准执行入口: `doc/site/github-pages/github-pages-architecture-svg-refresh.project.md`

## 目标
- 提升首页架构图 `architecture-flow.svg` 的科技感与信息层次，让用户一眼看懂系统闭环。
- 在不增加图片体积负担的前提下，保持 SVG 的清晰缩放与静态部署友好性。
- 保持现有页面接入路径不变，只替换图稿内容。

## 范围
- 范围内
  - 重绘 `site/assets/images/architecture-flow.svg`：主链路层、数据证据层、反馈控制层三层结构。
  - 优化视觉语言：更强的节点区分、箭头流向、科技网格和高亮带。
  - 优化文案密度：每个节点保留一句职责说明，避免图内文字过载。
- 范围外
  - 改动页面结构或 JS 交互逻辑。
  - 新增位图素材（PNG/WebP）替代 SVG。

## 接口/数据
- 主要文件
  - `site/assets/images/architecture-flow.svg`
- 引用关系
  - `site/index.html` 与 `site/en/index.html` 继续使用同一文件路径，无需改引用。
- 可访问性
  - 保留 `title/desc/role/aria-labelledby` 语义标记。

## 里程碑
- M1：文档与任务拆解
  - 输出设计文档与项目管理文档。
- M2：图稿重绘
  - 完成主流程节点、连接箭头、底层能力带与背景层。
- M3：验证收口
  - 校验 SVG 结构可用、文件引用不变，更新项目管理文档与 devlog。

## 风险
- 风险：信息元素过多导致图面拥挤
  - 缓解：每层最多 4 个主节点，文字长度控制在一行。
- 风险：颜色对比不足影响阅读
  - 缓解：提升主标题/节点文本与背景对比，弱化装饰线透明度。
- 风险：不同视口缩放后文字不清晰
  - 缓解：使用适中字号和清晰边距，避免极细描边。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
