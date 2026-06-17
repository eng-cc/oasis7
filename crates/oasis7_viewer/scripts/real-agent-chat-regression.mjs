#!/usr/bin/env node
import { createRequire } from "node:module";
import { mkdir, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const require = createRequire(import.meta.url);
const { chromium } = require("playwright");

function usage() {
  console.log(`Usage: node crates/oasis7_viewer/scripts/real-agent-chat-regression.mjs --url <url> [options]

Options:
  --url <url>                 Viewer URL to open
  --out-dir <path>            Artifact directory
  --agent-id <id>             Target agent id (default: agent-0)
  --chat-message <text>       Chat message to send
  --expect-contains <text>    Required Agent reply fragment; repeatable
  --forbid-contains <text>    Forbidden Agent reply fragment; repeatable
  --timeout-ms <ms>           Wait timeout for Agent reply (default: 90000)
  --headed                    Run headed instead of headless
  --help                      Show help`);
}

function parseArgs(argv) {
  const options = {
    url: "",
    outDir: "",
    agentId: "agent-0",
    chatMessage: "你在哪里？身边有什么资源？请直接回答。",
    expectContains: [],
    forbidContains: [],
    timeoutMs: 90000,
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
      case "--agent-id":
        options.agentId = argv[++index] || "";
        break;
      case "--chat-message":
        options.chatMessage = argv[++index] || "";
        break;
      case "--expect-contains":
        options.expectContains.push(argv[++index] || "");
        break;
      case "--forbid-contains":
        options.forbidContains.push(argv[++index] || "");
        break;
      case "--timeout-ms":
        options.timeoutMs = Number(argv[++index] || "0");
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
  if (!options.agentId) throw new Error("--agent-id cannot be empty");
  if (!options.chatMessage) throw new Error("--chat-message cannot be empty");
  if (!Number.isFinite(options.timeoutMs) || options.timeoutMs <= 0) {
    throw new Error("--timeout-ms must be a positive number");
  }
  return options;
}

function appendViewerParams(rawUrl) {
  const url = new URL(rawUrl);
  url.searchParams.set("render_mode", "viewer");
  url.searchParams.set("test_api", "1");
  return url.toString();
}

async function writeJson(path, payload) {
  await writeFile(path, JSON.stringify(payload, null, 2), "utf8");
}

async function getState(page) {
  return await page.evaluate(() => window.__AW_TEST__?.getState?.() ?? null);
}

function cssString(value) {
  return String(value).replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

function findMatchingAgentReply(state, options) {
  const history = Array.isArray(state?.chatHistory) ? state.chatHistory : [];
  return history.find((entry) => {
    if (!entry || entry.source !== "event" || entry.agentId !== options.agentId) return false;
    const message = String(entry.message ?? "");
    if (!options.expectContains.every((needle) => message.includes(String(needle)))) return false;
    if (!options.forbidContains.every((needle) => !message.includes(String(needle)))) return false;
    return true;
  }) || null;
}

async function sendChatThroughUi(page, options) {
  const evidence = {
    selectionMode: "ui-agent-marker",
    focusMode: "ui",
    chatInputMode: "ui",
  };
  const agentMarker = page.locator(`[data-pixel-world-agent-marker="true"][data-agent-id="${cssString(options.agentId)}"]`).first();
  await agentMarker.waitFor({ state: "visible", timeout: 30000 });
  await agentMarker.click();
  await page.waitForFunction((agentId) => window.__AW_TEST__?.getState?.()?.selectedId === agentId, options.agentId, { timeout: 6000 });

  const focusEntry = page.locator(".pixel-world-focus-entry button").first();
  await focusEntry.waitFor({ state: "visible", timeout: 10000 });
  const alreadyFocused = await page.locator('.pixel-world-host[data-world-focus="true"]').count();
  if (!alreadyFocused) {
    await focusEntry.click();
  }
  await page.waitForSelector('.pixel-world-host[data-world-focus="true"]', { timeout: 10000 });

  const commandDrawer = page.locator(".pixel-world-focus-drawer--command").first();
  await commandDrawer.waitFor({ state: "visible", timeout: 10000 });
  const drawerOpen = await commandDrawer.evaluate((node) => node.open);
  if (!drawerOpen) {
    await page.locator(".pixel-world-focus-control--primary").first().click();
  }
  await page.waitForFunction(() => document.querySelector(".pixel-world-focus-drawer--command")?.open === true, null, { timeout: 10000 });

  const chatInput = page.locator("#agent-chat-message");
  await chatInput.waitFor({ state: "visible", timeout: 10000 });
  await chatInput.fill(options.chatMessage);
  await page.waitForFunction((chatMessage) => document.querySelector("#agent-chat-message")?.value === chatMessage, options.chatMessage, { timeout: 5000 });

  const sendButton = page.locator('button[data-chat-send="1"]').first();
  await sendButton.waitFor({ state: "visible", timeout: 10000 });
  await page.waitForFunction(() => {
    const button = document.querySelector('button[data-chat-send="1"]');
    return button && !button.disabled;
  }, null, { timeout: 10000 });
  await sendButton.click({ force: true });
  return evidence;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  options.url = appendViewerParams(options.url);
  options.outDir = resolve(options.outDir);
  await mkdir(options.outDir, { recursive: true });

  const browser = await chromium.launch({ headless: !options.headed });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  const consoleLines = [];
  page.on("console", (message) => {
    consoleLines.push(`[${message.type()}] ${message.text()}`);
  });
  page.on("pageerror", (error) => {
    consoleLines.push(`[pageerror] ${error.stack || error.message}`);
  });

  let finalState = null;
  let reply = null;
  let uiEvidence = null;
  try {
    await page.goto(options.url, { waitUntil: "networkidle", timeout: 60000 });
    await page.waitForFunction(() => typeof window.__AW_TEST__ === "object", null, { timeout: 20000 });
    await page.waitForFunction(() => window.__AW_TEST__?.getState?.()?.connectionStatus === "connected", null, { timeout: 30000 });
    await writeJson(join(options.outDir, "initial_state.json"), await getState(page));

    uiEvidence = await sendChatThroughUi(page, options);
    await page.waitForFunction(() => window.__AW_TEST__?.getState?.()?.lastChatFeedback?.stage === "ack", null, { timeout: options.timeoutMs });
    await page.waitForFunction(
      ({ agentId, chatMessage }) => {
        const history = window.__AW_TEST__?.getState?.()?.chatHistory ?? [];
        return history.some((entry) => entry?.source === "player" && entry.targetAgentId === agentId && entry.message === chatMessage);
      },
      { agentId: options.agentId, chatMessage: options.chatMessage },
      { timeout: 5000 },
    );
    await writeJson(join(options.outDir, "after_chat_ack_state.json"), await getState(page));

    await page.waitForFunction(
      ({ agentId, expectContains, forbidContains }) => {
        const history = window.__AW_TEST__?.getState?.()?.chatHistory ?? [];
        return history.some((entry) => {
          if (!entry || entry.source !== "event" || entry.agentId !== agentId) return false;
          const message = String(entry.message ?? "");
          return expectContains.every((needle) => message.includes(String(needle)))
            && forbidContains.every((needle) => !message.includes(String(needle)));
        });
      },
      {
        agentId: options.agentId,
        expectContains: options.expectContains,
        forbidContains: options.forbidContains,
      },
      { timeout: options.timeoutMs },
    );

    finalState = await getState(page);
    reply = findMatchingAgentReply(finalState, options);
    await writeJson(join(options.outDir, "final_state.json"), finalState);
    await page.screenshot({ path: join(options.outDir, "real-agent-chat.png"), fullPage: true });
  } catch (error) {
    finalState = await getState(page).catch(() => null);
    await writeJson(join(options.outDir, "failure_state.json"), finalState);
    await page.screenshot({ path: join(options.outDir, "failure.png"), fullPage: true }).catch(() => {});
    throw error;
  } finally {
    await writeFile(join(options.outDir, "browser-console.log"), consoleLines.join("\n") + "\n", "utf8");
    await browser.close();
  }

  const summary = {
    caseId: "PWT-001",
    scriptName: "real-agent-chat-regression.mjs",
    ok: true,
    gameUrl: options.url,
    artifactPath: options.outDir,
    agentId: options.agentId,
    chatMessage: options.chatMessage,
    inputMode: "ui",
    selectionMode: uiEvidence?.selectionMode ?? null,
    focusMode: uiEvidence?.focusMode ?? null,
    chatInputMode: uiEvidence?.chatInputMode ?? null,
    requiredContains: options.expectContains,
    forbiddenContains: options.forbidContains,
    noMockMarkersDetected: options.forbidContains.every((needle) => !String(reply?.message ?? "").includes(String(needle))),
    agentReply: reply?.message ?? null,
  };
  await writeJson(join(options.outDir, "real-agent-chat-summary.json"), summary);
  await writeFile(
    join(options.outDir, "real-agent-chat-summary.md"),
    [
      "# Real Agent chat regression",
      "",
      `- caseId: \`${summary.caseId}\``,
      `- scriptName: \`${summary.scriptName}\``,
      `- ok: \`${summary.ok}\``,
      `- agentId: \`${summary.agentId}\``,
      `- inputMode: \`${summary.inputMode}\``,
      `- selectionMode: \`${summary.selectionMode}\``,
      `- chatInputMode: \`${summary.chatInputMode}\``,
      `- noMockMarkersDetected: \`${summary.noMockMarkersDetected}\``,
      `- artifactPath: \`${summary.artifactPath}\``,
      `- agentReply: \`${summary.agentReply}\``,
      `- gameUrl: \`${summary.gameUrl}\``,
    ].join("\n") + "\n",
    "utf8",
  );

  console.log(`ok: real Agent reply observed for ${options.agentId}`);
  console.log(`agent_reply=${summary.agentReply}`);
  console.log(`artifacts=${options.outDir}`);
}

main().catch((error) => {
  console.error(`error: ${error.message}`);
  process.exit(1);
});
