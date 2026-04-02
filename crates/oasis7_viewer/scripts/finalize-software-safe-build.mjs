import { access, copyFile, readdir, rm } from "node:fs/promises";
import { relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptsDir = fileURLToPath(new URL(".", import.meta.url));
const viewerRoot = resolve(scriptsDir, "..");
const tempOutDir = resolve(viewerRoot, ".software-safe-build");
const builtBundlePath = resolve(tempOutDir, "software_safe.js");
const finalBundlePath = resolve(viewerRoot, "software_safe.js");

async function listFilesRecursively(dirPath) {
  const entries = await readdir(dirPath, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const entryPath = resolve(dirPath, entry.name);
    if (entry.isDirectory()) {
      files.push(...await listFilesRecursively(entryPath));
      continue;
    }
    if (entry.isFile()) {
      files.push(entryPath);
    }
  }
  return files;
}

await access(builtBundlePath);
const emittedFiles = (await listFilesRecursively(tempOutDir))
  .map((filePath) => relative(tempOutDir, filePath))
  .sort();
if (emittedFiles.length !== 1 || emittedFiles[0] !== "software_safe.js") {
  throw new Error(`unexpected software_safe bundle outputs: ${emittedFiles.join(", ") || "(none)"}`);
}
await copyFile(builtBundlePath, finalBundlePath);
await rm(tempOutDir, { recursive: true, force: true });

console.log(`software_safe build finalized: ${finalBundlePath}`);
