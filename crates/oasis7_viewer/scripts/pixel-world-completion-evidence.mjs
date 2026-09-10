import { join } from 'node:path';

export async function completionEvidence({name,url,outDir,evalJson,browserJson,runBrowser,writeJson,assert}) {
  await browserJson(['open',url]);
  await evalJson('new Promise(r=>setTimeout(()=>r(true),700))');
  const rectScript=`(() => { const target=document.querySelector('[data-renderer-target="true"][data-agent-id="agent-0"]');const r=target.getBoundingClientRect();return {x:r.left+r.width/2,y:r.top+r.height/2,width:r.width,height:r.height,camera:window.__AW_TEST__.getState().pixelWorldCamera};})()`;
  const before=await evalJson(rectScript);
  await runBrowser(['mouse','move',String(Math.round(before.x)),String(Math.round(before.y))]);
  await runBrowser(['mouse','down']);
  await runBrowser(['mouse','move',String(Math.round(before.x+30)),String(Math.round(before.y+10))]);
  await runBrowser(['mouse','up']);
  await evalJson('new Promise(r=>setTimeout(()=>r(true),150))');
  const after=await evalJson(rectScript);
  assert(after.camera.pan_x_px !== before.camera.pan_x_px,'target-origin drag did not reach renderer',{before,after});
  await evalJson(`(() => {window.__AW_TEST__.select({kind:'agent',id:'agent-1'});return true;})()`);
  const keyboardBefore=await evalJson(`window.__OASIS7_PIXEL_WORLD_RENDER_DTO__().selection`);
  await evalJson(`(() => {const t=document.querySelector('[data-renderer-target="true"][data-agent-id="agent-0"]');t.focus();return true;})()`);
  const focused=await evalJson(`({id:document.activeElement.dataset.agentId,focusVisible:document.activeElement.matches(':focus-visible')})`);
  await runBrowser(['press','Enter']);
  const keyboard=await evalJson(`({selection:window.__OASIS7_PIXEL_WORLD_RENDER_DTO__().selection,focused:document.activeElement.dataset.agentId})`);
  assert(focused.id === 'agent-0' && keyboard.selection.id === 'agent-0','keyboard target activation lost identity',{focused,keyboard});
  await evalJson(`(() => {window.__AW_TEST__.select({kind:'agent',id:'agent-1'});return true;})()`);
  const touchBefore=await evalJson(`window.__OASIS7_PIXEL_WORLD_RENDER_DTO__().selection`);
  const endpointResult=await browserJson(['get','cdp-url']);
  const endpoint=JSON.stringify(endpointResult).match(/ws:\/\/[^"\\\s]+/)?.[0];
  assert(endpoint,'browser did not expose touch emulation transport',endpointResult);
  const socket=new WebSocket(endpoint);
  await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  let id=0;const requests=new Map();
  socket.addEventListener('message',event=>{const message=JSON.parse(event.data);const request=requests.get(message.id);if(request){requests.delete(message.id);message.error?request.reject(new Error(JSON.stringify(message.error))):request.resolve(message.result);}});
  const cdp=(method,params={},sessionId)=>new Promise((resolve,reject)=>{const next=++id;requests.set(next,{resolve,reject});socket.send(JSON.stringify({id:next,method,params,sessionId}));});
  try {
    const {targetInfos}=await cdp('Target.getTargets');
    const target=targetInfos.find(target=>target.type==='page' && target.url.includes('pixel_world_visual_fixture='));
    const {sessionId}=await cdp('Target.attachToTarget',{targetId:target.targetId,flatten:true});
    await cdp('Emulation.setTouchEmulationEnabled',{enabled:true,maxTouchPoints:1},sessionId);
    await evalJson(`(() => {window.__completionPointers=[];document.addEventListener('pointerdown',e=>window.__completionPointers.push({type:e.pointerType,id:e.pointerId}),{capture:true});return true;})()`);
    const touchPoint=await evalJson(rectScript);
    await cdp('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:touchPoint.x,y:touchPoint.y}]},sessionId);
    await cdp('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]},sessionId);
  } finally {socket.close();}
  const touch=await evalJson(`({pointers:window.__completionPointers,selection:window.__OASIS7_PIXEL_WORLD_RENDER_DTO__().selection})`);
  assert(touch.pointers.some(pointer=>pointer.type==='touch') && touch.selection.id==='agent-0','touch activation did not reach selected entity',touch);
  await runBrowser(['screenshot','--full',join(outDir,`${name}-completion-input.png`)]);
  const feedStates=[];
  for (const [status,seq] of [['ready',1],['ready',2],['replay',2],['gap',2],['unavailable',2]]) {
    const feed={schema_version:'world_feed/v1',world_id:'completion-world',reorg_epoch:0,cursor:null,status,events:status==='gap'||status==='unavailable'?[]:[{event_seq:seq,kind:'resource_change',summary:`Published resource event ${seq}`,detail:'fixture journal entry',receipt_ref:null}],gap_reason:status==='gap'?'cursor_gap':null,unavailable_reason:status==='unavailable'?'source_unavailable':null,snapshot_reload_required:false};
    const consumed=await evalJson(`window.__AW_TEST__.injectWorldFeedForTest(${JSON.stringify(feed)})`);
    await evalJson('new Promise(r=>setTimeout(()=>r(true),150))');
    const observed={feed:consumed,...await evalJson(`({text:document.querySelector('[data-viewer-overlay="feed"]').textContent})`)};
    assert(observed.feed.status===status,'feed consumer status differs from published fixture',{status,observed});
    await runBrowser(['screenshot','--full',join(outDir,`${name}-feed-${status}-${seq}.png`)]);
    feedStates.push({status,seq,observed});
  }
  const result={before,after,focused,keyboardBefore,keyboard,touchBefore,touch,feedStates};
  const pending=await evalJson(`(async()=>{
    const snapshot=window.__OASIS7_PIXEL_WORLD_VISUAL_FIXTURES__.selected_blocker();
    snapshot.model.agents={};
    snapshot.player_gameplay={stage_id:'awaiting_agent_claim',stage_status:'ready',execution_state:'waiting_for_intent',available_actions:[{action_id:'claim_first_agent',label:'Claim First Agent',protocol_action:'gameplay_action.submit',target_agent_id:'agent-0',disabled_reason:null}],recent_feedback:null};
    window.__AW_TEST__.injectSnapshot(snapshot);
    await new Promise(r=>setTimeout(r,200));
    const button=document.querySelector('[data-primary-action="true"]');button.focus();
    button.click();
    const next=document.querySelector('[data-primary-action="true"]');
    return {feedback:window.__AW_TEST__.getState().lastGameplayActionFeedback,sameNode:button===next,focused:document.activeElement===next,busy:next.getAttribute('aria-busy'),text:next.textContent,receipt:document.querySelector('[data-viewer-overlay="receipt"]')?.textContent,render:window.__OASIS7_PIXEL_WORLD_RENDER_DTO__().commercial_surface};
  })()`);
  assert(pending.sameNode && pending.focused && pending.busy==='true','pending transition lost in-place action',pending);
  pending.detailsLayout=await evalJson(`(()=>{const panel=document.querySelector('[data-viewer-overlay="world-summary"]');const summary=panel.querySelector('summary');const r=summary.getBoundingClientRect();return {background:getComputedStyle(panel).backgroundColor,top:r.top,bottom:r.bottom,visible:panel.contains(document.elementFromPoint(r.left+20,r.top+20)),scrollable:getComputedStyle(panel).overflowY};})()`);
  assert(pending.detailsLayout.visible && pending.detailsLayout.background==='rgb(20, 29, 25)','details introduction is obscured',pending.detailsLayout);
  await runBrowser(['screenshot','--full',join(outDir,`${name}-pending-action.png`)]);
  result.pending=pending;
  await browserJson(['open',`${url}&pixel_world_renderer=defer`]);
  await evalJson('new Promise(r=>setTimeout(()=>r(true),700))');
  result.fallback=await evalJson(`({status:window.__AW_TEST__.getState().pixelWorldRuntimeStatus,source:window.__AW_TEST__.getState().pixelWorldRuntimeSource,text:document.body.textContent})`);
  assert(result.fallback.status==='unavailable','explicit deferred renderer did not expose fallback',result.fallback);
  result.fallback.layout=await evalJson(`(()=>{const card=document.querySelector('[data-viewer-overlay="renderer-unavailable"]');const button=card.querySelector('button');const r=button.getBoundingClientRect();const feed=document.querySelector('[data-viewer-overlay="feed"]').getBoundingClientRect();return {card:card.getBoundingClientRect().toJSON(),feed:feed.toJSON(),retry:r.toJSON(),retryVisible:button.contains(document.elementFromPoint(r.left+r.width/2,r.top+r.height/2))};})()`);
  assert(result.fallback.layout.retryVisible && result.fallback.layout.retry.height>=44,'renderer recovery action is obscured',result.fallback.layout);
  await runBrowser(['screenshot','--full',join(outDir,`${name}-renderer-unavailable.png`)]);
  writeJson(`${name}-completion-evidence.json`,result);
  return result;
}
