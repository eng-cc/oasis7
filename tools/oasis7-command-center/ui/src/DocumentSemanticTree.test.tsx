import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { createSignal } from 'solid-js';
import DocumentSemanticTree from './DocumentSemanticTree';
import type { DocumentSemanticTreePage, IndexedDocument } from './generated/core';
import type { Snapshot } from './types';

const api = vi.hoisted(() => ({documentSemanticTree:vi.fn()}));
vi.mock('./bridge', () => ({bridge:api,errorMessage:String,observedTime:()=>'核验时间'}));
const file: IndexedDocument = {fileKey:'file',path:'doc/product/module/prd.md',title:'模块需求',kind:'prd',state:'read',reason:null,revision:'rev',bytes:10,dirty:false,lifecycle:null,identity:null,pairings:null,upstream:null,allocations:null,navigation:null};
const snapshot = (id='project',generation='1'): Snapshot => ({project:{id,path:'/repo',head:'head',branch:'main',gitCommonDir:'/repo/.git',environment:'local'},sources:[{kind:'local',generation,state:'partial',lastAttempt:'1',lastSuccess:'1',sourceRevision:'head',coverage:'',error:null}],sections:[],decisions:[],preferences:{projectPath:'/repo',query:'',view:'rules'},associations:[],readingPositions:[],git:{status:'',worktrees:''}});
const entry = (extra: Partial<DocumentSemanticTreePage['entries'][number]> = {}): DocumentSemanticTreePage['entries'][number] => ({occurrenceId:'module',label:'模块需求',path:file.path,kind:'prd',matchingFiles:1,historicalReferenceFiles:0,historicalReferencePaths:[],boundary:null,cycle:false,depthLimited:false,sourcePath:'doc/product/README.md',sourceLine:51,relationship:null,file,hasChildren:true,...extra});
const page = (extra: Partial<DocumentSemanticTreePage> = {}): DocumentSemanticTreePage => ({schemaVersion:1,projectId:'project',snapshotId:'snapshot',baselineOid:'head',generatedAt:'1',view:'product',parentOccurrence:'',breadcrumbs:[],candidates:10,read:9,omitted:1,stale:false,error:null,eligibleFiles:8,matchingFiles:2,hiddenHistoryFiles:0,lifecycleFilter:'current',kindFilter:'all',pairingFilter:'all',totalChildren:1,entries:[entry()],nextCursor:null,issues:[],...extra});
beforeEach(() => api.documentSemanticTree.mockReset()); afterEach(cleanup);

it('navigates explicit occurrences and breadcrumbs in one snapshot while reusing the original file', async () => {
  api.documentSemanticTree.mockResolvedValueOnce(page()).mockResolvedValueOnce(page({parentOccurrence:'module',breadcrumbs:[{occurrenceId:'module',label:'模块需求'}],entries:[entry({occurrenceId:'module/topic',label:'专题需求',hasChildren:false})]})).mockResolvedValue(page());
  const select=vi.fn();render(()=><DocumentSemanticTree snapshot={snapshot()} view="product" selectFile={select}/>);
  fireEvent.click(await screen.findByRole('button',{name:'展开：模块需求（匹配 1 个文件）'}));
  await screen.findByRole('button',{name:'查看文档：专题需求'});
  expect(api.documentSemanticTree).toHaveBeenLastCalledWith(expect.objectContaining({view:'product',parentOccurrence:'module',inventory:expect.objectContaining({snapshotId:'snapshot',cursor:null})}));
  expect(screen.getByRole('button',{name:'模块需求'})).toHaveAttribute('aria-current','location');
  expect(screen.getByText('声明来源：doc/product/README.md · 第 51 行')).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button',{name:'查看文档：专题需求'}));expect(select).toHaveBeenLastCalledWith(file,expect.any(HTMLElement));
  fireEvent.click(screen.getByRole('button',{name:'产品树根'}));
  await waitFor(()=>expect(api.documentSemanticTree).toHaveBeenLastCalledWith(expect.objectContaining({parentOccurrence:'',inventory:expect.objectContaining({snapshotId:'snapshot'})})));
  await waitFor(()=>expect(screen.getByRole('list',{name:'当前层树项目'})).toHaveFocus());
});

it('keeps multiple occurrences, unassigned files, relationship diagnostics and bounded stops visible', async () => {
  api.documentSemanticTree.mockResolvedValue(page({stale:true,error:'scan failed',issues:['doc/product/README.md:50 导航段落未解析'],entries:[
    entry({occurrenceId:'one',label:'来源甲',hasChildren:false}),entry({occurrenceId:'two',label:'来源乙',hasChildren:false}),
    entry({occurrenceId:'unassigned',label:'未归属',file:null,path:null}),
    entry({occurrenceId:'cross',label:'跨域目标',boundary:'cross_domain',hasChildren:false}),
    entry({occurrenceId:'cycle',label:'循环位置',cycle:true}),entry({occurrenceId:'deep',label:'深层位置',depthLimited:true}),
    entry({occurrenceId:'broken',label:'缺失目标',hasChildren:false,file:null,relationship:{declarationLine:51,destination:'missing.md#topic',target:'doc/missing.md#topic',targetKind:'prd',resolution:'missing_fragment',targetSection:null,targetRevision:null}}),
  ]}));
  render(()=><DocumentSemanticTree snapshot={snapshot()} view="professional" selectFile={vi.fn()}/>);
  await screen.findByRole('button',{name:'查看文档：来源甲'});expect(screen.getByRole('button',{name:'查看文档：来源乙'})).toBeInTheDocument();
  expect(screen.getByRole('button',{name:'展开：未归属（匹配 1 个文件）'})).toBeInTheDocument();
  expect(screen.queryByRole('button',{name:/展开：循环位置/})).not.toBeInTheDocument();
  expect(screen.queryByRole('button',{name:/展开：深层位置/})).not.toBeInTheDocument();
  expect(screen.getByText(/跨域引用：仅保留导航目标/)).toBeInTheDocument();
  expect(screen.getByText(/检测到循环引用/)).toBeInTheDocument();expect(screen.getByText(/已达树展开预算/)).toBeInTheDocument();
  expect(screen.getByText(/关系目标：doc\/missing.md#topic · 目标章节缺失/)).toBeInTheDocument();
  expect(screen.getByText(/导航段落未解析/)).toBeInTheDocument();expect(screen.getByText(/树使用过期清单/)).toBeInTheDocument();
  expect(screen.getByText(/扫描候选 10.*树范围 8.*全范围匹配 2/)).toBeInTheDocument();
});

it('supports layer keyboard navigation without inventing an expanded tree role', async () => {
  api.documentSemanticTree.mockResolvedValue(page({entries:[entry(),entry({occurrenceId:'second',label:'第二模块',hasChildren:false})]}));
  render(()=><DocumentSemanticTree snapshot={snapshot()} view="product" selectFile={vi.fn()}/>);
  const expand=await screen.findByRole('button',{name:'展开：模块需求（匹配 1 个文件）'});
  const list=screen.getByRole('list',{name:'当前层树项目'});list.focus();fireEvent.keyDown(list,{key:'ArrowDown'});expect(expand).toHaveFocus();
  fireEvent.keyDown(expand,{key:'End'});expect(screen.getByRole('button',{name:'查看文档：第二模块'})).toHaveFocus();
  fireEvent.keyDown(document.activeElement!,{key:'ArrowUp'});expect(screen.getByRole('button',{name:'查看文档：模块需求'})).toHaveFocus();
  fireEvent.keyDown(document.activeElement!,{key:'Home'});expect(expand).toHaveFocus();
});

it('applies combined filters explicitly, preserves them through paging and history reveal, and handles IME', async () => {
  api.documentSemanticTree.mockResolvedValue(page({nextCursor:'next',hiddenHistoryFiles:1}));
  render(()=><DocumentSemanticTree snapshot={snapshot()} view="product" selectFile={vi.fn()}/>);
  const input=screen.getByRole('textbox',{name:'筛选树文档'});await screen.findByRole('button',{name:'显示树历史匹配'});
  fireEvent.input(input,{target:{value:'  专题  '}});fireEvent.compositionStart(input);fireEvent.submit(input.closest('form')!);expect(api.documentSemanticTree).toHaveBeenCalledTimes(1);
  fireEvent.compositionEnd(input);
  fireEvent.change(screen.getByRole('combobox',{name:'树读取结果'}),{target:{value:'omitted'}});
  fireEvent.change(screen.getByRole('combobox',{name:'树文档类型'}),{target:{value:'design'}});
  fireEvent.change(screen.getByRole('combobox',{name:'树配对缺口'}),{target:{value:'missing'}});
  fireEvent.submit(input.closest('form')!);await waitFor(()=>expect(api.documentSemanticTree).toHaveBeenCalledTimes(2));
  await screen.findByRole('button',{name:'显示树历史匹配'});
  fireEvent.input(input,{target:{value:'未提交'}});fireEvent.change(screen.getByRole('combobox',{name:'树文档类型'}),{target:{value:'entry'}});
  fireEvent.click(screen.getByRole('button',{name:'下一页树（最多 100 项）'}));
  await waitFor(()=>expect(api.documentSemanticTree).toHaveBeenLastCalledWith(expect.objectContaining({kindFilter:'design',pairingFilter:'missing',lifecycleFilter:'current',inventory:expect.objectContaining({query:'专题',readFilter:'omitted',snapshotId:'snapshot',cursor:'next'})})));
  await screen.findByRole('button',{name:'显示树历史匹配'});fireEvent.click(screen.getByRole('button',{name:'显示树历史匹配'}));
  await waitFor(()=>expect(api.documentSemanticTree).toHaveBeenLastCalledWith(expect.objectContaining({kindFilter:'design',pairingFilter:'missing',lifecycleFilter:'all',inventory:expect.objectContaining({query:'专题',cursor:null})})));
  expect(input).toHaveValue('专题');expect(screen.getByRole('combobox',{name:'树文档类型'})).toHaveValue('design');
});

it('rejects late responses across project, local generation and tree-view changes', async () => {
  let finish!: (value:DocumentSemanticTreePage)=>void;
  api.documentSemanticTree.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;})).mockResolvedValue(page({entries:[],view:'professional'}));
  const [data,setData]=createSignal(snapshot());const [view,setView]=createSignal<'product'|'professional'>('product');
  render(()=><DocumentSemanticTree snapshot={data()} view={view()} selectFile={vi.fn()}/>);
  setView('professional');await screen.findByText('当前树层没有匹配项目；这不代表仓库没有文档。');
  finish(page());await Promise.resolve();expect(screen.queryByRole('button',{name:'查看文档：模块需求'})).not.toBeInTheDocument();
  expect(api.documentSemanticTree).toHaveBeenLastCalledWith(expect.objectContaining({view:'professional'}));
  setData({...data(),preferences:{...data().preferences,query:'personal'}});await Promise.resolve();expect(api.documentSemanticTree).toHaveBeenCalledTimes(2);
  setData(snapshot('other'));await waitFor(()=>expect(api.documentSemanticTree).toHaveBeenCalledTimes(3));
  setData(snapshot('other','2'));await waitFor(()=>expect(api.documentSemanticTree).toHaveBeenCalledTimes(4));
  expect(api.documentSemanticTree).toHaveBeenLastCalledWith(expect.objectContaining({inventory:expect.objectContaining({projectId:'other',snapshotId:null}),kindFilter:'all',pairingFilter:'all'}));
});

it('recovers an expired snapshot using an explicit unbound root query', async () => {
  api.documentSemanticTree.mockResolvedValueOnce(page()).mockRejectedValueOnce(new Error('snapshot expired')).mockRejectedValueOnce(new Error('snapshot expired')).mockResolvedValue(page());
  render(()=><DocumentSemanticTree snapshot={snapshot()} view="product" selectFile={vi.fn()}/>);
  fireEvent.click(await screen.findByRole('button',{name:'展开：模块需求（匹配 1 个文件）'}));
  await screen.findByRole('alert');fireEvent.click(screen.getByRole('button',{name:'重试当前树层'}));
  await waitFor(()=>expect(api.documentSemanticTree).toHaveBeenLastCalledWith(expect.objectContaining({parentOccurrence:'module',inventory:expect.objectContaining({snapshotId:'snapshot'})})));
  await screen.findByRole('alert');fireEvent.click(screen.getByRole('button',{name:'重新查询树根'}));
  await waitFor(()=>expect(api.documentSemanticTree).toHaveBeenLastCalledWith(expect.objectContaining({parentOccurrence:'',inventory:expect.objectContaining({snapshotId:null,cursor:null})})));
  await screen.findByRole('button',{name:'查看文档：模块需求'});
});

it('supports exact original lifecycle declarations and rejects empty or composing submissions', async () => {
  api.documentSemanticTree.mockResolvedValue(page({nextCursor:'next',hiddenHistoryFiles:1}));
  render(()=><DocumentSemanticTree snapshot={snapshot()} view="professional" selectFile={vi.fn()}/>);
  await screen.findByRole('button',{name:'查看文档：模块需求'});
  fireEvent.change(screen.getByRole('combobox',{name:'树生命周期'}),{target:{value:'raw'}});
  expect(screen.getByRole('button',{name:'筛选树'})).toBeDisabled();
  const raw=screen.getByRole('textbox',{name:'树生命周期原始声明'});fireEvent.input(raw,{target:{value:'target-contract'}});fireEvent.compositionStart(raw);fireEvent.submit(raw.closest('form')!);expect(api.documentSemanticTree).toHaveBeenCalledTimes(1);
  fireEvent.compositionEnd(raw);fireEvent.submit(raw.closest('form')!);
  await waitFor(()=>expect(api.documentSemanticTree).toHaveBeenLastCalledWith(expect.objectContaining({lifecycleFilter:'raw:target-contract'})));
  await screen.findByRole('button',{name:'查看文档：模块需求'});fireEvent.change(screen.getByRole('combobox',{name:'树生命周期'}),{target:{value:'all'}});
  fireEvent.click(screen.getByRole('button',{name:'下一页树（最多 100 项）'}));
  await waitFor(()=>expect(api.documentSemanticTree).toHaveBeenLastCalledWith(expect.objectContaining({lifecycleFilter:'raw:target-contract',inventory:expect.objectContaining({cursor:'next'})})));
  await screen.findByRole('button',{name:'显示树历史匹配'});fireEvent.change(screen.getByRole('combobox',{name:'树生命周期'}),{target:{value:'raw'}});
  fireEvent.input(screen.getByRole('textbox',{name:'树生命周期原始声明'}),{target:{value:'未提交声明'}});fireEvent.click(screen.getByRole('button',{name:'显示树历史匹配'}));
  await waitFor(()=>expect(api.documentSemanticTree).toHaveBeenLastCalledWith(expect.objectContaining({lifecycleFilter:'all',inventory:expect.objectContaining({query:'',cursor:null})})));
  expect(screen.getByRole('combobox',{name:'树生命周期'})).toHaveValue('all');
});

it('labels retained historical navigation occurrences with original lifecycle and explicit source evidence', async () => {
  const historical={...file,lifecycle:{status:'retired',category:'retired',rawStatus:'retired',successorDeclaration:null,successorTargets:[],successorReason:null,issue:null}};
  api.documentSemanticTree.mockResolvedValue(page({entries:[entry({file:historical,historicalReferenceFiles:24,historicalReferencePaths:['doc/product/source.prd.md']})]}));
  const select=vi.fn();render(()=><DocumentSemanticTree snapshot={snapshot()} view="product" selectFile={select}/>);
  expect(await screen.findByText('生命周期：已退役 · 原声明：retired')).toBeInTheDocument();
  expect(screen.getByText(/历史引用占位：被 24 个非明确历史的已读文档显式引用/)).toBeInTheDocument();
  expect(screen.getByText('来源路径样本（最多 20）：doc/product/source.prd.md')).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button',{name:'查看文档：模块需求'}));expect(select).toHaveBeenLastCalledWith(historical,expect.any(HTMLElement));
});
