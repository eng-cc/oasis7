import type { RuntimeOperation, RuntimeRequest, RuntimeSession, RuntimeSnapshot, RuntimeTarget } from './types';
export const runtimeLabels: Record<string, string> = {
  disabled: '默认关闭 / 已停用', disconnected: '观察未连接', connecting: '连接中', ready: '已连接', degraded: '观察降级', error: '连接错误',
  idle: '空闲', active: '运行中', waiting: '等待输入或批准', completed: '本轮结束', unknown: '尚未确认结果', clear: '未观测到后台工作',
  pending: '等待处理', awaiting: '等待回执', confirmed: '回执已确认', failed: '来源已确认失败', answered: '已回复', resolved: '来源已处理',
  sourceResolved: '来源不再等待此请求 · 工作结果另行核实',
};
export const runtimeLabel = (state: string) => runtimeLabels[state] ?? state;
export const fitsUtf8 = (value: string, limit: number) => new TextEncoder().encode(value).length <= limit;
export const sessionKey = (s: RuntimeSession) => JSON.stringify([s.instanceId, s.threadId]);
export const requestKey = (r: RuntimeRequest) => JSON.stringify([r.instanceId, r.threadId, r.turnId, r.requestId]);
export const targetOf = (s: RuntimeSession): RuntimeTarget | null => s.turnId ? {instanceId:s.instanceId, threadId:s.threadId, turnId:s.turnId} : null;
export const unresolved = (op: RuntimeOperation) => ['pending', 'awaiting', 'unknown'].includes(op.state);
export const unresolvedFor = (snapshot: RuntimeSnapshot, session: RuntimeSession) => snapshot.operations.some(op => unresolved(op) && op.instanceId === session.instanceId && (!op.threadId || op.threadId === session.threadId));
export const ownsCurrent = (snapshot: RuntimeSnapshot, session: RuntimeSession) => ['ready','degraded'].includes(snapshot.state) && snapshot.instanceId === session.instanceId && session.owned;
export const canStart = (snapshot: RuntimeSnapshot, session: RuntimeSession) => snapshot.state === 'ready' && ownsCurrent(snapshot, session) && snapshot.enabled && !snapshot.exitWaiting && snapshot.capabilities.includes('startTurn') && ['idle', 'completed'].includes(session.state) && session.backgroundState === 'clear' && !unresolvedFor(snapshot, session);
export const canSteer = (snapshot: RuntimeSnapshot, session: RuntimeSession) => snapshot.state === 'ready' && ownsCurrent(snapshot, session) && snapshot.capabilities.includes('steerTurn') && session.state === 'active' && !!session.turnId && !unresolvedFor(snapshot, session);
export const canInterrupt = (snapshot: RuntimeSnapshot, session: RuntimeSession) => ownsCurrent(snapshot, session) && snapshot.capabilities.includes('interruptTurn') && ['active','waiting','unknown'].includes(session.state) && !!session.turnId && !snapshot.operations.some(op => op.state !== 'failed' && op.method === 'turn/interrupt' && op.instanceId === session.instanceId && op.threadId === session.threadId && op.turnId === session.turnId);
export const canContinue = (snapshot: RuntimeSnapshot, session: RuntimeSession) => snapshot.state === 'ready' && snapshot.enabled && !snapshot.exitWaiting && snapshot.capabilities.includes('continueSession') && session.owned && !unresolvedFor(snapshot,session) && !snapshot.requests.some(r => r.instanceId === session.instanceId && r.threadId === session.threadId && ['pending','awaiting','unknown'].includes(r.state)) && ((ownsCurrent(snapshot,session) && ['idle','completed'].includes(session.state) && session.backgroundState === 'clear') || (session.instanceId !== snapshot.instanceId && session.restorableHistory));
export function isRuntimeSnapshot(value: unknown): value is RuntimeSnapshot {
  if (!value || typeof value !== 'object') return false;
  const s = value as Partial<RuntimeSnapshot>;
  return typeof s.projectId === 'string' && typeof s.enabled === 'boolean' && typeof s.state === 'string' && Array.isArray(s.sessions) && Array.isArray(s.requests) && Array.isArray(s.operations) && Array.isArray(s.logs) && Array.isArray(s.capabilities);
}
export function visibleOperations(snapshot: RuntimeSnapshot): RuntimeOperation[] {
  const ids = new Set(snapshot.operations.slice(0, 100).map(op => op.id));
  return snapshot.operations.filter(op => ids.has(op.id) || unresolved(op));
}
export function capabilityLabel(value: string): string {
  return ({createSession:'建立明确会话',continueSession:'核对并继续原会话',startTurn:'启动明确本轮',steerTurn:'追加到当前运行',interruptTurn:'请求中断原目标',answerRequest:'回复原始请求',readThread:'只读核对历史',reconcileOperation:'只读核对原操作来源'} as Record<string,string>)[value] ?? `未识别能力 ${value}`;
}
