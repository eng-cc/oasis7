import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@solidjs/testing-library";
import { PasskeyAccountSurface } from "./viewer_passkey_account_surface.jsx";

afterEach(() => { cleanup(); vi.unstubAllGlobals(); });
describe("hosted account surface compatibility", () => {
  it("preserves legacy email entry only when v2 is absent", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => ({ ok: false, status: 404, json: async () => { throw new Error("HTML response"); } })));
    render(() => <PasskeyAccountSurface legacyForm={() => <div>Legacy email login</div>} />);
    await waitFor(() => expect(screen.getByText("Legacy email login")).toBeTruthy());
  });
  it("keeps an unavailable v2 service visible without exposing a legacy bypass", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => ({ ok: false, status: 503, json: async () => ({ ok: false, error_code: "identity_store_unavailable" }) })));
    render(() => <PasskeyAccountSurface legacyForm={() => <div>Legacy email login</div>} />);
    await waitFor(() => expect(screen.getByRole("status").textContent).toContain("账号服务暂时不可用"));
    expect(screen.queryByText("Legacy email login")).toBeNull();
  });
});
