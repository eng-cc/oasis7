import { createServer } from "node:http";
import { mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, extname, join, normalize, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const viewerRoot = resolve(scriptDir, "..");
const repoRoot = resolve(viewerRoot, "../..");
const runId = new Date().toISOString().replace(/[:.]/g, "-");
const outDir = resolve(repoRoot, "output/playwright/pixel-world-hotspot-visual", runId);
const agentBrowserBin = process.env.AGENT_BROWSER_BIN || "agent-browser";
const session = `pixel-world-hotspot-visual-${process.pid}`;
const summary = { status: "running", fixtureName: "hotspot_tooltip", startedAt: new Date().toISOString(), viewports: {} };
mkdirSync(outDir, { recursive: true });

function fail(message, details) { throw new Error(`${message}${details ? `\n${JSON.stringify(details, null, 2)}` : ""}`); }
function assert(condition, message, details) { if (!condition) fail(message, details); }
function writeJson(name, value) { const path = join(outDir, name); writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`); return path; }
function contentType(pathname) { return ({ ".html": "text/html; charset=utf-8", ".js": "text/javascript; charset=utf-8", ".wasm": "application/wasm", ".css": "text/css; charset=utf-8" })[extname(pathname)] || "application/octet-stream"; }
function serveFile(request, response) {
  const requestUrl = new URL(request.url || "/", "http://127.0.0.1");
  const rawPath = decodeURIComponent(requestUrl.pathname === "/" ? "/viewer.html" : requestUrl.pathname);
  const normalized = normalize(rawPath).replace(/^(\.\.(\/|\\|$))+/, "");
  const filePath = normalized.startsWith("/pixel-world-bridge/") ? resolve(viewerRoot, "dist", `.${normalized}`) : resolve(viewerRoot, `.${normalized}`);
  if (!relative(viewerRoot, filePath) || relative(viewerRoot, filePath).startsWith("..")) { response.writeHead(403); response.end("forbidden"); return; }
  try { if (!statSync(filePath).isFile()) throw new Error("not file"); response.writeHead(200, { "Content-Type": contentType(filePath), "Cache-Control": "no-store" }); response.end(readFileSync(filePath)); } catch { response.writeHead(404); response.end("not found"); }
}
function ensureBrowser() { if (spawnSync(agentBrowserBin, ["--version"], { stdio: "ignore" }).status !== 0) fail(`missing required browser automation command: ${agentBrowserBin}`); }
function closeBrowser() { spawnSync(agentBrowserBin, ["--session", session, "close"], { stdio: "ignore", timeout: 10_000 }); }
function runBrowser(args, options = {}) {
  return new Promise((resolveRun, rejectRun) => {
    const child = spawn(agentBrowserBin, ["--session", session, ...args], { stdio: ["pipe", "pipe", "pipe"] }); let stdout = ""; let stderr = "";
    const timer = setTimeout(() => { child.kill("SIGTERM"); rejectRun(new Error(`agent-browser timed out: ${args.join(" ")}`)); }, options.timeout ?? 30_000);
    child.stdout.setEncoding("utf8"); child.stderr.setEncoding("utf8"); child.stdout.on("data", (chunk) => { stdout += chunk; }); child.stderr.on("data", (chunk) => { stderr += chunk; });
    child.on("error", (error) => { clearTimeout(timer); rejectRun(error); }); child.on("close", (code) => { clearTimeout(timer); code === 0 ? resolveRun(stdout) : rejectRun(new Error(`agent-browser failed: ${args.join(" ")}\n${stdout}\n${stderr}`)); });
    child.stdin.end(options.input);
  });
}
async function browserJson(args, options) { const output = await runBrowser(["--json", ...args], options); const parsed = JSON.parse(output); if (!parsed.success) fail(parsed.error || "agent-browser JSON failure"); return parsed.data; }
async function evalJson(source) { const data = await browserJson(["eval", "--stdin"], { input: source, timeout: 60_000 }); return typeof data.result === "string" ? JSON.parse(data.result) : data.result; }
function screenshotBitmap(path) { const bmpPath = path.replace(/\.png$/, ".bmp"); const converted = spawnSync("sips", ["-s", "format", "bmp", path, "--out", bmpPath], { encoding: "utf8" }); if (converted.status !== 0) fail("could not inspect screenshot pixels", { path, stderr: converted.stderr }); const bytes = readFileSync(bmpPath); const offset = bytes.readUInt32LE(10); const width = bytes.readInt32LE(18); const height = Math.abs(bytes.readInt32LE(22)); const bitCount = bytes.readUInt16LE(28); return { bytes, offset, width, height, bitCount, stride: Math.ceil((width * bitCount) / 32) * 4 }; }
function pixelAt(bitmap, x, y) { const index = bitmap.offset + (y * bitmap.stride) + (x * (bitmap.bitCount / 8)); return [bitmap.bytes[index], bitmap.bytes[index + 1], bitmap.bytes[index + 2]]; }
function screenshotStats(path) { const bitmap = screenshotBitmap(path); let brightness = 0; let nonBlack = 0; let count = 0; for (let y = 0; y < bitmap.height; y += 1) { for (let x = 0; x < bitmap.width; x += 1) { const value = pixelAt(bitmap, x, y).reduce((total, channel) => total + channel, 0); brightness += value; if (value > 24) nonBlack += 1; count += 1; } } return { meanBrightness: Number((brightness / Math.max(1, count)).toFixed(2)), nonBlackRatio: Number((nonBlack / Math.max(1, count)).toFixed(4)) }; }
function screenshotDifference(leftPath, rightPath) { const left = screenshotBitmap(leftPath); const right = screenshotBitmap(rightPath); assert(left.width === right.width && left.height === right.height && left.bitCount === right.bitCount, "visible and cleared screenshot formats differ", { left, right }); let changed = 0; let totalDelta = 0; const total = left.width * left.height; for (let y = 0; y < left.height; y += 1) { for (let x = 0; x < left.width; x += 1) { const delta = pixelAt(left, x, y).reduce((sum, channel, index) => sum + Math.abs(channel - pixelAt(right, x, y)[index]), 0); totalDelta += delta; if (delta > 12) changed += 1; } } return { changedPixelRatio: Number((changed / Math.max(1, total)).toFixed(6)), meanChannelDelta: Number((totalDelta / Math.max(1, total * 3)).toFixed(4)) }; }
function pageStateScript() { return String.raw`(() => { const canvas = document.querySelector('#pixel-world-embedded-runtime-canvas'); const state = window.__AW_TEST__?.getState?.() || {}; const gl = canvas?.getContext('webgl2') || canvas?.getContext('webgl'); const debug = gl?.getExtension('WEBGL_debug_renderer_info'); return { rendererReady: document.querySelector('.pixel-world-canvas')?.dataset.rendererReady === 'true', runtimeStatus: state.pixelWorldRuntimeStatus, runtimeSource: state.pixelWorldRuntimeSource, fatal: state.pixelWorldFatal || state.lastError || null, canvas: canvas ? { width: canvas.width, height: canvas.height, rect: (() => { const r = canvas.getBoundingClientRect(); return { width:r.width, height:r.height }; })() } : null, browserEnv: { userAgent: navigator.userAgent, devicePixelRatio: window.devicePixelRatio, webglRenderer: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : null, webglVendor: debug ? gl.getParameter(debug.UNMASKED_VENDOR_WEBGL) : null }, fixture: document.querySelector('.pixel-world-host')?.dataset.visualFixture || null, renderDto: window.__OASIS7_PIXEL_WORLD_RENDER_DTO__?.() || null }; })()`; }
function bringCanvasIntoViewScript() { return String.raw`(async () => { const canvas = document.querySelector('#pixel-world-embedded-runtime-canvas'); const host = document.querySelector('.pixel-world-host'); if (!canvas || !host) throw new Error('canvas host unavailable'); canvas.scrollIntoView({ block: 'center', inline: 'nearest', behavior: 'instant' }); await new Promise((resolve) => requestAnimationFrame(() => resolve())); const rect = canvas.getBoundingClientRect(); return JSON.stringify({ scrollY: window.scrollY, viewport: { width: window.innerWidth, height: window.innerHeight }, canvas: { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom, width: rect.width, height: rect.height } }); })()`; }
function receiptScript(method) { return String.raw`(async () => { const probe = window.__OASIS7_PIXEL_WORLD_HOTSPOT_POINTER_PROBE__; if (!probe) throw new Error('test-only hotspot pointer probe unavailable'); const receipt = await probe.${method}(); const tooltip = document.querySelector('[data-hotspot-tooltip]'); const viewport = { width: window.innerWidth, height: window.innerHeight }; const rect = tooltip?.getBoundingClientRect(); return JSON.stringify({ receipt, viewport, tooltip: tooltip ? { text: tooltip.textContent.trim(), visible: getComputedStyle(tooltip).display !== 'none', rect: { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom, width: rect.width, height: rect.height } } : null }); })()`; }

ensureBrowser();
const server = createServer(serveFile);
try {
  await new Promise((resolveServer) => server.listen(0, "127.0.0.1", resolveServer));
  const address = server.address();
  const url = `http://127.0.0.1:${address.port}/viewer.html?test_api=1&connect=0&locale=en&pixel_world_visual_fixture=hotspot_tooltip`;
  summary.url = url; closeBrowser(); await browserJson(["open", url], { timeout: 45_000 });
  for (const [name, width, height] of [["desktop", 1440, 900], ["narrow", 390, 844]]) {
    await browserJson(["set", "viewport", String(width), String(height)]);
    const state = await evalJson(String.raw`(async()=>{const read=()=>(${pageStateScript()}); const deadline=Date.now()+15000; while(Date.now()<deadline){const s=read(); if(s.rendererReady && s.runtimeStatus==='ready') return JSON.stringify(s); await new Promise(r=>setTimeout(r,100));} throw new Error('renderer not ready');})()`);
    assert(state.fixture === "hotspot_tooltip" && state.rendererReady && state.runtimeStatus === "ready" && !state.fatal, "fixture renderer did not become ready", state);
    assert(state.renderDto?.agents?.some((agent) => agent.id === "agent-0"), "fixture Render DTO omits agent-0", state.renderDto);
    assert(state.renderDto?.receipt_target?.agent_id === "agent-0" && state.renderDto?.receipt_target?.state === "blocked", "fixture Render DTO omits the blocked agent-0 receipt target", state.renderDto);
    assert(state.browserEnv.webglRenderer, "WebGL renderer identity unavailable", state.browserEnv);
    const statePath = writeJson(`${name}-state.json`, state); const envPath = writeJson(`${name}-browser_env.json`, state.browserEnv);
    const stateDigest = createHash("sha256").update(JSON.stringify(state)).digest("hex");
    const canvasViewport = await evalJson(bringCanvasIntoViewScript());
    assert(canvasViewport.canvas.top >= 0 && canvasViewport.canvas.bottom <= canvasViewport.viewport.height, "canvas is not fully visible before pointer dispatch", canvasViewport);
    const visible = await evalJson(receiptScript("hover"));
    assert(visible.receipt.dispatched && visible.receipt.visible && visible.tooltip?.visible && visible.tooltip.text === visible.receipt.tooltipLabel, "canvas pointermove did not produce visible tooltip", visible);
    assert(visible.tooltip.rect.left >= 0 && visible.tooltip.rect.top >= 0 && visible.tooltip.rect.right <= visible.viewport.width && visible.tooltip.rect.bottom <= visible.viewport.height, "visible tooltip is not fully inside the screenshot viewport", visible);
    const visiblePng = join(outDir, `${name}-visible.png`); await runBrowser(["screenshot", visiblePng]); const visibleStats = screenshotStats(visiblePng); assert(visibleStats.nonBlackRatio > 0.01 && visibleStats.meanBrightness > 3, "visible screenshot is black", visibleStats);
    const cleared = await evalJson(receiptScript("leave"));
    assert(cleared.receipt.dispatched && cleared.receipt.cleared && cleared.tooltip === null, "canvas pointerleave did not clear tooltip", cleared);
    const clearedPng = join(outDir, `${name}-cleared.png`); await runBrowser(["screenshot", clearedPng]); const clearedStats = screenshotStats(clearedPng); assert(clearedStats.nonBlackRatio > 0.01 && clearedStats.meanBrightness > 3, "cleared screenshot is black", clearedStats);
    const screenshotDiff = screenshotDifference(visiblePng, clearedPng); assert(screenshotDiff.changedPixelRatio > 0.0001 && screenshotDiff.meanChannelDelta > 0.01, "visible and cleared screenshots lack tooltip pixel difference", screenshotDiff);
    const consoleOutput = await runBrowser(["console"]); const consolePath = join(outDir, `${name}-console.log`); writeFileSync(consolePath, consoleOutput);
    assert(!/\b(?:fatal|CONTEXT_LOST_WEBGL|webgl.*error)\b/i.test(consoleOutput), "browser console reports WebGL fatal", { consolePath, consoleOutput });
    const pointerPath = writeJson(`${name}-pointer-receipt.json`, { visible: visible.receipt, cleared: cleared.receipt });
    summary.viewports[name] = { width, height, statePath, envPath, stateDigest, canvasViewport, pointerPath, consolePath, visiblePng, clearedPng, visibleStats, clearedStats, screenshotDiff };
  }
  summary.status = "passed"; summary.completedAt = new Date().toISOString(); writeJson("summary.json", summary); console.log(`pixel-world hotspot visual smoke passed: ${join(outDir, "summary.json")}`);
} catch (error) { summary.status = "failed"; summary.completedAt = new Date().toISOString(); summary.failure = { message: error instanceof Error ? error.message : String(error) }; writeJson("summary.json", summary); console.error(`pixel-world hotspot visual smoke failed: ${join(outDir, "summary.json")}`); throw error; }
finally { closeBrowser(); await new Promise((resolveClose) => server.close(resolveClose)); }
