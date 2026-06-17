# Unified World Hosted Access Announcement Template

审计轮次: 1

## Meta
- `world_id`:
- `deployment_mode`: `hosted_public_join`
- `operator_owner`:
- `channel`: `group` / `social` / `dm` / `other`
- `public_join_url`:
- `preview_window`:

## 使用规则
- 对外只出现 `public_join_url`，不要带 operator/control URL。
- 对外口径只允许使用 `limited playable technical preview`，不要升级成 hosted access readiness / `production-ready` 承诺。
- 若统一持久大世界的 hosted access 仍在收口 hardening，可直接写“可能需要重新获取 hosted player session”，不要承诺零中断。
- 如果已经发生过误分享或 revoke，优先改用 `hosted-world-share-correction-template.md`。

## Short Announcement
适合群消息、评论区、快速转发。

```text
玩家入口如下：
<public_join_url>

这是一个 limited playable technical preview。请只通过上面的玩家入口进入；如果页面提示 Hosted Recovery / Re-acquire Hosted Player Session，请按页面提示重新获取会话。
```

## Full Announcement
适合公告、社区帖、置顶消息。

```text
本次统一持久大世界的 hosted player entry 如下：

<public_join_url>

当前阶段仍是 limited playable technical preview。请只使用上面的玩家入口进入世界，不要把其他控制台或管理地址当成玩家链接传播。

如果你打开页面后看到 Hosted Recovery / Re-acquire Hosted Player Session，请按页面提示重新获取会话；这属于当前 preview 阶段的正常受控行为，不代表世界已经进入 production-ready 状态。
```

## Repeat Reminder
适合二次提醒正确入口，避免群内旧链接继续传播。

```text
提醒：当前有效的玩家入口只有这一条：
<public_join_url>

统一持久大世界当前仍处于 limited playable technical preview。请不要转发其他地址；若你保留了旧页面，请关闭后通过上述玩家入口重新进入。
```

## Internal Record
- `posted_at`:
- `posted_by`:
- `message_variant`: `short` / `full` / `repeat_reminder`
- `claim_boundary_checked`: `yes` / `no`
- `operator_url_excluded`: `yes` / `no`
