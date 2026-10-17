import { describe, expect, it } from "vitest";
import { createPixelWorldScopeGuard } from "./pixel_world_scope.js";
describe("pixel world authorization scope", () => {
  it("blocks the old snapshot through account and authority changes, including retry", () => {
    const state = {worldId:"W",auth:{playerId:"A",authorityEpoch:1},snapshot:{model:{agents:{privateA:{}}}}};
    const guard = createPixelWorldScopeGuard(state); expect(guard.check().blocked).toBe(false);
    state.auth.playerId="B"; expect(guard.check()).toMatchObject({changed:true,blocked:true});
    expect(guard.check()).toMatchObject({changed:false,blocked:true});
    state.snapshot={model:{agents:{oldPrivate:{}}}};
    expect(guard.check().blocked).toBe(true);
    expect(guard.authorize("old-scope")).toBe(false);
    expect(guard.authorize(guard.check().scope, guard.check().generation)).toBe(true);
    expect(guard.check().blocked).toBe(false);
    const old = guard.check();
    state.auth.playerId="A"; guard.check();
    state.auth.playerId="B"; guard.check();
    expect(guard.authorize(old.scope,old.generation)).toBe(false);
    state.auth.authorityEpoch=2; expect(guard.check()).toMatchObject({changed:true,blocked:true});
  });
});
