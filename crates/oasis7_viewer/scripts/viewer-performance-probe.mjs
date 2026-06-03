import { createServer } from "node:http";
import { mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, extname, join, normalize, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn, spawnSync } from "node:child_process";
import {
  VIEWER_PERF_PROFILES,
  buildViewerPerformanceMarkdown,
  normalizeViewerPerfThresholds,
  summarizeViewerPerformance,
} from "../software_safe_src/performance_metrics.js";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const viewerRoot = resolve(scriptDir, "..");
const repoRoot = resolve(viewerRoot, "../..");
const agentBrowserBin = process.env.AGENT_BROWSER_BIN || "agent-browser";
const session = `viewer-performance-probe-${process.pid}`;

function positiveInteger(value, fallback) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return fallback;
  }
  return Math.max(1, Math.floor(numeric));
}

function normalizeProbeOptions(options) {
  options.agents = positiveInteger(options.agents, 80);
  options.locations = positiveInteger(options.locations, 24);
  options.viewport = [
    positiveInteger(options.viewport?.[0], 1440),
    positiveInteger(options.viewport?.[1], 1000),
  ];
  return options;
}

function parseArgs(argv) {
  const options = {
    profile: "smoke",
    durationMs: null,
    outDir: null,
    agents: 80,
    locations: 24,
    viewport: [1440, 1000],
    thresholds: {},
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = () => {
      index += 1;
      if (index >= argv.length) throw new Error(`missing value for ${arg}`);
      return argv[index];
    };
    switch (arg) {
      case "--profile": options.profile = next(); break;
      case "--duration-ms": options.durationMs = Number(next()); break;
      case "--out-dir": options.outDir = next(); break;
      case "--agents": options.agents = Number(next()); break;
      case "--locations": options.locations = Number(next()); break;
      case "--viewport": {
        const [width, height] = next().split(/[x,]/).map((part) => Number(part.trim()));
        options.viewport = [width, height];
        break;
      }
      case "--min-frame-samples": options.thresholds.minFrameSamples = Number(next()); break;
      case "--min-fps": options.thresholds.minFps = Number(next()); break;
      case "--max-frame-p95-ms": options.thresholds.maxFrameP95Ms = Number(next()); break;
      case "--max-frame-p99-ms": options.thresholds.maxFrameP99Ms = Number(next()); break;
      case "--max-long-task-count": options.thresholds.maxLongTaskCount = Number(next()); break;
      case "--max-long-task-total-ms": options.thresholds.maxLongTaskTotalMs = Number(next()); break;
      case "--max-dom-content-loaded-ms": options.thresholds.maxDomContentLoadedMs = Number(next()); break;
      case "--max-load-event-ms": options.thresholds.maxLoadEventMs = Number(next()); break;
      case "--max-interaction-p95-ms": options.thresholds.maxInteractionP95Ms = Number(next()); break;
      case "-h":
      case "--help":
        options.help = true;
        break;
      default:
        throw new Error(`unknown option: ${arg}`);
    }
  }
  return normalizeProbeOptions(options);
}

function usage() {
  return `Usage: node ./scripts/viewer-performance-probe.mjs [options]

Measures the current software-safe viewer in a real browser with agent-browser.

Options:
  --profile <smoke|release>       Threshold/duration profile (default: smoke)
  --duration-ms <n>               Override sampling duration
  --out-dir <path>                Artifact directory
  --agents <n>                    Synthetic agents in injected snapshot (default: 80)
  --locations <n>                 Synthetic locations in injected snapshot (default: 24)
  --viewport <width>x<height>     Browser viewport (default: 1440x1000)
  --min-frame-samples <n>         Gate: minimum frame samples
  --min-fps <n>                   Gate: average FPS
  --max-frame-p95-ms <n>          Gate: p95 frame interval
  --max-frame-p99-ms <n>          Gate: p99 frame interval
  --max-long-task-count <n>       Gate: browser long task count
  --max-long-task-total-ms <n>    Gate: browser long task total time
  --max-dom-content-loaded-ms <n> Gate: DOMContentLoaded
  --max-load-event-ms <n>         Gate: load event
  --max-interaction-p95-ms <n>    Gate: interaction p95
`;
}

function contentType(pathname) {
  switch (extname(pathname)) {
    case ".html": return "text/html; charset=utf-8";
    case ".js":
    case ".mjs": return "text/javascript; charset=utf-8";
    case ".wasm": return "application/wasm";
    case ".ico": return "image/x-icon";
    case ".png": return "image/png";
    default: return "application/octet-stream";
  }
}

function serveFile(request, response) {
  const requestUrl = new URL(request.url || "/", "http://127.0.0.1");
  let rawPath = "";
  try {
    rawPath = decodeURIComponent(requestUrl.pathname === "/" ? "/software_safe.html" : requestUrl.pathname);
  } catch {
    response.writeHead(400);
    response.end("bad request");
    return;
  }
  const normalized = normalize(rawPath).replace(/^(\.\.(\/|\\|$))+/, "");
  const filePath = resolve(viewerRoot, `.${normalized}`);
  const relativePath = relative(viewerRoot, filePath);
  if (!relativePath || relativePath.startsWith("..")) {
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
    response.writeHead(200, { "Content-Type": contentType(filePath), "Cache-Control": "no-store" });
    response.end(readFileSync(filePath));
  } catch {
    response.writeHead(404);
    response.end("not found");
  }
}

function ensureAgentBrowser() {
  const result = spawnSync(agentBrowserBin, ["--version"], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
  if (result.status !== 0) throw new Error(`missing required browser automation command: ${agentBrowserBin}`);
}

function runAgentBrowser(args, options = {}) {
  const timeout = options.timeout ?? 30_000;
  return new Promise((resolveRun, rejectRun) => {
    const child = spawn(agentBrowserBin, ["--session", session, ...args], { stdio: ["pipe", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    const timer = setTimeout(() => {
      child.kill("SIGTERM");
      rejectRun(new Error(`agent-browser timed out: ${args.join(" ")}`));
    }, timeout);
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
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
    child.stdin.end(options.input ?? "");
  });
}

async function runAgentBrowserJson(args, options = {}) {
  const output = await runAgentBrowser(["--json", ...args], options);
  const parsed = JSON.parse(output);
  if (!parsed.success) throw new Error(parsed.error || `agent-browser command failed: ${args.join(" ")}`);
  return parsed.data;
}

async function evalJson(source, options = {}) {
  const data = await runAgentBrowserJson(["eval", "--stdin"], { input: source, ...options });
  return typeof data.result === "string" ? JSON.parse(data.result) : data.result;
}

function closeBrowser() {
  spawnSync(agentBrowserBin, ["--session", session, "close"], { stdio: "ignore", timeout: 10_000 });
}

function probeScript({ durationMs, agents, locations }) {
  return `
    (async () => {
      const startedAt = performance.now();
      const api = window.__AW_TEST__;
      if (!api) throw new Error("__AW_TEST__ is unavailable; open viewer with test_api=1");
      const model = { agents: {}, locations: {}, agent_prompt_profiles: {}, agent_execution_debug_contexts: {} };
      for (let index = 0; index < ${locations}; index += 1) {
        const locationId = "loc-" + index;
        model.locations[locationId] = {
          id: locationId,
          name: "Perf Location " + index,
          pos: { x_cm: (index % 8) * 400000, y_cm: Math.floor(index / 8) * 260000, z_cm: 0 },
          profile: { radius_cm: 25000 + (index % 4) * 5000, radiation_emission_per_tick: index % 3, material: "silicate" },
          resources: { ore: { amount: 20 + index, unit: "t" }, energy: { amount: 60 + (index % 20), unit: "%" } },
        };
      }
      for (let index = 0; index < ${agents}; index += 1) {
        const agentId = "agent-" + index;
        model.agents[agentId] = {
          id: agentId,
          name: "Perf Agent " + index,
          location_id: "loc-" + (index % ${locations}),
          kind: index % 2 === 0 ? "builder" : "surveyor",
          resources: { energy: { amount: 40 + (index % 60), unit: "%" }, ore: { amount: index % 9, unit: "t" } },
        };
      }
      const longTasks = [];
      let observer = null;
      if ("PerformanceObserver" in window) {
        try {
          observer = new PerformanceObserver((list) => {
            for (const entry of list.getEntries()) {
              longTasks.push({ name: entry.name || "longtask", startTime: entry.startTime, duration: entry.duration });
            }
          });
          observer.observe({ type: "longtask", buffered: true });
        } catch {}
      }
      api.injectSnapshot({
        time: 128,
        config: { space: { width_cm: 6000000, depth_cm: 4000000, height_cm: 800000 } },
        model,
        player_gameplay: {
          stage_id: "post_onboarding",
          stage_status: "blocked",
          execution_state: "blocked",
          accepted_intent_id: "gameplay_action:build_factory_smelter_mk1",
          intent_summary: "Queue build_factory_smelter_mk1 for agent-0",
          intent_scope: "gameplay_action",
          intent_target: "agent-0",
          status_reason: "Performance probe fixture exercises dense viewer rendering.",
          last_world_change: null,
          resume_anchor: "Recover the first sustainable line",
          resume_next_step: "Advance after replenishing materials to confirm the line resumes.",
          goal_id: "post_onboarding.recover_capability",
          goal_kind: "RecoverCapability",
          goal_title: "Recover sustainable capability",
          objective: "Stabilize the first production line before expanding.",
          progress_detail: "The primary line is blocked by missing material input.",
          progress_percent: 68,
          blocker_kind: "material_shortage",
          blocker_detail: "iron input exhausted at factory-0",
          causality_kind: "world_constraint",
          causality_detail: "iron input exhausted at factory-0",
          next_step_hint: "Replenish upstream materials, then advance again to confirm the line resumes.",
          branch_hint: null,
          available_actions: [
            { action_id: "build_factory_smelter_mk1", target_agent_id: "agent-0", label: "Build smelter mk1", protocol_action: "gameplay_action.submit", disabled_reason: null },
            { action_id: "request_snapshot", label: "Request snapshot", protocol_action: "world.request_snapshot", disabled_reason: null },
          ],
          recent_feedback: null,
          agent_claim: null,
        },
      }, { returnState: false });
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const readyMs = performance.now() - startedAt;
      const frameIntervals = [];
      const heartbeat = document.createElement("div");
      heartbeat.setAttribute("aria-hidden", "true");
      heartbeat.style.cssText = [
        "position:fixed",
        "left:0",
        "top:0",
        "width:1px",
        "height:1px",
        "opacity:0",
        "pointer-events:none",
        "will-change:transform",
      ].join(";");
      document.body.appendChild(heartbeat);
      let previous = null;
      const sampleStartedAt = performance.now();
      const until = sampleStartedAt + ${durationMs};
      await new Promise((resolve) => {
        const tick = (timestamp) => {
          if (previous != null) frameIntervals.push(timestamp - previous);
          previous = timestamp;
          heartbeat.style.transform = "translateX(" + (frameIntervals.length % 2) + "px)";
          if (performance.now() >= until) {
            resolve();
            return;
          }
          requestAnimationFrame(tick);
        };
        requestAnimationFrame(tick);
      });
      const sampleDurationMs = performance.now() - sampleStartedAt;
      heartbeat.remove();
      const navigation = performance.getEntriesByType("navigation")[0] || {};
      observer?.disconnect?.();
      return JSON.stringify({
        readyMs,
        durationMs: ${durationMs},
        sampleDurationMs,
        frameIntervals,
        longTasks,
        interactionLatencies: [],
        domReadiness: {
          domInteractiveMs: navigation.domInteractive ?? null,
          domContentLoadedMs: navigation.domContentLoadedEventEnd ?? null,
          loadEventMs: navigation.loadEventEnd ?? null,
          responseEndMs: navigation.responseEnd ?? null,
        },
        dom: {
          nodeCount: document.querySelectorAll("*").length,
          panelCount: document.querySelectorAll(".panel").length,
          interactiveElementCount: document.querySelectorAll("button,a,input,textarea,select,[tabindex]").length,
        },
        viewport: { width: window.innerWidth, height: window.innerHeight, devicePixelRatio: window.devicePixelRatio },
        browser: { userAgent: navigator.userAgent, renderMeta: window.__OASIS7_VIEWER_RENDER_META || {} },
        finalState: api.getState ? api.getState() : null,
      });
    })()
  `;
}

const options = parseArgs(process.argv.slice(2));
if (options.help) {
  console.log(usage());
  process.exit(0);
}

const profile = VIEWER_PERF_PROFILES[options.profile];
if (!profile) throw new Error(`unknown profile: ${options.profile}`);
const runId = new Date().toISOString().replace(/[:.]/g, "-");
const outDir = resolve(options.outDir || join(repoRoot, "output/playwright/viewer-performance", runId));
const summaryJsonPath = join(outDir, "summary.json");
const summaryMdPath = join(outDir, "summary.md");
const screenshotPath = join(outDir, "viewer-performance.png");
const durationMs = Number.isFinite(options.durationMs) && options.durationMs > 0 ? options.durationMs : profile.durationMs;
const thresholds = normalizeViewerPerfThresholds({ ...profile.thresholds, ...options.thresholds });

mkdirSync(outDir, { recursive: true });
ensureAgentBrowser();

const server = createServer(serveFile);

try {
  await new Promise((resolveServer) => server.listen(0, "127.0.0.1", resolveServer));
  const address = server.address();
  const url = `http://127.0.0.1:${address.port}/software_safe.html?test_api=1&connect=0&locale=en&pixel_world_renderer=defer&hosted_bootstrap=0&t=${Date.now()}`;

  closeBrowser();
  console.log(`opening viewer performance probe: ${url}`);
  await runAgentBrowserJson(["open", url], { timeout: 45_000 });
  await runAgentBrowserJson(["set", "viewport", String(options.viewport[0]), String(options.viewport[1])]);

  console.log(`sampling frames for ${durationMs}ms with profile=${options.profile}`);
  const rawProbe = await evalJson(probeScript({ durationMs, agents: options.agents, locations: options.locations }), {
    timeout: durationMs + 45_000,
  });
  const summary = summarizeViewerPerformance({
    runId,
    url,
    profile: options.profile,
    scenario: { name: "dense", agents: options.agents, locations: options.locations },
    thresholds,
    ...rawProbe,
  });
  summary.artifacts = { summaryJson: summaryJsonPath, summaryMarkdown: summaryMdPath, screenshot: screenshotPath };

  await runAgentBrowser(["screenshot", screenshotPath], { timeout: 20_000 });
  writeFileSync(summaryJsonPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
  writeFileSync(summaryMdPath, buildViewerPerformanceMarkdown(summary), "utf8");

  console.log(`viewer performance status: ${summary.status}`);
  console.log(`summary json: ${summaryJsonPath}`);
  console.log(`summary md: ${summaryMdPath}`);
  console.log(`screenshot: ${screenshotPath}`);
  if (summary.status !== "pass") {
    const failed = summary.gates
      .filter((gate) => gate.status === "fail")
      .map((gate) => `${gate.id} actual=${gate.actual} ${gate.comparator} threshold=${gate.threshold}`)
      .join("; ");
    throw new Error(`viewer performance gates failed: ${failed}`);
  }
} finally {
  closeBrowser();
  await new Promise((resolveClose) => server.close(resolveClose));
}
