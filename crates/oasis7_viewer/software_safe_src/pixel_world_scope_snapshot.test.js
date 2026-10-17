import { describe, it, expect, vi } from 'vitest';
import { createScopeSnapshotTransport } from './pixel_world_scope_snapshot.js';

describe('scope snapshot recovery correlation',()=>{
  function setup() {
    let scope='old',connection={};const sent=[];
    const transport=createScopeSnapshotTransport({send:message=>sent.push(message),getScope:()=>scope,getConnection:()=>connection});
    return {transport,sent,setScope:value=>{scope=value;},setConnection:value=>{connection=value;}};
  }
  it('ignores an ordinary snapshot arriving later on the same socket',async()=>{
    const {transport,sent}=setup();const accepted=vi.fn();
    const pending=transport.request().then(snapshot=>{accepted(snapshot);return snapshot;});
    expect(transport.handle({type:'snapshot',snapshot:{private:'old'}})).toBe(false);
    await Promise.resolve();expect(accepted).not.toHaveBeenCalled();
    const fresh={model:{agents:{}}};
    expect(transport.handle({type:'scope_snapshot',request_generation:sent[0].request_generation,snapshot:fresh})).toBe(true);
    expect(await pending).toBe(fresh);expect(accepted).toHaveBeenCalledOnce();
  });
  it('rejects a correlated reply captured for an old authorization scope',async()=>{
    const {transport,sent,setScope}=setup();const pending=transport.request();setScope('new');
    transport.handle({type:'scope_snapshot',request_generation:sent[0].request_generation,snapshot:{private:'old'}});
    await expect(pending).rejects.toThrow('scope changed');
  });
  it('rejects a correlated reply captured for an old socket',async()=>{
    const {transport,sent,setConnection}=setup();const pending=transport.request();setConnection({});
    transport.handle({type:'scope_snapshot',request_generation:sent[0].request_generation,snapshot:{private:'old'}});
    await expect(pending).rejects.toThrow('scope changed');
  });
  it('consumes an uncorrelated scoped response without adopting its contents',async()=>{
    const {transport,sent}=setup();let settled=false;const pending=transport.request().then(snapshot=>{settled=true;return snapshot;});
    expect(transport.handle({type:'scope_snapshot',request_generation:'different',snapshot:{private:'old'}})).toBe(true);
    await Promise.resolve();expect(settled).toBe(false);
    transport.handle({type:'scope_snapshot',request_generation:sent[0].request_generation,snapshot:{fresh:true}});
    expect(await pending).toEqual({fresh:true});
  });
});

it('supersedes an old scope request even after the same identity returns',async()=>{
 let scope='A';const sent=[];const connection={};
 const transport=createScopeSnapshotTransport({send:m=>sent.push(m),getScope:()=>scope,getConnection:()=>connection});
 const old=transport.request();scope='B';const middle=transport.request();scope='A';const latest=transport.request();
 await expect(old).rejects.toThrow('superseded');await expect(middle).rejects.toThrow('superseded');
 transport.handle({type:'scope_snapshot',request_generation:sent[0].request_generation,snapshot:{old:true}});
 transport.handle({type:'scope_snapshot',request_generation:sent[2].request_generation,snapshot:{fresh:true}});
 expect(await latest).toEqual({fresh:true});
});
it('rechecks scope before resolving the caller microtask',async()=>{
 let scope='A';const sent=[];const connection={};
 const transport=createScopeSnapshotTransport({send:m=>sent.push(m),getScope:()=>scope,getConnection:()=>connection});
 const pending=transport.request();transport.handle({type:'scope_snapshot',request_generation:sent[0].request_generation,snapshot:{old:true}});scope='B';
 await expect(pending).rejects.toThrow('scope changed');
});
