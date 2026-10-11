import { beforeEach, expect, it, vi } from 'vitest';
const mock = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: mock.invoke }));
import { bridge } from './bridge';
beforeEach(() => mock.invoke.mockReset());
it('binds decisions and historical reads to finite camelCase IPC commands', async () => {
  mock.invoke.mockResolvedValue(undefined);
  const draft = {id:null,content:'决定',reason:'理由',scope:'工程',relatedSection:'doc/core/prd.md#3',confirmed:true};
  await bridge.saveDecision(draft);
  expect(mock.invoke).toHaveBeenLastCalledWith('save_decision_draft', {draft});
  await bridge.readVersion('doc/core/prd.md#3','saved-sha');
  expect(mock.invoke).toHaveBeenLastCalledWith('read_version', {sectionId:'doc/core/prd.md#3',revision:'saved-sha'});
  await bridge.associateWork('https://github.com/eng-cc/oasis7/pull/1',['goal-a','goal-b']);
  expect(mock.invoke).toHaveBeenLastCalledWith('associate_work', {workUrl:'https://github.com/eng-cc/oasis7/pull/1',goalSectionIds:['goal-a','goal-b']});
});
it('bounds Chinese search and refreshes without an execution command', async () => {
  mock.invoke.mockResolvedValue([]);
  await bridge.search('合入');
  expect(mock.invoke).toHaveBeenLastCalledWith('search_rules',{query:'合入',limit:100});
  await bridge.refresh();
  expect(mock.invoke).toHaveBeenLastCalledWith('refresh_source',{});
  expect(mock.invoke.mock.calls.some(([name]) => /start|input|interrupt/.test(name))).toBe(false);
});
