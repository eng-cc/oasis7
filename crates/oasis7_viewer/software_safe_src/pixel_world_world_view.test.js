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

it('accumulates relation pages with their endpoints and downgrades the entire frozen cut',async()=>{
 const relation={relation_id:'r',kind:'logistics_route',source_class:'runtime_projection',status:'active',freshness:'current',valid_at_source_cut:'10',from:{kind:'agent',id:'A'},to:{kind:'factory',id:'F'}};
 let call=0;const client=createWorldViewClient({request:async q=>page({request_generation:q.request_generation,...(++call===1?{entities:[{kind:'agent',entity_id:'A'}],relations:[relation],next_cursor:'p2',completeness:'partial'}:{entities:[{kind:'factory',entity_id:'F'}],source_frozen:true})})});
 client.reset('s');await client.query({});const view=client.state().view;
 expect(view.relations).toHaveLength(1);expect(view.relations[0].freshness).toBe('last_known');
 const {worldViewRelations}=await import('./pixel_world_world_view.js');expect(worldViewRelations(view)).toHaveLength(1);
 expect(worldViewRelations({...view,entities:view.entities.slice(0,1)})).toHaveLength(0);
});
it('counts routes and entities together against the response page budget',async()=>{
 const client=createWorldViewClient({request:async q=>page({request_generation:q.request_generation,entities:Array.from({length:200},(_,i)=>({kind:'agent',entity_id:String(i)})),relations:[{relation_id:'overflow'}]})});
 client.reset('s');expect(await client.query({})).toBeNull();expect(client.state().error).toMatch(/binding/);
});
it('does not invent coordinates from missing or blank decimal fields',async()=>{
 const {worldViewPosition}=await import('./pixel_world_world_view.js');
 for (const x_cm of [null,'',undefined]) expect(worldViewPosition({position:{x_cm,y_cm:'2',z_cm:'0'},provenance:{freshness:'current',geometry_source:'authoritative',valid_at_source_cut:'10'}},page())).toBeNull();
});
it('keeps verified pinned geometry when a dense scope reaches the display budget',async()=>{
 const entities=Array.from({length:1001},(_,i)=>({kind:'agent',entity_id:i===0?'original-target':`A${i}`,position:{x_cm:'1',y_cm:'2',z_cm:'0'},provenance:{freshness:'current',geometry_source:'authoritative',valid_at_source_cut:'10'}}));
 let calls=0;const client=createWorldViewClient({request:async q=>{calls++;const offset=Number(q.cursor||0);const end=offset+q.page_size;return page({request_generation:q.request_generation,entities:entities.slice(offset,end),next_cursor:end<entities.length?String(end):null,completeness:end<entities.length?'partial':'complete'});}});
 client.reset('s');const view=await client.query({pinned_entities:[{kind:'agent',id:'original-target'}]});
 expect(calls).toBe(5);expect(view.entities).toHaveLength(1000);expect(view.budget_limited).toBe(true);expect(view.completeness).toBe('partial');expect(view.next_cursor).toBe('1000');
 expect(client.state().stale).toBe(false);expect(client.state().error).toBeNull();expect(client.state().loading).toBe(false);
 const {worldViewPosition}=await import('./pixel_world_world_view.js');expect(worldViewPosition(view.entities[0],view)).toEqual({x_cm:1,y_cm:2,z_cm:0});
});
it('stops before a whole fixed-size cursor page would exceed the combined object budget',async()=>{
 const sizes=[];const client=createWorldViewClient({request:async q=>{sizes.push(q.page_size);const offset=Number(q.cursor||0);return page({request_generation:q.request_generation,entities:Array.from({length:q.page_size},(_,i)=>({kind:'agent',entity_id:`A${offset+i}`})),next_cursor:String(offset+q.page_size),completeness:'partial'});}});
 client.reset('s');const view=await client.query({page_size:67});expect(view.entities).toHaveLength(938);expect(sizes).toEqual(Array(14).fill(67));expect(view.budget_limited).toBe(true);expect(client.state().stale).toBe(false);
});
