import { createMemo, createSignal, For, Show } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import { contextBridge, errorMessage } from './bridge';
import type { ContextBinding, PrivateInputPreview, RuntimeSnapshot } from './generated/core';

export default function PrivateContextInput(props: { threadId: string; input: () => string; disabled: () => boolean; send: (action: () => Promise<RuntimeSnapshot>) => Promise<void> }) {
  const [bindings, setBindings] = createSignal<ContextBinding[]>([]);
  const [entries, setEntries] = createSignal<{id:string;title:string}[]>([]);
  const [binding, setBinding] = createSignal<ContextBinding>();
  const [preview, setPreview] = createSignal<PrivateInputPreview>();
  const [entryId,setEntryId]=createSignal('');
  const [startLine,setStartLine]=createSignal('');
  const [endLine,setEndLine]=createSignal('');
  const [error, setError] = createSignal('');
  const [busy, setBusy] = createSignal(false);
  const remaining = createMemo(() => { const value = preview(); return value ? value.maxInputBytes - value.attachmentBytes - new TextEncoder().encode(props.input()).length : 0; });
  let generation = 0;
  async function load() {
    setBusy(true); setError('');
    try { setBindings(await contextBridge.bindings()); } catch(e) { setError(errorMessage(e)); }
    finally { setBusy(false); }
  }
  async function select(id: string) {
    const version=++generation;
    const selected=bindings().find(b=>b.id===id);
    setBinding(selected); setPreview(undefined); setEntryId(''); setStartLine(''); setEndLine(''); setEntries([]); setError('');
    if(!selected) return;
    setBusy(true);
    try { const result=await contextBridge.list(selected); if(version===generation) setEntries(result.entries); }
    catch(e) { if(version===generation) setError(errorMessage(e)); }
    finally { if(version===generation) setBusy(false); }
  }
  async function read(id: string) {
    setEntryId(id);
    const selected=binding(); if(!selected || !id) { setPreview(undefined); return; }
    const version=++generation; setPreview(undefined); setBusy(true); setError('');
    try {
      const result=await invoke<PrivateInputPreview>('preview_private_input',{bindingId:selected.id,bindingVersion:Number(selected.bindingVersion),entryId:id,startLine:startLine() ? Number(startLine()) : null,endLine:endLine() ? Number(endLine()) : null});
      if(version===generation) setPreview(result);
    } catch(e) { if(version===generation) setError(errorMessage(e)); }
    finally { if(version===generation) setBusy(false); }
  }
  return <details class="private-input"><summary>显式使用私有资料</summary>
    <p class="notice">发送会把下方正文提供给所选模型。原运行时及本机运行数据库可能保存派生请求和会话状态；此线程后续轮次与恢复将持续标记为私有。</p>
    <button disabled={busy()} onClick={()=>void load()}>读取已绑定资料库</button>
    <label>资料库<select onChange={e=>void select(e.currentTarget.value)}><option value="">选择来源</option><For each={bindings()}>{b=><option value={b.id}>{b.name}</option>}</For></select></label>
    <label>条目<select disabled={!binding() || busy()} onChange={e=>void read(e.currentTarget.value)}><option value="">选择并预览</option><For each={entries()}>{entry=><option value={entry.id}>{entry.title} · {entry.id}</option>}</For></select></label>
    <p class="muted small">留空提供整条资料，或明确选择原文行范围。</p>
    <label>片段起始行<input type="number" min="1" value={startLine()} onInput={e=>{setStartLine(e.currentTarget.value);setPreview(undefined);}}/></label>
    <label>片段终止行<input type="number" min="1" value={endLine()} onInput={e=>{setEndLine(e.currentTarget.value);setPreview(undefined);}}/></label>
    <button disabled={busy() || !entryId()} onClick={()=>void read(entryId())}>重新预览选定范围</button>
    <Show when={preview()}>{value=><><p class="metadata">目标会话 {props.threadId} · 来源 {value().source.entryId} · 修订 {value().source.revision} · 范围 {value().source.scope}</p><pre>{value().text}</pre>
      <p role="status">合计包含来源说明，剩余 {remaining()} UTF-8 字节；超出时请缩小资料范围或草稿，不会截断。</p>
      <button disabled={busy() || props.disabled() || !props.input().trim() || remaining() < 0} onClick={()=> {
        const selected=preview(); if(!selected || props.disabled() || !props.input().trim() || remaining() < 0) return;
        void props.send(()=>invoke<RuntimeSnapshot>('start_private_runtime_turn',{threadId:props.threadId,input:props.input(),source:selected.source}));
      }}>将预览资料与草稿发送到此会话</button></>}</Show>
    <Show when={error()}><p role="status" class="notice">{error()}</p></Show>
  </details>;
}
