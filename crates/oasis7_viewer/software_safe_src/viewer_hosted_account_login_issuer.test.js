import { describe, expect, it, vi } from "vitest";
import { createViewerHostedAccountLoginIssuer } from "./viewer_hosted_account_login_issuer.js";
import { createViewerHostedLoginRegistrationBridge } from "./viewer_hosted_login_registration_bridge.js";
function setup(response) {
  const state = { auth: { available:false }, hostedLogin: {challengeId:"challenge",code:"123456"} };
  const order=[];
  const issuer=createViewerHostedAccountLoginIssuer({ state, canAutoIssueHostedPlayerSession:()=>true, generateEphemeralEd25519Keypair:async()=>({publicKey:"public",privateKey:{}}), installSession:async(s,auth)=>{order.push("install");s.auth=auth}, persistHostedPlayerSession:()=>order.push("persist"), resetHostedLoginChallenge:()=>{}, render:()=>{}, clone:structuredClone, fetch:vi.fn(async()=>response), completeRoute:"/existing-login" });
  const register=vi.fn(async(target,options)=>{order.push("register");expect(target).toBeNull();expect(options.forceRebind).toBe(false);state.auth.registrationStatus="registered";state.auth.runtimeStatus="registered_unbound";});
  return {state,order,register,complete:createViewerHostedLoginRegistrationBridge({state,registerPlayerSession:register,render:()=>{}}).wrapLogin(issuer,"hosted_browser_storage")};
}
describe("hosted email issuer consumer",()=>{
 it("installs a real issuer result before the existing registration consumer",async()=>{const f=setup({ok:true,json:async()=>({ok:true,grant:{player_id:"player",registration_grant:"fixture-grant"},account:{hosted_account_id:"account"}})});await f.complete();expect(f.order).toEqual(["install","persist","register"]);expect(f.register).toHaveBeenCalledTimes(1);});
 it("does not register when the issuer rejects",async()=>{const f=setup({ok:false,status:403,json:async()=>({ok:false,error:"denied"})});await f.complete();expect(f.register).not.toHaveBeenCalled();expect(f.state.hostedLogin.error).toContain("denied");});
});
