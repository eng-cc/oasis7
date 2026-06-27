import { readFileSync } from "node:fs";
import assert from "node:assert/strict";

const coreSource = readFileSync("crates/oasis7_viewer/software_safe_src/legacy_core.js", "utf8");
const runnerSource = readFileSync("scripts/viewer-aw-test-completeness-playthrough.sh", "utf8");

const requiredApiMethods = [
  "getState",
  "describeControls",
  "fillControlExample",
  "sendControl",
  "sendGameplayAction",
  "runSteps",
  "select",
  "focus",
  "sendAgentChat",
  "sendPromptControl",
  "injectSnapshot",
];

for (const method of requiredApiMethods) {
  assert.match(
    coreSource,
    new RegExp(`\\b${method}\\b`),
    `legacy_core.js should define or expose __AW_TEST__.${method}`,
  );
  assert.match(
    runnerSource,
    new RegExp(`__AW_TEST__\\.${method}\\b|\\['${method}'\\]|"${method}"`),
    `completeness runner should verify or exercise __AW_TEST__.${method}`,
  );
}

for (const action of ["play", "pause", "step"]) {
  assert.match(coreSource, new RegExp(`action:\\s*"${action}"`), `describeControls should publish ${action}`);
}

assert.match(runnerSource, /assert_api_surface/, "runner must have an explicit API surface assertion step");
assert.match(runnerSource, /assert_api_progression/, "runner must prove API-driven progression/action evidence");

console.log("__AW_TEST__ completeness guard passed");
