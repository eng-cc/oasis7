import assert from "node:assert/strict";
import { access, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { writeViewerCompatAliases } from "./viewer-compat-aliases.mjs";

const tempRoot = await mkdtemp(join(tmpdir(), "oasis7-viewer-compat-aliases-"));
const viewerRoot = join(tempRoot, "viewer");
const distDir = join(viewerRoot, "dist");
const canonicalHtml = '<!doctype html><script type="module" src="./viewer.js"></script>\n';
const canonicalClaimEvidence = `<!doctype html>
<html>
  <head>
    <title>viewer first agent claim evidence</title>
  </head>
  <body>
    <iframe id="viewer-frame" src="./viewer.html?test_api=1&connect=0" title="viewer first agent claim evidence"></iframe>
    <script>
      const snapshot = { eligible_balance_after: 0, upkeep_runway_epochs: 0 };
      const frame = document.getElementById("viewer-frame");
    </script>
  </body>
</html>
`;

try {
  await mkdir(viewerRoot, { recursive: true });
  await writeFile(join(viewerRoot, "viewer.html"), canonicalHtml, "utf8");
  await writeFile(join(viewerRoot, "viewer_first_agent_claim_evidence.html"), canonicalClaimEvidence, "utf8");
  await writeViewerCompatAliases(viewerRoot, distDir);

  assert.equal(await readFile(join(distDir, "software_safe.html"), "utf8"), canonicalHtml);
  assert.equal(
    await readFile(join(distDir, "software_safe.js"), "utf8"),
    '// Generated compat alias; canonical bundle truth lives in ./viewer.js.\nimport "./viewer.js";\n',
  );

  const compatClaimEvidence = await readFile(join(distDir, "software_safe_first_agent_claim_evidence.html"), "utf8");
  assert.match(compatClaimEvidence, /<title>software_safe compatibility first agent claim evidence<\/title>/);
  assert.match(compatClaimEvidence, /id="software-safe-frame"/);
  assert.match(compatClaimEvidence, /src="\.\/software_safe\.html\?test_api=1&connect=0"/);
  assert.match(compatClaimEvidence, /document\.getElementById\("software-safe-frame"\)/);
  assert.match(compatClaimEvidence, /eligible_balance_after: 0/);
  assert.match(compatClaimEvidence, /upkeep_runway_epochs: 0/);
  assert.doesNotMatch(compatClaimEvidence, /src="\.\/viewer\.html/);

  await assert.rejects(access(join(viewerRoot, "software_safe.js")));
  await assert.rejects(access(join(viewerRoot, "software_safe.html")));
  await assert.rejects(access(join(viewerRoot, "software_safe_first_agent_claim_evidence.html")));
} finally {
  await rm(tempRoot, { recursive: true, force: true });
}

console.log("viewer-compat-aliases.test: OK");
