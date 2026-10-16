import { createEffect, createMemo, createSignal, For, on, Show } from 'solid-js';
import { bridge, errorMessage, observedTime } from './bridge';
import type { Snapshot } from './types';
import type { DocumentSemanticTreePage, IndexedDocument } from './generated/core';

const lifecycleLabels: Record<string,string> = {current:'隐藏明确历史',all:'全部（含历史）',proposed:'提议',draft:'草案',active:'现行',superseded:'已替代',retired:'已退役',other:'其他声明',unknown:'类别未知'};
const kindLabels: Record<string,string> = {all:'全部类型',prd:'需求',design:'设计',entry:'结构入口'};
const pairingLabels: Record<string,string> = {all:'全部配对状态',unpaired:'未发现显式配对',unparsed:'尚未解析配对',missing:'配对目标或章节缺失',unread:'配对目标未读取',ambiguous:'配对锚点歧义',outside_scope:'配对范围外或路径不安全',incomplete:'配对解析不完整'};
const resolutionLabels: Record<string,string> = {resolved:'目标文件已读取',fragment_unchecked:'章节尚未核对',missing:'目标已删除',missing_fragment:'目标章节缺失',ambiguous:'目标锚点歧义',unread:'目标未读取',outside_scope:'范围外或路径不安全'};
const defaults = () => ({query:'',read:'all',lifecycle:'current',kind:'all',pairing:'all'});

export default function DocumentSemanticTree(props: {
  snapshot: Snapshot;
  view: 'product' | 'professional';
  selectFile: (file?: IndexedDocument, origin?: HTMLElement) => void;
}) {
  const [page,setPage] = createSignal<DocumentSemanticTreePage>();
  const [parent,setParent] = createSignal('');
  const [query,setQuery] = createSignal('');
  const [read,setRead] = createSignal('all');
  const [lifecycle,setLifecycle] = createSignal('current');
  const [rawStatus,setRawStatus] = createSignal('');
  const [kind,setKind] = createSignal('all');
  const [pairing,setPairing] = createSignal('all');
  const [busy,setBusy] = createSignal(false);
  const [error,setError] = createSignal('');
  const [composing,setComposing] = createSignal(false);
  const title = () => props.view === 'product' ? '产品树' : '专业系统树';
  const identity = createMemo(() => JSON.stringify([props.snapshot.project?.id,props.snapshot.sources.find(s => s.kind === 'local')?.generation,props.view]));
  let generation = 0;
  let snapshotId: string | null = null;
  let applied = defaults();
  let list: HTMLUListElement | undefined;

  async function load(occurrence = '', next = false, apply = false, refresh = false) {
    const projectId = props.snapshot.project?.id;
    if (!projectId) return;
    const token = ++generation;
    const source = identity();
    const previous = page();
    if (apply) applied = {query:query().trim(),read:read(),lifecycle:lifecycle() === 'raw' ? `raw:${rawStatus()}` : lifecycle(),kind:kind(),pairing:pairing()};
    props.selectFile(undefined);
    setParent(occurrence); setPage(undefined); setBusy(true); setError('');
    try {
      const result = await bridge.documentSemanticTree({
        view:props.view,parentOccurrence:occurrence,lifecycleFilter:applied.lifecycle,kindFilter:applied.kind,pairingFilter:applied.pairing,
        inventory:{schemaVersion:1,projectId,snapshotId:refresh ? null : snapshotId,
          query:applied.query,readFilter:applied.read,cursor:next ? previous?.nextCursor ?? null : null},
      });
      if (token === generation && source === identity()) {
        snapshotId = result.snapshotId; setPage(result);
        queueMicrotask(() => { if (token === generation && source === identity()) list?.focus(); });
      }
    } catch (failure) { if (token === generation && source === identity()) setError(errorMessage(failure)); }
    finally { if (token === generation) setBusy(false); }
  }
  createEffect(on(identity, () => {
    ++generation; snapshotId = null; applied = defaults(); setQuery(''); setRead('all'); setLifecycle('current'); setKind('all'); setPairing('all'); setRawStatus('');
    setPage(undefined); setParent(''); setError(''); setBusy(false); props.selectFile(undefined);
    if (props.snapshot.project) void load();
  }));
  const stopCompositionEnter = (event: KeyboardEvent) => {
    if ((composing() || event.isComposing || event.keyCode === 229) && event.key === 'Enter') event.preventDefault();
  };
  function moveFocus(event: KeyboardEvent) {
    if (!list || !['ArrowDown','ArrowUp','Home','End'].includes(event.key)) return;
    const buttons = Array.from(list.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'));
    if (!buttons.length) return;
    const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const next = event.key === 'Home' ? 0 : event.key === 'End' ? buttons.length-1 : event.key === 'ArrowDown' ? Math.min(index+1,buttons.length-1) : Math.max(index-1,0);
    event.preventDefault(); buttons[next]?.focus();
  }
  return <section class="semantic-tree" aria-label={`${title()}导航`}>
    <p class="muted small">{props.view === 'product' ? '从产品入口的四大模块及活跃产品专题出发，沿显式导航展开。' : '从专业域的结构入口出发，沿显式导航展开。'}同一文件可在多个父级出现；这些位置不代表唯一主责或实现完成。未归属文件与关系诊断保留可见；循环、深度或预算限制会停止展开。全文、目录路径仍可使用文件清单和文档目录。</p>
    <form class="inventory-filters" onSubmit={event => {event.preventDefault(); if (!composing() && (lifecycle() !== 'raw' || rawStatus())) void load('',false,true);}}>
      <label>标题、路径或文档 ID<input aria-label="筛选树文档" value={query()} onInput={e => setQuery(e.currentTarget.value)} onCompositionStart={() => setComposing(true)} onCompositionEnd={() => setComposing(false)} onKeyDown={stopCompositionEnter}/></label>
      <label>读取结果<select aria-label="树读取结果" value={read()} onChange={e => setRead(e.currentTarget.value)}><option value="all">全部文件</option><option value="read">已读取</option><option value="omitted">仅看遗漏</option></select></label>
      <label>文档类型<select aria-label="树文档类型" value={kind()} onChange={e => setKind(e.currentTarget.value)}><For each={Object.entries(kindLabels)}>{([value,label]) => <option value={value}>{label}</option>}</For></select></label>
      <label>配对缺口<select aria-label="树配对缺口" value={pairing()} onChange={e => setPairing(e.currentTarget.value)}><For each={Object.entries(pairingLabels)}>{([value,label]) => <option value={value}>{label}</option>}</For></select></label>
      <label>生命周期<select aria-label="树生命周期" value={lifecycle()} onChange={e => setLifecycle(e.currentTarget.value)}><For each={Object.entries(lifecycleLabels)}>{([value,label]) => <option value={value}>{label}</option>}</For><option value="raw">原始声明</option></select></label>
      <Show when={lifecycle() === 'raw'}><label>原始声明（精确匹配）<input aria-label="树生命周期原始声明" value={rawStatus()} maxLength={1020} onInput={event => setRawStatus(event.currentTarget.value)} onCompositionStart={() => setComposing(true)} onCompositionEnd={() => setComposing(false)} onKeyDown={stopCompositionEnter}/></label></Show>
      <button type="submit" disabled={busy() || composing() || (lifecycle() === 'raw' && !rawStatus())}>筛选树</button>
    </form>
    <nav class="actions" aria-label={`${title()}路径`}>
      <button disabled={busy()} aria-current={!parent() ? 'location' : undefined} onClick={() => void load('')}>{title()}根</button>
      <For each={page()?.breadcrumbs ?? []}>{item => <button disabled={busy()} aria-current={item.occurrenceId === parent() ? 'location' : undefined} onClick={() => void load(item.occurrenceId)}>{item.label}</button>}</For>
    </nav>
    <Show when={busy()}><p role="status">正在查询{title()}…</p></Show>
    <Show when={error()}><div class="notice" role="alert">{error()}<p>可重试当前层；快照失效时重新查询树根。</p><button onClick={() => void load(parent())}>重试当前树层</button><button onClick={() => void load('',false,false,true)}>重新查询树根</button></div></Show>
    <Show when={page()}>{result => <>
      <p role="status">扫描候选 {result().candidates} · 已读 {result().read} · 遗漏 {result().omitted}；树范围 {result().eligibleFiles} 个文件 · 全范围匹配 {result().matchingFiles} · 当前层 {result().totalChildren} 项</p>
      <p class="muted small">基线 {result().baselineOid} · 生成 {observedTime(result().generatedAt)} · 覆盖与匹配计数按文件去重；同一文件的多个树位置不增加计数。搜索与筛选保留通向命中的祖先。类型筛选：{kindLabels[result().kindFilter]}；配对筛选：{pairingLabels[result().pairingFilter]}；生命周期筛选：{lifecycleLabels[result().lifecycleFilter] ?? result().lifecycleFilter}。</p>
      <Show when={result().hiddenHistoryFiles > 0}><p role="status">同一筛选范围中隐藏了 {result().hiddenHistoryFiles} 个明确历史文件。<button onClick={() => {setQuery(applied.query);setRead(applied.read);setKind(applied.kind);setPairing(applied.pairing);setLifecycle('all');applied = {...applied,lifecycle:'all'};void load('');}}>显示树历史匹配</button></p></Show>
      <Show when={result().stale}><p role="status" class="notice">树使用过期清单，保留原版本：{result().error}</p></Show>
      <Show when={result().issues.length}><div class="notice" role="status"><p>树解析诊断</p><ul><For each={result().issues}>{issue => <li>{issue}</li>}</For></ul></div></Show>
      <p class="muted small">在本层使用方向上/下、Home、End 移动按钮焦点，Enter 或空格打开；面包屑返回上层。</p>
      <ul class="semantic-tree-list" ref={list} tabindex="-1" aria-label="当前层树项目" onKeyDown={moveFocus}>
        <For each={result().entries}>{entry => <li>
          <div class="actions">
            <Show when={entry.hasChildren && !entry.cycle && !entry.depthLimited}><button onClick={() => void load(entry.occurrenceId)}>展开：{entry.label}（匹配 {entry.matchingFiles} 个文件）</button></Show>
            <Show when={entry.file} fallback={<Show when={!entry.hasChildren || entry.cycle || entry.depthLimited}><span>{entry.label}</span></Show>}>{file => <button onClick={event => props.selectFile(file(),event.currentTarget)}>查看文档：{entry.label}</button>}</Show>
          </div>
          <Show when={entry.path}><p class="muted small">{entry.path}</p></Show>
          <Show when={entry.file}>{file => <p class="muted small">生命周期：{lifecycleLabels[file().lifecycle?.category ?? 'unknown'] ?? '类别未知'} · 原声明：{file().lifecycle?.rawStatus ?? file().lifecycle?.status ?? '未声明'}</p>}</Show>
          <Show when={entry.historicalReferenceFiles > 0}><div class="notice"><p>历史引用占位：被 {entry.historicalReferenceFiles} 个非明确历史的已读文档显式引用；保留原生命周期，不代表现行、主责或实现完成。</p><p class="muted small">来源路径样本（最多 20）：{entry.historicalReferencePaths.join('、')}</p></div></Show>
          <Show when={entry.sourcePath}><p class="muted small">声明来源：{entry.sourcePath}<Show when={entry.sourceLine != null}> · 第 {entry.sourceLine} 行</Show></p></Show>
          <Show when={entry.relationship}>{edge => <p class="muted small">关系目标：{edge().target ?? edge().destination} · {resolutionLabels[edge().resolution] ?? edge().resolution}</p>}</Show>
          <Show when={entry.boundary === 'cross_domain'}><p class="muted small">跨域引用：仅保留导航目标，不展开，不提升为当前树的归属。</p></Show>
          <Show when={entry.cycle}><p role="status" class="notice">检测到循环引用，已停止此位置展开；原文件仍可查看。</p></Show>
          <Show when={entry.depthLimited}><p role="status" class="notice">已达树展开预算，停止此位置展开；完整文件仍可从文件清单查询。</p></Show>
        </li>}</For>
      </ul>
      <Show when={!result().entries.length}><p class="empty">当前树层没有匹配项目；这不代表仓库没有文档。</p></Show>
      <div class="actions"><button disabled={busy() || !result().nextCursor} onClick={() => void load(parent(),true)}>下一页树（最多 100 项）</button><button disabled={busy()} onClick={() => void load(parent())}>返回本树层首页</button><button disabled={busy()} onClick={() => void load('',false,false,true)}>重新查询树根</button></div>
    </>}</Show>
  </section>;
}
