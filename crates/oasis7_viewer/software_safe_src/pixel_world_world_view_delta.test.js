import { describe,it,expect } from 'vitest';
import { applyWorldViewDelta, worldViewAuthorityKey } from './pixel_world_world_view_delta.js';
const entity = id => ({kind:'factory',entity_id:id,provenance:{freshness:'unknown',valid_at_source_cut:'A'}});
const baseline = () => ({world_id:'W',scope_id:'S',reorg_epoch:'9',query_id:'Q',source_cut:'A',update_sequence:'1',entities:[entity('retained'),entity('removed')],relations:[],aggregates:[],events:[]});
const update = extra => ({delivery:'delta',world_id:'W',scope_id:'S',reorg_epoch:'9',query_id:'Q',base_cut:'A',source_cut:'B',previous_sequence:'1',update_sequence:'2',observed_at_tick:'5',carry_forward:true,entities:[],removals:[],...extra});
describe('atomic world view updates',()=>{
 it('removes only the projection record and carries retained unknown provenance to the confirmed cut',()=>{
  const previous=baseline();const next=applyWorldViewDelta(previous,update({entities:[{...entity('new'),provenance:{freshness:'unknown',valid_at_source_cut:'B'}}],removals:[{record_kind:'entity',kind:'factory',id:'removed',reason:'projection_removed',world_deleted:false}]}));
  expect(next.entities.map(value=>value.entity_id)).toEqual(['retained','new']);expect(next.entities[0].provenance).toEqual({freshness:'unknown',valid_at_source_cut:'B',observed_at_tick:'5'});
  expect(previous.entities).toHaveLength(2);expect(previous.source_cut).toBe('A');
 });
 it.each([{base_cut:'old'},{previous_sequence:'0'},{update_sequence:'1'},{update_sequence:'3'},{query_id:'other'},{scope_id:'other'},{reorg_epoch:'10'},{carry_forward:false}])('rejects a continuity mismatch atomically: %j',extra=>{
  const previous=baseline();expect(()=>applyWorldViewDelta(previous,update(extra))).toThrow();expect(previous.entities).toHaveLength(2);
 });
 it('rejects simultaneous upsert and removal instead of publishing a half update',()=>{
  expect(()=>applyWorldViewDelta(baseline(),update({entities:[{...entity('new'),provenance:{freshness:'unknown',valid_at_source_cut:'B'}}],removals:[{record_kind:'entity',kind:'factory',id:'new',reason:'deleted',world_deleted:true}]}))).toThrow();
 });
 it('counts aggregates and events against the same bound',()=>{
  expect(()=>applyWorldViewDelta(baseline(),update({aggregates:Array.from({length:1000},(_,i)=>({aggregate_id:String(i)}))}))).toThrow(/budget/);
 });
});

it.each([
 [{record_kind:'entity',kind:'factory',id:'removed',reason:'projection_removed',world_deleted:false},{record_kind:'entity',kind:'factory',id:'removed',reason:'factory_recycled',world_deleted:true}],
 [{record_kind:'entity',kind:'factory',id:'not-in-baseline',reason:'factory_recycled',world_deleted:true}],
 [{record_kind:'entity',kind:'world_event',id:'removed',reason:'projection_removed',world_deleted:false}],
 [{record_kind:'entity',kind:'factory',id:'',reason:'projection_removed',world_deleted:false}],
])('rejects duplicate, unknown or invalid removal identities atomically: %j',removals=>{
 const previous=baseline();expect(()=>applyWorldViewDelta(previous,update({removals}))).toThrow();expect(previous.entities).toHaveLength(2);
});
it('rebinds event source and validity plus observed tick without upgrading freshness',()=>{
 const previous=baseline();previous.events=[{event_id:'W:9:7',identity:{world_id:'W',reorg_epoch:'9',event_seq:'7'},feed_ref:{world_id:'W',reorg_epoch:'9',event_seq:'7'},logical_time:'2',source_class:'runtime_journal_payload',source_cut:'A',valid_at_source_cut:'A',observed_at_tick:'1',freshness:'last_known'}];
 const next=applyWorldViewDelta(previous,update({delivery:'unchanged'}));
 expect(next.events[0]).toMatchObject({source_cut:'B',valid_at_source_cut:'B',observed_at_tick:'5',freshness:'last_known'});expect(next.entities[0].provenance).toMatchObject({observed_at_tick:'5',freshness:'unknown'});
});

it('fences public authority changes while allowing normal source advancement',()=>{
 const source=(permission,position)=>({version:{visibility_scope:'public',projection_revision:'v1',commit:{world:{world_id:'W',genesis_digest:'G'},position,state_root_ref:`root-${position}`,binding:{provider_world_id:'P',branch_id:'main',reorg_generation:'0',authority_generation:'1',permission_generation:permission,governing_manifest_ref:'M'}}}});
 const previous={...baseline(),source_binding:source('1',1)};
 expect(()=>applyWorldViewDelta(previous,update({source_binding:source('1',2)}))).not.toThrow();
 expect(()=>applyWorldViewDelta(previous,update({source_binding:source('2',2)}))).toThrow(/continuity/);
 const changedReader={...previous,source_binding:{...previous.source_binding,reader_binding:'not-part-of-public-authority',reader_summary:'not-read'}};
 expect(worldViewAuthorityKey(changedReader)).toBe(worldViewAuthorityKey(previous));
});
