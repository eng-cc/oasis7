import { expect,it,vi } from 'vitest';
import { beginTreeReadingWriter,queueTreeReadingWrite } from './documentTreeReading';
it('serializes dispatched writes and discards an old view’s queued save behind a new owner',async()=>{
  const calls:string[]=[];let finish!:()=>void;const old=beginTreeReadingWriter();
  const first=queueTreeReadingWrite(old,()=>true,async()=>{calls.push('old-dispatched');await new Promise<void>(resolve=>{finish=resolve;});});
  await Promise.resolve();await Promise.resolve();
  const obsolete=queueTreeReadingWrite(old,()=>true,async()=>{calls.push('old-queued');});const current=beginTreeReadingWriter();
  const latest=queueTreeReadingWrite(current,()=>true,async()=>{calls.push('new-view');});finish();await Promise.all([first,obsolete,latest]);expect(calls).toEqual(['old-dispatched','new-view']);
});
it('checks current source validity at dispatch time instead of enqueue time',async()=>{
  const owner=beginTreeReadingWriter();let valid=true;const write=vi.fn(async()=>{});const result=queueTreeReadingWrite(owner,()=>valid,write);valid=false;await result;expect(write).not.toHaveBeenCalled();
});
