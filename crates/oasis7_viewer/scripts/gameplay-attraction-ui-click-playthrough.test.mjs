import { readFileSync } from "node:fs";
import assert from "node:assert/strict";

const scriptPath = "scripts/viewer-gameplay-attraction-ui-click-playthrough.sh";
const source = readFileSync(scriptPath, "utf8");

assert.match(source, /find_testid_click\(\)/, "UI-click runner must click player-visible controls by test id");
assert.match(source, /viewer-playthrough-select-agent/, "runner must click the visible target-agent control");
assert.match(source, /viewer-playthrough-action-step/, "runner must click the visible step control");
assert.match(source, /viewer-playthrough-action-request-snapshot/, "runner must click the visible refresh-snapshot control");
assert.match(source, /viewer-playthrough-action-recommended/, "runner must click the visible recommended-action control");
assert.doesNotMatch(
  source,
  /__AW_TEST__\.(sendControl|sendGameplayAction|runSteps)\s*\(/,
  "__AW_TEST__ may be used for assertions/evidence only, not for gameplay progression in the UI-click runner",
);

const automationSource = readFileSync("scripts/verify-gameplay-attraction-automation.sh", "utf8");
assert.match(
  automationSource,
  /live_browser_30m_ui_click_playthrough/,
  "live automation tier must include the actual UI-click playthrough command",
);

console.log("TASK-GAME-076 UI-click playthrough guard passed");
