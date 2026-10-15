import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { createSignal } from 'solid-js';
import DocumentInventory from './DocumentInventory';
import type { Snapshot, DocumentSection } from './types';
import type { DocumentInventoryPage, IndexedDocument } from './generated/core';
const api = vi.hoisted(() => ({ documentInventory: vi.fn() }));
vi.mock('./bridge', () => ({ bridge: api, errorMessage: (error: unknown) => String(error), observedTime: () => '核验时间' }));
const section: DocumentSection = { id: 'source', path: 'doc/design.md', title: '设计章节', revision: 'rev', committedRevision: 'head', dirty: false, content: '# 设计', line: 1, factKind: 'sourceFact', lifecycle: null };
const file = (path = 'doc/design.md', extra: Partial<IndexedDocument> = {}): IndexedDocument => ({
  fileKey: path, path, title: path === section.path ? '设计文档' : path,
  kind: 'design', state: 'read', reason: null, revision: 'rev', bytes: 10, dirty: false, lifecycle: null, pairings: null, ...extra,
});
const snapshot = (id = 'project', generation = '1'): Snapshot => ({
  project: { id, path: '/repo', head: 'head', branch: 'main', gitCommonDir: '/repo/.git', environment: 'local' },
  sources: [{ kind: 'local', generation, state: 'partial', lastAttempt: '1', lastSuccess: '1', sourceRevision: 'head', coverage: '', error: null }],
  sections: [section], decisions: [], preferences: { projectPath: '/repo', query: '', view: 'rules' }, associations: [], readingPositions: [], git: {status:'',worktrees:''},
});
const page = (files = [file()], extra: Partial<DocumentInventoryPage> = {}): DocumentInventoryPage => ({
  schemaVersion: 1, projectId: 'project', snapshotId: 'snapshot', baselineOid: 'head', generatedAt: '1',
  candidates: 3, read: 1, omitted: 2, readBytes: 10, totalMatches: files.length,
  partial: true, stale: false, error: null, files, nextCursor: null, ...extra,
});
beforeEach(() => api.documentInventory.mockReset());
afterEach(cleanup);
it('shows per-file omissions and opens only chapters from the matching file revision', async () => {
  api.documentInventory.mockResolvedValue(page([file(), file('doc/empty.md', {bytes:0}), file('doc/deleted.md', {state:'deleted',reason:'deleted',revision:null,bytes:null})]));
  const showSource = vi.fn();
  render(() => <DocumentInventory snapshot={snapshot()} showSource={showSource} />);
  await screen.findByRole('button', {name:'设计文档'});
  fireEvent.click(screen.getByRole('button', {name:'设计文档'}));
  fireEvent.click(screen.getByRole('button', {name:'设计章节 · 第 1 行'}));
  expect(showSource).toHaveBeenCalledWith(section, expect.any(HTMLElement));
  fireEvent.click(screen.getByRole('button', {name:'doc/empty.md'}));
  expect(screen.getByText('已读取，文件没有可展示的章节。')).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', {name:'doc/deleted.md'}));
  expect(screen.queryByRole('button', {name:/第 1 行/})).not.toBeInTheDocument();
  expect(screen.getAllByText(/工作树文件已删除/).length).toBeGreaterThan(0);
});
it('binds paging to applied filters and snapshot, and preserves IME composition', async () => {
  api.documentInventory.mockResolvedValueOnce(page()).mockResolvedValue(page([file()], {nextCursor:'next'}));
  render(() => <DocumentInventory snapshot={snapshot()} showSource={vi.fn()} />);
  await screen.findByRole('button', {name:'设计文档'});
  const input = screen.getByRole('textbox', {name:'筛选文档文件'});
  fireEvent.input(input, {target:{value:'设计'}});
  fireEvent.compositionStart(input);
  fireEvent.submit(input.closest('form')!);
  expect(api.documentInventory).toHaveBeenCalledTimes(1);
  fireEvent.compositionEnd(input);
  fireEvent.change(screen.getByRole('combobox'), {target:{value:'omitted'}});
  fireEvent.click(screen.getByRole('button', {name:'应用筛选'}));
  await waitFor(() => expect(api.documentInventory).toHaveBeenCalledTimes(2));
  await waitFor(() => expect(screen.getByRole('button', {name:/下一页/})).toBeEnabled());
  // Unsubmitted edits must not silently change the next cursor's filter.
  fireEvent.input(input, {target:{value:'未提交的新筛选'}});
  fireEvent.click(screen.getByRole('button', {name:/下一页/}));
  await waitFor(() => expect(api.documentInventory).toHaveBeenCalledTimes(3));
  expect(api.documentInventory).toHaveBeenLastCalledWith({ schemaVersion:1, projectId:'project', snapshotId:'snapshot', query:'设计', readFilter:'omitted', cursor:'next' });
});
it('rejects late responses after a project switch and after a same-project refresh', async () => {
  let late!: (value: DocumentInventoryPage) => void;
  api.documentInventory.mockImplementationOnce(() => new Promise(resolve => {late = resolve;})).mockResolvedValue(page([file('doc/new.md')]));
  const [state, setState] = createSignal(snapshot());
  render(() => <DocumentInventory snapshot={state()} showSource={vi.fn()} />);
  setState(snapshot('other', '2'));
  await screen.findByRole('button', {name:'doc/new.md'});
  late(page([file('doc/old.md')]));
  await Promise.resolve();
  expect(screen.queryByRole('button', {name:'doc/old.md'})).not.toBeInTheDocument();
  api.documentInventory.mockImplementationOnce(() => new Promise(resolve => {late = resolve;})).mockResolvedValue(page([file('doc/refreshed.md')]));
  fireEvent.click(screen.getByRole('button', {name:'重新查询首页'}));
  setState(snapshot('other', '3'));
  await screen.findByRole('button', {name:'doc/refreshed.md'});
  late(page([file('doc/obsolete.md')]));
  await Promise.resolve();
  expect(screen.queryByRole('button', {name:'doc/obsolete.md'})).not.toBeInTheDocument();
});
it('distinguishes cache failure from empty results and keeps stale coverage visible', async () => {
  api.documentInventory.mockRejectedValueOnce(new Error('清单缺失')).mockResolvedValue(page([], {stale:true,error:'源读取失败',totalMatches:0}));
  render(() => <DocumentInventory snapshot={snapshot()} showSource={vi.fn()} />);
  await screen.findByRole('alert');
  expect(screen.queryByText(/当前筛选没有匹配文件/)).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', {name:'重新查询首页'}));
  await screen.findByText(/当前筛选没有匹配文件/);
  expect(screen.getByText(/清单已过期/)).toBeInTheDocument();
});

it('preserves filters, page and file selection when only personal snapshot data changes', async () => {
  api.documentInventory.mockResolvedValue(page([file()], {nextCursor:'next'}));
  const [state, setState] = createSignal(snapshot());
  render(() => <DocumentInventory snapshot={state()} showSource={vi.fn()} />);
  await screen.findByRole('button', {name:'设计文档'});
  const input = screen.getByRole('textbox', {name:'筛选文档文件'});
  fireEvent.input(input, {target:{value:'设计'}});
  fireEvent.click(screen.getByRole('button', {name:'应用筛选'}));
  await waitFor(() => expect(api.documentInventory).toHaveBeenCalledTimes(2));
  await waitFor(() => expect(screen.getByRole('button', {name:/下一页/})).toBeEnabled());
  fireEvent.click(screen.getByRole('button', {name:/下一页/}));
  await waitFor(() => expect(api.documentInventory).toHaveBeenCalledTimes(3));
  fireEvent.click(await screen.findByRole('button', {name:'设计文档'}));
  expect(screen.getByRole('region', {name:'文件读取详情'})).toBeInTheDocument();
  setState({...snapshot(), preferences:{...snapshot().preferences, query:'个人偏好更新'}});
  await Promise.resolve();
  expect(api.documentInventory).toHaveBeenCalledTimes(3);
  expect(input).toHaveValue('设计');
  expect(screen.getByRole('region', {name:'文件读取详情'})).toBeInTheDocument();
});

it('distinguishes legacy pairing coverage, explicit absence and unresolved targets', async () => {
  api.documentInventory.mockResolvedValue(page([file(), file('doc/none.md', {pairings:{targets:[],issue:null}}), file('doc/paired.md', {pairings:{targets:[{declarationLine:8,destination:'design.md#section',target:'doc/design.md#section',resolution:'fragment_unchecked'}],issue:'incomplete'}})]));
  render(() => <DocumentInventory snapshot={snapshot()} showSource={vi.fn()} />);
  fireEvent.click(await screen.findByRole('button', {name:'设计文档'}));
  expect(screen.getByText('旧清单尚未解析配对，请刷新本地事实。')).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', {name:'doc/none.md'}));
  expect(screen.getByText('本次范围未发现显式配对系统设计链接。')).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', {name:'doc/paired.md'}));
  expect(screen.getByText(/第 8 行 · doc\/design.md#section · 文件已读取，章节锚点尚未核对/)).toBeInTheDocument();
  expect(screen.getByText('配对解析不完整：incomplete')).toBeInTheDocument();
});
