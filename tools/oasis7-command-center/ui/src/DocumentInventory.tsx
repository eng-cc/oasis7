import { createEffect, createMemo, createSignal, For, on, Show } from 'solid-js';
import { bridge, errorMessage, observedTime } from './bridge';
import type { Snapshot, DocumentSection } from './types';
import type { DocumentInventoryPage, IndexedDocument } from './generated/core';

const reasons: Record<string, string> = {
  deleted: '工作树文件已删除', permission_denied: '没有读取权限',
  read_error: '读取失败', path_unavailable: '路径不可用', invalid_utf8: '不是有效 UTF-8',
  file_count_budget: '超过 3000 文件读取预算', file_budget: '超过单文件 1 MiB',
  total_budget: '超过总量 16 MiB', symlink: '符号链接未读取',
  not_regular_file: '不是普通文件', unsupported_confined_read: '当前平台不支持受限读取',
};
const pairingIssues: Record<string, string> = {
  'multiline pairing declaration; projection incomplete': '声明跨多行，目前仅解析首行链接',
  'pairing declaration exceeds 4096 bytes; projection incomplete': '声明超过 4096 字节，未解析该字段',
  'pairing target limit 32 exceeded; projection incomplete': '超过 32 个目标，仅展示前 32 条',
  'pairing declaration has no explicit Markdown link': '声明中没有可识别的 Markdown 链接',
};
const resolutions: Record<string, string> = { resolved: '目标文件已读取', fragment_unchecked: '文件已读取，章节锚点尚未核对', missing: '目标已删除', unread: '目标尚未读取', outside_scope: '目标不在本次清单或路径不安全' };
const states: Record<string, string> = { read: '已读取', skipped: '已跳过', deleted: '已删除', error: '读取错误' };
const categories: Record<string, string> = { proposed: '提议', draft: '草案', active: '现行', superseded: '已替代', retired: '已退役', other: '其他声明', unknown: '未声明' };
const kinds: Record<string, string> = { entry: '结构入口', prd: '需求', design: '设计', other: '其他文档' };

export default function DocumentInventory(props: {
  snapshot: Snapshot;
  showSource: (section: DocumentSection, origin?: HTMLElement) => void;
}) {
  const [page, setPage] = createSignal<DocumentInventoryPage>();
  const [selected, setSelected] = createSignal<IndexedDocument>();
  const [query, setQuery] = createSignal('');
  const [filter, setFilter] = createSignal('all');
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal('');
  const [composing, setComposing] = createSignal(false);
  let generation = 0;
  let applied = { query: '', filter: 'all' };
  let list: HTMLDivElement | undefined;

  async function load(next = false) {
    const projectId = props.snapshot.project?.id;
    if (!projectId) return;
    const request = ++generation;
    const current = page();
    if (!next) {
      applied = { query: query().trim(), filter: filter() };
      setPage(undefined);
      setSelected(undefined);
    }
    setBusy(true); setError('');
    try {
      const result = await bridge.documentInventory({
        schemaVersion: 1, projectId,
        snapshotId: next ? current?.snapshotId ?? null : null,
        query: applied.query, readFilter: applied.filter,
        cursor: next ? current?.nextCursor ?? null : null,
      });
      if (request === generation && props.snapshot.project?.id === projectId) {
        setPage(result); setSelected(undefined);
        if (next) queueMicrotask(() => list?.focus());
      }
    } catch (failure) {
      if (request === generation) setError(errorMessage(failure));
    } finally { if (request === generation) setBusy(false); }
  }
  const sourceIdentity = createMemo(() => JSON.stringify([
    props.snapshot.project?.id, props.snapshot.sources.find(s => s.kind === 'local')?.generation,
  ]));
  createEffect(on(
    sourceIdentity,
    () => {
      ++generation; setPage(undefined); setSelected(undefined); setQuery(''); setFilter('all'); setError('');
      if (props.snapshot.project) void load();
    },
  ));
  const chapters = () => {
    const file = selected();
    return file?.state === 'read' ? props.snapshot.sections.filter(s => s.path === file.path && s.revision === file.revision) : [];
  };
  return <section class="panel" aria-label="文档文件清单">
    <div class="panel-title"><h3>文件清单与读取缺口</h3><button disabled={busy()} onClick={() => void load()}>重新查询首页</button></div>
    <div class="panel-body">
      <p class="muted small">仅列当前工作树纳入索引的 tracked Markdown，未跟踪文件不在此范围。按文件去重；读取状态不代表设计或实现完成。详情展示文件头的显式配对系统设计声明；尚未提供产品/专业树和配对跳转。</p>
      <form class="inventory-filters" onSubmit={event => { event.preventDefault(); if (!composing()) void load(); }}>
        <label>标题或路径<input aria-label="筛选文档文件" value={query()} onInput={e => setQuery(e.currentTarget.value)} onCompositionStart={() => setComposing(true)} onCompositionEnd={() => setComposing(false)} onKeyDown={e => { if (composing() || e.isComposing || e.keyCode === 229) { if (e.key === 'Enter') e.preventDefault(); } }} /></label>
        <label>读取结果<select aria-label="文档读取结果" value={filter()} onChange={e => setFilter(e.currentTarget.value)}><option value="all">全部文件</option><option value="read">已读取</option><option value="omitted">仅看遗漏</option></select></label>
        <button disabled={busy() || composing()} type="submit">应用筛选</button>
      </form>
      <Show when={busy()}><p role="status">正在读取文件清单…</p></Show>
      <Show when={error()}><div class="notice" role="alert">{error()}<p>可重新查询；若清单缺失或损坏，请刷新本地事实。个人记录保持保留。</p></div></Show>
      <Show when={page()}>{result => <>
        <p role="status">候选 {result().candidates} · 已读 {result().read} · 遗漏 {result().omitted} · 已读 {result().readBytes} 字节 · 筛选命中 {result().totalMatches}</p>
        <p class="muted small">工作树预览 · 基线 {result().baselineOid} · 生成 {observedTime(result().generatedAt)} · {result().partial ? '存在读取缺口' : '本次候选全部读取'}</p>
        <Show when={result().stale}><div class="notice" role="status">清单已过期：保留最近一次成功扫描的文件记录。{result().error}</div></Show>
        <div class="inventory-list" ref={list} tabindex="-1" aria-label="当前页文档文件">
          <For each={result().files}>{file => <div class="row"><div class="row-content">
            <button class="row-title" aria-pressed={selected()?.fileKey === file.fileKey} onClick={() => setSelected(file)}>{file.title}</button>
            <div class="metadata"><span>{file.path}</span><span>{kinds[file.kind] ?? file.kind}</span><span>{states[file.state] ?? file.state}</span><span>{file.dirty ? '工作树有修改' : '未检测到修改'}</span></div>
            <Show when={file.reason}><p class="muted small">{reasons[file.reason!] ?? file.reason}</p></Show>
          </div></div>}</For>
          <Show when={!result().files.length}><p class="empty">当前筛选没有匹配文件；这不代表仓库没有文档。</p></Show>
        </div>
        <div class="actions"><button disabled={busy() || !result().nextCursor} onClick={() => void load(true)}>下一页（最多 100 项）</button></div>
      </>}</Show>
      <Show when={selected()}>{file => <section class="inventory-detail" aria-label="文件读取详情"><h4>{file().path}</h4>
        <p>{states[file().state] ?? file().state} · {file().reason ? reasons[file().reason!] ?? file().reason : `${file().bytes ?? 0} 字节`}</p>
        <Show when={file().revision}><p class="muted small">内容摘要 {file().revision}</p></Show>
        <Show when={file().lifecycle}><p class="muted small">生命周期原声明：{file().lifecycle?.rawStatus ?? file().lifecycle?.status ?? '未声明'} · 类别 {categories[file().lifecycle?.category ?? 'unknown'] ?? '未分类'}<Show when={file().lifecycle?.issue}> · {file().lifecycle?.issue}</Show></p></Show>
        <Show when={file().state === 'read'}>
          <h5>显式配对系统设计</h5>
          <Show when={file().pairings} fallback={<p>旧清单尚未解析配对，请刷新本地事实。</p>}>{pairings => <>
            <Show when={pairings().issue}><p role="status">配对解析不完整：{pairingIssues[pairings().issue!] ?? pairings().issue}</p></Show>
            <Show when={!pairings().targets.length && !pairings().issue}><p>本次范围未发现显式配对系统设计链接。</p></Show>
            <For each={pairings().targets}>{edge => <p class="small">第 {edge.declarationLine} 行 · {edge.target ?? edge.destination} · {resolutions[edge.resolution] ?? edge.resolution}</p>}</For>
          </>}</Show>
        </Show>
        <Show when={file().state === 'read' && !chapters().length}><p>已读取，文件没有可展示的章节。</p></Show>
        <For each={chapters()}>{section => <p><button onClick={e => props.showSource(section, e.currentTarget)}>{section.title} · 第 {section.line} 行</button></p>}</For>
        <button onClick={() => setSelected(undefined)}>收起文件详情</button>
      </section>}</Show>
    </div>
  </section>;
}
