import { createEffect, createMemo, createSignal, For, on, Show } from 'solid-js';
import { bridge, errorMessage, observedTime } from './bridge';
import type { Snapshot } from './types';
import type { DocumentDirectoryPage, IndexedDocument } from './generated/core';

export default function DocumentDirectory(props: { snapshot: Snapshot; selectFile: (file?: IndexedDocument) => void }) {
  const [page, setPage] = createSignal<DocumentDirectoryPage>();
  const [parent, setParent] = createSignal('');
  const [query, setQuery] = createSignal('');
  const [filter, setFilter] = createSignal('all');
  const [kind, setKind] = createSignal('all');
  const [lifecycle, setLifecycle] = createSignal('current');
  const [rawStatus, setRawStatus] = createSignal('');
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal('');
  const [composing, setComposing] = createSignal(false);
  let generation = 0;
  let applied = {query:'',filter:'all',lifecycle:'current',kind:'all'};
  let list: HTMLUListElement | undefined;
  const lifecycleLabel = (value: string) => ({current:'隐藏明确历史',all:'全部（含历史）',proposed:'提议',draft:'草案',active:'现行',superseded:'已替代',retired:'已退役',other:'其他声明',unknown:'类别未知'}[value] ?? (value.startsWith('raw:') ? `原始声明：${value.slice(4)}` : value));
  const identity = createMemo(() => JSON.stringify([props.snapshot.project?.id, props.snapshot.sources.find(s => s.kind === 'local')?.generation]));
  async function load(path: string, next = false, apply = false) {
    const projectId = props.snapshot.project?.id;
    if (!projectId) return;
    const token = ++generation;
    const source = identity();
    const previous = page();
    if (apply) applied = {query:query().trim(),filter:filter(),kind:kind(),lifecycle:lifecycle() === 'raw' ? `raw:${rawStatus()}` : lifecycle()};
    props.selectFile(undefined);
    setParent(path); setPage(undefined); setBusy(true); setError('');
    try {
      const result = await bridge.documentDirectory({parentPath:path, lifecycleFilter:applied.lifecycle, kindFilter:applied.kind, inventory:{
        schemaVersion:1,projectId,snapshotId:(!next && path === '') ? null : previous?.snapshotId ?? null,
        query:applied.query,readFilter:applied.filter,cursor:next ? previous?.nextCursor ?? null : null,
      }});
      if (token === generation && source === identity()) {
        setPage(result);
        if (next) queueMicrotask(() => list?.focus());
      }
    } catch (failure) {if (token === generation && source === identity()) setError(errorMessage(failure));}
    finally {if (token === generation) setBusy(false);}
  }
  createEffect(on(identity, () => {
    ++generation; setQuery(''); setFilter('all'); setLifecycle('current'); setKind('all'); setRawStatus(''); applied = {query:'',filter:'all',lifecycle:'current',kind:'all'};
    setParent(''); setPage(undefined); setError(''); props.selectFile(undefined);
    if (props.snapshot.project) void load('');
  }));
  const ancestors = () => parent().split('/').filter(Boolean).map((name,index,array) => ({name,path:array.slice(0,index+1).join('/')}));
  return <section aria-label="文档目录层级">
    <p class="muted small">按当前清单路径组织 PRD、设计及结构入口（README / AGENTS），不推断产品归属或主责。隐藏历史时保留已核实显式引用的历史占位。搜索保留命中文件的目录入口；普通正文仍可从文件清单查询。逐层分页，目录深度最多 64，尚未提供产品树、专业系统树或持久阅读偏好。</p>
    <form class="inventory-filters" onSubmit={e => {e.preventDefault(); if (!composing() && (lifecycle() !== 'raw' || rawStatus())) void load('',false,true);}}>
      <label>标题、路径或文档 ID<input aria-label="筛选目录文档" value={query()} onInput={e => setQuery(e.currentTarget.value)} onCompositionStart={() => setComposing(true)} onCompositionEnd={() => setComposing(false)} onKeyDown={e => {if ((composing() || e.isComposing || e.keyCode === 229) && e.key === 'Enter') e.preventDefault();}} /></label>
      <label>读取结果<select aria-label="目录读取结果" value={filter()} onChange={e => setFilter(e.currentTarget.value)}><option value="all">全部文件</option><option value="read">已读取</option><option value="omitted">仅看遗漏</option></select></label>
      <label>文档类型<select aria-label="目录文档类型" value={kind()} onChange={e => setKind(e.currentTarget.value)}><option value="all">全部类型</option><option value="prd">需求</option><option value="design">设计</option><option value="entry">结构入口</option></select></label>
      <label>生命周期<select aria-label="目录生命周期" value={lifecycle()} onChange={e => setLifecycle(e.currentTarget.value)}>
        <option value="current">隐藏明确历史</option><option value="all">全部（含历史）</option>
        <option value="proposed">提议</option><option value="draft">草案</option><option value="active">现行</option><option value="superseded">已替代</option><option value="retired">已退役</option><option value="other">其他声明</option><option value="unknown">类别未知</option><option value="raw">原始声明</option>
      </select></label>
      <Show when={lifecycle() === 'raw'}><label>原始声明（精确匹配）<input aria-label="目录生命周期原始声明" value={rawStatus()} maxLength={1020} onInput={e => setRawStatus(e.currentTarget.value)} onCompositionStart={() => setComposing(true)} onCompositionEnd={() => setComposing(false)} onKeyDown={e => {if ((composing() || e.isComposing || e.keyCode === 229) && e.key === 'Enter') e.preventDefault();}} /></label></Show>
      <button type="submit" disabled={busy() || composing() || (lifecycle() === 'raw' && !rawStatus())}>筛选目录</button>
    </form>
    <nav aria-label="文档目录路径" class="actions"><button disabled={busy()} onClick={() => void load('')}>目录根</button><For each={ancestors()}>{item => <button disabled={busy()} aria-current={item.path === parent() ? 'location' : undefined} onClick={() => void load(item.path)}>{item.name}</button>}</For></nav>
    <Show when={busy()}><p role="status">正在查询目录层级…</p></Show>
    <Show when={error()}><div role="alert" class="notice">{error()}<p>可重新查询当前层；快照失效时返回目录根或刷新本地事实。</p><button onClick={() => void load(parent())}>重试当前目录</button></div></Show>
    <Show when={page()}>{result => <>
      <p role="status">扫描候选 {result().candidates} · 已读 {result().read} · 遗漏 {result().omitted}；目录范围 {result().eligibleFiles} 个文件 · 全范围匹配 {result().matchingFiles} · 当前层 {result().totalChildren} 项</p>
      <p class="muted small">基线 {result().baselineOid} · 生成 {observedTime(result().generatedAt)} · 计数按文件去重，不随当前层分页裁剪。类型筛选：{{all:'全部类型',prd:'需求',design:'设计',entry:'结构入口'}[result().kindFilter]}；生命周期筛选：{lifecycleLabel(result().lifecycleFilter)}；类别未知包含未声明、未读取及尚未规范化项，不能视作现行。</p>
      <Show when={result().hiddenHistoryFiles > 0}><p role="status">同一类型、搜索与读取范围中隐藏了 {result().hiddenHistoryFiles} 个明确历史文件。<button onClick={() => {setQuery(applied.query); setFilter(applied.filter); setKind(applied.kind); setLifecycle('all'); applied = {...applied,lifecycle:'all'}; void load('');}}>显示历史匹配</button></p></Show>
      <Show when={result().stale}><p role="status" class="notice">目录使用过期清单，保留原版本：{result().error}</p></Show>
      <ul ref={list} tabindex="-1" aria-label="当前层目录项目">
        <For each={result().entries}>{entry => <li>
          <Show when={entry.kind === 'directory'} fallback={<><button onClick={() => props.selectFile(entry.file ?? undefined)}>{entry.file?.title ?? entry.name} · {entry.path}</button><span class="muted small"> 生命周期：{lifecycleLabel(entry.file?.lifecycle?.category ?? 'unknown')}</span><Show when={entry.historicalReferenceFiles > 0}><div class="notice"><p>历史引用占位：被 {entry.historicalReferenceFiles} 个非明确历史的已读文档显式引用；保留原生命周期，不代表现行、主责或实现完成。</p><p class="muted small">来源路径样本（最多 20）：{entry.historicalReferencePaths.join('、')}</p></div></Show></>}>
            <button disabled={entry.depthLimited} onClick={() => void load(entry.path)}>进入目录：{entry.name}（匹配 {entry.matchingFiles} 个文件）</button>
            <Show when={entry.depthLimited}><p class="muted small">已达目录深度预算：{entry.path}；深层文件仍保留在文件清单。</p></Show>
          </Show>
        </li>}</For>
      </ul>
      <Show when={!result().entries.length}><p class="empty">当前目录没有匹配项目；这不代表仓库没有文档。</p></Show>
      <div class="actions"><button disabled={busy() || !result().nextCursor} onClick={() => void load(parent(),true)}>下一页目录（最多 100 项）</button><button disabled={busy()} onClick={() => void load(parent())}>返回本层首页</button></div>
    </>}</Show>
  </section>;
}
