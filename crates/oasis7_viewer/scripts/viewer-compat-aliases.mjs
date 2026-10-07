import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const softwareSafeBundleAlias = [
  "// Generated compat alias; canonical bundle truth lives in ./viewer.js.",
  'import "./viewer.js";',
  "",
].join("\n");

const claimEvidenceReplacements = [
  ["<title>viewer first agent claim evidence</title>", "<title>software_safe compatibility first agent claim evidence</title>"],
  ['id="viewer-frame"', 'id="software-safe-frame"'],
  ['./viewer.html?test_api=1&connect=0', './software_safe.html?test_api=1&connect=0'],
  ['title="viewer first agent claim evidence"', 'title="software_safe compatibility first agent claim evidence"'],
  ['document.getElementById("viewer-frame")', 'document.getElementById("software-safe-frame")'],
];

function replaceOne(source, before, after) {
  const first = source.indexOf(before);
  if (first === -1 || source.indexOf(before, first + before.length) !== -1) {
    throw new Error(`canonical first-agent claim page must contain exactly one ${JSON.stringify(before)}`);
  }
  return `${source.slice(0, first)}${after}${source.slice(first + before.length)}`;
}

export function createSoftwareSafeBundleAlias() {
  return softwareSafeBundleAlias;
}

export function createSoftwareSafeClaimEvidenceAlias(canonicalHtml) {
  return claimEvidenceReplacements.reduce(
    (html, [before, after]) => replaceOne(html, before, after),
    canonicalHtml,
  );
}

export async function writeViewerCompatAliases(viewerRoot, distDir) {
  const canonicalHtmlPath = resolve(viewerRoot, "viewer.html");
  const canonicalClaimEvidencePath = resolve(viewerRoot, "viewer_first_agent_claim_evidence.html");
  const outputDir = resolve(distDir);
  const [canonicalHtml, canonicalClaimEvidence] = await Promise.all([
    readFile(canonicalHtmlPath, "utf8"),
    readFile(canonicalClaimEvidencePath, "utf8"),
  ]);

  await mkdir(outputDir, { recursive: true });
  await Promise.all([
    writeFile(resolve(outputDir, "software_safe.html"), canonicalHtml, "utf8"),
    writeFile(resolve(outputDir, "software_safe.js"), createSoftwareSafeBundleAlias(), "utf8"),
    writeFile(
      resolve(outputDir, "software_safe_first_agent_claim_evidence.html"),
      createSoftwareSafeClaimEvidenceAlias(canonicalClaimEvidence),
      "utf8",
    ),
  ]);
}

async function runCli() {
  const [, , viewerRoot, distDir] = process.argv;
  if (!viewerRoot || !distDir) {
    throw new Error("usage: node viewer-compat-aliases.mjs <viewer-root> <dist-dir>");
  }
  await writeViewerCompatAliases(viewerRoot, distDir);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await runCli();
}
