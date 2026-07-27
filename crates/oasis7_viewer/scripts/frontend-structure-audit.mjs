import assert from "node:assert/strict";
import { access, readFile, readdir } from "node:fs/promises";
import { basename, join, relative, resolve } from "node:path";

const viewerRoot = resolve(new URL("..", import.meta.url).pathname);
const repoRoot = resolve(viewerRoot, "..", "..");
const sourceDir = resolve(viewerRoot, "software_safe_src");
const canonicalHtmlPath = resolve(viewerRoot, "viewer.html");
const compatHtmlPath = resolve(viewerRoot, "software_safe.html");
const compatBundlePath = resolve(viewerRoot, "software_safe.js");
const viewerDistDir = resolve(viewerRoot, "dist");
const distViewerBundlePath = resolve(viewerDistDir, "viewer.js");
const pixelWorldRuntimeDir = resolve(viewerDistDir, "pixel-world-bridge");
const canonicalBundlePath = resolve(viewerRoot, "viewer.js");
const canonicalBundleBanner = "// Generated canonical Viewer bundle; source truth lives in ./software_safe_src/.\n";

const PRODUCTION_SOURCE_LINE_LIMIT = 1200;
const TEST_SOURCE_LINE_LIMIT = 1600;

const knownLineDebt = new Map(Object.entries({
  "crates/oasis7_viewer/viewer.html": {
    maxLines: 2842,
    owner: "viewer_engineer",
    reason: "task_4b9393c487e244e4bf5f2811eb5ef95c adds one bounded Next Step hierarchy treatment while preserving the canonical and compat HTML contract; extracting unrelated shell styles would broaden the visual-polish task",
    nextTrigger: "the next Viewer inline-style or document-shell behavior change must extract a coherent style/token boundary or record a narrower owner-tagged exemption",
  },
  "crates/oasis7_viewer/software_safe_src/legacy_core.js": {
    maxLines: 4435,
    owner: "viewer_engineer",
    reason: "task_36a85651cfec4ce2b35a20545990c69d adds a bounded power-survival quote facade while quote state, request signing, rendering, and fixtures stay outside legacy_core",
    nextTrigger: "next legacy_core behavior change must extract the remaining quote protocol dispatch/test-api facade",
  },
  "crates/oasis7_viewer/software_safe_src/main.jsx": {
    maxLines: 4379,
    owner: "viewer_engineer",
    reason: "task_36a85651cfec4ce2b35a20545990c69d composes the extracted bounded PowerSurvivalQuoteGameplayPanel without expanding its rendering logic into main.jsx",
    nextTrigger: "next main.jsx UI behavior change must extract another named composition boundary or display-model helper",
  },
  "crates/oasis7_viewer/software_safe_src/main.test.jsx": {
    maxLines: 3363,
    owner: "viewer_engineer",
    reason: "task_4b9393c487e244e4bf5f2811eb5ef95c adds focused Next Step semantic assertions to the existing Viewer baseline; moving the broad fixture would exceed this visual-polish task",
    nextTrigger: "the next broad UI test addition must extract the chat-history fixture/query helpers or place the new behavior in a narrower adjacent test file",
  },
  "crates/oasis7_viewer/software_safe_src/pixel_world_host.jsx": {
    maxLines: 1680,
    owner: "viewer_engineer",
    reason: "Pixel World host keeps runtime bridge composition and visual overlay surfaces; test-only snapshot fixture data now lives in pixel_world_visual_fixture_data.js",
    nextTrigger: "next Pixel World UI/runtime host behavior change should extract a named widget or service boundary",
  },
}));

function repoRelative(path) {
  return relative(repoRoot, path).replaceAll("\\", "/");
}

function countLines(text) {
  if (text.length === 0) {
    return 0;
  }
  return text.endsWith("\n") ? text.split("\n").length - 1 : text.split("\n").length;
}

async function listSourceFiles(dirPath) {
  const entries = await readdir(dirPath, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const entryPath = join(dirPath, entry.name);
    if (entry.isDirectory()) {
      files.push(...await listSourceFiles(entryPath));
      continue;
    }
    if (entry.isFile() && /\.(js|jsx|html)$/.test(entry.name)) {
      files.push(entryPath);
    }
  }
  return files;
}

function validateDebtRecord(filePath, lineCount, limit, failures) {
  const relPath = repoRelative(filePath);
  if (lineCount <= limit) {
    return null;
  }

  const debt = knownLineDebt.get(relPath);
  if (!debt) {
    failures.push(`${relPath} has ${lineCount} lines, above ${limit}, without a frontend structure debt exemption`);
    return null;
  }
  if (!debt.owner || !debt.reason || !debt.nextTrigger) {
    failures.push(`${relPath} debt exemption must include owner, reason, and nextTrigger`);
  }
  if (lineCount > debt.maxLines) {
    failures.push(`${relPath} grew from debt cap ${debt.maxLines} to ${lineCount}; split or update task evidence with a new owner-tagged exemption`);
  }
  return { relPath, lineCount, limit, ...debt };
}

async function validateLineThresholds() {
  const failures = [];
  const debtFindings = [];
  const files = [canonicalHtmlPath, ...await listSourceFiles(sourceDir)];
  for (const filePath of files) {
    const source = await readFile(filePath, "utf8");
    const relPath = repoRelative(filePath);
    const limit = /\.test\.(js|jsx|html)$/.test(basename(filePath))
      ? TEST_SOURCE_LINE_LIMIT
      : PRODUCTION_SOURCE_LINE_LIMIT;
    const finding = validateDebtRecord(filePath, countLines(source), limit, failures);
    if (finding) {
      debtFindings.push(finding);
    }
    if (knownLineDebt.has(relPath) && !finding) {
      failures.push(`${relPath} has a stale frontend structure debt exemption; remove it from frontend-structure-audit.mjs`);
    }
  }
  for (const relPath of knownLineDebt.keys()) {
    const filePath = resolve(repoRoot, relPath);
    if (!files.includes(filePath)) {
      failures.push(`${relPath} has a frontend structure debt exemption but is not audited as source`);
    }
  }
  return { failures, debtFindings };
}

async function validateCanonicalCompatContracts() {
  const failures = [];
  const [canonicalHtml, compatHtml, canonicalBundle, compatBundle] = await Promise.all([
    readFile(canonicalHtmlPath, "utf8"),
    readFile(compatHtmlPath, "utf8"),
    readFile(canonicalBundlePath, "utf8"),
    readFile(compatBundlePath, "utf8"),
  ]);

  if (compatHtml !== canonicalHtml) {
    failures.push("software_safe.html must remain a byte-for-byte compat copy of viewer.html");
  }
  if (!canonicalHtml.includes('<script type="module" src="./viewer.js"></script>')) {
    failures.push("viewer.html must reference canonical viewer.js bundle");
  }
  const diagnosticStripRule = canonicalHtml.match(/\.command-surface__diagnostic-strip \.badge\s*\{([^}]*)\}/);
  if (!diagnosticStripRule || /(?:^|[;\s])color\s*:/.test(diagnosticStripRule[1])) {
    failures.push("diagnostic strip base styling must preserve badge status colors");
  }
  if (!canonicalBundle.startsWith(canonicalBundleBanner)) {
    failures.push("viewer.js must carry the generated canonical bundle banner");
  }
  assert.equal(
    compatBundle,
    "// Generated compat alias; canonical bundle truth lives in ./viewer.js.\nimport \"./viewer.js\";\n",
    "software_safe.js must stay a generated compat alias",
  );
  return failures;
}

async function fileExists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

async function validateGeneratedRuntimeContracts() {
  const failures = [];
  if (!await fileExists(viewerDistDir)) {
    return failures;
  }

  const [canonicalBundle, distBundle] = await Promise.all([
    readFile(canonicalBundlePath, "utf8"),
    readFile(distViewerBundlePath, "utf8").catch(() => null),
  ]);
  if (distBundle == null) {
    failures.push("dist/viewer.js must exist when crates/oasis7_viewer/dist exists");
  } else if (distBundle !== canonicalBundle) {
    failures.push("dist/viewer.js must remain a finalize-managed copy of canonical viewer.js");
  }

  if (!await fileExists(pixelWorldRuntimeDir)) {
    failures.push("dist/pixel-world-bridge must exist when crates/oasis7_viewer/dist exists");
    return failures;
  }

  const rootRuntimeFiles = new Set(await readdir(pixelWorldRuntimeDir));
  if (!rootRuntimeFiles.has("pixel_world_bridge.js")) {
    failures.push("dist/pixel-world-bridge/pixel_world_bridge.js must exist as the generated runtime selector");
  }
  for (const backend of ["webgl2"]) {
    const backendDir = resolve(pixelWorldRuntimeDir, backend);
    if (!await fileExists(backendDir)) {
      failures.push(`dist/pixel-world-bridge/${backend} must exist as a finalize-managed runtime backend`);
      continue;
    }
    const backendFiles = new Set(await readdir(backendDir));
    for (const requiredFile of [
      "pixel_world_bridge.js",
      "pixel_world_bridge_bindgen.js",
      "pixel_world_bridge_bindgen_bg.wasm",
    ]) {
      if (!backendFiles.has(requiredFile)) {
        failures.push(`dist/pixel-world-bridge/${backend}/${requiredFile} must exist as a finalize-managed runtime artifact`);
      }
    }
  }
  return failures;
}

const lineAudit = await validateLineThresholds();
const compatFailures = await validateCanonicalCompatContracts();
const generatedRuntimeFailures = await validateGeneratedRuntimeContracts();
const failures = [...lineAudit.failures, ...compatFailures, ...generatedRuntimeFailures];

if (failures.length > 0) {
  console.error("frontend structure audit failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log("frontend structure audit passed");
for (const finding of lineAudit.debtFindings) {
  console.log(
    `debt: ${finding.relPath} lines=${finding.lineCount}/${finding.limit} owner=${finding.owner} next=${finding.nextTrigger}`,
  );
}
