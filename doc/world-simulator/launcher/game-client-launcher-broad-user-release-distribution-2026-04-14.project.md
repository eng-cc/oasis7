# 客户端启动器面向普通用户的跨平台发行设计（2026-04-14）项目管理文档

- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-broad-user-release-distribution-2026-04-14.prd.md`
- 关联发布流水线专题: `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.project.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] consumer-release-distribution-design (PRD-WORLD_SIMULATOR-043) [test_tier_required]: 完成普通用户向跨平台发行专题 PRD / project 建模，明确三平台主包、支持边界、分阶段门禁，并回写 `world-simulator` 主 PRD/project 与 release pipeline 文档。 Trace: .pm/tasks/task_5d40365b4e714e5799a3baa834e84515.yaml
- [x] windows-standard-release-installer (PRD-WORLD_SIMULATOR-043) [test_tier_required]: 将 Windows 公共主包从自解压 bundle `.exe` 升级为标准安装器，补齐开始菜单/桌面/卸载入口，并将 `Release Packages` 的 Windows 打包依赖从 `7zip` 切到 `NSIS/makensis`。 Trace: .pm/tasks/task_22a0d58d0d9445b1ad74127c25768d8d.yaml
- [ ] windows-release-codesign-trust-chain (PRD-WORLD_SIMULATOR-043) [test_tier_required]: 为 Windows 公共主包补齐代码签名、Publisher trust chain 与证书型 CI 阻断；在仓库未配置签名凭据前，不宣称 Windows 已达到普通用户“无安全警告”目标。 Trace: .pm/tasks/task_1bf477181357609321ff82e92ca9050e.yaml
- [ ] macos-notarized-release-dmg (PRD-WORLD_SIMULATOR-043) [test_tier_required]: 将 macOS 公共主包升级为签名并 notarized 的 `.app + .dmg`，补齐图标、应用元数据、Gatekeeper 通过路径与 CI 阻断。 Trace: .pm/tasks/task_f7299eb703f78c1ae0d2086fc4b88c29.yaml
- [x] linux-appimage-primary-release-asset (PRD-WORLD_SIMULATOR-043) [test_tier_required]: 将 Linux 公共主包切到 `AppImage`，同时把 `.deb` 降级为高级/次级入口，并补齐支持发行版说明。 Trace: .pm/tasks/task_85eb196085584f8e9ebb9b3f988cd169.yaml
- [x] site-single-primary-release-cta (PRD-WORLD_SIMULATOR-043) [test_tier_required]: 改造 `site/index.html`、`site/en/index.html`、`site/assets/{app.js,styles.css}` 与 `scripts/site-download-check.sh`，使官网按平台只展示一个主 CTA，并前置显示系统要求、checksums 与失败支持路径。 Trace: .pm/tasks/task_3ec0a2b3318344209a2569fa294d05d1.yaml
- [x] release-upgrade-path-overwrite-policy (PRD-WORLD_SIMULATOR-043) [test_tier_required]: 基于当前代码真值收口“重新下载覆盖安装/替换”的升级口径，明确 `config.toml`、`.oasis7_launcher_ux_state.json` 与 `output/chain-runtime/<node_id>/reward-runtime-execution-world/` 仍按实际启动工作目录解析；Windows 卸载器会删除 `$INSTDIR`，因此“卸载重装”不得被写成保留本地状态的等价升级路径。同步更新官网下载文案、bundle README 与 release pipeline 文档，并把应用内更新继续保留为后续独立专题。 Trace: .pm/tasks/task_30d0e01f74a3ced99f104893beb9d53f.yaml

## 依赖
- `doc/world-simulator/prd.md`
- `doc/world-simulator/project.md`
- `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.{prd,project}.md`
- `.github/workflows/release-packages.yml`
- `scripts/{build-game-launcher-bundle.sh,package-native-installer.sh,validate-release-platform-entrypoints.sh,site-download-check.sh}`
- `site/{index.html,en/index.html}`

## 状态
- 最近更新：2026-04-15
- 当前阶段: in_progress
- 当前任务: T0/T1/T3/T4/T5 已完成；T1A/T2 待继续实施
- 备注: 当前官网下载面已经切到“按平台单主 CTA + 支持边界 + checksums 前置”的默认决策面；升级口径也已明确为“手动覆盖安装/替换 + 用户自备份相对路径状态”。Windows 代码签名与 macOS notarization 仍未闭环，因此整体普通用户发行目标尚未完成。
