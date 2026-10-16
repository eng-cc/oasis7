import { For, Show } from 'solid-js';
import type { DocumentNavigation as Navigation, DocumentPairing, DocumentSection } from './generated/core';

const resolutions: Record<string,string> = {resolved:'目标文件已读取',fragment_unchecked:'章节尚未核对',missing:'目标已删除',missing_fragment:'目标章节缺失',ambiguous:'目标锚点歧义',unread:'目标未读取',outside_scope:'范围外或路径不安全'};
const issues: Record<string,string> = {
  'navigation bound exceeded; projection incomplete':'导航目标超过 256 条或原链接超过 4096 字节，投影不完整',
  'product module table contains an unsupported fifth or invalid module target':'产品模块表含非四模块目标或无效链接，未作为额外模块纳入',
};
export default function DocumentNavigation(props: {
  navigation: Navigation | null | undefined;
  sections: DocumentSection[];
  showSource: (section: DocumentSection, origin?: HTMLElement) => void;
}) {
  function targetSection(edge: DocumentPairing) {
    if (edge.resolution !== 'resolved' || !edge.target || !edge.targetSection || !edge.targetRevision) return undefined;
    return props.sections.find(section => section.path === edge.target!.split('#')[0] && section.id === edge.targetSection && section.revision === edge.targetRevision);
  }
  return <section aria-label="显式树导航声明">
    <h5>显式树导航声明</h5>
    <p class="muted small">仅展示约定入口和导航章节内的有界显式链接，按声明顺序保留。普通正文链接、配对与上游关系不自动成为树子节点，未发现导航也不代表没有下游文档。</p>
    <Show when={props.navigation} fallback={<p>旧清单尚未解析树导航，请刷新本地事实。</p>}>{navigation => <>
      <Show when={navigation().relationships.issue}><p role="status">导航解析不完整：{issues[navigation().relationships.issue!] ?? navigation().relationships.issue}</p></Show>
      <Show when={!navigation().relationships.targets.length && !navigation().relationships.issue}><p>本次约定章节范围未发现显式树导航。</p></Show>
      <ol><For each={navigation().relationships.targets}>{(edge,index) => <li>
        <p>{navigation().labels[index()] ?? edge.destination} · 第 {edge.declarationLine} 行</p>
        <p class="muted small">原链接：{edge.destination}</p>
        <p class="muted small">解析目标：{edge.target ?? '未规范化'} · {resolutions[edge.resolution] ?? edge.resolution}</p>
        <Show when={targetSection(edge)}>{target => <button onClick={event => props.showSource(target(),event.currentTarget)}>打开导航章节：{target().title}</button>}</Show>
        <Show when={edge.resolution === 'resolved' && !targetSection(edge)}><p class="muted small">目标在清单中，但当前快照没有可定位的同版本章节。</p></Show>
      </li>}</For></ol>
    </>}</Show>
  </section>;
}
