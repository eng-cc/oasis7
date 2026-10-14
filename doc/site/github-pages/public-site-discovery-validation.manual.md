# 官网发现与静态内容验证方法

- 主责：viewer_engineer / qa_engineer
- 系统设计：[官网发现与展示](public-site-discovery-and-showcase.design.md#verification)
- 上游要求：[产品 AC](../../product/player-entry-distribution/public-site-discovery-and-showcase.prd.md#ac-site-discovery-001)

## 自动检查

执行 `./scripts/site-link-check.sh`、`./scripts/site-homepage-claim-check.sh`、`./scripts/site-download-check.sh` 与 `./scripts/site-manual-sync-check.sh`。首页门禁保留既有公开口径检查并核对四步静态阅读路径、正文/摘要/RSS 与双语身份；下载门禁覆盖实际候选资产、缺失 checksum、混版/重复/不可信链接。结构检查不证明原材料真实、真人理解或线上可用。

## 外部浏览器验证

用 `python3 -m http.server 48760 --bind 127.0.0.1 --directory site` 启动本候选，使用安装了 Playwright 的 Node 环境，设置 `OASIS7_PLAYWRIGHT_MODULE` 为模块解析路径（已在 Node 搜索路径时可省略）。设置 `OASIS7_SITE_QA_BROWSER` 为实际外部 Chrome 可执行文件；`OASIS7_SITE_QA_URL` 可覆盖静态服务器 URL。执行 `node scripts/site-discovery-browser.test.cjs`。默认 headed，显式设置 `OASIS7_SITE_QA_HEADLESS=1` 只覆盖自动行为，不代签外部 headed 图形体验。

此脚本用受控只读响应覆盖发行元数据成功/失败、缺资产/checksum、未知设备、无脚本、键盘菜单/步骤、200% 文字缩放、reader 空/错误/显式重试/异章迟到/同章迟到缓存，以及项目子路径的显式语言映射。产物写入忽略的 `output/site-discovery-qa/`。fixture 不证明真实资产、生产环境或模型行为。脚本异常保留错误，不以后来 PASS 改写旧失败。

另外按 [Site/S6 分层](../../../testing-manual.md#testing-s6-qa-receiver)为首页 CN/EN、观测列表、开发日志列表/详情及小说入口保留 1440×900、390×844、360×800 的真实截图；核对标题换行、主次 CTA、预览限制、来源类型、正文可读性、无横向溢出与可达恢复入口。无对应小说语言不制造路由。完整浏览器缩放、对比度、资源加载量/LCP/布局偏移另行测量，不以文字缩放或截图存在代签。

## 内容发布与纠正

观测与开发日志均使用同一详情/摘要/RSS 合同，分别要求 kind=observation/devlog。观测必须提供可见 provenance 标签；runtime_capture 额外要求固定 source version、build、Runtime profile、scenario、window 及指向该提交的受信来源链接。字段检查只证明关联完整，不证明材料真实，公开来源仍需专业核对。详情是唯一正文编辑源。同步同语言首页摘要、列表与 RSS；id/slug、标题、类型、日期、语言、来源版本、链接和状态一起核对。首期仅维护显式双语内容集合；缺少译文时保留实际语言提示，不伪造正文。运行记录未取得合法来源时保留空列表，不用概念图填成实机证据。

双语同一内容必须保持 publication status 一致；不能只撤回一种语言而让译文、推荐或订阅源继续公开旧内容。撤回使用同 URL 与 RSS GUID，删除 `data-content-body` 和推荐摘要，新增非空 `data-content-correction`，RSS 描述与安全纠正一致。不得只删 `data-content-summary` 属性却继续公开原文；敏感信息仍按现有事件/历史清理主责处理。本方法不承诺清除第三方缓存。

真人理解按 AC 独立观察：记录陌生访客对玩家/Agent 分工、证据类型、成本与当前参与边界的原话和误解。开发者自述、模型回答、门禁与截图均不能代签真人理解；游戏运行、安装和发行继续使用各自权威验证。
