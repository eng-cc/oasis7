import { createServer } from "node:http";
import { mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, extname, join, normalize, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn, spawnSync } from "node:child_process";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const viewerRoot = resolve(scriptDir, "..");
const repoRoot = resolve(viewerRoot, "../..");
const runId = new Date().toISOString().replace(/[:.]/g, "-");
const outDir = resolve(repoRoot, "output/playwright/pixel-world-module-visual", runId);
const session = `pixel-world-module-visual-${process.pid}`;
const browser = process.env.AGENT_BROWSER_BIN || "agent-browser";
const summary = { status: "running", startedAt: new Date().toISOString(), viewports: {} };
mkdirSync(outDir, { recursive: true });

function fail(message, details) { throw new Error(`${message}${details ? `\n${JSON.stringify(details, null, 2)}` : ""}`); }
function assert(condition, message, details) { if (!condition) fail(message, details); }
function writeJson(name, value) { const path = join(outDir, name); writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`); return path; }
function contentType(pathname) { return ({ ".html": "text/html; charset=utf-8", ".js": "text/javascript; charset=utf-8", ".wasm": "application/wasm" })[extname(pathname)] || "application/octet-stream"; }
function serveFile(request, response) {
  const requestUrl = new URL(request.url || "/", "http://127.0.0.1");
  const rawPath = decodeURIComponent(requestUrl.pathname === "/" ? "/viewer.html" : requestUrl.pathname);
  const normalized = normalize(rawPath).replace(/^(\.\.(\/|\\|$))+/, "");
  const filePath = normalized.startsWith("/pixel-world-bridge/") ? resolve(viewerRoot, "dist", `.${normalized}`) : resolve(viewerRoot, `.${normalized}`);
  if (!relative(viewerRoot, filePath) || relative(viewerRoot, filePath).startsWith("..")) { response.writeHead(403); response.end("forbidden"); return; }
  try { if (!statSync(filePath).isFile()) throw new Error("not file"); response.writeHead(200, { "Content-Type": contentType(filePath), "Cache-Control": "no-store" }); response.end(readFileSync(filePath)); } catch { response.writeHead(404); response.end("not found"); }
}
function closeBrowser() { spawnSync(browser, ["--session", session, "close"], { stdio: "ignore", timeout: 10_000 }); }
function runBrowser(args, input) { return new Promise((resolveRun, rejectRun) => { const child = spawn(browser, ["--session", session, ...args], { stdio: ["pipe", "pipe", "pipe"] }); let stdout = ""; let stderr = ""; child.stdout.setEncoding("utf8"); child.stderr.setEncoding("utf8"); child.stdout.on("data", (chunk) => { stdout += chunk; }); child.stderr.on("data", (chunk) => { stderr += chunk; }); child.on("error", rejectRun); child.on("close", (code) => code === 0 ? resolveRun(stdout) : rejectRun(new Error(`${args.join(" ")} failed\n${stdout}\n${stderr}`))); child.stdin.end(input); }); }
async function browserJson(args, input) { const result = JSON.parse(await runBrowser(["--json", ...args], input)); if (!result.success) fail(result.error || "browser JSON failure"); return result.data; }
async function evalJson(script) { const data = await browserJson(["eval", "--stdin"], script); return typeof data.result === "string" ? JSON.parse(data.result) : data.result; }
function pageStateScript() { return String.raw`(() => { const state = window.__AW_TEST__?.getState?.() || {}; const canvas = document.querySelector('#pixel-world-embedded-runtime-canvas'); const dto = window.__OASIS7_PIXEL_WORLD_RENDER_DTO__?.() || null; return JSON.stringify({ runtimeStatus: state.pixelWorldRuntimeStatus, runtimeSource: state.pixelWorldRuntimeSource, fatal: state.pixelWorldFatal || state.lastError || null, canvas: Boolean(canvas), fixture: document.querySelector('.pixel-world-host')?.dataset.visualFixture || null, modules: (dto?.module_visual_entities || []).map(({ id, kind, pos }) => ({ id, kind, pos })), control: Boolean(window.__OASIS7_MODULE_VISUAL_FIXTURE_CONTROL__) }); })()`; }
function canvasDimensionsScript() { return String.raw`(() => { const canvas = document.querySelector('#pixel-world-embedded-runtime-canvas'); if (!canvas) throw new Error('pixel world canvas unavailable'); const rect = canvas.getBoundingClientRect(); return JSON.stringify({ cssWidth: rect.width, cssHeight: rect.height, bitmapWidth: canvas.width, bitmapHeight: canvas.height }); })()`; }
function scrollCanvasIntoViewScript() { return String.raw`(() => { const canvas = document.querySelector('#pixel-world-embedded-runtime-canvas'); if (!canvas) throw new Error('pixel world canvas unavailable'); canvas.scrollIntoView({ block: 'center', inline: 'center' }); return JSON.stringify(true); })()`; }

if (spawnSync(browser, ["--version"], { stdio: "ignore" }).status !== 0) fail(`missing ${browser}`);
const server = createServer(serveFile);
try {
  await new Promise((resolveServer) => server.listen(0, "127.0.0.1", resolveServer));
  const address = server.address();
  const url = `http://127.0.0.1:${address.port}/viewer.html?test_api=1&connect=0&locale=en&pixel_world_visual_fixture=module_visual_entities`;
  summary.url = url;
  closeBrowser();
  await browserJson(["open", url]);
  for (const [name, width, height] of [["desktop", 1440, 900], ["narrow", 390, 844]]) {
    if (name !== "desktop") await browserJson(["open", url]);
    await browserJson(["set", "viewport", String(width), String(height)]);
    const state = await evalJson(String.raw`(async()=>{const deadline=Date.now()+15000; let state; while(Date.now()<deadline){state=JSON.parse(${pageStateScript()}); if(state.runtimeStatus === 'ready' && state.canvas) return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,100));} throw new Error(JSON.stringify(state));})()`);
    assert(state.fixture === "module_visual_entities" && state.control && !state.fatal, "module fixture was not ready", state);
    const expectedInitialModules = [
      { id: "module-absolute", kind: "beacon", pos: { x_cm: 1_850_000, y_cm: 3_600_000, z_cm: 0 } },
      { id: "module-agent", kind: "future_module_kind", pos: { x_cm: 2_900_000, y_cm: 3_450_000, z_cm: 0 } },
      { id: "module-relay", kind: "relay", pos: { x_cm: 1_850_000, y_cm: 3_600_000, z_cm: 0 } },
    ];
    assert(JSON.stringify(state.modules) === JSON.stringify(expectedInitialModules), "module kind and anchor evidence is not present in stable order", { state, expectedInitialModules });
    assert(state.modules[0].pos.x_cm === state.modules[2].pos.x_cm && state.modules[0].pos.y_cm === state.modules[2].pos.y_cm, "beacon and relay must remain co-anchored for the renderer smoke", state.modules);
    const beforePng = join(outDir, `${name}-before.png`); await runBrowser(["screenshot", "--full", beforePng]);
    const beforeCanvasDimensions = await evalJson(canvasDimensionsScript());
    assert(beforeCanvasDimensions.cssWidth > 0 && beforeCanvasDimensions.cssHeight > 0, "canvas has no visible element dimensions", beforeCanvasDimensions);
    const beforeCanvasPng = join(outDir, `${name}-before-canvas.png`); await runBrowser(["screenshot", "#pixel-world-embedded-runtime-canvas", beforeCanvasPng]);
    await evalJson(scrollCanvasIntoViewScript());
    const beforeViewportPng = join(outDir, `${name}-before-viewport.png`); await runBrowser(["screenshot", beforeViewportPng]);
    const cleared = await evalJson(String.raw`(() => { window.__OASIS7_MODULE_VISUAL_FIXTURE_CONTROL__.clear(); return JSON.stringify(true); })()`);
    assert(cleared === true, "module fixture clear control failed");
    const clearedState = await evalJson(String.raw`(async()=>{const deadline=Date.now()+5000; while(Date.now()<deadline){const state=JSON.parse(${pageStateScript()}); if(state.modules.length === 0) return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,50));} throw new Error('module markers remained after clear');})()`);
    const clearedPng = join(outDir, `${name}-cleared.png`); await runBrowser(["screenshot", "--full", clearedPng]);
    const clearedCanvasDimensions = await evalJson(canvasDimensionsScript());
    const clearedCanvasPng = join(outDir, `${name}-cleared-canvas.png`); await runBrowser(["screenshot", "#pixel-world-embedded-runtime-canvas", clearedCanvasPng]);
    await evalJson(scrollCanvasIntoViewScript());
    const clearedViewportPng = join(outDir, `${name}-cleared-viewport.png`); await runBrowser(["screenshot", clearedViewportPng]);
    const updated = await evalJson(String.raw`(() => { window.__OASIS7_MODULE_VISUAL_FIXTURE_CONTROL__.update({"module-update":{"entity_id":"module-update","module_id":"fixture-module","kind":"future_module_kind","anchor":{"type":"absolute","data":{"x_cm":1850000,"y_cm":3600000,"z_cm":0}}}}); return JSON.stringify(true); })()`);
    assert(updated === true, "module fixture update control failed");
    const updatedState = await evalJson(String.raw`(async()=>{const deadline=Date.now()+5000; while(Date.now()<deadline){const state=JSON.parse(${pageStateScript()}); if(JSON.stringify(state.modules) === JSON.stringify([{ id:"module-update", kind:"future_module_kind", pos:{ x_cm:1850000, y_cm:3600000, z_cm:0 } }])) return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,50));} throw new Error('unknown fallback update was not rendered or stale markers remained');})()`);
    const updatedPng = join(outDir, `${name}-updated.png`); await runBrowser(["screenshot", "--full", updatedPng]);
    const updatedCanvasDimensions = await evalJson(canvasDimensionsScript());
    const updatedCanvasPng = join(outDir, `${name}-updated-canvas.png`); await runBrowser(["screenshot", "#pixel-world-embedded-runtime-canvas", updatedCanvasPng]);
    await evalJson(scrollCanvasIntoViewScript());
    const updatedViewportPng = join(outDir, `${name}-updated-viewport.png`); await runBrowser(["screenshot", updatedViewportPng]);
    const consolePath = join(outDir, `${name}-console.log`); writeFileSync(consolePath, await runBrowser(["console"]));
    summary.viewports[name] = { width, height, state, clearedState, updatedState, beforePng, beforeCanvasPng, beforeViewportPng, clearedPng, clearedCanvasPng, clearedViewportPng, updatedPng, updatedCanvasPng, updatedViewportPng, canvasDimensions: { before: beforeCanvasDimensions, cleared: clearedCanvasDimensions, updated: updatedCanvasDimensions }, consolePath };
  }
  summary.status = "passed";
} catch (error) { summary.status = "failed"; summary.failure = { message: error instanceof Error ? error.message : String(error) }; throw error; }
finally { summary.completedAt = new Date().toISOString(); writeJson("summary.json", summary); closeBrowser(); await new Promise((resolveClose) => server.close(resolveClose)); }
