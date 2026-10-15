import { describe, it, expect, vi } from "vitest";
import { renderAccountEmailRecovery } from "./viewer_account_email_recovery.js";

describe("optional v2 email surface", () => {
  it("does not offer first-passkey upgrade for an active server passkey", () => {
    const root = document.createElement("div");
    const render = (credentials) => {
      root.replaceChildren();
      renderAccountEmailRecovery(root, { state: {}, account: { id: "player-1" }, credentials,
        capabilities: { email: { available: true } }, client: {}, run: vi.fn(), render: vi.fn() });
    };

    render([{ id: "passkey-1", state: "active", type: "passkey" }]);
    expect([...root.querySelectorAll("button")].map((button) => button.textContent))
      .not.toContain("验证邮箱并添加首个通行密钥");

    render([{ id: "email-1", state: "active", type: "email" }]);
    expect([...root.querySelectorAll("button")].map((button) => button.textContent))
      .toContain("验证邮箱并添加首个通行密钥");
  });

  it("uses the email adapter through explicit input and never invokes an authenticator automatically", async () => {
    const root = document.createElement("div");
    const state = {};
    const client = { emailOptions: vi.fn(async () => ({ otp_id: "otp" })), emailComplete: vi.fn(async () => ({ ok: true })) };
    const run = async (action) => { await action(); render(); };
    const render = () => { root.replaceChildren(); renderAccountEmailRecovery(root, { state, account: null, credentials: [],
      capabilities: { email: { available: true } }, client, run, render }); };
    render();
    root.querySelector("button").click();
    await Promise.resolve();
    expect(client.emailOptions).not.toHaveBeenCalled();
    const input = root.querySelector("input"); input.value = "owner@example.test"; input.dispatchEvent(new Event("input"));
    [...root.querySelectorAll("button")].find((button) => button.textContent === "发送本次操作验证码").click();
    await Promise.resolve(); await Promise.resolve();
    expect(client.emailOptions).toHaveBeenCalledWith("owner@example.test");
    const code = root.querySelector("input"); code.value = "123456"; code.dispatchEvent(new Event("input"));
    [...root.querySelectorAll("button")].find((button) => button.textContent === "确认本次操作").click();
    await Promise.resolve(); await Promise.resolve();
    expect(client.emailComplete).toHaveBeenCalledWith({ otp_id: "otp" }, "123456");
    expect(state.code).toBe(""); expect(state.otp).toBeNull();
  });
});
