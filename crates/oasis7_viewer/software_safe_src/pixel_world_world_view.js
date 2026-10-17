import { pixelWorldScope } from "./pixel_world_scope.js";
// A read-only client. Request generation fences callbacks; it is not a world version.
export function worldViewScopeKey(state, deployment = '') {
  return JSON.stringify([deployment, pixelWorldScope(state)]);
}
const binding = view => JSON.stringify([view.world_id, view.scope_id, view.source_cut, view.world_bounds]);
export function createWorldViewClient({ request, onChange = () => {} }) {
  let generation = 0, scope = null, expectedWorld = null;
  let state = { view: null, loading: false, stale: false, error: null };
  const publish = patch => { state = { ...state, ...patch }; onChange(state); };
  return {
    state: () => state,
    reset(nextScope, worldId = null) { generation += 1; scope = nextScope; expectedWorld = worldId; publish({ view: null, loading: false, stale: false, error: null }); },
    async query(query = {}) {
      const current = ++generation, currentScope = scope;
      const active = () => current === generation && currentScope === scope;
      const requestGeneration = String(current);
      publish({ loading: true, error: null });
      let first = null, cursor, entities = new Map(), seen = new Set(), pages = 0;
      const deadline = Date.now() + 30000;
      try {
        do {
          if (++pages > 20 || Date.now() > deadline) throw Error('World view request budget exceeded');
          const page = await request({ ...query, page_size: Math.min(200, query.page_size || 200), request_generation: requestGeneration, ...(cursor ? { cursor } : {}) });
          if (!active()) return null;
          if (page?.unavailable_reasons?.length && page.completeness === 'unknown') throw Error('World view scope unavailable');
          if (expectedWorld && page?.world_id !== expectedWorld) throw Error('World view world binding mismatch');
          if (!page || !Array.isArray(page.entities) || !page.world_id || !page.source_cut || page.request_generation !== requestGeneration) throw Error('World view response binding mismatch');
          if (new TextEncoder().encode(JSON.stringify(page)).byteLength > 2 * 1024 * 1024) throw Error('World view page byte budget exceeded');
          if (page.entities.length > 200 || (first && binding(first) !== binding(page))) throw Error('World view page binding mismatch');
          first ||= page;
          if (page.source_frozen) {
            first = { ...first, source_frozen: true };
            entities = new Map([...entities].map(([key, entity]) => [key, entity.provenance?.freshness === 'current' ? { ...entity, provenance: { ...entity.provenance, freshness: 'last_known', reason: 'frozen_pagination_cut' } } : entity]));
          }
          for (let entity of page.entities) {
            if (first.source_frozen && entity.provenance?.freshness === 'current') entity = { ...entity, provenance: { ...entity.provenance, freshness: 'last_known', reason: 'frozen_pagination_cut' } };
            if (!entity.entity_id || !entity.kind) throw Error('World view entity identity missing');
            const key = `${entity.kind}:${entity.entity_id}`;
            if (entities.has(key) && JSON.stringify(entities.get(key)) !== JSON.stringify(entity)) throw Error('World view conflicting entity');
            entities.set(key, entity);
          }
          if (entities.size > 1000) throw Error('World view entity budget exceeded');
          cursor = page.next_cursor || null;
          if (cursor && seen.has(cursor)) throw Error('World view cursor cycle');
          if (cursor) seen.add(cursor);
          publish({ view: { ...first, entities: [...entities.values()], next_cursor: cursor, completeness: cursor ? 'partial' : page.completeness }, stale: false });
        } while (cursor);
        if (active()) { publish({ loading: false }); return state.view; }
        return null;
      } catch (error) {
        if (active()) publish({ loading: false, stale: Boolean(state.view), error: String(error?.message || error) });
        return null;
      }
    },
  };
}

// Only a current, verified authoritative point can move the camera or sustain follow.
export function worldViewPosition(entity, view, stale = false, allowDerived = false) {
  const p = entity?.provenance;
  if (stale || p?.freshness !== 'current' || p?.valid_at_source_cut !== view?.source_cut || !(p?.geometry_source === 'authoritative' || (allowDerived && p?.geometry_source === 'location_derived'))) return null;
  const point = entity.position;
  if (!point) return null;
  const converted = Object.fromEntries(['x_cm','y_cm','z_cm'].map(k => [k, Number(point[k])]));
  return Object.values(converted).every(Number.isSafeInteger) ? converted : null;
}

export function createWorldViewTransport({ send, timeoutMs = 10000, getScope = () => null, getConnection = () => null }) {
  const pending = new Map(); let sequence = 0;
  return {
    request(query) {
      const id = `${query.request_generation}:${++sequence}`;
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => { pending.delete(id); reject(Error('World view request timed out')); }, timeoutMs);
        pending.set(id, { resolve, reject, timer, scope: getScope(), connection: getConnection() });
        try { send({ type: 'request_world_view', query: { ...query, request_generation: id } }); }
        catch (error) { clearTimeout(timer); pending.delete(id); reject(error); }
      }).then(view => ({ ...view, request_generation: query.request_generation }));
    },
    handle(message) {
      if (message?.type !== 'world_view' && message?.type !== 'world_view_error') return false;
      const payload = message.view || message.error;
      const item = pending.get(payload?.request_generation);
      if (item) {
        clearTimeout(item.timer); pending.delete(payload.request_generation);
        if (item.scope !== getScope() || item.connection !== getConnection()) item.reject(Error('World view scope changed'));
        else if (message.type === 'world_view') item.resolve(payload); else item.reject(Error(payload.message || payload.code || 'World view unavailable'));
      }
      return true;
    },
    clear() { for (const item of pending.values()) { clearTimeout(item.timer); item.reject(Error('World view scope disconnected')); } pending.clear(); },
  };
}
