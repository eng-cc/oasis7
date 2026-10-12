# 游戏发布前的测试手册(你不能改这个文档)

- 对应设计文档: `doc/playability_test_result/game-test.design.md`
- 可变任务状态与历史: GitHub task issue evidence comments

审计轮次: 5

- 执行证据与任务状态: GitHub task issue evidence comments

## 可玩性测试
你就是游戏玩家，不能读其他的文档或代码（`agent-browser` 操作方法文档除外），直接通过 `agent-browser` 来实际玩游戏，并填写 `playability_test_card`，同时需要在测试过程中给 `agent-browser` 录屏，方便后续检查

以下启动准备由测试编排者完成；实际玩家会话继续遵守上述黑盒限制。构建、登录或工具阻断应记录为执行前置问题，不补造游玩动作或继续意愿。

<a id="game-test-entry-contract"></a>

## 一键启动（避免参数错误）
当前 worktree 回归统一使用脚本启动，不要手工拼接 `oasis7_viewer_live` 与 `run-viewer-web.sh` 参数：

```bash
./scripts/worktree-harness.sh up
```

脚本会自动完成：
- 启动 `oasis7_game_launcher`（默认托管游戏进程、Web 静态服务与 WebSocket bridge）
- 按当前控制面参数启动内置 Web viewer 服务（不再单独启动 `run-viewer-web.sh` 进程）
- 端口与主页就绪检查
- 通过 `./scripts/worktree-harness.sh url` 读取本次实际 URL，不复制固定端口或其他 worktree 的地址

正式 L4B / 发布前玩家体验的默认目标为 release 产物与正式 UI。准备者须记录实际 bundle/build；URL 带 `test_api=1` 也不会让 release 安装 `__AW_TEST__`，进入游戏不等待该接口。显式 visual-test 产物只用于所声明的受控回归，不能代签 release UI 试玩。若使用 producer 启动器，以它本次输出的 URL 为准；启动器打开的另一个 headed 窗口不代表下述执行会话已经登录。

<a id="game-test-session-evidence"></a>

## `agent-browser` 进入游戏

下面默认读取 harness URL。使用 producer 时，准备者先把它本次输出的实际地址赋给 `GAME_TEST_URL`，再运行公共开页步骤。准备完成后保留浏览器，将 `session info` 中本次 owned session ID 交给实际玩家；后续独立 shell 显式使用该已记录 ID，不重新生成会话。

```bash
set -euo pipefail
command -v agent-browser >/dev/null || { echo "missing agent-browser" >&2; exit 1; }
agent-browser --version
agent-browser doctor --offline --quick --json
if [[ -z "${GAME_TEST_URL:-}" ]]; then
  GAME_TEST_URL="$(./scripts/worktree-harness.sh url)"
fi
test -n "$GAME_TEST_URL"
AGENT_BROWSER_SESSION="$(agent-browser session id --scope worktree --prefix game-test)"
export AGENT_BROWSER_SESSION
agent-browser --session "$AGENT_BROWSER_SESSION" --headed open "$GAME_TEST_URL"
agent-browser --session "$AGENT_BROWSER_SESSION" wait --load domcontentloaded
agent-browser --session "$AGENT_BROWSER_SESSION" session info --json
agent-browser --session "$AGENT_BROWSER_SESSION" snapshot -i
# 准备阶段不关闭浏览器；实际玩家接续使用已记录的同一 owned session。
```

- 对默认 `hosted_public_join`，在实际执行 session 识别 guest，经正式登录/注册成为 authenticated，再确认当前 player 与 gameplay readiness。恢复已有会话也须重新核对身份；不得用另一窗口的认证状态代签，不自动签发 Hosted 身份或切换 trusted local。
- 缺账号/配置、登录失败、身份不匹配或产物/驱动不匹配时停止相应路径，保留当前尝试的停止阶段、原因与已有证据；无实际试玩时保持 L4B `missing`，不写“玩家不愿继续”。凭据输入期间暂停/不采集录屏与截图，确认脱敏后开始/恢复玩法录屏，日志也不得包含凭据。
- 将实际 URL（敏感值脱敏）、owned session、候选/build、执行模式、当前 attempt 和 RunResult 引用写入本次卡片/summary。记录玩家动作、可归因结果、理解、下一决策及继续理由；进度增长或 ACK 不能自动填杠杆满分，缺观察保持未知，评审结论由制作人/QA 核对。
- 编排者在完成、阻断或取消后，先停录并保存卡片、已有证据和初步结果，再用已记录的 session ID 执行 `agent-browser --session "$AGENT_BROWSER_SESSION" close`；仅对本次拥有的 harness 栈执行 `./scripts/worktree-harness.sh down`。实际清理后追加退出结果与残留资源，失败不能吞掉或覆盖原停止原因。producer 所有的进程由其生命周期收尾；不要关闭其他任务的会话或栈。
