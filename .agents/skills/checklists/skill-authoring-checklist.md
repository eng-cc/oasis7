# Skill Authoring Checklist

## Scope Fit

- [ ] 这是 repo-owned / 明确场景专属能力，而不是一次性 task 复盘
- [ ] 这份内容更适合做 skill，而不是写进 `AGENTS.md`、模块 `prd.md` / `project.md`、handoff 或脚本校验

## Frontmatter

- [ ] `name` 使用小写字母、数字、连字符
- [ ] `description` 以 `Use when...` 开头
- [ ] `description` 只写触发条件，不复述完整 workflow
- [ ] frontmatter 与文件夹命名、后续引用保持一致

## Body

- [ ] 写清 `When to Use`
- [ ] 写清 repo-specific workflow / pattern / helper
- [ ] 写清 oasis7 相关路径、命令、review 边界或验证入口
- [ ] 写清 guardrails / anti-patterns
- [ ] 如需 supporting files，理由是 heavy reference 或 reusable tool，而不是把普通正文拆碎

## Truth Alignment

- [ ] 引用到的命令、脚本、路径、helper 在仓库里真实存在
- [ ] 没有残留 upstream 安装说明、无关 harness 说明或过期平台话术
- [ ] 若 skill 改变推荐方法，相关 role card / topic doc / project truth 已同步回写
- [ ] 若 skill 只是 bounded borrowing，已明确哪些部分仍保持 deferred / rejected

## Verification

- [ ] 至少跑过一轮 repo-owned 验证，而不是只靠静态阅读
- [ ] 文档类改动已跑 `./scripts/doc-governance-check.sh`
- [ ] 任务收口前已跑 `./scripts/pm/lint.sh`
- [ ] diff 干净可提交：`git diff --check`

## Optional

- [ ] 若来自外部源，已判断是否需要更新 `skills-lock.json`
- [ ] 若 skill 预期会被角色频繁触发，已考虑是否要补 role card 推荐入口
