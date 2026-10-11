import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, expect, it, vi } from 'vitest';
import SourceSuccession, { resolveSuccessor } from './SourceSuccession';
import type { DocumentSection } from './types';
afterEach(cleanup);
const source = (overrides:Partial<DocumentSection>={}):DocumentSection => ({id:'doc/old.md#rule',path:'doc/old.md',title:'旧条款',content:'历史原文',line:1,revision:'old-version',committedRevision:'old-base',dirty:false,factKind:'source',lifecycle:null,...overrides});
it('preserves saved history while explicitly following the current declaration to the exact target', () => {
  const saved=source(); const current=source({revision:'current-version',committedRevision:'current-base',lifecycle:{status:'superseded',successorDeclaration:'doc/new.md#exact',successorTargets:['doc/new.md#exact'],successorReason:'明确迁移原因',issue:null}});
  const target=source({id:'doc/new.md#exact',path:'doc/new.md',title:'精确新条款',revision:'new-version'}); const navigate=vi.fn();
  render(() => <SourceSuccession source={saved} sections={[current,target]} showSource={navigate}/>);
  expect(screen.getByText(/当前文档声明版本：current-version/)).toHaveTextContent('old-version');
  expect(screen.getByText('声明理由：明确迁移原因')).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button',{name:'doc/new.md#exact'})); expect(navigate).toHaveBeenCalledWith(target);
  expect(saved.content).toBe('历史原文'); expect(saved.lifecycle).toBeNull();
});
it('disables missing or ambiguous targets without guessing from a title', () => {
  const duplicate=source({id:'doc/new.md#exact',path:'doc/new.md',title:'同名'});
  const declared=source({lifecycle:{status:'unknown',successorDeclaration:'原始声明',successorTargets:['doc/new.md#exact','doc/missing.md#same-title'],successorReason:null,issue:null}});
  render(() => <SourceSuccession source={declared} sections={[declared,duplicate,{...duplicate,line:2}]} showSource={vi.fn()}/>);
  expect(screen.getByRole('button',{name:'doc/new.md#exact'})).toBeDisabled();
  expect(screen.getByRole('button',{name:'doc/missing.md#same-title'})).toBeDisabled();
  expect(resolveSuccessor('https://example.com/a',[])).toBeUndefined();
  expect(resolveSuccessor('../doc/new.md',[duplicate])).toBeUndefined();
});
it('treats active as a source declaration and supports a plain indexed document only', () => {
  const active=source({lifecycle:{status:'active',successorDeclaration:null,successorTargets:[],successorReason:null,issue:null}});
  render(() => <SourceSuccession source={active} sections={[active]} showSource={vi.fn()}/>);
  expect(screen.getByText('声明 active')).toBeInTheDocument();
  expect(screen.getByText(/不代表条款已生效、实现已完成或模型已接受/)).toBeInTheDocument();
  const later=source({line:7}); expect(resolveSuccessor('doc/old.md',[later,active])).toBe(active);
  expect(resolveSuccessor('doc/old.md',[active,{...active}])).toBeUndefined();
});
it('labels old declarations as historical when current metadata is absent or inconsistent', () => {
  const saved=source({lifecycle:{status:'superseded',successorDeclaration:'doc/new.md',successorTargets:['doc/new.md'],successorReason:null,issue:null}});
  render(() => <SourceSuccession source={saved} sections={[source({revision:'current-version'})]} showSource={vi.fn()}/>);
  expect(screen.getByText(/当前文档声明缺失、未读取或不一致/)).toBeInTheDocument();
  expect(screen.getByText(/所选原文版本声明来源/)).toHaveTextContent('old-version');
});
