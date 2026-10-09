import { render, cleanup } from "@solidjs/testing-library";
import { afterEach, expect, it } from "vitest";
import { CanonicalAgentGoal } from "./canonical_agent_goal.jsx";
afterEach(cleanup);
it("shows owner goal and pending confirmation with localized labels, hides a foreign key",()=>{
  const view={agent_id:"a",player_id:"p",public_key:"k",world_id:"w",reorg_epoch:1,goal:{message:"继续铁锭生产"}};
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
