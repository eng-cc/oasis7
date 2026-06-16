#!/usr/bin/env bash
set -euo pipefail

snapshot_commit="${1:-0d6fd50849cae07bac17883cca14f141ede93196}"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

round006_current="doc/core/reviews/round-006-reviewed-files.md"
round007_current="doc/core/reviews/round-007-reviewed-files.md"
round006_history="$tmp_dir/round-006-reviewed-files.md"
round007_history="$tmp_dir/round-007-reviewed-files.md"

git show "${snapshot_commit}:${round006_current}" > "$round006_history"
git show "${snapshot_commit}:${round007_current}" > "$round007_history"

require_patterns() {
  local file="$1"
  shift
  for pattern in "$@"; do
    rg -q "$pattern" "$file"
  done
}

count_detail_rows_after_heading() {
  local file="$1"
  local heading="$2"
  awk -v heading="$heading" '
    $0 == heading { in_rows = 1; next }
    in_rows && /^\| `/ { count++ }
    END { print count + 0 }
  ' "$file"
}

round006_fields=(
  '^\| 文档路径 \|'
  '^\| 当前类型 \|'
  '^\| 目标类型 \|'
  '^\| 是否需重命名 \|'
  '^\| 是否需拆分/合并 \|'
  '^\| design 缺口 \|'
  '^\| 索引回写 \|'
  '^\| 引用回写 \|'
  '^\| 改造动作 \|'
  '^\| owner role \|'
  '^\| 状态 \|'
  '^\| 备注 \|'
)

round007_fields=(
  '^\| 文档路径 \|'
  '^\| 当前类型 \|'
  '^\| 边界判定 \|'
  '^\| 主要问题编号 \|'
  '^\| 整改动作 \|'
  '^\| 索引回写 \|'
  '^\| 引用回写 \|'
  '^\| owner role \|'
  '^\| 状态 \|'
  '^\| 备注 \|'
)

require_patterns "$round006_current" \
  '当前目标范围文档数: 870' \
  '当前已完成治理文档数: 870' \
  'ROUND-006 总范围' \
  'compact historical snapshot entrypoint'
require_patterns "$round007_current" \
  '当前目标范围文档数: 874' \
  '当前已完成复核文档数: 874' \
  'ROUND-007 总范围' \
  'compact historical snapshot entrypoint'

require_patterns "$round006_current" "${round006_fields[@]}"
require_patterns "$round007_current" "${round007_fields[@]}"

require_patterns "$round006_history" \
  '^\| 文档路径 \| 当前类型 \| 目标类型 \| 是否需重命名 \| 是否需拆分/合并 \| design 缺口 \| 索引回写 \| 引用回写 \| 改造动作 \| owner role \| 状态 \| 备注 \|$'
require_patterns "$round007_history" \
  '^\| 文档路径 \| 当前类型 \| 边界判定 \| 主要问题编号 \| 整改动作 \| 索引回写 \| 引用回写 \| owner role \| 状态 \| 备注 \|$'

require_patterns "$round006_history" \
  '当前目标范围文档数: 870' \
  '当前已完成治理文档数: 870' \
  '^## 逐文档清单$'
require_patterns "$round007_history" \
  '当前目标范围文档数: 874' \
  '当前已完成复核文档数: 874' \
  '^## 明细$'

round006_rows="$(count_detail_rows_after_heading "$round006_history" '## 逐文档清单')"
round007_rows="$(count_detail_rows_after_heading "$round007_history" '## 明细')"

if (( round006_rows < 870 )); then
  echo "ROUND-006 historical detail rows below denominator: $round006_rows < 870" >&2
  exit 1
fi

if (( round007_rows != 874 )); then
  echo "ROUND-007 historical detail rows mismatch: $round007_rows != 874" >&2
  exit 1
fi

echo "doc-evidence-snapshot-check: OK"
echo "ROUND-006 historical detail rows: ${round006_rows} (denominator 870; duplicate/backfill-aware)"
echo "ROUND-007 historical detail rows: ${round007_rows}"
