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
function screenshotBitmap(path) {
  const bmpPath = path.replace(/\.png$/, ".bmp");
  const converted = spawnSync("sips", ["-s", "format", "bmp", path, "--out", bmpPath], { encoding: "utf8" });
  if (converted.status !== 0) fail("could not inspect metadata screenshot pixels", { path, stderr: converted.stderr });
  const bytes = readFileSync(bmpPath);
  const offset = bytes.readUInt32LE(10);
  const width = bytes.readInt32LE(18);
  const height = Math.abs(bytes.readInt32LE(22));
  const bitCount = bytes.readUInt16LE(28);
  return { bytes, offset, width, height, bitCount, stride: Math.ceil((width * bitCount) / 32) * 4 };
}
function screenshotDifference(leftPath, rightPath) {
  const left = screenshotBitmap(leftPath);
  const right = screenshotBitmap(rightPath);
  assert(left.width === right.width && left.height === right.height && left.bitCount === right.bitCount, "metadata screenshot formats differ", { left, right });
  let changedPixelCount = 0;
  let totalDelta = 0;
  const total = left.width * left.height;
  for (let y = 0; y < left.height; y += 1) {
    for (let x = 0; x < left.width; x += 1) {
      const leftOffset = left.offset + (y * left.stride) + (x * (left.bitCount / 8));
      const rightOffset = right.offset + (y * right.stride) + (x * (right.bitCount / 8));
      let pixelDelta = 0;
      for (let channel = 0; channel < 3; channel += 1) {
        pixelDelta += Math.abs(left.bytes[leftOffset + channel] - right.bytes[rightOffset + channel]);
      }
      totalDelta += pixelDelta;
      if (pixelDelta > 18) changedPixelCount += 1;
    }
  }
  return {
    changedPixelCount,
    changedPixelRatio: Number((changedPixelCount / Math.max(1, total)).toFixed(6)),
    meanChannelDelta: Number((totalDelta / Math.max(1, total * 3)).toFixed(4)),
  };
}
function contentType(pathname) { return ({ ".html": "text/html; charset=utf-8", ".js": "text/javascript; charset=utf-8", ".wasm": "application/wasm", ".css": "text/css; charset=utf-8" })[extname(pathname)] || "application/octet-stream"; }
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
async function clickVisible(selector) {
  await browserJson(["scrollintoview", selector]);
  const target = await evalJson(`(() => { const node = document.querySelector(${JSON.stringify(selector)}); if (!node) throw new Error("missing visible target"); const rect = node.getBoundingClientRect(); const x = rect.left + rect.width / 2; const y = rect.top + rect.height / 2; const hit = document.elementFromPoint(x, y); const style = getComputedStyle(node); return JSON.stringify({ visible: rect.width > 0 && rect.height > 0 && rect.top >= 0 && rect.bottom <= innerHeight && rect.left >= 0 && rect.right <= innerWidth && style.display !== "none" && style.visibility !== "hidden", targetTop: hit === node || node.contains(hit), rect: { x: rect.x, y: rect.y, right: rect.right, bottom: rect.bottom, width: rect.width, height: rect.height }, hash: location.hash, active: document.activeElement?.id || null, hit: hit?.outerHTML?.slice(0, 200) || null }); })()`);
  assert(target.visible && target.targetTop, `target is not the visible top hit: ${selector}`, target);
  const x = Math.round((target.rect.x + target.rect.right) / 2);
  const y = Math.round((target.rect.y + target.rect.bottom) / 2);
  await browserJson(["mouse", "move", String(x), String(y)]);
  await browserJson(["mouse", "down"]);
  await browserJson(["mouse", "up"]);
  return { selector, target };
}
function pageStateScript() { return String.raw`(() => { const state = window.__AW_TEST__?.getState?.() || {}; const canvas = document.querySelector('#pixel-world-embedded-runtime-canvas'); const gl = canvas?.getContext('webgl2'); const debug = gl?.getExtension('WEBGL_debug_renderer_info'); const dto = window.__OASIS7_PIXEL_WORLD_RENDER_DTO__?.() || null; const moduleMarkers = [...document.querySelectorAll('[data-pixel-world-module-marker="true"]')].map((marker) => ({ id: marker.dataset.moduleId || null, kind: marker.dataset.moduleKind || null, selected: marker.dataset.selected === 'true', rendererTarget: marker.dataset.rendererTarget === 'true', ariaLabel: marker.getAttribute('aria-label') })); const eventLocators = [...document.querySelectorAll('[data-world-feed-module-locate]')].map((button) => ({ id: button.dataset.worldFeedModuleLocate || null, label: button.textContent.trim(), ariaLabel: button.getAttribute('aria-label'), tagName: button.tagName, tabIndex: button.tabIndex })); const detailsPanel = document.querySelector('#viewer-details-panel'); const detailsStyle = detailsPanel ? getComputedStyle(detailsPanel) : null; const detailsRect = detailsPanel?.getBoundingClientRect(); const detailsVisible = Boolean(detailsPanel && !detailsPanel.hidden && detailsPanel.getAttribute('aria-hidden') !== 'true' && detailsPanel.getAttribute('inert') === null && detailsStyle?.display !== 'none' && detailsStyle?.visibility !== 'hidden' && Number(detailsStyle?.opacity || '1') > 0 && detailsRect?.width > 0 && detailsRect?.height > 0 && detailsRect.right > 0 && detailsRect.bottom > 0 && detailsRect.left < innerWidth && detailsRect.top < innerHeight); const detailsActive = Boolean(detailsPanel && detailsPanel.getAttribute('data-viewer-route-panel') === 'command' && detailsPanel.getAttribute('data-viewer-surface') === 'command' && (window.location.hash === '#viewer-details-panel' || document.activeElement === detailsPanel)); const moduleDetails = detailsPanel?.querySelector('[data-viewer-module-details="true"]'); const moduleDetailsStyle = moduleDetails ? getComputedStyle(moduleDetails) : null; const moduleDetailsRect = moduleDetails?.getBoundingClientRect(); const moduleDetailsVisible = Boolean(moduleDetails && !moduleDetails.hidden && moduleDetailsStyle?.display !== 'none' && moduleDetailsStyle?.visibility !== 'hidden' && Number(moduleDetailsStyle?.opacity || '1') > 0 && moduleDetailsRect?.width > 0 && moduleDetailsRect?.height > 0 && moduleDetailsRect.right > 0 && moduleDetailsRect.bottom > 0 && moduleDetailsRect.left < innerWidth && moduleDetailsRect.top < innerHeight); const detailsText = detailsPanel?.textContent?.trim() || ''; const rect = canvas?.getBoundingClientRect(); return JSON.stringify({ renderMode: state.renderMode, runtimeStatus: state.pixelWorldRuntimeStatus, runtimeSource: state.pixelWorldRuntimeSource, runtimeModuleUrl: state.pixelWorldRuntimeModuleUrl || null, camera: state.pixelWorldCamera || null, selectedKind: state.selectedKind || null, selectedId: state.selectedId || null, fatal: state.pixelWorldFatal || state.lastError || null, canvas: Boolean(canvas), canvasRect: rect ? { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom, width: rect.width, height: rect.height } : null, webgl2: Boolean(gl), browserEnv: { userAgent: navigator.userAgent, devicePixelRatio: window.devicePixelRatio, webglRenderer: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : null, webglVendor: debug ? gl.getParameter(debug.UNMASKED_VENDOR_WEBGL) : null }, fixture: document.querySelector('.pixel-world-host')?.dataset.visualFixture || null, renderLocale: dto?.locale || null, modules: (dto?.module_visual_entities || []).map(({ id, kind, pos }) => ({ id, kind, pos })), moduleMetadata: (dto?.module_visual_entities || []).map(({ id, module_id, kind, label, anchor, pos }) => ({ id, module_id, kind, label, anchor, pos })), links: dto?.links || [], visualHotspots: dto?.visual_hotspots || [], moduleMarkers, eventLocators, detailsText, detailsVisible, detailsActive, detailsRect: detailsRect ? { left: detailsRect.left, top: detailsRect.top, right: detailsRect.right, bottom: detailsRect.bottom, width: detailsRect.width, height: detailsRect.height } : null, moduleDetailsVisible, moduleDetailsRect: moduleDetailsRect ? { left: moduleDetailsRect.left, top: moduleDetailsRect.top, right: moduleDetailsRect.right, bottom: moduleDetailsRect.bottom, width: moduleDetailsRect.width, height: moduleDetailsRect.height } : null, control: Boolean(window.__OASIS7_MODULE_VISUAL_FIXTURE_CONTROL__) }); })()`; }
function canvasDimensionsScript() { return String.raw`(() => { const canvas = document.querySelector('#pixel-world-embedded-runtime-canvas'); if (!canvas) throw new Error('pixel world canvas unavailable'); const rect = canvas.getBoundingClientRect(); return JSON.stringify({ cssWidth: rect.width, cssHeight: rect.height, bitmapWidth: canvas.width, bitmapHeight: canvas.height }); })()`; }
function scrollCanvasIntoViewScript() { return String.raw`(() => { const canvas = document.querySelector('#pixel-world-embedded-runtime-canvas'); if (!canvas) throw new Error('pixel world canvas unavailable'); canvas.scrollIntoView({ block: 'center', inline: 'center' }); return JSON.stringify(true); })()`; }
function zoomCanvasScript(deltaY, repeats) { return String.raw`(() => { const canvas = document.querySelector('#pixel-world-embedded-runtime-canvas'); if (!canvas) throw new Error('pixel world canvas unavailable'); const rect = canvas.getBoundingClientRect(); const clientX = rect.left + (rect.width / 2); const clientY = rect.top + (rect.height / 2); for (let index = 0; index < ${repeats}; index += 1) canvas.dispatchEvent(new WheelEvent('wheel', { deltaY: ${deltaY}, clientX, clientY, bubbles: true, cancelable: true })); return JSON.stringify(true); })()`; }
function panCanvasScript() { return String.raw`(() => { const canvas = document.querySelector('#pixel-world-embedded-runtime-canvas'); if (!canvas) throw new Error('pixel world canvas unavailable'); canvas.setPointerCapture = () => {}; canvas.releasePointerCapture = () => {}; const rect = canvas.getBoundingClientRect(); const pointerId = 77; const startX = rect.left + (rect.width / 2); const startY = rect.top + (rect.height / 2); const endX = startX + Math.max(rect.width * 0.4, 230); canvas.dispatchEvent(new PointerEvent('pointerdown', { pointerId, clientX: startX, clientY: startY, button: 0, buttons: 1, bubbles: true })); canvas.dispatchEvent(new PointerEvent('pointermove', { pointerId, clientX: endX, clientY: startY, buttons: 1, bubbles: true })); canvas.dispatchEvent(new PointerEvent('pointerup', { pointerId, clientX: endX, clientY: startY, button: 0, bubbles: true })); return JSON.stringify(true); })()`; }
function clickModuleScript(id) { return String.raw`(() => { const expectedId = ${JSON.stringify(id)}; const marker = [...document.querySelectorAll('[data-pixel-world-module-marker="true"]')].find((candidate) => candidate.dataset.moduleId === expectedId && candidate.dataset.rendererTarget === 'true'); if (!marker) throw new Error(JSON.stringify({ message: 'actual-WASM module target unavailable', expectedId, markers: [...document.querySelectorAll('[data-pixel-world-module-marker="true"]')].map((candidate) => ({ id: candidate.dataset.moduleId, rendererTarget: candidate.dataset.rendererTarget, ariaLabel: candidate.getAttribute('aria-label') })) })); const keyboardReachable = marker instanceof HTMLButtonElement && marker.tabIndex >= 0; if (!keyboardReachable) throw new Error(JSON.stringify({ message: 'module target is not keyboard reachable', id: expectedId, tagName: marker.tagName, tabIndex: marker.tabIndex })); marker.focus(); const focusedBeforeClick = document.activeElement === marker; marker.click(); return JSON.stringify({ id: expectedId, ariaLabel: marker.getAttribute('aria-label'), keyboardReachable, focused: focusedBeforeClick }); })()`; }
function metadataSnapshotScript(stage) {
  return String.raw`(() => {
    const fixtures = window.__OASIS7_PIXEL_WORLD_VISUAL_FIXTURES__;
    const inject = window.__AW_TEST__?.injectSnapshot;
    const align = window.__OASIS7_PIXEL_WORLD_VISUAL_FIXTURE_AUTH_ALIGNMENT__;
    if (!fixtures?.module_visual_entities || typeof inject !== 'function' || typeof align !== 'function') {
      throw new Error('metadata fixture injection API unavailable');
    }
    const snapshot = structuredClone(fixtures.module_visual_entities());
    const baseline = ${JSON.stringify(stage === 'baseline')};
    const metadataStage = ${JSON.stringify(stage === 'metadata')};
    const explicitRouteLabel = ${JSON.stringify(stage === 'baseline' || stage === 'metadata')};
    const zhFallback = ${JSON.stringify(stage === 'metadata' || stage === 'fallback-zh')};
    const relation = {
      kind: 'logistics_route',
      status: 'active',
      source_class: 'runtime_projection',
      freshness: 'current',
    };
    snapshot.locale = zhFallback ? 'zh' : 'en';
    const localeButton = document.querySelector('[data-locale="' + snapshot.locale + '"]');
    if (!localeButton) throw new Error('locale switch unavailable for ' + snapshot.locale);
    localeButton.click();
    snapshot.model.agents['agent-0'].relation = {
      ...relation,
      ...(explicitRouteLabel ? { label: baseline ? 'Initial logistics route' : 'Updated logistics route' } : {}),
    };
    snapshot.model.module_visual_entities['module-absolute'] = {
      entity_id: 'module-absolute',
      module_id: 'fixture-module',
      kind: baseline ? 'beacon' : 'relay',
      label: baseline ? 'Beacon marker' : 'Beacon marker updated',
      anchor: { type: 'absolute', data: { x_cm: 1850000, y_cm: 3600000, z_cm: 0 } },
    };
    snapshot.model.module_visual_entities['module-relay'] = {
      entity_id: 'module-relay',
      module_id: 'fixture-module',
      kind: baseline ? 'relay' : 'beacon',
      label: baseline ? 'Relay marker' : 'Relay marker updated',
      anchor: { type: 'absolute', data: { x_cm: 1850000, y_cm: 3600000, z_cm: 0 } },
    };
    snapshot.model.module_visual_entities['module-agent'] = {
      entity_id: 'module-agent',
      module_id: 'fixture-module',
      kind: baseline ? 'future_module_kind' : 'future_module_kind_updated',
      label: baseline ? 'Unknown marker' : 'Unknown marker updated',
      anchor: { type: 'agent', data: { agent_id: 'agent-0' } },
    };
    snapshot.player_gameplay.goal_title = baseline ? 'Initial objective' : '更新目标';
    snapshot.player_gameplay.blocker_kind = baseline ? 'material_shortage' : metadataStage ? 'power_shortage' : 'updated_blocker';
    snapshot.player_gameplay.blocker_label = baseline ? 'Initial blocker' : 'Updated blocker';
    inject(snapshot, { returnState: false });
    align();
    return JSON.stringify({ stage: ${JSON.stringify(stage)}, locale: snapshot.locale, explicitRouteLabel });
  })()`;
}
function injectModuleEventFeedScript() { return String.raw`(() => { const inject = window.__AW_TEST__?.injectWorldFeedForTest; if (typeof inject !== 'function') throw new Error('world-feed test injection API unavailable'); const state = inject({ schema_version: 'world_feed/v1', world_id: 'fixture-world', reorg_epoch: 0, cursor: '102', status: 'ready', snapshot_reload_required: false, events: [{ event_seq: 102, kind: 'module_visual_entity_updated', summary: 'Module Agent updated', detail: 'module event fixture', module_visual_entity_id: 'module-agent' }] }); return JSON.stringify({ status: state.status, stale: state.stale, events: state.events }); })()`; }
function clickModuleLocatorScript(id) { return String.raw`(() => { const expectedId = ${JSON.stringify(id)}; const locator = document.querySelector('[data-world-feed-module-locate="' + expectedId + '"]'); if (!locator) throw new Error(JSON.stringify({ message: 'module event locator unavailable', expectedId, locators: [...document.querySelectorAll('[data-world-feed-module-locate]')].map((candidate) => ({ id: candidate.dataset.worldFeedModuleLocate, label: candidate.textContent.trim() })) })); const keyboardReachable = locator instanceof HTMLButtonElement && locator.tabIndex >= 0; if (!keyboardReachable) throw new Error(JSON.stringify({ message: 'module event locator is not keyboard reachable', expectedId, tagName: locator.tagName, tabIndex: locator.tabIndex })); locator.focus(); const focusedBeforeClick = document.activeElement === locator; locator.click(); return JSON.stringify({ id: expectedId, ariaLabel: locator.getAttribute('aria-label'), keyboardReachable, focused: focusedBeforeClick }); })()`; }

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
    assert(state.renderMode === "viewer", "module smoke did not use the viewer render mode", state);
    assert(state.runtimeSource === "wasm_bindgen_runtime" && /pixel_world_bridge\.js(?:$|[?#])/.test(String(state.runtimeModuleUrl || "")), "module smoke did not load the actual bindgen WASM bridge", state);
    assert(state.webgl2 && state.browserEnv?.webglRenderer, "WebGL2 renderer identity unavailable", state);
    const expectedInitialModules = [
      { id: "module-absolute", kind: "beacon", pos: { x_cm: 1_850_000, y_cm: 3_600_000, z_cm: 0 } },
      { id: "module-agent", kind: "future_module_kind", pos: { x_cm: 2_900_000, y_cm: 3_450_000, z_cm: 0 } },
      { id: "module-relay", kind: "relay", pos: { x_cm: 1_850_000, y_cm: 3_600_000, z_cm: 0 } },
    ];
    assert(JSON.stringify(state.modules) === JSON.stringify(expectedInitialModules), "module kind and anchor evidence is not present in stable order", { state, expectedInitialModules });
    assert(state.modules[0].pos.x_cm === state.modules[2].pos.x_cm && state.modules[0].pos.y_cm === state.modules[2].pos.y_cm, "beacon and relay must remain co-anchored for the renderer smoke", state.modules);
    const initialMetadata = Object.fromEntries(state.moduleMetadata.map((module) => [module.id, module]));
    assert(initialMetadata["module-absolute"]?.module_id === "fixture-module" && initialMetadata["module-absolute"]?.label === "Beacon marker" && initialMetadata["module-absolute"]?.anchor?.type === "absolute", "absolute module identity metadata is missing from the real Render DTO", initialMetadata["module-absolute"]);
    assert(initialMetadata["module-agent"]?.module_id === "fixture-module" && initialMetadata["module-agent"]?.label === "Unknown marker" && initialMetadata["module-agent"]?.anchor?.type === "agent", "agent-anchored module identity metadata is missing from the real Render DTO", initialMetadata["module-agent"]);
    assert(initialMetadata["module-relay"]?.module_id === "fixture-module" && initialMetadata["module-relay"]?.anchor?.type === "absolute", "co-anchored relay identity metadata is missing from the real Render DTO", initialMetadata["module-relay"]);

    await evalJson(metadataSnapshotScript("baseline"));
    const metadataBaselineState = await evalJson(String.raw`(async()=>{const deadline=Date.now()+5000; let state; while(Date.now()<deadline){state=JSON.parse(${pageStateScript()}); if(state.renderLocale === 'en' && state.links.some((link)=>link.label === 'Initial logistics route') && state.visualHotspots.some((hotspot)=>hotspot.label === 'Initial objective') && state.visualHotspots.some((hotspot)=>hotspot.label === 'Missing Material')) return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,50));} throw new Error(JSON.stringify({message:'metadata baseline did not reach the real Render DTO',state}));})()`);
    await evalJson(String.raw`new Promise((resolve)=>setTimeout(resolve,250))`);
    const metadataBaselineStableState = await evalJson(pageStateScript());
    const metadataBaselineCanvasPng = join(outDir, `${name}-metadata-baseline-canvas.png`);
    await browserJson(["screenshot", "#pixel-world-embedded-runtime-canvas", metadataBaselineCanvasPng]);

    await evalJson(metadataSnapshotScript("metadata"));
    const metadataUpdatedState = await evalJson(String.raw`(async()=>{const deadline=Date.now()+5000; let state; while(Date.now()<deadline){state=JSON.parse(${pageStateScript()}); if(state.renderLocale === 'zh' && state.links.some((link)=>link.label === 'Updated logistics route') && state.visualHotspots.some((hotspot)=>hotspot.label === '更新目标') && state.visualHotspots.some((hotspot)=>hotspot.label === '缺电') && state.moduleMarkers.some((marker)=>marker.id === 'module-absolute' && marker.kind === 'relay')) return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,50));} throw new Error(JSON.stringify({message:'same-geometry metadata update did not reach the real Render DTO and renderer targets',state}));})()`);
    await evalJson(String.raw`new Promise((resolve)=>setTimeout(resolve,250))`);
    const metadataUpdatedStableState = await evalJson(pageStateScript());
    assert(JSON.stringify(metadataUpdatedStableState.camera) === JSON.stringify(metadataBaselineStableState.camera), "same-geometry metadata update changed the renderer camera", { before: metadataBaselineStableState.camera, after: metadataUpdatedStableState.camera });
    assert(JSON.stringify(metadataUpdatedStableState.moduleMetadata.map(({ id, pos }) => ({ id, pos }))) === JSON.stringify(metadataBaselineStableState.moduleMetadata.map(({ id, pos }) => ({ id, pos }))), "module metadata update changed anchor geometry", { before: metadataBaselineStableState.moduleMetadata, after: metadataUpdatedStableState.moduleMetadata });
    assert(JSON.stringify(metadataUpdatedStableState.links.map(({ id, from, to }) => ({ id, from, to }))) === JSON.stringify(metadataBaselineStableState.links.map(({ id, from, to }) => ({ id, from, to }))), "route metadata update changed endpoint geometry", { before: metadataBaselineStableState.links, after: metadataUpdatedStableState.links });
    assert(JSON.stringify(metadataUpdatedStableState.visualHotspots.map(({ id, pos }) => ({ id, pos }))) === JSON.stringify(metadataBaselineStableState.visualHotspots.map(({ id, pos }) => ({ id, pos }))), "hotspot metadata update changed anchor geometry", { before: metadataBaselineStableState.visualHotspots, after: metadataUpdatedStableState.visualHotspots });
    assert(metadataUpdatedStableState.moduleMarkers.some((marker) => marker.id === "module-absolute" && marker.kind === "relay" && /Beacon marker updated/i.test(marker.ariaLabel || "")), "module identity metadata did not update in the renderer target", metadataUpdatedStableState.moduleMarkers);
    const metadataUpdatedCanvasPng = join(outDir, `${name}-metadata-updated-canvas.png`);
    await browserJson(["screenshot", "#pixel-world-embedded-runtime-canvas", metadataUpdatedCanvasPng]);
    const metadataCanvasDelta = screenshotDifference(metadataBaselineCanvasPng, metadataUpdatedCanvasPng);
    assert(metadataCanvasDelta.changedPixelCount >= 48, "same-geometry metadata update did not change rendered canvas pixels", metadataCanvasDelta);

    await evalJson(metadataSnapshotScript("fallback-zh"));
    const localeZhState = await evalJson(String.raw`(async()=>{const deadline=Date.now()+5000; let state; while(Date.now()<deadline){state=JSON.parse(${pageStateScript()}); if(state.renderLocale === 'zh' && state.links.some((link)=>link.kind === 'logistics_route' && !link.label)) return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,50));} throw new Error(JSON.stringify({message:'locale fallback baseline did not reach the real Render DTO',state}));})()`);
    await evalJson(String.raw`new Promise((resolve)=>setTimeout(resolve,250))`);
    const localeZhStableState = await evalJson(pageStateScript());
    const localeZhCanvasPng = join(outDir, `${name}-metadata-locale-zh-canvas.png`);
    await browserJson(["screenshot", "#pixel-world-embedded-runtime-canvas", localeZhCanvasPng]);
    await evalJson(metadataSnapshotScript("fallback-en"));
    const localeEnState = await evalJson(String.raw`(async()=>{const deadline=Date.now()+5000; let state; while(Date.now()<deadline){state=JSON.parse(${pageStateScript()}); if(state.renderLocale === 'en' && state.links.some((link)=>link.kind === 'logistics_route' && !link.label)) return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,50));} throw new Error(JSON.stringify({message:'locale fallback update did not reach the real Render DTO',state}));})()`);
    await evalJson(String.raw`new Promise((resolve)=>setTimeout(resolve,250))`);
    const localeEnStableState = await evalJson(pageStateScript());
    assert(JSON.stringify(localeEnStableState.camera) === JSON.stringify(localeZhStableState.camera), "locale-only fallback update changed the renderer camera", { before: localeZhStableState.camera, after: localeEnStableState.camera });
    const localeEnCanvasPng = join(outDir, `${name}-metadata-locale-en-canvas.png`);
    await browserJson(["screenshot", "#pixel-world-embedded-runtime-canvas", localeEnCanvasPng]);
    const localeCanvasDelta = screenshotDifference(localeZhCanvasPng, localeEnCanvasPng);
    assert(localeCanvasDelta.changedPixelCount >= 16, "locale-only fallback update did not change rendered canvas pixels", localeCanvasDelta);
    const metadataEvidence = {
      baseline: metadataBaselineStableState,
      updated: metadataUpdatedStableState,
      updatedCanvasDelta: metadataCanvasDelta,
      localeZh: localeZhStableState,
      localeEn: localeEnStableState,
      localeCanvasDelta,
      canvas: { baseline: metadataBaselineCanvasPng, updated: metadataUpdatedCanvasPng, localeZh: localeZhCanvasPng, localeEn: localeEnCanvasPng },
    };

    const feedInjection = await evalJson(injectModuleEventFeedScript());
    assert(feedInjection.status === "ready" && feedInjection.stale === false && feedInjection.events?.some((event) => event.module_visual_entity_id === "module-agent"), "module event fixture did not enter the authoritative World Feed state", feedInjection);
    const moduleClick = await evalJson(clickModuleScript("module-absolute"));
    assert(moduleClick.id === "module-absolute" && moduleClick.keyboardReachable, "module marker click did not dispatch through a keyboard-reachable renderer target", moduleClick);
    const selectedModuleState = await evalJson(String.raw`(async()=>{const deadline=Date.now()+5000; let state; while(Date.now()<deadline){state=JSON.parse(${pageStateScript()}); if(state.selectedKind === 'module_visual' && state.selectedId === 'module-absolute' && state.moduleMarkers.some((marker)=>marker.id === 'module-absolute' && marker.selected)) return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,50));} throw new Error(JSON.stringify({message:'module selection did not reach the viewer state',state}));})()`);
    assert(selectedModuleState.detailsVisible && selectedModuleState.detailsActive && selectedModuleState.moduleDetailsVisible, "selected module did not expose the visible active Command details panel", selectedModuleState);
    assert(/Module Details/i.test(selectedModuleState.detailsText) && selectedModuleState.detailsText.includes("fixture-module") && /beacon/i.test(selectedModuleState.detailsText) && selectedModuleState.detailsText.includes("Beacon marker") && /absolute/i.test(selectedModuleState.detailsText), "selected module details omitted module/kind/label/anchor evidence", selectedModuleState);
    const moduleDetailsStatePath = writeJson(`${name}-module-details-state.json`, selectedModuleState);
    const moduleDetailsPng = join(outDir, `${name}-module-details.png`); await runBrowser(["screenshot", "--full", moduleDetailsPng]);

    const preEventState = await evalJson(pageStateScript());
    let worldState = preEventState;
    if (preEventState.detailsVisible && preEventState.detailsActive) {
      await clickVisible("#viewer-details-panel .panel__route-close");
      worldState = await evalJson(String.raw`(async()=>{const deadline=Date.now()+5000; let state; while(Date.now()<deadline){state=JSON.parse(${pageStateScript()}); if(window.location.hash === '#viewer-stage-panel' && document.activeElement?.id === 'viewer-stage-panel') return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,50));} throw new Error(JSON.stringify({message:'module detail route did not return to World after visible Back interaction',state,hash:window.location.hash,activeElement:document.activeElement?.id || null}));})()`);
    } else {
      assert(!preEventState.detailsVisible && !preEventState.detailsActive && preEventState.selectedId === 'module-absolute' && (!preEventState.hash || preEventState.hash === '#viewer-stage-panel'), "module route was neither an active visible Command panel nor an already-returned World state", preEventState);
    }
    assert(!worldState.detailsActive && worldState.selectedId === 'module-absolute', "World route did not preserve the selected module", worldState);
    await clickVisible("#viewer-world-feed > summary");
    const feedReadyState = await evalJson(String.raw`(async()=>{const deadline=Date.now()+5000; let state; while(Date.now()<deadline){state=JSON.parse(${pageStateScript()}); if(state.eventLocators.some((locator)=>locator.id === 'module-agent')) return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,50));} throw new Error(JSON.stringify({message:'module event locator did not render',state}));})()`);

    const locateTarget = await evalJson(String.raw`(() => { const locator = document.querySelector('[data-world-feed-module-locate="module-agent"]'); if (!locator) throw new Error('module event locator unavailable'); const rect = locator.getBoundingClientRect(); const style = getComputedStyle(locator); return JSON.stringify({ id: locator.dataset.worldFeedModuleLocate, keyboardReachable: locator instanceof HTMLButtonElement && locator.tabIndex >= 0, visible: rect.width > 0 && rect.height > 0 && rect.top >= 0 && rect.bottom <= innerHeight && style.display !== 'none' && style.visibility !== 'hidden', rect: { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom } }); })()`);
    assert(locateTarget.id === "module-agent" && locateTarget.keyboardReachable && locateTarget.visible, "module event locator was not a visible keyboard-reachable control", locateTarget);
    const locateClick = await clickVisible('[data-world-feed-module-locate="module-agent"]');
    const locatedModuleState = await evalJson(String.raw`(async()=>{const deadline=Date.now()+5000; let state; while(Date.now()<deadline){state=JSON.parse(${pageStateScript()}); if(state.selectedKind === 'module_visual' && state.selectedId === 'module-agent' && state.moduleMarkers.some((marker)=>marker.id === 'module-agent' && marker.selected) && /Module Details/i.test(state.detailsText)) return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,50));} throw new Error(JSON.stringify({message:'module event locator did not select its module',state}));})()`);
    assert(locatedModuleState.detailsVisible && locatedModuleState.detailsActive && locatedModuleState.moduleDetailsVisible, "located module did not expose the visible active Command details panel", locatedModuleState);
    assert(/future_module_kind/i.test(locatedModuleState.detailsText) && locatedModuleState.detailsText.includes("Unknown marker") && /agent\s*[·:]\s*agent-0/i.test(locatedModuleState.detailsText), "module event locate did not expose the target module details", locatedModuleState);
    await evalJson(String.raw`new Promise((resolve)=>setTimeout(resolve,250))`);
    const locatedAfterFocusState = await evalJson(pageStateScript());
    assert(locatedAfterFocusState.camera && selectedModuleState.camera && (locatedAfterFocusState.camera.pan_x_px !== selectedModuleState.camera.pan_x_px || locatedAfterFocusState.camera.pan_y_px !== selectedModuleState.camera.pan_y_px), "module event locate did not move the actual renderer camera to the target", { before: selectedModuleState.camera, after: locatedAfterFocusState.camera, locatedModuleState });
    const moduleLocateStatePath = writeJson(`${name}-module-event-locate-state.json`, locatedAfterFocusState);
    const moduleLocatePng = join(outDir, `${name}-module-event-locate.png`); await runBrowser(["screenshot", "--full", moduleLocatePng]);

    const staleUpdate = await evalJson(String.raw`(() => { const control = window.__OASIS7_MODULE_VISUAL_FIXTURE_CONTROL__; if (!control?.update) throw new Error('module fixture update control unavailable'); return JSON.stringify(control.update({"module-absolute":{"entity_id":"module-absolute","module_id":"fixture-module","kind":"beacon","label":"Beacon marker","anchor":{"type":"absolute","data":{"x_cm":1850000,"y_cm":3600000,"z_cm":0}}},"module-relay":{"entity_id":"module-relay","module_id":"fixture-module","kind":"relay","label":"Relay marker","anchor":{"type":"absolute","data":{"x_cm":1850000,"y_cm":3600000,"z_cm":0}}}})); })()`);
    assert(staleUpdate === true, "module fixture stale-target update control failed", staleUpdate);
    const staleState = await evalJson(String.raw`(async()=>{const deadline=Date.now()+5000; let state; while(Date.now()<deadline){state=JSON.parse(${pageStateScript()}); if(state.modules.length === 2 && !state.modules.some((module)=>module.id === 'module-agent') && state.selectedKind === null && state.selectedId === null && !state.moduleMarkers.some((marker)=>marker.id === 'module-agent') && !state.eventLocators.some((locator)=>locator.id === 'module-agent')) return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,50));} throw new Error(JSON.stringify({message:'stale module selection or event locator remained after removal',state}));})()`);
    assert(!/Module Details|module-agent/i.test(staleState.detailsText), "stale module details remained visible after the target was removed", staleState);
    const staleStatePath = writeJson(`${name}-module-stale-state.json`, staleState);
    const stalePng = join(outDir, `${name}-module-stale.png`); await runBrowser(["screenshot", "--full", stalePng]);
    await evalJson(zoomCanvasScript(-120, 12));
    const closeState = await evalJson(String.raw`(async()=>{const deadline=Date.now()+5000; let state; while(Date.now()<deadline){state=JSON.parse(${pageStateScript()}); if(state.camera?.zoom >= 1.75) return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,50));} throw new Error(JSON.stringify(state));})()`);
    await evalJson(panCanvasScript());
    const closePanState = await evalJson(String.raw`(async()=>{const deadline=Date.now()+5000; let state; while(Date.now()<deadline){state=JSON.parse(${pageStateScript()}); if(state.camera?.pan_x_px > ${closeState.camera.pan_x_px}) return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,50));} throw new Error(JSON.stringify(state));})()`);
    await evalJson(scrollCanvasIntoViewScript());
    const closeViewportPng = join(outDir, `${name}-close-labels-viewport.png`); await runBrowser(["screenshot", closeViewportPng]);
    await evalJson(zoomCanvasScript(120, 16));
    const overviewState = await evalJson(String.raw`(async()=>{const deadline=Date.now()+5000; let state; while(Date.now()<deadline){state=JSON.parse(${pageStateScript()}); if(state.camera?.zoom < 1.75) return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,50));} throw new Error(JSON.stringify(state));})()`);
    await evalJson(scrollCanvasIntoViewScript());
    const overviewViewportPng = join(outDir, `${name}-overview-glyphs-viewport.png`); await runBrowser(["screenshot", overviewViewportPng]);
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
    const fallbackClick = await evalJson(clickModuleScript("module-update"));
    assert(fallbackClick.id === "module-update" && fallbackClick.keyboardReachable, "unknown-kind fallback module marker was not keyboard reachable", fallbackClick);
    const fallbackState = await evalJson(String.raw`(async()=>{const deadline=Date.now()+5000; let state; while(Date.now()<deadline){state=JSON.parse(${pageStateScript()}); if(state.selectedKind === 'module_visual' && state.selectedId === 'module-update' && /future_module_kind:module-update/i.test(state.detailsText)) return JSON.stringify(state); await new Promise((resolve)=>setTimeout(resolve,50));} throw new Error(JSON.stringify({message:'unknown module fallback label was not exposed in details',state}));})()`);
    const fallbackStatePath = writeJson(`${name}-module-fallback-state.json`, fallbackState);
    const updatedPng = join(outDir, `${name}-updated.png`); await runBrowser(["screenshot", "--full", updatedPng]);
    const updatedCanvasDimensions = await evalJson(canvasDimensionsScript());
    const updatedCanvasPng = join(outDir, `${name}-updated-canvas.png`); await runBrowser(["screenshot", "#pixel-world-embedded-runtime-canvas", updatedCanvasPng]);
    await evalJson(scrollCanvasIntoViewScript());
    const updatedViewportPng = join(outDir, `${name}-updated-viewport.png`); await runBrowser(["screenshot", updatedViewportPng]);
    const consolePath = join(outDir, `${name}-console.log`); const consoleOutput = await runBrowser(["console"]); writeFileSync(consolePath, consoleOutput);
    assert(!/\b(?:fatal|CONTEXT_LOST_WEBGL|webgl.*error)\b/i.test(consoleOutput), "browser console reports a renderer fatal", { consolePath, consoleOutput });
    summary.viewports[name] = { width, height, state, metadataEvidence, feedReadyState, selectedModuleState, locatedAfterFocusState, staleState, updatedState, fallbackState, closeState, closePanState, overviewState, closeViewportPng, overviewViewportPng, moduleDetailsStatePath, moduleDetailsPng, moduleLocateStatePath, moduleLocatePng, staleStatePath, stalePng, fallbackStatePath, beforePng, beforeCanvasPng, beforeViewportPng, clearedPng, clearedCanvasPng, clearedViewportPng, updatedPng, updatedCanvasPng, updatedViewportPng, canvasDimensions: { before: beforeCanvasDimensions, cleared: clearedCanvasDimensions, updated: updatedCanvasDimensions }, consolePath };
  }
  summary.status = "passed";
} catch (error) { summary.status = "failed"; summary.failure = { message: error instanceof Error ? error.message : String(error) }; throw error; }
finally { summary.completedAt = new Date().toISOString(); writeJson("summary.json", summary); closeBrowser(); await new Promise((resolveClose) => server.close(resolveClose)); }
