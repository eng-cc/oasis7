import { spawn } from "node:child_process";
import { access, copyFile, mkdir, readdir, rm } from "node:fs/promises";
import { relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptsDir = fileURLToPath(new URL(".", import.meta.url));
const viewerRoot = resolve(scriptsDir, "..");
const workspaceRoot = resolve(viewerRoot, "..", "..");
const tempOutDir = resolve(viewerRoot, ".software-safe-build");
const softwareSafeSrcDir = resolve(viewerRoot, "software_safe_src");
const builtBundlePath = resolve(tempOutDir, "software_safe.js");
const finalBundlePath = resolve(viewerRoot, "software_safe.js");
const pixelWorldRuntimeDir = resolve(viewerRoot, "pixel-world-bridge");
const pixelWorldRuntimeSourcePath = resolve(softwareSafeSrcDir, "pixel_world_runtime_module_wasm.js");
const pixelWorldRuntimeModulePath = resolve(pixelWorldRuntimeDir, "pixel_world_bridge.js");
const pixelWorldCompiledWasmPath = resolve(
  workspaceRoot,
  "target",
  "wasm32-unknown-unknown",
  "debug",
  "pixel_world_bridge.wasm",
);

function runChecked(command, args, options = {}) {
  return new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(command, args, {
      cwd: options.cwd || workspaceRoot,
      stdio: "inherit",
      env: options.env || process.env,
    });
    child.on("error", rejectPromise);
    child.on("exit", (code) => {
      if (code === 0) {
        resolvePromise();
        return;
      }
      rejectPromise(new Error(`${command} ${args.join(" ")} exited with code ${code}`));
    });
  });
}

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
await runChecked("env", [
  "-u",
  "RUSTC_WRAPPER",
  "cargo",
  "build",
  "-p",
  "pixel_world_bridge",
  "--target",
  "wasm32-unknown-unknown",
]);
await access(pixelWorldCompiledWasmPath);
await rm(pixelWorldRuntimeDir, { recursive: true, force: true });
await mkdir(pixelWorldRuntimeDir, { recursive: true });
await runChecked("wasm-bindgen", [
  "--target",
  "web",
  "--out-dir",
  pixelWorldRuntimeDir,
  "--out-name",
  "pixel_world_bridge_bindgen",
  pixelWorldCompiledWasmPath,
]);
await copyFile(pixelWorldRuntimeSourcePath, pixelWorldRuntimeModulePath);
await rm(tempOutDir, { recursive: true, force: true });

console.log(`software_safe build finalized: ${finalBundlePath}`);
