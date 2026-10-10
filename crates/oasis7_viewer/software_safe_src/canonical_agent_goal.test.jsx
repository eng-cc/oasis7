import { render, cleanup } from "@solidjs/testing-library";
import { afterEach, expect, it } from "vitest";
import { CanonicalAgentGoal } from "./canonical_agent_goal.jsx";
afterEach(cleanup);
it("shows owner goal and pending confirmation with localized labels, hides a foreign key",()=>{
  const view={agent_id:"a",player_id:"p",public_key:"k",world_id:"w",reorg_epoch:1,current_intent_id:"goal-1",goal:{intent_id:"goal-1",status:"accepted",message:"继续铁锭生产"}};
  const state={auth:{playerId:"p",publicKey:"k",boundAgentId:"a"},worldFeed:{worldId:"w",reorgEpoch:1},
    viewerProtocol:{capabilities:["canonical_agent_chat_v1"]},canonicalAgentOwnerView:view,
    lastChatFeedback:{agentId:"a",stage:"pending",canonicalRequest:{},accepted:false}};
  const rendered=render(()=><CanonicalAgentGoal state={state} agentId="a" locale="zh"/>);
  expect(rendered.container.textContent).toContain("当前目标: 继续铁锭生产");
  expect(rendered.container.textContent).toContain("等待世界确认");
  cleanup();state.auth.publicKey="foreign";
  const hidden=render(()=><CanonicalAgentGoal state={state} agentId="a" locale="en"/>);
  expect(hidden.container.querySelector("[data-canonical-agent-goal]")).toBeNull();
});

for (const status of ["completed", "rejected", "expired", "cancelled", "superseded"]) {
  it(`hides terminal ${status} goals even if an old intent ID remains`, () => {
    const state = {auth:{playerId:"p",publicKey:"k",boundAgentId:"a"},worldFeed:{worldId:"w",reorgEpoch:1},
      viewerProtocol:{capabilities:["canonical_agent_chat_v1"]},canonicalAgentOwnerView:{agent_id:"a",player_id:"p",public_key:"k",world_id:"w",reorg_epoch:1,
        current_intent_id:"goal-1",goal:{intent_id:"goal-1",status,message:"terminal private goal"}},lastChatFeedback:null};
    const result = render(() => <CanonicalAgentGoal state={state} agentId="a" locale="en"/>);
    expect(result.container.querySelector("[data-canonical-agent-goal]")).toBeNull();
    cleanup();state.canonicalAgentOwnerView.current_intent_id = null;
    expect(render(() => <CanonicalAgentGoal state={state} agentId="a" locale="en"/>).container.querySelector("[data-canonical-agent-goal]")).toBeNull();
  });
}
for (const status of ["accepted", "blocked"]) {
  it(`shows only matching active ${status} intent`, () => {
    const state = {auth:{playerId:"p",publicKey:"k",boundAgentId:"a"},worldFeed:{worldId:"w",reorgEpoch:1},
      viewerProtocol:{capabilities:["canonical_agent_chat_v1"]},canonicalAgentOwnerView:{agent_id:"a",player_id:"p",public_key:"k",world_id:"w",reorg_epoch:1,
        current_intent_id:"goal-1",goal:{intent_id:"goal-1",status,message:"active private goal"}},lastChatFeedback:null};
    expect(render(() => <CanonicalAgentGoal state={state} agentId="a" locale="en"/>).container.textContent).toContain("active private goal");
    cleanup();state.canonicalAgentOwnerView.current_intent_id = null;
    expect(render(() => <CanonicalAgentGoal state={state} agentId="a" locale="en"/>).container.querySelector("[data-canonical-agent-goal]")).toBeNull();
    cleanup();state.canonicalAgentOwnerView.current_intent_id = "other";
    expect(render(() => <CanonicalAgentGoal state={state} agentId="a" locale="en"/>).container.querySelector("[data-canonical-agent-goal]")).toBeNull();
  });
}
