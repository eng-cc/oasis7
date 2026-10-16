import { For, Show } from 'solid-js';
import DocumentRelations from './DocumentRelations';
import type { DocumentAllocations as Allocations, DocumentSection } from './generated/core';
const issues: Record<string, string> = {
  'allocation section has no table': '发现承接章节，但没有可识别的 Markdown 表格',
  'allocation table requires unique upstream and design columns': '表头必须各有唯一的“上游条款”和“本设计条款”列，未解析该表',
  'allocation row limit 32 exceeded': '超过每文件 32 行，仅展示前 32 行',
  'allocation link limit 64 exceeded': '超过每文件 64 个链接，仅保留预算内链接',
  'allocation row exceeds 4096 bytes': '存在超过 4096 字节的行，该行未解析链接',
};
export default function DocumentAllocations(props: {
  allocations: Allocations | null | undefined;
  sections: DocumentSection[];
  showSource: (section: DocumentSection, origin?: HTMLElement) => void;
}) {
  return <section aria-label="正文需求承接与分配表"><h5>正文需求承接与分配表</h5>
    <p class="muted small">仅解析顶层“需求承接与分配表”章节内的明确条款列，按行保留声明。链接存在不代表承接完整、主责确认或实现验收；普通表格与验证映射不推断承接。</p>
    <Show when={props.allocations} fallback={<p>旧清单尚未解析承接表，请刷新本地事实。</p>}>{table => <>
      <Show when={table().issue}><p role="status">承接表解析不完整：{issues[table().issue!] ?? table().issue}</p></Show>
      <Show when={!table().rows.length && !table().issue}><p>本次明确章节范围未发现需求承接表行。</p></Show>
      <For each={table().rows}>{row => <section aria-label={`承接表第 ${row.declarationLine} 行`}>
        <h6>声明行 {row.declarationLine}</h6><p class="muted small">原行样本（最多 512 字符）：{row.rawSample}</p>
        <DocumentRelations relations={row.upstream} mode="allocation_upstream" sections={props.sections} showSource={props.showSource} />
        <DocumentRelations relations={row.design} mode="allocation_design" sections={props.sections} showSource={props.showSource} />
      </section>}</For>
    </>}</Show>
  </section>;
}
