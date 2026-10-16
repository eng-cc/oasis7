import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { createSignal } from 'solid-js';
import DocumentDirectory from './DocumentDirectory';
import type { DocumentDirectoryPage, IndexedDocument } from './generated/core';
import type { Snapshot } from './types';
const api = vi.hoisted(() => ({documentDirectory:vi.fn()}));
vi.mock('./bridge', () => ({bridge:api,errorMessage:String,observedTime:()=>'核验时间'}));
const file: IndexedDocument = {fileKey:'file',path:'doc/a/design.md',title:'A设计',kind:'design',state:'read',reason:null,revision:'rev',bytes:10,dirty:false,lifecycle:null,identity:null,pairings:null,upstream:null,allocations:null};
const snapshot = (id='project',generation='1'): Snapshot => ({project:{id,path:'/repo',head:'head',branch:'main',gitCommonDir:'/repo/.git',environment:'local'},sources:[{kind:'local',generation,state:'partial',lastAttempt:'1',lastSuccess:'1',sourceRevision:'head',coverage:'',error:null}],sections:[],decisions:[],preferences:{projectPath:'/repo',query:'',view:'rules'},associations:[],readingPositions:[],git:{status:'',worktrees:''}});
const page = (extra: Partial<DocumentDirectoryPage> = {}): DocumentDirectoryPage => ({schemaVersion:1,projectId:'project',snapshotId:'snapshot',baselineOid:'head',generatedAt:'1',parentPath:'',candidates:10,read:9,omitted:1,stale:false,error:null,eligibleFiles:8,matchingFiles:2,hiddenHistoryFiles:0,lifecycleFilter:'current',totalChildren:1,entries:[{path:'doc',name:'doc',kind:'directory',matchingFiles:2,depthLimited:false,historicalReferenceFiles:0,historicalReferencePaths:[],file:null}],nextCursor:null,...extra});
beforeEach(() => api.documentDirectory.mockReset()); afterEach(cleanup);
it('navigates candidate ancestors with bound snapshots and selects the original file', async () => {
  api.documentDirectory.mockResolvedValueOnce(page()).mockResolvedValueOnce(page({parentPath:'doc',entries:[{path:file.path,name:'design.md',kind:'design',matchingFiles:1,depthLimited:false,historicalReferenceFiles:0,historicalReferencePaths:[],file}]})).mockResolvedValue(page());
  const select = vi.fn(); render(() => <DocumentDirectory snapshot={snapshot()} selectFile={select}/>);
  fireEvent.click(await screen.findByRole('button',{name:'进入目录：doc（匹配 2 个文件）'}));
  await screen.findByRole('button',{name:'A设计 · doc/a/design.md'});
  expect(api.documentDirectory).toHaveBeenLastCalledWith(expect.objectContaining({parentPath:'doc',inventory:expect.objectContaining({snapshotId:'snapshot',cursor:null})}));
  fireEvent.click(screen.getByRole('button',{name:'A设计 · doc/a/design.md'})); expect(select).toHaveBeenLastCalledWith(file);
  fireEvent.click(screen.getByRole('button',{name:'目录根'}));
  await waitFor(()=>expect(api.documentDirectory).toHaveBeenLastCalledWith(expect.objectContaining({parentPath:'',inventory:expect.objectContaining({snapshotId:null})})));
});
it('binds paging to applied filters, handles IME and keeps full counts and focus', async () => {
  api.documentDirectory.mockResolvedValue(page({nextCursor:'next'}));
  render(() => <DocumentDirectory snapshot={snapshot()} selectFile={vi.fn()}/>);
  const input = screen.getByRole('textbox',{name:'筛选目录文档'});
  await screen.findByRole('button',{name:'进入目录：doc（匹配 2 个文件）'});
  fireEvent.input(input,{target:{value:'  设计  '}}); fireEvent.compositionStart(input); fireEvent.submit(input.closest('form')!);expect(api.documentDirectory).toHaveBeenCalledTimes(1);
  fireEvent.compositionEnd(input); fireEvent.change(screen.getByRole('combobox',{name:'目录读取结果'}),{target:{value:'omitted'}});fireEvent.submit(input.closest('form')!);
  await waitFor(()=>expect(api.documentDirectory).toHaveBeenCalledTimes(2));
  await screen.findByRole('button',{name:'进入目录：doc（匹配 2 个文件）'});
  fireEvent.input(input,{target:{value:'not applied'}});fireEvent.click(screen.getByRole('button',{name:'下一页目录（最多 100 项）'}));
  await waitFor(()=>expect(api.documentDirectory).toHaveBeenLastCalledWith(expect.objectContaining({inventory:expect.objectContaining({query:'设计',readFilter:'omitted',snapshotId:'snapshot',cursor:'next'})})));
  await waitFor(()=>expect(screen.getByRole('list',{name:'当前层目录项目'})).toHaveFocus());
  expect(screen.getByText(/扫描候选 10.*目录范围 8.*全范围匹配 2/)).toBeInTheDocument();
});
it('ignores late project responses and does not reset on personal snapshot changes', async () => {
  let resolveOld!: (value:DocumentDirectoryPage)=>void;
  api.documentDirectory.mockImplementationOnce(()=>new Promise(resolve=>{resolveOld=resolve;})).mockResolvedValue(page({projectId:'new',entries:[]}));
  const [data,setData]=createSignal(snapshot());render(()=><DocumentDirectory snapshot={data()} selectFile={vi.fn()}/>);
  setData(snapshot('new'));await screen.findByText('当前目录没有匹配项目；这不代表仓库没有文档。');
  resolveOld(page()); await Promise.resolve();expect(screen.queryByRole('button',{name:'进入目录：doc（匹配 2 个文件）'})).not.toBeInTheDocument();
  setData({...data(),preferences:{...data().preferences,query:'personal'}});await Promise.resolve();expect(api.documentDirectory).toHaveBeenCalledTimes(2);
  setData(snapshot('new','2'));await waitFor(()=>expect(api.documentDirectory).toHaveBeenCalledTimes(3));
});
it('shows stale coverage and depth limits, and recovers failed queries explicitly', async () => {
  api.documentDirectory.mockRejectedValueOnce(new Error('expired')).mockResolvedValue(page({stale:true,error:'scan failed',entries:[{path:'deep',name:'deep',kind:'directory',matchingFiles:1,depthLimited:true,historicalReferenceFiles:0,historicalReferencePaths:[],file:null}]}));
  render(()=><DocumentDirectory snapshot={snapshot()} selectFile={vi.fn()}/>);
  await screen.findByRole('alert');fireEvent.click(screen.getByRole('button',{name:'重试当前目录'}));
  expect(await screen.findByRole('button',{name:'进入目录：deep（匹配 1 个文件）'})).toBeDisabled();
  expect(screen.getByText(/目录使用过期清单/)).toBeInTheDocument();expect(screen.getByText(/深层文件仍保留在文件清单/)).toBeInTheDocument();
});

it('defaults to current, reveals hidden history with applied filters and binds lifecycle paging', async () => {
  api.documentDirectory.mockResolvedValue(page({hiddenHistoryFiles:3,nextCursor:'next'}));
  render(()=><DocumentDirectory snapshot={snapshot()} selectFile={vi.fn()}/>);
  await screen.findByRole('button',{name:'显示历史匹配'});
  expect(api.documentDirectory).toHaveBeenLastCalledWith(expect.objectContaining({lifecycleFilter:'current'}));
  const input=screen.getByRole('textbox',{name:'筛选目录文档'});
  fireEvent.input(input,{target:{value:'unapplied'}});
  fireEvent.click(screen.getByRole('button',{name:'显示历史匹配'}));
  await waitFor(()=>expect(api.documentDirectory).toHaveBeenLastCalledWith(expect.objectContaining({parentPath:'',lifecycleFilter:'all',inventory:expect.objectContaining({query:'',snapshotId:null})})));
  expect(input).toHaveValue('');
  fireEvent.change(screen.getByRole('combobox',{name:'目录生命周期'}),{target:{value:'raw'}});
  const raw=screen.getByRole('textbox',{name:'目录生命周期原始声明'});
  expect(screen.getByRole('button',{name:'筛选目录'})).toBeDisabled();
  fireEvent.input(raw,{target:{value:'target-contract'}});fireEvent.compositionStart(raw);fireEvent.submit(raw.closest('form')!);
  expect(api.documentDirectory).toHaveBeenCalledTimes(2);
  fireEvent.compositionEnd(raw);fireEvent.submit(raw.closest('form')!);
  await waitFor(()=>expect(api.documentDirectory).toHaveBeenLastCalledWith(expect.objectContaining({lifecycleFilter:'raw:target-contract'})));
  await screen.findByRole('button',{name:'进入目录：doc（匹配 2 个文件）'});
  fireEvent.change(screen.getByRole('combobox',{name:'目录生命周期'}),{target:{value:'retired'}});
  fireEvent.click(screen.getByRole('button',{name:'下一页目录（最多 100 项）'}));
  await waitFor(()=>expect(api.documentDirectory).toHaveBeenLastCalledWith(expect.objectContaining({lifecycleFilter:'raw:target-contract',inventory:expect.objectContaining({cursor:'next'})})));
});

it('shows historical reference evidence and keeps the original historical file selectable', async () => {
  const historical={...file,lifecycle:{status:'retired',category:'retired',rawStatus:'retired',successorDeclaration:null,successorTargets:[],successorReason:null,issue:null}};
  api.documentDirectory.mockResolvedValue(page({entries:[{path:file.path,name:'design.md',kind:'design',matchingFiles:1,depthLimited:false,file:historical,historicalReferenceFiles:26,historicalReferencePaths:['doc/source.design.md']}]}));
  const select=vi.fn();render(()=><DocumentDirectory snapshot={snapshot()} selectFile={select}/>);
  expect(await screen.findByText(/历史引用占位：被 26 个/)).toBeInTheDocument();
  expect(screen.getByText(/来源路径样本.*doc\/source.design.md/)).toBeInTheDocument();
  expect(screen.getByText('生命周期：已退役')).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button',{name:'A设计 · doc/a/design.md'}));
  expect(select).toHaveBeenLastCalledWith(historical);
});
