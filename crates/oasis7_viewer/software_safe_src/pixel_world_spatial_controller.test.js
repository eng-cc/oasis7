import { afterEach, describe, expect, it, vi } from 'vitest';
import { createRoot } from 'solid-js';
import { createPixelWorldSpatialController } from './pixel_world_spatial_controller.js';
const bounds = n => ({min_x_cm:String(n),max_x_cm:String(n+100),min_y_cm:'0',max_y_cm:'100'});
const page = (query, extra={}) => ({world_id:'W',scope_id:'S',source_cut:'cut',request_generation:query.request_generation,entities:[],relations:[],completeness:'complete',...extra});
let dispose;
afterEach(()=>{dispose?.();vi.useRealTimers();});
function setup(request) {
 const core={state:{worldId:'W',auth:{playerId:'A'},viewerProtocol:{capabilities:['world_view','world_view_pins_v1']},selectedKind:null,selectedId:null},requestWorldView:vi.fn(request|| (async q=>page(q))),applySelection:vi.fn(),buildGameplaySummary:()=>({})};
 let controller;createRoot(cleanup=>{dispose=cleanup;controller=createPixelWorldSpatialController({core,enabled:()=>true,getRenderState:()=>null,getLocale:()=> 'en',debounceMs:180});});
 return {core,controller};
}
describe('viewport query controller',()=>{
 it('coalesces camera events using only Rust bounds and ignores apply echoes',async()=>{
  vi.useFakeTimers();const {core,controller}=setup();
  controller.cameraChanged({viewport_bounds:bounds(1)});controller.cameraChanged({viewport_bounds:bounds(2)});controller.cameraChanged({viewport_bounds:bounds(3)});
  expect(core.requestWorldView).not.toHaveBeenCalled();await vi.advanceTimersByTimeAsync(180);
  expect(core.requestWorldView).toHaveBeenCalledOnce();expect(core.requestWorldView.mock.calls[0][0].bounds).toEqual(bounds(3));
  controller.cameraChanged({viewport_bounds:bounds(3)});await vi.advanceTimersByTimeAsync(1000);
  expect(core.requestWorldView).toHaveBeenCalledOnce();expect(controller.navigation()).toBeNull();
 });
 it('invalidates an old result immediately when camera moves before the debounce',async()=>{
  vi.useFakeTimers();let finish;const {controller}=setup(q=>new Promise(resolve=>{finish=()=>resolve(page(q,{entities:[{kind:'agent',entity_id:'old'}]}));}));
  const old=controller.query({});controller.cameraChanged({viewport_bounds:bounds(30)});finish();await old;
  expect(controller.state().view).toBeNull();
 });
 it('cancels queued queries on scope change and disposal',async()=>{
  vi.useFakeTimers();const {core,controller}=setup();controller.cameraChanged({viewport_bounds:bounds(4)});
  core.state.auth.playerId='B';controller.resetScope();await vi.advanceTimersByTimeAsync(180);expect(core.requestWorldView).not.toHaveBeenCalled();
  controller.cameraChanged({viewport_bounds:bounds(5)});dispose();await vi.advanceTimersByTimeAsync(180);expect(core.requestWorldView).not.toHaveBeenCalled();
 });
});

it('pins the inspected facility, original target and explicit follow without changing actor control',async()=>{
 const {core,controller}=setup(async q=>page(q,{entities:[{kind:'factory',entity_id:'F',label:'Furnace'}]}));
 core.buildGameplaySummary=()=>({acceptedIntentTarget:'original'});core.state.selectedKind='agent';core.state.selectedId='old';
 await controller.query({});expect(controller.select({kind:'module_visual',id:'F'})).toEqual({kind:'factory',id:'F'});
 controller.follow({kind:'agent',id:'following'});await controller.query({bounds:bounds(40)});
 expect(core.requestWorldView.mock.lastCall[0].pinned_entities).toEqual([{kind:'factory',id:'F'},{kind:'agent',id:'original'},{kind:'agent',id:'following'}]);
 expect(core.applySelection).not.toHaveBeenCalled();expect(core.state.selectedId).toBe('old');
});
it('preserves viewport bounds through world refresh without issuing navigation',async()=>{
 vi.useFakeTimers();const {core,controller}=setup();controller.cameraChanged({viewport_bounds:bounds(10)});await vi.advanceTimersByTimeAsync(180);
 const snapshot={time:5};controller.refresh(snapshot);await Promise.resolve();controller.refresh(snapshot);
 expect(core.requestWorldView).toHaveBeenCalledTimes(2);expect(core.requestWorldView.mock.lastCall[0].bounds).toEqual(bounds(10));expect(controller.navigation()).toBeNull();
});
it('returns to the original target then reads its viewport without retaining a hidden search filter',async()=>{
 vi.useFakeTimers();const {core,controller}=setup(async q=>page(q,{entities:[{kind:'agent',entity_id:'main',position:{x_cm:'10',y_cm:'20',z_cm:'0'},provenance:{geometry_source:'authoritative',freshness:'current',valid_at_source_cut:'cut'}}]}));
 core.buildGameplaySummary=()=>({acceptedIntentTarget:'main'});await controller.returnTarget();expect(controller.navigation()).toMatchObject({mode:'locate',target:{kind:'agent',id:'main'}});
 controller.cameraChanged({viewport_bounds:bounds(6)});await vi.advanceTimersByTimeAsync(180);
 expect(core.requestWorldView.mock.lastCall[0].search).toBeUndefined();expect(core.requestWorldView.mock.lastCall[0].pinned_entities).toContainEqual({kind:'agent',id:'main'});
});
it('never borrows a stale snapshot selection for a new projected entity or a changed scope',async()=>{
 const {core,controller}=setup(async q=>page(q,{entities:[{kind:'agent',entity_id:'new'}]}));
 core.state.selectedKind='agent';core.state.selectedId='old';await controller.query({});
 expect(controller.select({kind:'agent',id:'new'})).toEqual({kind:'agent',id:'new'});expect(controller.selection()).toEqual({kind:'agent',id:'new'});expect(core.applySelection).not.toHaveBeenCalled();
 expect(controller.select({kind:'location',id:'not-authorized'})).toBeNull();
 core.state.auth.playerId='B';expect(controller.selection()).toBeNull();expect(controller.select({kind:'agent',id:'new'})).toBeNull();
});

it('keeps querying an older world_view server without sending the unnegotiated pins extension',async()=>{
 vi.useFakeTimers();const {core,controller}=setup();core.state.viewerProtocol.capabilities=['world_view'];
 core.state.selectedKind='agent';core.state.selectedId='visible';await controller.query({search:'ore'});
 expect(core.requestWorldView.mock.lastCall[0]).not.toHaveProperty('pinned_entities');
 controller.cameraChanged({viewport_bounds:bounds(8)});await vi.advanceTimersByTimeAsync(180);
 expect(core.requestWorldView.mock.lastCall[0]).toMatchObject({search:'ore',bounds:bounds(8)});
 expect(core.requestWorldView.mock.lastCall[0]).not.toHaveProperty('pinned_entities');expect(controller.state().error).toBeNull();
});
