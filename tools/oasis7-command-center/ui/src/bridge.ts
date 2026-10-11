import { invoke, isTauri } from '@tauri-apps/api/core';
import type { DecisionDraft, DecisionRecord, DocumentSection, GitHubSnapshot, Preferences, Snapshot, WorkAssociation, RuntimeSnapshot, RuntimeTarget, RequestAnswer, QuitStatus, SessionConfig } from './types';
import { listen } from '@tauri-apps/api/event';
export const connected = () => isTauri();
function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (!connected()) return Promise.reject(new Error('浏览器预览未接入桌面核心。请从 App 打开真实项目。'));
  return invoke<T>(command, args);
}
// Explicit finite commands: no arbitrary method, shell, SQL or filesystem proxy.
export const bridge = {
  snapshot: () => call<Snapshot>('query_view'),
  openProject: (path: string) => call<Snapshot>('open_project', { path }),
  chooseProject: () => call<Snapshot | null>('choose_project'),
  refresh: () => call<Snapshot>('refresh_source'),
  search: (query: string) => call<DocumentSection[]>('search_rules', { query, limit: 100 }),
  saveDecision: (draft: DecisionDraft) => call<DecisionRecord>('save_decision_draft', { draft }),
  savePreferences: (preferences: Preferences) => call<void>('save_preference', { preferences }),
  recordReading: (s: DocumentSection) => call<void>('read_source', { sectionId: s.id, revision: s.revision, line: s.line }),
  rebuildCache: () => call<Snapshot>('rebuild_cache'),
  github: () => call<GitHubSnapshot>('get_github_snapshot'),
  refreshGithub: (ghPath: string | null) => call<GitHubSnapshot>('refresh_github', { ghPath }),
  openSource: (target: string) => call<void>('open_source_target', { target }),
  readVersion: (sectionId: string, revision: string) => call<DocumentSection | null>('read_version', { sectionId, revision }),
  associateWork: (workUrl: string, goalSectionIds: string[]) => call<WorkAssociation>('associate_work', { workUrl, goalSectionIds }),
};
export const runtimeBridge = {
  snapshot: () => call<RuntimeSnapshot>('get_runtime'),
  reconcile: (operationId: string) => call<RuntimeSnapshot>('reconcile_runtime_operation', { operationId }),
  enabled: (enabled: boolean) => call<RuntimeSnapshot>('set_runtime_enabled', { enabled }),
  connect: (binaryPath: string | null) => call<RuntimeSnapshot>('connect_runtime', { binaryPath }),
  create: (config: SessionConfig) => call<RuntimeSnapshot>('create_runtime_session', { config }),
  continue: (threadId: string) => call<RuntimeSnapshot>('continue_runtime_session', { threadId }),
  start: (threadId: string, input: string) => call<RuntimeSnapshot>('start_runtime_turn', { threadId, input }),
  steer: (target: RuntimeTarget, input: string) => call<RuntimeSnapshot>('steer_runtime_turn', { target, input }),
  interrupt: (target: RuntimeTarget) => call<RuntimeSnapshot>('interrupt_runtime_turn', { target }),
  answer: (reply: RequestAnswer) => call<RuntimeSnapshot>('answer_runtime_request', { reply }),
  exitStatus: () => call<QuitStatus>('get_exit_status'),
  waitExit: () => call<QuitStatus>('request_exit_wait'),
  stopExit: () => call<QuitStatus>('request_exit_stop'),
  background: () => call<QuitStatus>('cancel_exit_wait'),
};
export async function subscribeRuntime(invalidate: () => void, exitRequired: () => void): Promise<() => void> {
  if (!connected()) return () => {};
  const handlers = await Promise.allSettled([
    listen('command-center:runtime-invalidated', invalidate),
    listen('command-center:exit-required', exitRequired),
  ]);
  const stops = handlers.flatMap(result => result.status === 'fulfilled' ? [result.value] : []);
  const failed = handlers.find(result => result.status === 'rejected');
  if (failed?.status === 'rejected') { stops.forEach(stop => stop()); throw failed.reason; }
  return () => stops.forEach(unlisten => unlisten());
}
export function errorMessage(error: unknown): string { return error instanceof Error ? error.message : String(error); }
export function statusLabel(state: string): string {
  return ({ loading: '读取中', ready: '已读取', disconnected: '未接入', success: '已读取', empty: '读取成功 · 空', partial: '部分读取', stale: '已过期', error: '读取失败', unconnected: '未接入' } as Record<string,string>)[state] ?? state;
}
export function observedTime(value: string | null | undefined): string {
  if (!value) return '尚无成功读取';
  const date = /^\d+$/.test(value) ? new Date(Number(value) * 1000) : new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN');
}
export function acceptsEnter(event: { key: string; isComposing: boolean; keyCode?: number }, composing: boolean): boolean {
  return event.key === 'Enter' && !event.isComposing && !composing && event.keyCode !== 229;
}
