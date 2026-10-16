import { describe, it, expect } from 'vitest';
import { createWorldViewClient } from './pixel_world_world_view.js';
const page = (extra = {}) => ({ world_id: 'w', scope_id: 'viewer_snapshot', source_cut: '10', request_generation:'2', world_bounds: { width_cm: 100, depth_cm: 100, height_cm: 20 }, entities: [], completeness: 'complete', ...extra });
describe('authorized world view client', () => {
  it('publishes complete only after every same cut page', async () => {
    const published=[]; let calls=0;
    const client=createWorldViewClient({request:async()=>++calls===1?page({entities:[{entity_id:'A',kind:'agent'}],next_cursor:'opaque',completeness:'partial'}):page({entities:[{entity_id:'B',kind:'agent'}]}),onChange:s=>published.push(s)});
    client.reset('scope'); const completed = await client.query({});
    expect(completed.entities).toHaveLength(2);
    expect(client.state().view.entities.map(e=>e.entity_id)).toEqual(['A','B']);
    expect(published.find(s=>s.view)?.view.completeness).toBe('partial');
    expect(client.state().view.completeness).toBe('complete');
  });
  it('rejects mixed cut without erasing prior authorized page',async()=>{
    let calls=0;const client=createWorldViewClient({request:async()=>++calls===1?page({entities:[{entity_id:'A',kind:'agent'}],next_cursor:'x',completeness:'partial'}):page({source_cut:'11'})});
    client.reset('s');await client.query({});expect(client.state().view.completeness).toBe('partial');expect(client.state().error).toMatch(/binding/);
  });
  it('clears private data and ignores old scope response',async()=>{
    let finish;const client=createWorldViewClient({request:()=>new Promise(r=>{finish=r;})});
    client.reset('old');const pending=client.query({});client.reset('new');finish(page({entities:[{entity_id:'secret',kind:'agent'}]}));expect(await pending).toBeNull();expect(client.state().view).toBeNull();
  });
  it('retains a failed refresh as stale rather than empty complete',async()=>{
    let fail=false;let generation=0;const client=createWorldViewClient({request:async(q)=>{if(fail)throw Error('offline');return page({request_generation:q.request_generation,entities:[{entity_id:'A',kind:'agent'}]});}});
    client.reset('s');await client.query({});fail=true;expect(await client.query({})).toBeNull();expect(client.state().view.entities).toHaveLength(1);expect(client.state().stale).toBe(true);
  });
});

describe('world view source and transport', () => {
  it('rejects unknown geometry for following, and unsafe decimal coordinates', async () => {
    const {worldViewPosition}=await import('./pixel_world_world_view.js');
    const entity={position:{x_cm:'10',y_cm:'20',z_cm:'0'},provenance:{freshness:'current',geometry_source:'authoritative',valid_at_source_cut:'10'}};
    expect(worldViewPosition(entity,page())).toEqual({x_cm:10,y_cm:20,z_cm:0});
    expect(worldViewPosition({...entity,provenance:{...entity.provenance,freshness:'unknown'}},page())).toBeNull();
    expect(worldViewPosition({...entity,position:{...entity.position,x_cm:'9007199254740993'}},page())).toBeNull();
    expect(worldViewPosition(entity,page(),true)).toBeNull();
  });
  it('correlates concurrent WS replies by unique request identity', async () => {
    const {createWorldViewTransport}=await import('./pixel_world_world_view.js');const sent=[];
    const transport=createWorldViewTransport({send:m=>sent.push(m)});
    const first=transport.request({request_generation:'2'}),second=transport.request({request_generation:'2',cursor:'page2'});
    transport.handle({type:'world_view',view:page({request_generation:sent[1].query.request_generation,source_cut:'second'})});
    transport.handle({type:'world_view',view:page({request_generation:sent[0].query.request_generation,source_cut:'first'})});
    expect((await first).source_cut).toBe('first');expect((await second).source_cut).toBe('second');transport.clear();
  });
  it('preserves prior data on explicit source unavailable',async()=>{
    let fail=false;const client=createWorldViewClient({request:async q=>page({request_generation:q.request_generation,...(fail?{completeness:'unknown',unavailable_reasons:['cursor_reset']}:{entities:[{kind:'agent',entity_id:'A'}]})})});
    client.reset('s','w');await client.query({});fail=true;await client.query({});expect(client.state().view.entities).toHaveLength(1);expect(client.state().stale).toBe(true);
  });
});

it('rejects replies after socket or authorization identity changes',async()=>{
  const {createWorldViewTransport}=await import('./pixel_world_world_view.js');let scope='a',socket={};const sent=[];
  const transport=createWorldViewTransport({send:m=>sent.push(m),getScope:()=>scope,getConnection:()=>socket});
  const first=transport.request({request_generation:'1'});scope='b';
  transport.handle({type:'world_view',view:page({request_generation:sent[0].query.request_generation})});
  await expect(first).rejects.toThrow('scope changed');
  const second=transport.request({request_generation:'2'});socket={};
  transport.handle({type:'world_view',view:page({request_generation:sent[1].query.request_generation})});
  await expect(second).rejects.toThrow('scope changed');transport.clear();
});

it('downgrades every accumulated page when the source is frozen',async()=>{
 let calls=0;const current=id=>({kind:'agent',entity_id:id,provenance:{freshness:'current'}});
 const client=createWorldViewClient({request:async q=>page({request_generation:q.request_generation,...(++calls===1?{entities:[current('A')],next_cursor:'p2',completeness:'partial'}:{entities:[current('B')],source_frozen:true})})});
 client.reset('s');await client.query({});expect(client.state().view.source_frozen).toBe(true);
 expect(client.state().view.entities.map(e=>e.provenance.freshness)).toEqual(['last_known','last_known']);
});
