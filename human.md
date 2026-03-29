本文档专为开发者准备，不建议agent读写

## DONE
- 解决 codex 沙盒的网络权限问题（codex现在已经支持单独配置网络权限）
- 想办法实现多Agent在这个仓库写代码，合到主分支(codex worktree)

## TODO
- 找到办法定期检查：检查文档描述的完成情况与实际代码的实际实现是否匹配
- 补充路线图

## run game
```bash
./scripts/build-game-launcher-bundle.sh --out-dir output/release/game-launcher-local --profile dev

output/release/game-launcher-local/run-client.sh
```

## env

npm install -g agent-browser
agent-browser install  # Download Chromium
