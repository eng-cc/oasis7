import { createServer } from "node:http";
import { mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, extname, join, normalize, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn, spawnSync } from "node:child_process";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const viewerRoot = resolve(scriptDir, "..");
const repoRoot = resolve(viewerRoot, "../..");
const runId = new Date().toISOString().replace(/[:.]/g, "-");
const outDir = resolve(repoRoot, "output/playwright/pixel-world-fragment-visual", runId);
const screenshotPath = join(outDir, "fragment-fallback-visual.png");
const actionReceiptScreenshotPath = join(outDir, "action-receipt-visual.png");
const summaryPath = join(outDir, "summary.json");
const agentBrowserBin = process.env.AGENT_BROWSER_BIN || "agent-browser";
const session = `pixel-world-fragment-visual-${process.pid}`;

mkdirSync(outDir, { recursive: true });

function assert(condition, message, details = undefined) {
  if (condition) {
    return;
  }
  const suffix = details === undefined ? "" : `\n${JSON.stringify(details, null, 2)}`;
  throw new Error(`${message}${suffix}`);
}

function ensureAgentBrowser() {
  const result = spawnSync(agentBrowserBin, ["--version"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status !== 0) {
    throw new Error(
      `missing required browser automation command: ${agentBrowserBin}\n`
        + "Install or expose agent-browser, or set AGENT_BROWSER_BIN to a compatible executable.",
    );
  }
}

function contentType(pathname) {
  switch (extname(pathname)) {
    case ".html":
      return "text/html; charset=utf-8";
    case ".js":
    case ".mjs":
      return "text/javascript; charset=utf-8";
    case ".wasm":
      return "application/wasm";
    case ".ico":
      return "image/x-icon";
    case ".png":
      return "image/png";
    default:
      return "application/octet-stream";
  }
}

function serveFile(request, response) {
  const requestUrl = new URL(request.url || "/", "http://127.0.0.1");
  const rawPath = decodeURIComponent(requestUrl.pathname === "/" ? "/software_safe.html" : requestUrl.pathname);
  const normalized = normalize(rawPath).replace(/^(\.\.(\/|\\|$))+/, "");
  const filePath = resolve(viewerRoot, `.${normalized}`);
  if (!relative(viewerRoot, filePath) || relative(viewerRoot, filePath).startsWith("..")) {
    response.writeHead(403);
    response.end("forbidden");
    return;
  }
  try {
    const stats = statSync(filePath);
    if (!stats.isFile()) {
      response.writeHead(404);
      response.end("not found");
      return;
    }
    response.writeHead(200, {
      "Content-Type": contentType(filePath),
      "Cache-Control": "no-store",
    });
    response.end(readFileSync(filePath));
  } catch {
    response.writeHead(404);
    response.end("not found");
  }
}

function runAgentBrowser(args, options = {}) {
  const timeout = options.timeout ?? 30_000;
  return new Promise((resolveRun, rejectRun) => {
    const child = spawn(agentBrowserBin, ["--session", session, ...args], {
      stdio: ["pipe", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    const timer = setTimeout(() => {
      child.kill("SIGTERM");
      rejectRun(new Error(`agent-browser timed out: ${args.join(" ")}`));
    }, timeout);

    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", (error) => {
      clearTimeout(timer);
      rejectRun(error);
    });
    child.on("close", (code, signal) => {
      clearTimeout(timer);
      if (code === 0) {
        resolveRun(stdout);
        return;
      }
      rejectRun(new Error([
        `agent-browser failed: ${args.join(" ")}`,
        `exit=${code ?? "null"} signal=${signal ?? "null"}`,
        stdout.trim() ? `stdout:\n${stdout.trim()}` : null,
        stderr.trim() ? `stderr:\n${stderr.trim()}` : null,
      ].filter(Boolean).join("\n")));
    });

    if (options.input !== undefined) {
      child.stdin.end(options.input);
    } else {
      child.stdin.end();
    }
  });
}

async function runAgentBrowserJson(args, options = {}) {
  const output = await runAgentBrowser(["--json", ...args], options);
  const parsed = JSON.parse(output);
  if (!parsed.success) {
    throw new Error(parsed.error || `agent-browser command failed: ${args.join(" ")}`);
  }
  return parsed.data;
}

async function evalJson(source) {
  const data = await runAgentBrowserJson(["eval", "--stdin"], { input: source });
  const result = data.result;
  if (typeof result === "string") {
    return JSON.parse(result);
  }
  return result;
}

function closeBrowser() {
  spawnSync(agentBrowserBin, ["--session", session, "close"], {
    stdio: "ignore",
    timeout: 10_000,
  });
}

function visualProbeScript() {
  return String.raw`
    (async () => {
      const snapshot = {
        time: 24,
        config: { space: { width_cm: 3000000, depth_cm: 2000000, height_cm: 500000 } },
        model: {
          agents: {
            "agent-0": {
              id: "agent-0",
              name: "Survey Agent",
              location_id: "loc-0",
              kind: "surveyor",
              resources: { energy: { amount: 72, unit: "%" } },
            },
          },
          locations: {
            "loc-0": {
              id: "loc-0",
              name: "Fragment Field Anchor",
              pos: { x_cm: 1500000, y_cm: 1000000, z_cm: 0 },
              profile: { radius_cm: 30000, radiation_emission_per_tick: 0, material: "silicate" },
              fragment_profile: {
                blocks: {
                  blocks: [
                    {
                      origin_cm: { x_cm: 3000, y_cm: 0, z_cm: 6000 },
                      size_cm: { x_cm: 9000, y_cm: 6000, z_cm: 9000 },
                      density_kg_per_m3: 3200,
                      compounds: { ppm: { silicate_matrix: 820000, water_ice: 180000 } },
                    },
                    {
                      origin_cm: { x_cm: 18500, y_cm: 0, z_cm: 12000 },
                      size_cm: { x_cm: 12000, y_cm: 6000, z_cm: 9000 },
                      density_kg_per_m3: 7800,
                      compounds: { ppm: { iron_nickel_alloy: 910000, sulfide_ore: 90000 } },
                    },
                    {
                      origin_cm: { x_cm: 10000, y_cm: 0, z_cm: 27000 },
                      size_cm: { x_cm: 9000, y_cm: 6000, z_cm: 12000 },
                      density_kg_per_m3: 920,
                      compounds: { ppm: { water_ice: 720000, hydrated_mineral: 280000 } },
                    },
                  ],
                },
              },
              resources: { ore: { amount: 28, unit: "t" } },
            },
          },
          agent_prompt_profiles: {},
          agent_execution_debug_contexts: {},
        },
        player_gameplay: {
          stage_id: "post_onboarding",
          stage_status: "blocked",
          goal_id: "post_onboarding.recover_capability",
          goal_kind: "RecoverCapability",
          goal_title: "Recover sustainable capability",
          objective: "Stabilize the first production line before expanding.",
          progress_detail: "The primary line is blocked by missing material input.",
          progress_percent: 68,
          blocker_kind: "material_shortage",
          blocker_detail: "iron input exhausted at factory-0",
          blocker_supplemental_detail: null,
          next_step_hint: "Replenish upstream materials, then advance again to confirm the line resumes.",
          branch_hint: null,
          available_actions: [],
          recent_feedback: null,
          agent_claim: null,
        },
      };

      const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
      const waitFor = async (predicate, timeoutMs = 12000) => {
        const deadline = Date.now() + timeoutMs;
        let lastError = null;
        while (Date.now() < deadline) {
          try {
            const value = predicate();
            if (value) {
              return value;
            }
          } catch (error) {
            lastError = error;
          }
          await sleep(100);
        }
        throw lastError || new Error("timed out waiting for visual probe condition");
      };
      const state = () => window.__AW_TEST__?.getState?.() || {};
      const badges = () => Array.from(document.querySelectorAll(".badge"))
        .map((element) => element.textContent.trim())
        .filter((text) => /^(locations|fragments|agents|links|hotspots|derived_positions|world_bounds|renderer|runtime|zoom|pan)=/.test(text));
      const textOf = (selector, root = document) => root.querySelector(selector)?.textContent.trim() || null;
      const rectOf = (element) => {
        const rect = element.getBoundingClientRect();
        return {
          x: Math.round(rect.x),
          y: Math.round(rect.y),
          width: Math.round(rect.width),
          height: Math.round(rect.height),
        };
      };
      const receiptOf = () => {
        const receipt = document.querySelector(".pixel-world-action-receipt");
        if (!receipt) {
          return null;
        }
        return {
          present: receipt.dataset.receiptPresent || null,
          state: receipt.dataset.receiptState || null,
          confidence: receipt.dataset.receiptConfidence || null,
          title: textOf(".pixel-world-action-receipt__title", receipt),
          summary: textOf(".pixel-world-action-receipt__summary", receipt),
          detail: textOf(".pixel-world-action-receipt__detail", receipt),
          meta: textOf(".pixel-world-action-receipt__meta", receipt),
          rect: rectOf(receipt),
        };
      };

      await waitFor(() => window.__AW_TEST__?.injectSnapshot);
      window.__PIXEL_WORLD_VISUAL_BASE_SNAPSHOT__ = snapshot;
      window.__AW_TEST__.injectSnapshot(snapshot);
      await waitFor(() => document.querySelector("#pixel-world-embedded-runtime-canvas"));
      await waitFor(() => state().pixelWorldRuntimeStatus === "ready");

      const canvas = document.querySelector("#pixel-world-embedded-runtime-canvas");
      const canvasRect = canvas.getBoundingClientRect();
      const wheel = new WheelEvent("wheel", {
        deltaY: -120,
        clientX: canvasRect.left + (canvasRect.width / 2),
        clientY: canvasRect.top + (canvasRect.height / 2),
        bubbles: true,
        cancelable: true,
      });
      const wheelCanceled = !canvas.dispatchEvent(wheel);
      await waitFor(() => state().pixelWorldCamera?.zoom > 1);

      const ready = {
        runtimeStatus: state().pixelWorldRuntimeStatus,
        runtimeSource: state().pixelWorldRuntimeSource,
        fatal: state().pixelWorldFatal,
        camera: state().pixelWorldCamera,
        wheelCanceled,
        cursor: getComputedStyle(canvas).cursor,
        badges: badges(),
      };

      const fallbackButton = Array.from(document.querySelectorAll("button"))
        .find((button) => /Host Fallback|切回/.test(button.textContent));
      if (!fallbackButton) {
        throw new Error("missing Host Fallback button");
      }
      fallbackButton.click();
      await waitFor(() => document.querySelectorAll(".pixel-world-fragment-terrain").length === 3);

      const stage = document.querySelector(".pixel-world-canvas");
      const fragments = Array.from(stage.querySelectorAll(".pixel-world-fragment-terrain"));
      const location = stage.querySelector(".pixel-world-entity--location");
      const agent = stage.querySelector(".pixel-world-entity--agent");
      const children = Array.from(stage.children);
      const fragmentRects = fragments.map(rectOf);
      const maxFragmentWidth = Math.max(...fragmentRects.map((rect) => rect.width));
      const agentRect = rectOf(agent);
      const locationOpacity = Number.parseFloat(location.style.opacity || getComputedStyle(location).opacity);

      return JSON.stringify({
        ready,
        fallback: {
          runtimeStatus: state().pixelWorldRuntimeStatus,
          fragmentCount: fragments.length,
          fragmentRects,
          maxFragmentWidth,
          agentRect,
          agentPositionSource: agent?.dataset.positionSource || null,
          locationMarkerRole: location?.dataset.markerRole || null,
          locationOpacity,
          fragmentTags: fragments.map((fragment) => fragment.tagName),
          fragmentTitles: fragments.map((fragment) => fragment.getAttribute("title")),
          domOrder: {
            fragmentsBeforeLocation: children.indexOf(fragments[0]) < children.indexOf(location),
            locationBeforeAgent: children.indexOf(location) < children.indexOf(agent),
          },
          actionReceipt: receiptOf(),
          badges: badges(),
        },
      });
    })()
  `;
}

function actionReceiptProbeScript() {
  return String.raw`
    (async () => {
      const baseSnapshot = window.__PIXEL_WORLD_VISUAL_BASE_SNAPSHOT__;
      if (!baseSnapshot) {
        throw new Error("missing base visual snapshot for action receipt probe");
      }
      const snapshot = JSON.parse(JSON.stringify(baseSnapshot));
      const gameplay = snapshot.player_gameplay || {};
      snapshot.player_gameplay = gameplay;
      gameplay.stage_status = "blocked";
      gameplay.execution_state = "blocked";
      gameplay.accepted_intent_id = "gameplay_action:build_factory_smelter_mk1";
      gameplay.intent_summary = "Queue build_factory_smelter_mk1 for agent-0";
      gameplay.intent_scope = "gameplay_action";
      gameplay.intent_target = "agent-0";
      gameplay.causality_kind = "world_constraint";
      gameplay.causality_detail = "iron input exhausted at factory-0";
      gameplay.last_world_change = "Smelter build request reached factory-0; iron shortage blocks construction.";
      gameplay.available_actions = [
        {
          action_id: "build_factory_smelter_mk1",
          target_agent_id: "agent-0",
          label: "Build smelter mk1",
          protocol_action: "gameplay_action.submit",
          disabled_reason: null,
        },
      ];
      gameplay.recent_feedback = {
        action: "build_factory_smelter_mk1",
        stage: "completed_no_progress",
        effect: "Smelter build request reached factory-0; iron shortage blocks construction.",
        reason: "iron input exhausted at factory-0",
        hint: "Replenish upstream materials, then advance again.",
        delta_logical_time: 1,
        delta_event_seq: 2,
      };

      const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
      const waitFor = async (predicate, timeoutMs = 12000) => {
        const deadline = Date.now() + timeoutMs;
        let lastError = null;
        while (Date.now() < deadline) {
          try {
            const value = predicate();
            if (value) {
              return value;
            }
          } catch (error) {
            lastError = error;
          }
          await sleep(100);
        }
        throw lastError || new Error("timed out waiting for action receipt visual probe condition");
      };
      const state = () => window.__AW_TEST__?.getState?.() || {};
      const textOf = (selector, root = document) => root.querySelector(selector)?.textContent.trim() || null;
      const rectOf = (element) => {
        const rect = element.getBoundingClientRect();
        return {
          x: Math.round(rect.x),
          y: Math.round(rect.y),
          width: Math.round(rect.width),
          height: Math.round(rect.height),
        };
      };
      const receiptOf = () => {
        const receipt = document.querySelector(".pixel-world-action-receipt");
        if (!receipt) {
          return null;
        }
        return {
          present: receipt.dataset.receiptPresent || null,
          state: receipt.dataset.receiptState || null,
          confidence: receipt.dataset.receiptConfidence || null,
          title: textOf(".pixel-world-action-receipt__title", receipt),
          summary: textOf(".pixel-world-action-receipt__summary", receipt),
          detail: textOf(".pixel-world-action-receipt__detail", receipt),
          meta: textOf(".pixel-world-action-receipt__meta", receipt),
          rect: rectOf(receipt),
        };
      };

      await waitFor(() => window.__AW_TEST__?.injectSnapshot);
      window.__AW_TEST__.injectSnapshot(snapshot);
      const receipt = await waitFor(() => {
        const current = receiptOf();
        return current?.present === "true" && current?.confidence === "world_delta" ? current : null;
      });
      await waitFor(() => state().pixelWorldRuntimeStatus === "ready");
      const fallbackButton = Array.from(document.querySelectorAll("button"))
        .find((button) => /Host Fallback|切回/.test(button.textContent));
      if (!fallbackButton) {
        throw new Error("missing Host Fallback button for action receipt probe");
      }
      fallbackButton.click();
      await waitFor(() => state().pixelWorldRuntimeStatus === "fallback");
      await waitFor(() => /Renderer Not Attached|Renderer 未接管/.test(document.body.textContent || ""));
      await waitFor(() => document.querySelectorAll(".pixel-world-fragment-terrain").length === 3);
      const fragments = Array.from(document.querySelectorAll(".pixel-world-fragment-terrain"));
      const stage = fragments[0]?.closest(".pixel-world-canvas") || document.querySelector(".pixel-world-canvas");
      const agent = stage?.querySelector(".pixel-world-entity--agent") || null;
      const blockerBadge = Array.from(document.querySelectorAll(".badge"))
        .map((element) => element.textContent.trim())
        .find((text) => text.startsWith("blocker=")) || null;
      await sleep(150);

      return JSON.stringify({
        runtimeStatus: state().pixelWorldRuntimeStatus,
        receipt,
        blockerBadge,
        fragmentCount: fragments.length,
        agentRect: agent ? rectOf(agent) : null,
      });
    })()
  `;
}

ensureAgentBrowser();

const server = createServer(serveFile);

try {
  await new Promise((resolveServer) => server.listen(0, "127.0.0.1", resolveServer));
  const address = server.address();
  const url = `http://127.0.0.1:${address.port}/software_safe.html?test_api=1&connect=0&locale=en&t=${Date.now()}`;

  closeBrowser();
  console.log(`opening pixel-world viewer visual smoke: ${url}`);
  await runAgentBrowserJson(["open", url], { timeout: 45_000 });
  await runAgentBrowserJson(["set", "viewport", "1440", "1000"]);

  console.log("probing wasm bridge and fallback visual hierarchy");
  const summary = await evalJson(visualProbeScript());

  assert(summary.ready.runtimeStatus === "ready", "wasm runtime did not become ready", summary.ready);
  assert(summary.ready.runtimeSource === "wasm_bindgen_runtime", "viewer did not use wasm bindgen runtime", summary.ready);
  assert(summary.ready.fatal === null, "pixel-world runtime reported a fatal error", summary.ready);
  assert(summary.ready.camera?.zoom > 1, "wheel interaction did not produce camera zoom telemetry", summary.ready);
  assert(summary.ready.wheelCanceled === true, "canvas did not capture wheel interaction", summary.ready);
  assert(summary.fallback.runtimeStatus === "fallback", "fallback mode was not entered for visual DOM assertions", summary.fallback);
  assert(summary.fallback.fragmentCount === 3, "expected exactly three fragment background markers", summary.fallback);
  assert(summary.fallback.locationMarkerRole === "logic_anchor", "location marker was not demoted to logic anchor", summary.fallback);
  assert(summary.fallback.locationOpacity < 0.5, "location marker remains too visually dominant", summary.fallback);
  assert(summary.fallback.agentPositionSource === "location_derived", "agent position was not derived from its location", summary.fallback);
  assert(summary.fallback.domOrder.fragmentsBeforeLocation, "fragment terrain is not behind the location layer", summary.fallback);
  assert(summary.fallback.domOrder.locationBeforeAgent, "agent layer is not in front of the location layer", summary.fallback);
  assert(summary.fallback.actionReceipt?.present === "false", "fallback hierarchy fixture should start from an honest no-receipt state", summary.fallback);
  assert(summary.fallback.actionReceipt?.confidence === "none", "no-receipt fixture should not claim action receipt confidence", summary.fallback);
  assert(summary.fallback.maxFragmentWidth > 0, "fragment marker boxes did not render with a measurable size", summary.fallback);
  assert(summary.fallback.maxFragmentWidth < summary.fallback.agentRect.width, "fragment blocks are not visually quieter than the agent marker", summary.fallback);

  await runAgentBrowser(["screenshot", screenshotPath], { timeout: 20_000 });
  console.log("probing action receipt visual state");
  summary.actionReceipt = await evalJson(actionReceiptProbeScript());
  assert(summary.actionReceipt.receipt?.present === "true", "action receipt did not become visible", summary.actionReceipt);
  assert(summary.actionReceipt.receipt?.state === "blocked", "action receipt did not expose the blocked state", summary.actionReceipt);
  assert(summary.actionReceipt.receipt?.confidence === "world_delta", "action receipt did not use world_delta confidence", summary.actionReceipt);
  assert(summary.actionReceipt.receipt?.title === "Action blocked", "action receipt title was not player-readable", summary.actionReceipt);
  assert(summary.actionReceipt.runtimeStatus === "fallback", "action receipt screenshot did not stay on the host fallback surface", summary.actionReceipt);
  assert(
    /iron shortage blocks construction/.test(summary.actionReceipt.receipt?.summary || ""),
    "action receipt summary did not describe the confirmed blocker",
    summary.actionReceipt,
  );
  assert(/agent=agent-0/.test(summary.actionReceipt.receipt?.meta || ""), "action receipt did not name the target agent", summary.actionReceipt);
  assert(summary.actionReceipt.receipt?.rect?.height > 40, "action receipt did not render with a measurable visual footprint", summary.actionReceipt);
  assert(summary.actionReceipt.fragmentCount === 3, "action receipt scenario should preserve fragment background markers", summary.actionReceipt);
  assert(summary.actionReceipt.agentRect?.width > 0, "action receipt scenario lost the readable agent marker", summary.actionReceipt);
  await runAgentBrowser(["screenshot", actionReceiptScreenshotPath], { timeout: 20_000 });
  summary.url = url;
  summary.screenshot = screenshotPath;
  summary.actionReceiptScreenshot = actionReceiptScreenshotPath;
  writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
  console.log(`pixel-world fragment visual smoke passed: ${summaryPath}`);
  console.log(`screenshot: ${screenshotPath}`);
  console.log(`action receipt screenshot: ${actionReceiptScreenshotPath}`);
} finally {
  closeBrowser();
  await new Promise((resolveClose) => server.close(resolveClose));
}
