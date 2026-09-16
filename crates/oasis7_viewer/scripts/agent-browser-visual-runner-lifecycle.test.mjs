import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const lifecycleModule = pathToFileURL(join(scriptDir, "agent-browser-visual-runner-lifecycle.mjs")).href;
const visualRunners = [
  "pixel-world-fragment-visual-smoke.mjs",
  "pixel-world-hotspot-visual-smoke.mjs",
  "pixel-world-location-frame-visual-smoke.mjs",
  "pixel-world-module-visual-smoke.mjs",
  "player-visual-feedback-browser-smoke.mjs",
  "viewer-performance-probe.mjs",
];

for (const runner of visualRunners) {
  const source = readFileSync(join(scriptDir, runner), "utf8");
  assert.match(source, /const session = `[^`]*-\$\{process\.pid\}`;/,
    `${runner} must use a PID-scoped session name`);
  assert.match(source, /createOwnedSessionLifecycle/,
    `${runner} must install owned-session signal cleanup`);
  assert.doesNotMatch(source, /close-all|close\s+--all|\[\s*["']close["']\s*,\s*["']--all["']\s*\]/i,
    `${runner} must never close all browser sessions`);
}

const root = mkdtempSync(join(tmpdir(), "oasis7-agent-browser-lifecycle-"));
const fakeBrowser = join(root, "fake-agent-browser.mjs");
const logPath = join(root, "close-calls.jsonl");
writeFileSync(fakeBrowser, `
import { appendFileSync } from "node:fs";
appendFileSync(process.argv[2], JSON.stringify(process.argv.slice(3)) + "\\n");
`);

function runFixture(mode, signal = "SIGTERM") {
  const fixture = `
    import { createOwnedSessionLifecycle } from ${JSON.stringify(lifecycleModule)};
    const lifecycle = createOwnedSessionLifecycle({
      command: process.execPath,
      prefixArgs: [${JSON.stringify(fakeBrowser)}, ${JSON.stringify(logPath)}],
      session: "visual-owned-session",
    });
    lifecycle.prepare();
    if (${JSON.stringify(mode)} === "signal") {
      const keepAlive = setInterval(() => {}, 1000);
      setTimeout(() => process.kill(process.pid, ${JSON.stringify(signal)}), 10);
    } else {
      setTimeout(() => { lifecycle.close(); lifecycle.close(); }, 10);
    }
  `;
  return spawnSync(process.execPath, ["--input-type=module", "-e", fixture], {
    encoding: "utf8",
  });
}

try {
  const normal = runFixture("normal");
  assert.equal(normal.status, 0, `normal fixture failed:\n${normal.stderr}`);
  const signalRuns = [
    ["SIGINT", 130],
    ["SIGTERM", 143],
  ];
  for (const [signal, expectedStatus] of signalRuns) {
    const signaled = runFixture("signal", signal);
    assert.equal(signaled.status, expectedStatus,
      `signal fixture did not exit via ${signal}:\n${signaled.stderr}`);
  }

  const calls = readFileSync(logPath, "utf8").trim().split("\n").filter(Boolean).map(JSON.parse);
  assert.equal(calls.length, 6, "each fixture should perform one prepare close and one terminal close");
  for (const args of calls) {
    assert.deepEqual(args, ["--session", "visual-owned-session", "close"],
      "cleanup must be session-scoped and must not use close --all");
  }
  console.log("agent-browser visual runner lifecycle tests passed");
} finally {
  rmSync(root, { recursive: true, force: true });
}
