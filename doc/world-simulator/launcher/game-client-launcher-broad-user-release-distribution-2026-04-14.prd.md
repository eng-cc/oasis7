# 客户端启动器面向普通用户的跨平台发行设计（2026-04-14）

- 对应项目管理文档: `doc/world-simulator/launcher/game-client-launcher-broad-user-release-distribution-2026-04-14.project.md`
- 关联发布流水线专题: `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: 当前 release 产物已经开始收口到 `.deb` / `.dmg` / `.exe`，但它们仍更接近“技术预览可直接打开”，还不是面向普通用户的安装分发体验。普通用户最容易在下载选择、系统信任、安装完成感、更新预期和失败后的支持路径上卡住。
- Proposed Solution: 把公开下载层升级为“每个平台一个普通用户主包”的产品级发行设计：Windows 使用标准安装器，macOS 使用签名/公证后的 `.app + .dmg`，Linux 使用可直接运行的 `AppImage` 作为公开主包；站点、Release、校验与支持说明统一围绕这一主包模型组织。
- Success Criteria:
  - SC-1: 官网与 GitHub Release 对每个平台只展示一个面向普通用户的主下载包，不再要求用户在多个技术形态之间自行判断。
  - SC-2: Windows 主包安装后必须自动生成应用入口与卸载入口，不再要求用户手工解压并寻找脚本。
  - SC-3: macOS 主包必须以签名并完成 notarization 的 `.app` 形态交付，首次打开不依赖“绕过未知开发者”手册。
  - SC-4: Linux 主包必须可在支持发行版上直接运行，不依赖 apt/dpkg 或 shell wrapper 发现路径。
  - SC-5: 下载页、Release 说明和失败支持路径必须在用户下载前就明确平台支持范围、系统要求和非目标边界。

## 2. User Experience & Functionality
- User Personas:
  - 普通新用户：首次接触 oasis7，需要“下载 -> 安装/打开 -> 进入应用”尽量接近主流桌面软件体验。
  - 内容创作者 / 试玩用户：会跨版本下载更新，但不愿意阅读终端说明或手工整理 bundle。
  - 支持 / liveops 人员：需要面向用户给出统一下载口径、系统要求和失败排障入口。
- User Scenarios & Frequency:
  - 首次下载与首次启动：每个新用户 1 次，高摩擦、高流失风险。
  - 版本升级：每个活跃用户每个 release 1 次，要求旧数据和新包关系可解释。
  - 下载失败 / 系统不兼容：低频但高支持成本，必须前置说明。
- User Stories:
  - PRD-WORLD_SIMULATOR-043: As a 普通用户 / 制作人 / 支持人员, I want one clearly recommended installable or runnable package for my platform, so that I can start oasis7 without reverse-engineering bundle contents, shell scripts, or trust bypass steps.
- Critical User Flows:
  1. Flow-LAUNCHER-RELEASE-001（官网直达下载）:
     `访问官网 -> 自动识别平台并显示唯一主 CTA -> 点击下载 -> 跳转 latest/download 主包 -> 下载完成后直接安装/打开`
  2. Flow-LAUNCHER-RELEASE-002（Windows 普通安装）:
     `双击 setup.exe -> 进入标准安装向导 -> 完成安装 -> 开始菜单/桌面可见 oasis7 Client Launcher -> 首次启动`
  3. Flow-LAUNCHER-RELEASE-003（macOS 普通安装）:
     `打开 dmg -> 将 oasis7 Client Launcher.app 拖入 /Applications -> 双击启动 -> 系统不要求参考额外“绕过未知开发者”说明`
  4. Flow-LAUNCHER-RELEASE-004（Linux 直接运行）:
     `下载 AppImage -> 赋予执行权限或由桌面环境允许 -> 双击 / 运行 -> 进入 oasis7 Client Launcher`
  5. Flow-LAUNCHER-RELEASE-005（不支持平台预先止损）:
     `访问下载页 -> 看到当前仅支持的 OS / arch / 最低版本 -> 若不兼容则直接看到支持边界与后续说明，而不是下载后才失败`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 平台主包选择 | `platform_id`、`primary_asset_name`、`minimum_os`、`supported_arch` | 官网按当前 OS 给出一个主 CTA，同时保留手动切换其他平台入口 | `detected -> selected` | 仅显示每平台一个主包；高级资产不进入默认比较面板 | 所有访问者可见 |
| Windows 安装器 | `oasis7-windows-x64.exe` | 双击进入安装向导，默认安装后可立即启动 | `downloaded -> installed -> launchable` | 默认生成开始菜单入口；可选桌面快捷方式 | 不要求用户先手工解包 |
| macOS 安装器 | `oasis7-macos-x64.dmg` + `oasis7 Client Launcher.app` | 打开 dmg 后拖拽到 `/Applications`，随后直接启动 `.app` | `mounted -> copied -> launchable` | `.app` 名称、图标与版本号需一致 | 必须完成 codesign + notarization |
| Linux 主包 | `oasis7-linux-x86_64.AppImage` | 下载后直接运行，不再要求用户理解 `.deb` 与 shell wrapper 差异 | `downloaded -> runnable` | 官网主入口只推荐一个 Linux 主包；发行版特化包放次级入口 | 不要求 root / 包管理器 |
| 支持与校验信息 | `release_notes_url`、`checksums_url`、`support_boundary` | 下载页在 CTA 附近显示系统要求、版本、校验、失败排障入口 | `visible before download` | 主 CTA 优先，高级校验信息折叠展示 | 所有访问者可见 |
| 更新路径 | `update_strategy`、`data_migration_policy` | v1.1 采用“重新下载主包覆盖安装/替换”，v2 再考虑应用内更新 | `manual_update -> optional_auto_update` | 同版本覆盖安装不得丢失用户配置与本地世界目录指向 | 已安装用户可见 |
- Acceptance Criteria:
  - AC-1: 官网与 GitHub Release 公共下载面必须对每个平台只暴露一个普通用户主包，并保证命名稳定可预期。
  - AC-2: Windows 公共主包必须升级为标准安装器形态，安装后提供可发现的应用入口和卸载入口。
  - AC-3: macOS 公共主包必须是签名并 notarized 的 `.app + .dmg` 路径，首次启动不再依赖绕过 Gatekeeper 的额外文案。
  - AC-4: Linux 公共主包必须切为 `AppImage` 这类可直接运行形态，`.deb` 等发行版特化包最多作为高级入口，不得继续作为普通用户唯一入口。
  - AC-5: 下载页必须在主 CTA 附近明确系统要求、支持架构、版本与 checksums 入口。
  - AC-6: 普通用户 happy path 不允许出现“手工解压 -> 找 shell 脚本 -> 读 README 再决定启动哪个入口”的前置要求。
  - AC-7: 当前不支持的平台、架构或系统版本必须在下载前即给出结构化说明，而不是仅在安装后失败。
  - AC-8: 版本升级的默认口径必须统一为“重新下载并覆盖安装/替换”，直到应用内更新被正式建模和交付。
- Non-Goals:
  - 不在本专题中要求接入 App Store / Microsoft Store / Snap Store 等商店分发。
  - 不在 v1.1 同时交付应用内自动更新、增量 patch 或后台常驻 updater。
  - 不要求 Linux 首期覆盖所有包管理器格式；只要求普通用户主包先统一。
  - 不改动游戏玩法、runtime/world 规则或 LLM/Provider 主链路。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 功能或模型评测要求。

## 4. Technical Specifications
- Architecture Overview:
  - Release 产物分三层：`bundle 真值层 -> 平台 packager 层 -> 官网/Release 公共下载层`。
  - `bundle 真值层` 继续保证二进制、静态资源和基础入口完整。
  - `平台 packager 层` 从“技术上可运行”升级为“普通用户可安装/可打开”的 OS-native 体验：Windows 安装器、macOS 签名/公证 app、Linux AppImage。
  - `公共下载层` 只暴露每个平台一个主包，并把 checksums、系统要求和高级资产降为辅助信息。
- Integration Points:
  - `.github/workflows/release-packages.yml`
  - `scripts/build-game-launcher-bundle.sh`
  - `scripts/package-native-installer.sh`
  - `scripts/validate-release-platform-entrypoints.sh`
  - `site/index.html`
  - `site/en/index.html`
  - `scripts/site-download-check.sh`
  - `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.{prd,project}.md`
- Edge Cases & Error Handling:
  - macOS signing/notarization 失败：Release 不得继续把该 DMG 作为普通用户主包发布；需回退为 failed run，而不是发布 unsigned 资产。
  - Windows 签名或安装器生成失败：Release 不得回退到自解压 bundle `.exe` 冒充 installer。
  - Linux 桌面环境缺少 AppImage 集成：允许用户直接运行，但下载页必须提供最小执行权限说明。
  - 用户无管理员权限：Windows 方案优先采用 per-user installer，避免“必须管理员权限”成为默认阻断。
  - 平台不兼容：下载页必须先提示支持边界；Release notes 也要给出当前支持矩阵。
  - 旧版本升级：覆盖安装不得 silently 改写用户现有 world/config 目录；若有迁移，必须给出明确迁移文案。
- Non-Functional Requirements:
  - NFR-1: 官网和 `latest/download` 的主包 URL 必须长期稳定，保证外部分享和社区口径可复用。
  - NFR-2: Windows / macOS 的公共主包必须具备可验证的签名信任链；Linux 主包必须具备可验证的 SHA256。
  - NFR-3: 公共下载面默认不展示 shell wrapper、bundle 内部结构或技术预览脚本名。
  - NFR-4: 三平台主包的应用名、图标、版本号和支持说明必须保持产品口径一致。
  - NFR-5: Release workflow 必须在“普通用户主包缺失、信任链缺失、公开资产名漂移”时阻断发布。
- Security & Privacy:
  - Windows 代码签名证书、macOS 签名与 notarization 凭据必须存放在 GitHub Actions secrets / 环境中，不允许写入仓库。
  - 公共主包不得内嵌开发态调试凭据、本地路径或发布流程私有说明。
  - Checksums 继续作为公开辅助校验手段，但不替代签名 / notarization 的平台信任职责。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP（当前技术预览基线）: `.deb` / `.dmg` / `.exe` 已经具备平台原生入口，但仍允许内部 bundle / README / 脚本语义外露。
  - v1.1（普通用户可用）: Windows 切到标准安装器并签名；macOS 完成 `.app` 签名和 notarization；Linux 公共主包改为 `AppImage`；下载页改为单主 CTA + 支持边界。
  - v1.2（支持闭环）: 增加版本说明、升级指南、常见失败签名与支持入口，压缩人工排障成本。
  - v2.0（更长期）: 再评估应用内更新、差分升级、发行版特化副资产与企业环境分发。
- Technical Risks:
  - 风险-1: Windows 标准安装器与签名链路会引入新 CI 凭据和 runner 依赖。
  - 风险-2: macOS notarization 往往增加外部依赖与等待时间，若未并行设计好可能拖慢 release。
  - 风险-3: Linux 若直接切 AppImage，需要补齐桌面集成、执行权限提示与运行时依赖验证。
  - 风险-4: 公开只保留一个主包会压缩“高级用户自由选择”的空间，因此必须保留次级高级入口但不混入默认路径。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-043 | `task_5d40365b4e714e5799a3baa834e84515` / `task_22a0d58d0d9445b1ad74127c25768d8d` / `task_85eb196085584f8e9ebb9b3f988cd169` | `test_tier_required` | `./scripts/doc-governance-check.sh` + `./scripts/pm/lint.sh` + `git diff --check` + `rg -n "AppImage|notariz|codesign|NSIS|single primary asset|普通用户主包" doc/world-simulator doc/site/github-pages` | 下载页口径、Release 资产策略、平台安装体验目标、支持边界与后续实现任务拆解 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-RELEASE-CONSUMER-001 | Windows 采用标准 installer `.exe`（优先 NSIS / 同类用户态安装器） | 继续使用自解压 bundle `.exe` | 自解压包仍要求用户理解“先解压再找入口”，不符合普通用户心智。 |
| DEC-RELEASE-CONSUMER-002 | macOS 采用签名并 notarized 的 `.app + .dmg` | 继续发布 unsigned `.dmg` 或改用 `.pkg` 作为默认首选 | `.app + .dmg` 更接近普通桌面应用习惯，且不需要把安装过程复杂化。 |
| DEC-RELEASE-CONSUMER-003 | Linux 公共主包采用 `AppImage` | 继续把 `.deb` 作为普通用户唯一公开资产 | `.deb` 只覆盖 Debian/Ubuntu 系，`AppImage` 更符合“一个包直接运行”的跨发行版目标。 |
| DEC-RELEASE-CONSUMER-004 | 官网每个平台只保留一个主 CTA，其余高级资产降级 | 继续把多个技术形态并列给普通用户选择 | 普通用户最怕“下载前要做技术判断”，默认路径必须唯一。 |
