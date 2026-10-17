const collections = {entity:['entities', value => `${value.kind}:${value.entity_id}`], relation:['relations', value => value.relation_id], aggregate:['aggregates', value => value.aggregate_id], event:['events', value => value.event_id]};
const entityKinds = new Set(['agent','location','factory']);
const sequence = value => typeof value === 'string' && /^(0|[1-9]\d*)$/.test(value);
// Public authority identity only: commit position/root/tick and reader evidence are excluded.
export function worldViewAuthorityKey(view) {
  const version=view?.source_binding?.version, commit=version?.commit, binding=commit?.binding;
  const world=commit?.world;
  return JSON.stringify([view?.world_id ?? null,view?.scope_id ?? null,view?.reorg_epoch ?? null,
    version?.visibility_scope ?? null,version?.projection_revision ?? null,
    world?.world_id ?? null,world?.genesis_digest ?? null,
    ...['provider_world_id','branch_id','reorg_generation','authority_generation','permission_generation','governing_manifest_ref'].map(key=>binding?.[key] ?? null)]);
}
export function worldViewQueryKey(query) {
  const {request_generation, cursor, resume_cursor, ...stable} = query;
  return JSON.stringify(stable);
}
export function applyWorldViewDelta(previous, update) {
  if (!previous || !['delta','unchanged'].includes(update.delivery)
    || worldViewAuthorityKey(previous) !== worldViewAuthorityKey(update)
    || previous.world_id !== update.world_id || previous.scope_id !== update.scope_id
    || String(previous.reorg_epoch) !== String(update.reorg_epoch) || update.carry_forward !== true
    || previous.query_id !== update.query_id || previous.source_cut !== update.base_cut
    || !sequence(previous.update_sequence) || update.previous_sequence !== previous.update_sequence
    || !sequence(update.update_sequence) || BigInt(update.update_sequence) !== BigInt(previous.update_sequence) + 1n
    || !update.source_cut || (Object.hasOwn(update,'world_bounds') && JSON.stringify(update.world_bounds) !== JSON.stringify(previous.world_bounds))) throw Error('World view continuity gap');
  if (!sequence(update.observed_at_tick)) throw Error('World view observation binding invalid');
  if (!Array.isArray(update.removals || [])) throw Error('World view removals invalid');
  const operations = Object.values(collections).reduce((n,[field])=>n+(update[field]?.length || 0),0)+(update.removals?.length || 0);
  if (update.delivery === 'unchanged' && operations !== 0) throw Error('World view unchanged records invalid');
  if (operations > 200 || new TextEncoder().encode(JSON.stringify(update)).byteLength > 2 * 1024 * 1024) throw Error('World view delta operation budget exceeded');
  const removalKeys = new Set();
  for (const removal of update.removals || []) {
    if (!Object.hasOwn(collections,removal?.record_kind) || typeof removal.id !== 'string' || !removal.id
      || typeof removal.world_deleted !== 'boolean' || typeof removal.reason !== 'string' || !removal.reason
      || removal.record_kind === 'entity' && !entityKinds.has(removal.kind)) throw Error('World view removal invalid');
    const [field,key] = collections[removal.record_kind];
    const id = removal.record_kind === 'entity' ? `${removal.kind}:${removal.id}` : removal.id;
    const identity = `${removal.record_kind}:${id}`;
    const old = (previous[field] || []).find(value=>key(value) === id);
    if (!old || removalKeys.has(identity) || removal.kind != null && removal.kind !== old.kind
      || removal.world_deleted && !(removal.record_kind === 'entity' && removal.kind === 'factory' && removal.reason === 'factory_recycled')) throw Error('World view removal identity conflict');
    removalKeys.add(identity);
  }
  const result = {...previous,...update};
  for (const [kind,[field,key]] of Object.entries(collections)) {
    const upserts = update[field] || [];
    if (!Array.isArray(upserts)) throw Error('World view delta records invalid');
    const records = new Map((previous[field] || []).map(value => [key(value),value]));
    const seen = new Set();
    for (const value of upserts) {
      const id = key(value);
      if (kind === 'entity' && (!entityKinds.has(value.kind) || typeof value.entity_id !== 'string' || !value.entity_id) || typeof id !== 'string' || !id) throw Error('World view identity invalid');
      const validCut = value.provenance?.valid_at_source_cut ?? (kind === 'event' ? value.source_cut : value.valid_at_source_cut);
      if (validCut != null && validCut !== update.source_cut) throw Error('World view upsert cut mismatch');
      if (!id || seen.has(id)) throw Error('World view delta identity conflict');
      seen.add(id); records.set(id,value);
    }
    for (const removal of update.removals || []) {
      if (!collections[removal.record_kind] || typeof removal.id !== 'string' || typeof removal.world_deleted !== 'boolean') throw Error('World view removal invalid');
      if (removal.record_kind !== kind) continue;
      const id = kind === 'entity' ? `${removal.kind}:${removal.id}` : removal.id;
      if (seen.has(id)) throw Error('World view delta removal conflict');
      records.delete(id);
    }
    result[field] = [...records.values()].map(value => {
      if (value.provenance) return {...value,provenance:{...value.provenance,valid_at_source_cut:update.source_cut,observed_at_tick:update.observed_at_tick}};
      return {...value,...(kind === 'event' ? {source_cut:update.source_cut} : {}),valid_at_source_cut:update.source_cut,observed_at_tick:update.observed_at_tick};
    });
  }
  if (Object.values(collections).reduce((count,[field]) => count + result[field].length,0) > 1000
    || new TextEncoder().encode(JSON.stringify(result)).byteLength > 2 * 1024 * 1024) throw Error('World view delta budget exceeded');
  return validateWorldViewEvolution(freezeWorldView(result));
}

export function validateWorldViewEvolution(view) {
  for (const event of view.events || []) {
    const identity=event.identity;
    if (!identity || identity.world_id !== view.world_id || String(identity.reorg_epoch) !== String(view.reorg_epoch)
      || !sequence(identity.event_seq) || !sequence(identity.reorg_epoch) || !sequence(event.logical_time) || event.event_id !== `${identity.world_id}:${identity.reorg_epoch}:${identity.event_seq}`
      || event.source_class !== 'runtime_journal_payload' || event.source_cut !== view.source_cut
      || event.feed_ref?.world_id !== identity.world_id || String(event.feed_ref?.reorg_epoch) !== String(identity.reorg_epoch) || String(event.feed_ref?.event_seq) !== String(identity.event_seq)) throw Error('World view event binding mismatch');
  }
  for (const aggregate of view.aggregates || []) {
    if (!aggregate.aggregate_id || !sequence(aggregate.member_count)
      || aggregate.provenance?.geometry_source !== 'display_aggregation'
      || aggregate.provenance?.valid_at_source_cut !== view.source_cut) throw Error('World view aggregate binding mismatch');
  }
  return view;
}
export function freezeWorldView(view) {
  if (!view.source_frozen) return view;
  const downgrade = value=>value === 'current' ? 'last_known' : value;
  return {...view,
    entities:(view.entities || []).map(value=>({...value,provenance:{...value.provenance,freshness:downgrade(value.provenance?.freshness)}})),
    aggregates:(view.aggregates || []).map(value=>({...value,provenance:{...value.provenance,freshness:downgrade(value.provenance?.freshness)}})),
    relations:(view.relations || []).map(value=>({...value,freshness:downgrade(value.freshness)})),
    events:(view.events || []).map(value=>({...value,freshness:downgrade(value.freshness)}))};
}
