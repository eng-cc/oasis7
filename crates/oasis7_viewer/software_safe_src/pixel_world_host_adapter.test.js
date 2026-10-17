import { describe, expect, it, vi } from "vitest";
const mock = vi.hoisted(() => ({ load: null }));
vi.mock("./pixel_world_runtime_loader.js", () => ({ createPixelWorldRuntimeBridge: (...args) => mock.load(...args) }));
import { createPixelWorldHostAdapter } from "./pixel_world_host_adapter.js";
function deferred() { let resolve; const promise = new Promise(r => { resolve = r; }); return { promise, resolve }; }
function runtime() { return { source: "test", bridge: { mount: vi.fn(() => ({status:"ready"})), unmount: vi.fn(), update: vi.fn() }, deriveRenderState: vi.fn(input => input) }; }
describe("pixel world host adapter lifecycle", () => {
  it("disposes a late runtime after unmount without mounting or exposing its DTO", async () => {
    const gate = deferred(); const old = runtime(); mock.load = () => gate.promise;
    const adapter = createPixelWorldHostAdapter({});
    const pending = adapter.mount({}, {agents:[{id:"private-A"}]});
    adapter.unmount(); gate.resolve(old);
    expect((await pending).status).toBe("detached");
    expect(old.bridge.mount).not.toHaveBeenCalled();
    expect(old.bridge.unmount).toHaveBeenCalled();
    expect(adapter.deriveRenderState({})).toBeNull();
  });
  it("ignores old runtime events and failures after a newer mount", async () => {
    const callbacks = []; const first = runtime(), second = runtime();
    mock.load = async options => { callbacks.push(options); return callbacks.length === 1 ? first : second; };
    const select = vi.fn(), fatal = vi.fn(); const adapter = createPixelWorldHostAdapter({onSelectEntity:select,onFatal:fatal});
    await adapter.mount({}, {}); await adapter.mount({}, {});
    callbacks[0].onEvent({type:"select_entity",selection:{kind:"agent",id:"old"}});
    callbacks[0].onFatal({code:"late_failure"});
    expect(select).not.toHaveBeenCalled(); expect(fatal).not.toHaveBeenCalled();
    expect(first.bridge.unmount).toHaveBeenCalled();
  });
  it("does not resurrect a render result when mount resolves after scope invalidation", async () => {
    const gate = deferred(), held = runtime(); held.bridge.mount.mockReturnValue(gate.promise); mock.load = async () => held;
    const adapter = createPixelWorldHostAdapter({}); const pending = adapter.mount({}, {private:true});
    await Promise.resolve(); adapter.unmount(); gate.resolve({status:"ready"});
    expect(await pending).toMatchObject({status:"detached",renderState:null});
    expect(held.bridge.unmount).toHaveBeenCalledTimes(2);
  });
});
