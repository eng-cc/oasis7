// Recovery snapshots are correlated reads. They never adopt state on receipt.
export function createScopeSnapshotTransport({ send, getScope, getConnection, timeoutMs = 10000 }) {
  const pending = new Map();
  let sequence = 0;
  return {
    request() {
      for (const item of pending.values()) { clearTimeout(item.timer); item.reject(Error('Scope snapshot request superseded')); }
      pending.clear();
      const requestGeneration = `scope:${++sequence}`;
      const scope = getScope(), connection = getConnection();
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => { pending.delete(requestGeneration); reject(Error('Scope snapshot request timed out')); }, timeoutMs);
        pending.set(requestGeneration, {resolve, reject, timer, scope, connection});
        try { send({type:'request_scope_snapshot',request_generation:requestGeneration}); }
        catch (error) { clearTimeout(timer); pending.delete(requestGeneration); reject(error); }
      }).then(snapshot => {
        if (scope !== getScope() || connection !== getConnection()) throw Error('Scope snapshot scope changed');
        return snapshot;
      });
    },
    handle(message) {
      if (message?.type !== 'scope_snapshot') return false;
      const item = pending.get(message.request_generation);
      if (!item) return true;
      clearTimeout(item.timer); pending.delete(message.request_generation);
      if (item.scope !== getScope() || item.connection !== getConnection()) item.reject(Error('Scope snapshot scope changed'));
      else if (!message.snapshot || typeof message.snapshot !== 'object') item.reject(Error('Scope snapshot unavailable'));
      else item.resolve(message.snapshot);
      return true;
    },
    clear() {
      for (const item of pending.values()) { clearTimeout(item.timer); item.reject(Error('Scope snapshot disconnected')); }
      pending.clear();
    },
  };
}
