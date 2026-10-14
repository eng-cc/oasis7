import { createMemo, For, Show } from 'solid-js';
import type { DocumentSection } from './types';

export function lifecycleLabel(section: DocumentSection): string {
  const raw = section.lifecycle?.rawStatus;
  if (raw !== undefined && raw !== null) return raw === '' ? '生命周期声明为空 · 尚未核实' : `声明 ${raw}`;
  const status = section.lifecycle?.status;
  const labels: Record<string,string> = {active:'声明 active',draft:'声明 draft',deprecated:'声明 deprecated',superseded:'声明 superseded',archived:'声明 archived',unknown:'声明冲突 / 尚未核实'};
  if (status === '') return '生命周期声明为空 · 尚未核实';
  return status ? (labels[status] ?? `声明 ${status}`) : '未读取生命周期声明';
}
export function resolveSuccessor(target: string, sections: DocumentSection[]): DocumentSection | undefined {
  if (!target || target.startsWith('/') || target.includes('://') || target.includes('\\') || target.split('/').some(part => part === '..' || part === '.')) return undefined;
  if (target.includes('#')) {
    const matches = sections.filter(s => s.id === target);
    return matches.length === 1 ? matches[0] : undefined;
  }
  const matches = sections.filter(s => s.path === target).sort((a,b) => a.line-b.line);
  return matches.length && (matches.length === 1 || matches[0].line !== matches[1].line) ? matches[0] : undefined;
}
export default function SourceSuccession(props: {source:DocumentSection; sections:DocumentSection[]; showSource:(section:DocumentSection)=>void}) {
  const currentDeclaration = createMemo(() => {
    const candidates = props.sections.filter(s => s.path === props.source.path);
    const declarations = new Set(candidates.map(s => JSON.stringify(s.lifecycle ?? null)));
    return declarations.size === 1 && candidates[0]?.lifecycle ? candidates[0] : undefined;
  });
  const declaration = () => currentDeclaration() ?? props.source;
  const historical = () => declaration().revision !== props.source.revision;
  return <div class="source-succession"><h3>文档声明 · 效力仍需核实</h3>
    <p class="muted small">生命周期与后继来自文档原始声明；不代表条款已生效、实现已完成或模型已接受。</p>
    <p>{lifecycleLabel(declaration())}</p>
    <Show when={declaration().lifecycle?.category}><p class="muted small">生命周期类别：{({proposed:'提议',draft:'草案',active:'现行',superseded:'已被替代',retired:'已退役',other:'其他已声明状态',unknown:'未声明状态'} as Record<string,string>)[declaration().lifecycle!.category!] ?? '未知类别'} · 仅用于文档分类</p></Show>
    <Show when={!currentDeclaration()}><p class="notice">当前文档声明缺失、未读取或不一致 · 尚未核实。所选版本中的声明仅为该版本历史事实，不代表当前文档声明。</p></Show>
    <p class="muted small">{currentDeclaration() ? '当前文档声明来源' : '所选原文版本声明来源'}：{declaration().path} · {declaration().revision} · {declaration().dirty ? '未提交工作树内容' : '与提交内容一致'} · 提交基线 {declaration().committedRevision}</p>
    <Show when={historical()}><p class="notice">当前文档声明版本：{declaration().revision} · 提交基线：{declaration().committedRevision}。下方保存原文仍为 {props.source.revision}，历史事实保持原样。</p></Show>
    <Show when={historical() && props.source.lifecycle}><details><summary>保存原文版本的声明 · {props.source.revision}</summary><p>{lifecycleLabel(props.source)}</p><pre>{props.source.lifecycle?.successorDeclaration?.slice(0,4096) ?? '该版本没有后继声明'}</pre><p>{props.source.lifecycle?.successorReason?.slice(0,4096)}</p></details></Show>
    <Show when={!currentDeclaration() && props.sections.some(s => s.path === props.source.path && s.lifecycle)}><p class="notice">当前文档声明不一致，不能据此确认当前后继；仅显示所选原文版本中的声明。</p></Show>
    <Show when={declaration().lifecycle?.successorDeclaration}><details open><summary>后继原始声明</summary><pre>{declaration().lifecycle?.successorDeclaration?.slice(0,4096)}</pre></details></Show>
    <Show when={declaration().lifecycle?.successorReason}><p>声明理由：{declaration().lifecycle?.successorReason?.slice(0,4096)}</p></Show>
    <Show when={declaration().lifecycle?.issue}><p class="notice">声明投影尚未核实：{declaration().lifecycle?.issue?.slice(0,4096)}</p></Show>
    <For each={declaration().lifecycle?.successorTargets ?? []}>{target => { const resolved = () => resolveSuccessor(target,props.sections); return <div class="row"><div class="row-content"><button class="row-title" disabled={!resolved()} onClick={() => { const next=resolved(); if(next) props.showSource(next); }}>{target}</button><p class="muted small">{resolved() ? `当前已索引目标 · ${resolved()!.title} · ${resolved()!.revision}` : '目标缺失、歧义或尚未读取 · 尚未核实，不能打开'}</p></div></div>; }}</For>
    <Show when={!declaration().lifecycle?.successorTargets.length}><p class="muted small">没有可明确定位的后继目标；不能推定没有后继或已经完成替代。</p></Show>
  </div>;
}
