import { renderAccountEmailRecovery } from "./viewer_account_email_recovery.js";
const API = "/api/public/hosted-account/v2";

export function decodePasskeyBytes(encoded) {
  if (typeof encoded !== "string" || !/^[A-Za-z0-9_-]*$/.test(encoded)) throw new Error("invalid_passkey_encoding");
  const binary = atob(encoded.replace(/-/g, "+").replace(/_/g, "/"));
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}

export function encodePasskeyBytes(bytes) {
  return btoa(String.fromCharCode(...new Uint8Array(bytes))).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

export function browserPasskeyOptions(options) {
  const result = structuredClone(options);
  const key = result.publicKey;
  if (!key) throw new Error("invalid_passkey_options");
  key.challenge = decodePasskeyBytes(key.challenge);
  if (key.user) key.user.id = decodePasskeyBytes(key.user.id);
  for (const list of [key.excludeCredentials, key.allowCredentials]) {
    for (const credential of list || []) credential.id = decodePasskeyBytes(credential.id);
  }
  return result;
}

export function serializePasskey(credential) {
  const response = { clientDataJSON: encodePasskeyBytes(credential.response.clientDataJSON) };
  if (credential.response.attestationObject) {
    response.attestationObject = encodePasskeyBytes(credential.response.attestationObject);
    if (typeof credential.response.getTransports === "function") response.transports = credential.response.getTransports();
  } else {
    response.authenticatorData = encodePasskeyBytes(credential.response.authenticatorData);
    response.signature = encodePasskeyBytes(credential.response.signature);
    response.userHandle = credential.response.userHandle ? encodePasskeyBytes(credential.response.userHandle) : null;
  }
  return { id: credential.id, rawId: encodePasskeyBytes(credential.rawId), type: credential.type,
    response, extensions: credential.getClientExtensionResults?.() || {} };
}

export function createPasskeyAccountClient({ fetch: fetchImpl = globalThis.fetch,
  credentials = globalThis.navigator?.credentials } = {}) {
  let csrf = null;
  let recoveryOperation = null;
  let recoveryCancelOperation = null;
  const pendingCeremonies = new Map();
  const pendingActions = new Map();
  async function request(path, body, method = body === undefined ? "GET" : "POST") {
    if (body !== undefined) { const capabilities = await request("/capabilities"); csrf = capabilities.csrf_token; }
    const headers = { Accept: "application/json" };
    if (body !== undefined) { headers["Content-Type"] = "application/json"; headers["X-Oasis7-CSRF"] = csrf || ""; }
    const response = await fetchImpl(`${API}${path}`, { method, headers, credentials: "same-origin", cache: "no-store",
      ...(body === undefined ? {} : { body: JSON.stringify(body) }) });
    let result;
    try { result = await response.json(); }
    catch { const error = new Error("account_request_failed"); error.status = response.status; throw error; }
    if (!response.ok || result.ok === false || result.phase === "operation_pending" || result.error_code === "operation_pending") {
      const error = new Error(result.error_code || "account_request_failed");
      error.code = result.error_code || (result.phase === "operation_pending" ? "operation_pending" : undefined);
      error.payload = result; error.status = response.status; throw error;
    }
    return result;
  }
  async function ceremony(path, body, kind) {
    if (!credentials?.[kind]) throw new Error("passkey_unsupported");
    const key = `${path}:${JSON.stringify(body)}`;
    let pending = pendingCeremonies.get(key);
    if (!pending) {
      pending = { started: await request(`${path}/options`, body), response: null };
      pendingCeremonies.set(key, pending);
    }
    const started = pending.started;
    if (!pending.response) {
      try {
        const credential = await credentials[kind](browserPasskeyOptions(started.options));
        if (!credential) throw new Error("passkey_cancelled");
        pending.response = serializePasskey(credential);
      } catch (error) {
        // Explicit cancellation is authority-confirmed before discarding state.
        // A lost cancellation response leaves the original ceremony available.
        try {
          const cancelled = await request(`/ceremonies/${encodeURIComponent(started.ceremony_id)}/cancel`, {});
          if (["cancelled", "expired", "exhausted"].includes(cancelled.phase)) pendingCeremonies.delete(key);
          else { error.pending_operation_id = started.operation_id || started.ceremony_id; error.next_action = "check_original_operation"; }
        }
        catch { /* Preserve pending state; never start a second registration. */ }
        throw error;
      }
    }
    pending.payload ||= { ceremony_id: started.ceremony_id,
      response: pending.response, operation_id: started.operation_id || started.ceremony_id, ...body };
    return finishCeremony(path, key, pending);
  }
  async function finishCeremony(path, key, pending) {
    const started = pending.started;
    try {
      const result = await request(`${path}/complete`, pending.payload);
      pendingCeremonies.delete(key);
      return { ...result, operation_id: result.operation_id || started.operation_id || started.ceremony_id };
    } catch (error) {
      const operationId = started.operation_id || started.ceremony_id;
      if ([400, 401, 403, 409, 410].includes(error.status)) {
        try {
          const status = await request(`/operations/${encodeURIComponent(operationId)}`);
          if (["consumed", "committed", "operation_committed"].includes(status.phase || status.state)) {
            pendingCeremonies.delete(key);
            return { ...status, operation_id: operationId, phase: "operation_committed", account_retained: true, next_action: "fresh_authentication" };
          }
          if (["expired", "cancelled", "exhausted"].includes(status.phase || status.state)) pendingCeremonies.delete(key);
          else if (status.phase === "pending" && [400, 401].includes(error.status)) {
            // Cancel atomically after a failed proof. A concurrent successful
            // completion must return its committed receipt instead of a new key.
            const cancelled = await request(`/ceremonies/${encodeURIComponent(started.ceremony_id)}/cancel`, {});
            if (["consumed", "committed", "operation_committed"].includes(cancelled.phase || cancelled.state)) {
              pendingCeremonies.delete(key);
              return { ...cancelled, operation_id: operationId, phase: "operation_committed", account_retained: true, next_action: "fresh_authentication" };
            }
            if (["cancelled", "ceremony_cancelled"].includes(cancelled.phase || cancelled.state)) pendingCeremonies.delete(key);
          }
        } catch { /* Unknown authority outcome: retain the original response. */ }
      }
      if (pendingCeremonies.has(key)) {
        error.pending_operation_id = operationId;
        error.next_action = "check_original_operation";
      }
      throw error;
    }
  }
  async function authorize(actionType, target, extra = {}) {
    const key = JSON.stringify({ actionType, target, extra });
    let action = pendingActions.get(key);
    if (!action) { action = await request("/actions", { action_type: actionType, target, ...extra }); pendingActions.set(key, action); }
    if (!action.authorized) {
      await ceremony(`/actions/${encodeURIComponent(action.action_id)}/reauth`, {}, "get");
      action.authorized = true;
    }
    return action.action_id;
  }
  function forgetAction(actionId) {
    for (const [key, action] of pendingActions) if (action.action_id === actionId) pendingActions.delete(key);
  }
  async function mutation(path, body) {
    try { return await request(path, body); }
    catch (error) {
      const id = body.operation_id;
      if (!id) throw error;
      try {
        const result = await request(`/operations/${encodeURIComponent(id)}`);
        if (["committed", "consumed", "operation_committed"].includes(result.phase || result.state)) return { ...result, operation_id: id, phase: "operation_committed" };
      } catch { /* Keep the original operation available for reconciliation. */ }
      if (!error.status || error.status >= 500 || error.status === 202 || error.status === 401) {
        error.pending_operation_id = id;
        error.next_action = "check_original_operation";
      }
      throw error;
    }
  }
  async function listAll(path, field) {
    const items = [];
    const seen = new Set();
    let cursor = null;
    let summary;
    for (let page = 0; page < 500; page += 1) {
      const result = await request(`${path}${cursor ? `?cursor=${encodeURIComponent(cursor)}` : ""}`);
      summary ||= result;
      if (!Array.isArray(result[field]) || items.length + result[field].length > 50_000) throw new Error("account_list_limit");
      items.push(...result[field]);
      cursor = result.next_cursor;
      if (!cursor) return { ...summary, [field]: items, next_cursor: null };
      if (typeof cursor !== "string" || seen.has(cursor)) throw new Error("account_list_limit");
      seen.add(cursor);
    }
    throw new Error("account_list_limit");
  }
  return {
    async capabilities() { const result = await request("/capabilities"); csrf = result.csrf_token; return result; },
    session: () => request("/session"),
    operation: (id) => request(`/operations/${encodeURIComponent(id)}`),
    credentials: () => listAll("/credentials", "credentials"),
    sessions: () => listAll("/sessions", "sessions"),
    register: () => ceremony("/passkeys/registration", { intent: "new_account" }, "create"),
    login: () => ceremony("/passkeys/authentication", {}, "get"),
    async addCredential() { const actionId = await authorize("bind");
      const result = await ceremony("/passkeys/registration", { intent: "bind", action_id: actionId }, "create");
      forgetAction(actionId); return result; },
    async revokeCredential(credential) { const actionId = await authorize("revoke_credential", credential.id);
      const result = await mutation(`/credentials/${encodeURIComponent(credential.id)}/revoke`,
        { action_id: actionId, expected_version: credential.version, operation_id: actionId }); forgetAction(actionId); return result; },
    async revokeSession(session) { const actionId = session.current ? undefined : await authorize("revoke_session", session.id);
      const result = await mutation(`/sessions/${encodeURIComponent(session.id)}/revoke`, { action_id: actionId, operation_id: actionId || session.id }); forgetAction(actionId); return result; },
    logout: () => request("/logout", {}),
    joinWorld: (payload) => request("/world/join", payload),
    refreshWorld: (payload) => request("/world/refresh", payload),
    releaseWorld: (payload) => request("/world/release", payload),
    emailOptions: (email) => request("/email/login/options", { email }),
    emailComplete: (otp, code) => request("/email/login/complete", { otp_id: otp.otp_id, otp_code: code, operation_id: otp.operation_id || otp.otp_id }),
    async emailBindingOptions(email) { const action_id = await authorize("bind_email", undefined, { email });
      return { ...(await request("/email/binding/options", { email, action_id })), action_id }; },
    async emailBindingComplete(otp, code) { const result = await mutation("/email/binding/complete", { otp_id: otp.otp_id, otp_code: code, action_id: otp.action_id, operation_id: otp.operation_id || otp.otp_id });
      forgetAction(otp.action_id); return result; },
    async unbindEmail(credential) { const action_id = await authorize("unbind_email", credential.id);
      const result = await mutation("/email/unbind", { credential_id: credential.id, expected_version: credential.version, action_id, operation_id: action_id }); forgetAction(action_id); return result; },
    async upgradeEmailOptions() { const action = await request("/actions", { action_type: "bind" });
      return { ...(await request(`/actions/${encodeURIComponent(action.action_id)}/email/options`, {})), action_id: action.action_id }; },
    async upgradeEmailComplete(otp, code) {
      if (!otp.authorized) {
        await request(`/actions/${encodeURIComponent(otp.action_id)}/email/complete`, { otp_id: otp.otp_id, otp_code: code }); otp.authorized = true;
      }
      return ceremony("/passkeys/registration", { intent: "bind", action_id: otp.action_id }, "create");
    },
    recoveryEmailOptions: (email, recovery_id) => request("/recovery/eligibility/options", { email, recovery_id }),
    async recoveryEligibility(otp, code, recovery_id) {
      const result = await request("/recovery/eligibility", { otp_id: otp.otp_id, otp_code: code, recovery_id });
      recoveryCancelOperation = result.cancel_operation_id || null;
      return result;
    },
    async createRecovery() {
      if (!recoveryOperation) {
        const candidate = await ceremony("/recovery/candidate", {}, "create");
        if (typeof candidate.operation_id !== "string" || candidate.operation_id.length !== 43) throw new Error("invalid_recovery_operation");
        recoveryOperation = candidate.operation_id;
      }
      return request("/recovery", { operation_id: recoveryOperation });
    },
    recoveryStatus: (id) => request(`/recovery/${encodeURIComponent(id)}`),
    async recoveryDecision(recovery, decision, authenticated) {
      const action_id = authenticated ? await authorize(`recovery_${decision}`, `${recovery.recovery_id}:${recovery.candidate_digest}:${recovery.version}`) : undefined;
      const operation_id = authenticated ? action_id : recoveryCancelOperation;
      if (!operation_id) throw new Error("recovery_reverification_required");
      const result = await request(`/recovery/${encodeURIComponent(recovery.recovery_id)}/${decision}`, { action_id, expected_version: recovery.version, operation_id }); forgetAction(action_id); return result;
    },
    recoveryCompletionOptions: (id) => request(`/recovery/${encodeURIComponent(id)}/completion/options`, {}),
    async recoveryComplete(id, started, code) {
      const key = `recovery-complete:${id}:${started.ceremony_id}`;
      let pending = pendingCeremonies.get(key);
      if (!pending) {
        pending = { started, response: null };
        pendingCeremonies.set(key, pending);
      }
      if (!pending.response) {
        const credential = await credentials.get(browserPasskeyOptions(started.options));
        if (!credential) throw new Error("passkey_cancelled");
        pending.response = serializePasskey(credential);
        pending.payload = { ceremony_id: started.ceremony_id, otp_id: started.otp_id, otp_code: code,
          response: pending.response, operation_id: started.operation_id || started.ceremony_id };
      }
      return finishCeremony(`/recovery/${encodeURIComponent(id)}`, key, pending);
    },
  };
}

const messages = {
  NotAllowedError: "操作已取消或超时。账号不会因此丢失，请重新点击。",
  passkey_unsupported: "当前浏览器无法使用通行密钥，请使用支持的浏览器、外部安全密钥或邮箱入口。",
  last_credential_protected: "请先添加并验证另一个通行密钥，再移除此凭证。",
  reauth_required: "请使用已登记的通行密钥确认这次操作。",
  state_changed: "账号状态已经变化，请刷新后重新确认。",
  registration_closed: "当前暂不开放新账号；已有账号仍可登录。",
  recovery_not_ready: "恢复保护期尚未结束，或通知仍待确认。请核对申请状态与最早完成时间。",
  recovery_expired: "恢复申请已经到期，请重新验证已绑定邮箱。",
  email_unavailable: "邮箱服务暂时不可用；通行密钥登录仍可继续使用。",
  recovery_unavailable: "当前环境未开放邮箱恢复，请使用其他已登记的登录方式。",
  identity_store_unavailable: "账号服务暂时不可用，请稍后重试。",
};

/** No authenticator UI is opened until an explicit button click. No bearer is persisted. */
export function mountPasskeyAccount(root, { client = createPasskeyAccountClient(), onAuthenticated = () => {},
  onJoinWorld, onSessionInvalidated = () => {} } = {}) {
  let disposed = false;
  let busy = false;
  let generation = 0;
  let account = null;
  let capabilities = {};
  let credentialList = [];
  let sessionList = [];
  let notice = "";
  let takeover = null;
  let pendingOperation = null;
  const emailRecoveryState = {};
  function element(tag, text) { const node = document.createElement(tag); if (text) node.textContent = text; return node; }
  function button(text, action, disabled = false) {
    const node = element("button", text); node.type = "button"; node.disabled = busy || disabled;
    node.addEventListener("click", () => run(action)); return node;
  }
  async function run(action) {
    if (busy || disposed) return;
    busy = true; notice = ""; render();
    try { await action(); await refresh(); }
    catch (error) {
      if (error.code === "control_elsewhere") {
        takeover = error.payload;
        notice = "另一设备仍控制此世界。你可以保留原设备控制，或明确选择在此设备继续。";
      } else if (error.next_action === "check_original_operation" || error.payload?.next_action === "check_original_operation" || error.code === "operation_pending") {
        pendingOperation = error.pending_operation_id || error.payload?.operation_id;
        notice = "原操作结果尚未确定，请核对原操作进度；不要重新创建账号或更换恢复候选。";
      } else notice = messages[error.code] || messages[error.name] || messages[error.message] || "操作未完成，请重试或选择其他已登记的登录方式。";
    }
    finally { busy = false; render(); }
  }
  function render() {
    if (disposed) return;
    root.replaceChildren(); root.setAttribute("aria-busy", String(busy));
    root.append(element("h2", account ? "账号与安全" : "进入 Oasis7"));
    const status = element("p", notice || (account ? "账号已登录。进入世界后才会申请玩家资格和控制会话。" : "使用设备解锁、PIN 或安全密钥创建账号，无需填写邮箱。"));
    status.setAttribute("role", "status"); root.append(status);
    if (pendingOperation) {
      root.append(element("p", `待核对操作：${pendingOperation}`));
      root.append(button("核对原操作进度", async () => {
        const result = await client.operation(pendingOperation);
        if (result.phase === "operation_committed" || result.state === "committed") {
          pendingOperation = null; notice = "原操作已完成。若当前没有登录会话，请使用刚登记的通行密钥重新登录。";
        }
      }));
    }
    if (!account) {
      root.append(button("免邮箱创建账号", async () => {
        const result = await client.register();
        if (result.next_action === "fresh_authentication" || result.phase === "operation_committed") {
          pendingOperation = null; notice = "账号已创建。请使用刚登记的通行密钥重新登录，再申请进入世界。";
        }
        await refresh(); if (!disposed && account) onAuthenticated(account);
      }, !capabilities.registration_enabled || !capabilities.passkey?.available));
      root.append(button("使用通行密钥登录", async () => {
        const result = await client.login();
        if (result.next_action === "fresh_authentication" || result.phase === "operation_committed") {
          pendingOperation = null; notice = "原登录操作已完成，但当前浏览器没有登录会话。请重新进行通行密钥认证。";
        }
        await refresh(); if (!disposed && account) onAuthenticated(account);
      }, !capabilities.passkey?.available));
      renderAccountEmailRecovery(root, { state: emailRecoveryState, account, credentials: credentialList, capabilities, client, run, render });
      return;
    }
    root.append(element("p", account.display_name || "玩家"));
    if (onJoinWorld) root.append(button("进入世界", () => onJoinWorld(account)));
    if (takeover && onJoinWorld) {
      root.append(button("确认在此设备继续，替换原世界控制", async () => {
        await onJoinWorld(account, { takeover: true, expectedEpoch: takeover.world_session_epoch ?? takeover.expected_epoch }); takeover = null;
      }));
      root.append(button("保持原设备控制", () => { takeover = null; }));
    }
    root.append(element("h3", "登录方式"));
    root.append(element("p", "一个通行密钥可能同步到多个设备；下方账号会话才表示各次登录。"));
    for (const credential of credentialList) {
      const row = element("div"); row.append(element("span", credential.label || "通行密钥"));
      row.append(button("移除", () => credential.type === "email" ? client.unbindEmail(credential) : client.revokeCredential(credential))); root.append(row);
    }
    if (credentialList.some((key) => key.state === "active" && key.type === "passkey")) root.append(button("添加备用通行密钥", () => client.addCredential()));
    renderAccountEmailRecovery(root, { state: emailRecoveryState, account, credentials: credentialList, capabilities, client, run, render });
    root.append(element("h3", "已登录会话"));
    for (const session of sessionList) {
      const row = element("div"); row.append(element("span", session.current ? "当前会话" : (session.device_label || "未知设备")));
      row.append(button(session.current ? "退出当前会话" : "退出该会话", () => client.revokeSession(session))); root.append(row);
    }
    root.append(button("退出登录", async () => { await client.logout(); account = null; onSessionInvalidated(); }));
  }
  async function refresh() {
    const revision = ++generation;
    const nextCapabilities = await client.capabilities();
    let nextAccount = null;
    try { const session = await client.session(); nextAccount = session.account || null; }
    catch (error) { if (!["session_expired", "session_revoked", "authentication_failed"].includes(error.code)) throw error; }
    let nextCredentials = []; let nextSessions = [];
    if (nextAccount) {
      const [keys, sessions] = await Promise.all([client.credentials(), client.sessions()]);
      nextCredentials = keys.credentials || []; nextSessions = sessions.sessions || [];
    }
    if (disposed || revision !== generation) return;
    if (account && !nextAccount) onSessionInvalidated();
    capabilities = nextCapabilities; account = nextAccount; credentialList = nextCredentials; sessionList = nextSessions;
    render();
  }
  render();
  refresh().catch(() => { notice = "账号服务暂时不可用，请稍后刷新。"; render(); });
  return { refresh, dispose() { disposed = true; ++generation; root.replaceChildren(); } };
}
