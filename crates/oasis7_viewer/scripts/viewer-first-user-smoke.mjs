#!/usr/bin/env node
import { createRequire } from "node:module";
import { mkdir, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join, resolve } from "node:path";

function loadPlaywright() {
  const moduleDirs = [
    process.env.OASIS7_PLAYWRIGHT_NODE_MODULES || "",
    "/Users/scc/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules",
  ].filter(Boolean);
  for (const moduleDir of moduleDirs) {
    if (!existsSync(join(moduleDir, "playwright"))) continue;
    return createRequire(join(moduleDir, "oasis7-viewer-first-user-smoke.cjs"))("playwright");
  }
  return createRequire(import.meta.url)("playwright");
}

const { chromium } = loadPlaywright();

function usage() {
  console.log(`Usage: node crates/oasis7_viewer/scripts/viewer-first-user-smoke.mjs --url <url> --out-dir <path> [options]

Options:
  --url <url>             Viewer URL to open.
  --out-dir <path>        Artifact directory.
  --chat-message <text>   Smoke chat message.
  --timeout-ms <ms>       Overall readiness timeout (default: 45000).
  --chrome-path <path>    Use a system Chrome/Chromium executable.
  --headed                Run headed instead of headless.
  --help                  Show help.`);
}

function parseArgs(argv) {
  const options = {
    url: "",
    outDir: "",
    chatMessage: "本地启动 smoke：请确认我可以给 Agent 发消息。",
    timeoutMs: 45000,
    chromePath: process.env.OASIS7_PLAYWRIGHT_CHROME_PATH || "",
    headed: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    switch (arg) {
      case "--url":
        options.url = argv[++index] || "";
        break;
      case "--out-dir":
        options.outDir = argv[++index] || "";
        break;
      case "--chat-message":
        options.chatMessage = argv[++index] || "";
        break;
      case "--timeout-ms":
        options.timeoutMs = Number(argv[++index] || "0");
        break;
      case "--chrome-path":
        options.chromePath = argv[++index] || "";
        break;
      case "--headed":
        options.headed = true;
        break;
      case "--help":
      case "-h":
        usage();
        process.exit(0);
        break;
      default:
        throw new Error(`unknown option: ${arg}`);
    }
  }
  if (!options.url) throw new Error("--url is required");
  if (!options.outDir) throw new Error("--out-dir is required");
  if (!options.chatMessage) throw new Error("--chat-message cannot be empty");
  if (!Number.isFinite(options.timeoutMs) || options.timeoutMs <= 0) {
    throw new Error("--timeout-ms must be a positive number");
  }
  return options;
}

function viewerUrl(rawUrl) {
  const url = new URL(rawUrl);
  url.searchParams.set("test_api", "1");
  url.searchParams.set("render_mode", "viewer");
  return url.toString();
}

async function getState(page) {
  return await page.evaluate(() => window.__AW_TEST__?.getState?.() ?? null);
}

async function installWebSocketRecorder(page) {
  await page.addInitScript(() => {
    const NativeWebSocket = window.WebSocket;
    window.__OASIS7_SMOKE_WS_SENT__ = [];
    window.__OASIS7_SMOKE_WS_RECEIVED__ = [];
    window.WebSocket = class extends NativeWebSocket {
      constructor(...args) {
        super(...args);
        this.addEventListener("message", (event) => {
          try {
            window.__OASIS7_SMOKE_WS_RECEIVED__.push(JSON.parse(event.data));
          } catch (_) {
          }
        });
      }

      send(data) {
        try {
          window.__OASIS7_SMOKE_WS_SENT__.push(JSON.parse(data));
        } catch (_) {
        }
        return super.send(data);
      }
    };
    window.WebSocket.CONNECTING = NativeWebSocket.CONNECTING;
    window.WebSocket.OPEN = NativeWebSocket.OPEN;
    window.WebSocket.CLOSING = NativeWebSocket.CLOSING;
    window.WebSocket.CLOSED = NativeWebSocket.CLOSED;
  });
}

async function getRecordedWebSocketTraffic(page) {
  return await page.evaluate(() => ({
    sent: window.__OASIS7_SMOKE_WS_SENT__ || [],
    received: window.__OASIS7_SMOKE_WS_RECEIVED__ || [],
  }));
}

async function writeJson(path, payload) {
  await writeFile(path, JSON.stringify(payload, null, 2), "utf8");
}

function summarizeState(state) {
  const agents = state?.snapshot?.model?.agents ?? {};
  const locations = state?.snapshot?.model?.locations ?? {};
  const runtimeState = state?.snapshot?.model?.state ?? state?.snapshot?.state ?? {};
  const starterOcClaims = runtimeState.starter_oc_claims
    ?? runtimeState.starterOcClaims
    ?? state?.snapshot?.model?.starter_oc_claims
    ?? state?.snapshot?.model?.starterOcClaims
    ?? state?.snapshot?.starter_oc_claims
    ?? state?.snapshot?.starterOcClaims
    ?? {};
  const auth = state?.auth ?? {
    available: state?.authReady ?? null,
    playerId: state?.authPlayerId ?? null,
    registrationStatus: state?.authRegistrationStatus ?? null,
    runtimeStatus: state?.authRuntimeStatus ?? null,
    boundAgentId: state?.authBoundAgentId ?? null,
    recoveryErrorCode: state?.authRecoveryErrorCode ?? null,
    recoveryErrorMessage: state?.authRecoveryErrorMessage ?? null,
  };
  return {
    connectionStatus: state?.connectionStatus ?? null,
    lastError: state?.lastError ?? null,
    auth,
    agentCount: Object.keys(agents).length || Number(state?.gameplaySummary?.agentCount ?? 0),
    locationCount: Object.keys(locations).length || Number(state?.gameplaySummary?.locationCount ?? 0),
    blockerKind: state?.snapshot?.player_gameplay?.blocker_kind ?? null,
    blockerDetail: state?.snapshot?.player_gameplay?.blocker_detail ?? null,
    starterOcClaimed: Object.keys(starterOcClaims).length > 0,
    lastChatFeedback: state?.lastChatFeedback ?? null,
  };
}

function chromeExecutablePath(requestedPath) {
  if (requestedPath && existsSync(requestedPath)) return requestedPath;
  const macChrome = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
  if (existsSync(macChrome)) return macChrome;
  return undefined;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  options.url = viewerUrl(options.url);
  options.outDir = resolve(options.outDir);
  await mkdir(options.outDir, { recursive: true });

  const browser = await chromium.launch({
    headless: !options.headed,
    executablePath: chromeExecutablePath(options.chromePath),
  });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  await installWebSocketRecorder(page);
  const consoleLines = [];
  page.on("console", (message) => {
    consoleLines.push(`[${message.type()}] ${message.text()}`);
  });
  page.on("pageerror", (error) => {
    consoleLines.push(`[pageerror] ${error.stack || error.message}`);
  });

  let finalState = null;
  let sendResult = null;
  let websocketTraffic = { sent: [], received: [] };
  try {
    await page.goto(options.url, { waitUntil: "domcontentloaded", timeout: options.timeoutMs });
    await page.waitForFunction(() => typeof window.__AW_TEST__ === "object", null, { timeout: options.timeoutMs });
    await page.waitForFunction(() => window.__AW_TEST__?.getState?.()?.connectionStatus === "connected", null, { timeout: options.timeoutMs });

    await page.waitForFunction(() => {
      const bodyText = document.body?.innerText ?? "";
      return bodyText.includes("认领第一个 Agent")
        || bodyText.includes("Claim First Agent")
        || Object.keys(window.__AW_TEST__?.getState?.()?.snapshot?.model?.agents ?? {}).length >= 1;
    }, null, { timeout: options.timeoutMs });
    const claimFirstAgentButton = page.getByTestId("viewer-playthrough-action-claim-first-agent").first();
    if (await claimFirstAgentButton.count()) {
      const disabled = await claimFirstAgentButton.evaluate((button) => button.disabled).catch(() => true);
      if (!disabled) {
        await claimFirstAgentButton.click();
      }
    }

    await page.waitForFunction(() => {
      const state = window.__AW_TEST__?.getState?.();
      const agents = state?.snapshot?.model?.agents ?? {};
      const agentCount = Object.keys(agents).length || Number(state?.gameplaySummary?.agentCount ?? 0);
      const registrationStatus = state?.auth?.registrationStatus ?? state?.authRegistrationStatus;
      const boundAgentId = state?.auth?.boundAgentId ?? state?.authBoundAgentId;
      const bodyText = document.body?.innerText ?? "";
      const emptyEntityVisible = bodyText.includes("当前快照里没有行动体")
        || bodyText.includes("runtime_snapshot_empty_entities");
      return (agentCount >= 1 || !emptyEntityVisible)
        && registrationStatus === "registered"
        && Boolean(boundAgentId);
    }, null, { timeout: options.timeoutMs });

    await writeJson(join(options.outDir, "ready_state.json"), await getState(page));

    await page.waitForFunction(() => !/pixel_world_render_state_unavailable/.test(document.body?.innerText ?? ""), null, {
      timeout: options.timeoutMs,
    });

    const claimStarterOcButton = page.getByTestId("viewer-playthrough-action-claim-starter-oc").first();
    if (await claimStarterOcButton.count()) {
      await page.waitForFunction(() => {
        const button = document.querySelector('[data-testid="viewer-playthrough-action-claim-starter-oc"]');
        return button && !button.disabled;
      }, null, { timeout: options.timeoutMs });
      await claimStarterOcButton.click();
      await page.waitForFunction(() => {
        const state = window.__AW_TEST__?.getState?.();
        const boundAgentId = state?.auth?.boundAgentId ?? state?.authBoundAgentId;
        const snapshot = state?.snapshot ?? {};
        const model = snapshot.model ?? {};
        const runtimeState = model.state ?? snapshot.state ?? {};
        const claims = runtimeState.starter_oc_claims
          || runtimeState.starterOcClaims
          || model.starter_oc_claims
          || model.starterOcClaims
          || snapshot.starter_oc_claims
          || snapshot.starterOcClaims
          || {};
        const balances = runtimeState.main_token_balances
          || runtimeState.mainTokenBalances
          || model.main_token_balances
          || model.mainTokenBalances
          || snapshot.main_token_balances
          || snapshot.mainTokenBalances
          || {};
        const balance = balances[boundAgentId] || null;
        const liquidBalance = Number(
          balance?.liquid_balance
            ?? balance?.liquidBalance
            ?? balance?.liquid
            ?? balance?.balance
            ?? 0,
        );
        return Boolean(boundAgentId && claims[boundAgentId])
          || (Number.isFinite(liquidBalance) && liquidBalance > 0)
          || Boolean(document.querySelector("#agent-chat-message") && !document.querySelector("#agent-chat-message").disabled);
      }, null, { timeout: options.timeoutMs });
    }

    await page.waitForSelector("#agent-chat-message", { state: "visible", timeout: options.timeoutMs });
    await page.waitForSelector('[data-chat-send="1"]', { state: "visible", timeout: options.timeoutMs });
    await page.waitForFunction(() => {
      const input = document.querySelector("#agent-chat-message");
      const button = document.querySelector('[data-chat-send="1"]');
      return input && !input.disabled && button && !button.disabled;
    }, null, { timeout: options.timeoutMs });

    sendResult = await page.evaluate((message) => {
      const state = window.__AW_TEST__?.getState?.();
      const agentId = state?.auth?.boundAgentId ?? state?.authBoundAgentId ?? state?.selectedId;
      return window.__AW_TEST__?.sendAgentChat?.(agentId, message);
    }, options.chatMessage);
    if (!sendResult?.ok && !sendResult?.feedback?.accepted) {
      throw new Error(`sendAgentChat rejected before send: ${sendResult?.reason || JSON.stringify(sendResult)}`);
    }
    await writeJson(join(options.outDir, "send_result.json"), sendResult);
    await page.waitForFunction(() => (
      (window.__OASIS7_SMOKE_WS_SENT__ || []).some((message) => message?.type === "agent_chat")
    ), null, { timeout: options.timeoutMs });
    await page.waitForFunction(() => (
      (window.__OASIS7_SMOKE_WS_RECEIVED__ || []).some((message) => (
        message?.type === "agent_chat_ack" || message?.type === "agent_chat_error"
      ))
    ), null, { timeout: options.timeoutMs });

    finalState = await getState(page);
    websocketTraffic = await getRecordedWebSocketTraffic(page);
    await writeJson(join(options.outDir, "websocket_traffic.json"), websocketTraffic);
    await writeJson(join(options.outDir, "final_state.json"), finalState);
    await page.screenshot({ path: join(options.outDir, "viewer-first-user-smoke.png"), fullPage: true });
  } catch (error) {
    finalState = await getState(page).catch(() => null);
    await writeJson(join(options.outDir, "failure_state.json"), {
      error: error.message,
      summary: summarizeState(finalState),
      state: finalState,
      websocketTraffic: await getRecordedWebSocketTraffic(page).catch(() => null),
    });
    await page.screenshot({ path: join(options.outDir, "failure.png"), fullPage: true }).catch(() => {});
    throw error;
  } finally {
    await writeFile(join(options.outDir, "browser-console.log"), consoleLines.join("\n") + "\n", "utf8");
    await browser.close();
  }

  const summary = {
    ok: true,
    url: options.url,
    artifactPath: options.outDir,
    chatMessage: options.chatMessage,
    agentChatSent: websocketTraffic.sent.some((message) => message?.type === "agent_chat"),
    agentChatCompleted: websocketTraffic.received.some((message) => (
      message?.type === "agent_chat_ack" || message?.type === "agent_chat_error"
    )),
    sendResult,
    state: summarizeState(finalState),
  };
  await writeJson(join(options.outDir, "viewer-first-user-smoke-summary.json"), summary);
  console.log(`ok: viewer first-user smoke passed; agent=${summary.state.auth?.boundAgentId ?? "<none>"}`);
  console.log(`artifacts=${options.outDir}`);
}

main().catch((error) => {
  console.error(`error: ${error.message}`);
  process.exit(1);
});
