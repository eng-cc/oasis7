import { cleanup, fireEvent, render, screen } from '@solidjs/testing-library';
import { afterEach, expect, it, vi } from 'vitest';
import DocumentNavigation from './DocumentNavigation';
import type { DocumentNavigation as Navigation, DocumentPairing, DocumentSection } from './generated/core';
const target: DocumentSection = {id:'topic',path:'doc/topic/prd.md',title:'专题章节',content:'# 专题',line:1,revision:'rev',committedRevision:'head',dirty:false,factKind:'sourceFact',lifecycle:null};
const edge = (extra:Partial<DocumentPairing>={}):DocumentPairing => ({declarationLine:23,destination:'../topic/prd.md#topic',target:'doc/topic/prd.md#topic',resolution:'resolved',targetSection:'topic',targetRevision:'rev',targetKind:'prd',...extra});
afterEach(cleanup);
it('keeps ordered source navigation declarations and only opens matching snapshot revisions', () => {
  const showSource=vi.fn();const navigation:Navigation={relationships:{targets:[edge(),edge({declarationLine:25,targetRevision:'older'})],issue:null},labels:['甲专题','<script>标题</script>']};
  const result=render(()=><DocumentNavigation navigation={navigation} sections={[target]} showSource={showSource}/>);
  expect(screen.getByText('甲专题 · 第 23 行')).toBeInTheDocument();expect(screen.getByText('<script>标题</script> · 第 25 行')).toBeInTheDocument();
  expect(result.container.querySelector('script')).toBeNull();expect(screen.getAllByText('原链接：../topic/prd.md#topic')).toHaveLength(2);
  expect(screen.getAllByRole('button',{name:'打开导航章节：专题章节'})).toHaveLength(1);
  fireEvent.click(screen.getByRole('button',{name:'打开导航章节：专题章节'}));expect(showSource).toHaveBeenCalledWith(target,expect.any(HTMLElement));
  expect(screen.getByText('目标在清单中，但当前快照没有可定位的同版本章节。')).toBeInTheDocument();
});
it('separates legacy unparsed navigation from explicitly empty navigation', () => {
  const first=render(()=><DocumentNavigation navigation={null} sections={[]} showSource={vi.fn()}/>);
  expect(screen.getByText('旧清单尚未解析树导航，请刷新本地事实。')).toBeInTheDocument();first.unmount();
  render(()=><DocumentNavigation navigation={{labels:[],relationships:{targets:[],issue:null}}} sections={[]} showSource={vi.fn()}/>);
  expect(screen.getByText('本次约定章节范围未发现显式树导航。')).toBeInTheDocument();expect(screen.queryByText(/旧清单尚未解析/)).not.toBeInTheDocument();
});
it('shows incomplete navigation and unsafe target diagnostics without promoting them to links', () => {
  render(()=><DocumentNavigation navigation={{labels:[],relationships:{targets:[edge({resolution:'outside_scope',target:null,targetRevision:null,targetSection:null,destination:'../../escape.md'})],issue:'navigation bound exceeded; projection incomplete'}}} sections={[target]} showSource={vi.fn()}/>);
  expect(screen.getByText(/导航目标超过 256 条/)).toBeInTheDocument();expect(screen.getByText('原链接：../../escape.md')).toBeInTheDocument();expect(screen.getByText('解析目标：未规范化 · 范围外或路径不安全')).toBeInTheDocument();expect(screen.queryByRole('button')).not.toBeInTheDocument();
});
