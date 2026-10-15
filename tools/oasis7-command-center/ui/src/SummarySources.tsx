import { For, Show } from 'solid-js';
import type { DocumentSection } from './types';

const sources = [
  ['项目状态摘要', 'doc/engineering/command-center/README.md'],
  ['架构与部署边界', 'doc/engineering/command-center/architecture/README.md'],
] as const;

export default function SummarySources(props: {
  sections: DocumentSection[];
  hasProject: boolean;
  showSource: (section: DocumentSection, origin?: HTMLElement) => void;
}) {
  return <section class="panel" aria-label="项目判断来源">
    <div class="panel-title"><h3>项目判断来源</h3><span class="tag">派生摘要</span></div>
    <div class="panel-body muted small">摘要正文保留各来源的核验时间与未知项；文件读取版本不代表事实已重新核验。四区聚合、来源冲突比较及完整首页验收尚未实现。</div>
    <For each={sources}>{([label, path]) => {
      const entries = () => props.sections.filter(section => section.path === path).sort((a, b) => a.line - b.line);
      return <div class="panel-body">
        <h4>{label}</h4>
        <Show when={entries().length} fallback={<p class="empty">{props.hasProject ? `${path} 未进入当前已读取范围。请核对文件与来源覆盖；不能据此推断没有待处理事项。` : '打开项目后读取摘要来源。'}</p>}>
          <p class="muted small">{path} · 当前索引 {entries().length} 个章节；逐章节阅读原文，完整性见来源覆盖。</p>
          <For each={entries().slice(0, 1)}>{section => sourceRow(section, props.showSource)}</For>
          <Show when={entries().length > 1}><details><summary>展开{label}章节 · {entries().length - 1} 项</summary>
          <For each={entries().slice(1)}>{section => sourceRow(section, props.showSource)}</For>
          </details></Show>
        </Show>
      </div>;
    }}</For>
  </section>;
}

function sourceRow(section: DocumentSection, showSource: (section: DocumentSection, origin?: HTMLElement) => void) {
  return <div class="row">
            <div class="row-content"><button class="row-title" onClick={e => showSource(section, e.currentTarget)}>{section.title}</button>
              <div class="metadata"><span>{section.path}:{section.line}</span><span>{section.dirty ? '工作树有修改' : '已提交内容'}</span><span>摘要声明 · 实现与部署验收须核实</span></div>
            </div><span class="tag">原文</span>
          </div>;
}
