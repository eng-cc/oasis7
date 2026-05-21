## 2026-05-21 20:28:00 CST / producer_system_designer

- 完成内容:
  - 复核 `superpowers-conflict-reconciliation-2026-05-20.md` 的 `## 4. Skill-by-skill 冲突与互借表`，确认表内仍把若干已吸收的 bounded borrowing 写成“状态未更新”的可读性问题。
  - 为该表新增状态说明，并把 `writing-plans`、`executing-plans`、`writing-skills` 三行改成“upstream 整体裁决 / 已完成 bounded borrowing / remaining deferred 条件”显式分离的写法。
  - 同步回写 topic/root project 与 `.pm` task，保证这次表格真值刷新可追溯。
- 遗留事项:
  - 这次只修表格表达，不变更 upstream skill 的正式 adopted / deferred / rejected 裁决；若后续要改裁决本身，仍需另建独立 task。
