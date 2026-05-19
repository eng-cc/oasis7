import { spawn } from "node:child_process";
import { access, copyFile, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptsDir = fileURLToPath(new URL(".", import.meta.url));
const viewerRoot = resolve(scriptsDir, "..");
const workspaceRoot = resolve(viewerRoot, "..", "..");
const tempOutDir = resolve(viewerRoot, ".software-safe-build");
const softwareSafeSrcDir = resolve(viewerRoot, "software_safe_src");
const builtBundlePath = resolve(tempOutDir, "viewer.js");
const finalCanonicalBundlePath = resolve(viewerRoot, "viewer.js");
const finalCompatBundlePath = resolve(viewerRoot, "software_safe.js");
const pixelWorldRuntimeDir = resolve(viewerRoot, "pixel-world-bridge");
const pixelWorldRuntimeSourcePath = resolve(softwareSafeSrcDir, "pixel_world_runtime_module_wasm.js");
const pixelWorldRuntimeModulePath = resolve(pixelWorldRuntimeDir, "pixel_world_bridge.js");
const pixelWorldCompiledWasmPath = resolve(
  workspaceRoot,
  "target",
  "wasm32-unknown-unknown",
  "release",
  "pixel_world_bridge.wasm",
);
const cargoHomeDir = process.env.CARGO_HOME
  ? resolve(process.env.CARGO_HOME)
  : resolve(process.env.HOME || workspaceRoot, ".cargo");
const cargoBinDir = resolve(cargoHomeDir, "bin");
const wasmBindgenCliPath = resolve(cargoBinDir, "wasm-bindgen");
const cargoLockPath = resolve(workspaceRoot, "Cargo.lock");

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

async function findWasmBindgenVersion() {
  const lockContents = await readFile(cargoLockPath, "utf8");
  const match = lockContents.match(/\[\[package\]\]\s+name = "wasm-bindgen"\s+version = "([^"]+)"/m);
  if (!match?.[1]) {
    throw new Error(`failed to resolve wasm-bindgen version from ${cargoLockPath}`);
  }
  return match[1];
}

async function wasmBindgenVersionMatches(commandPath, expectedVersion) {
  if (!commandPath) {
    return false;
  }
  try {
    await access(commandPath);
  } catch {
    return false;
  }

  return await new Promise((resolvePromise) => {
    const child = spawn(commandPath, ["--version"], {
      cwd: workspaceRoot,
      stdio: ["ignore", "pipe", "ignore"],
      env: process.env,
    });
    let stdout = "";
    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString();
    });
    child.on("error", () => resolvePromise(false));
    child.on("exit", (code) => {
      resolvePromise(code === 0 && stdout.trim() === `wasm-bindgen ${expectedVersion}`);
    });
  });
}

async function resolveWasmBindgenCommand() {
  const wasmBindgenVersion = await findWasmBindgenVersion();
  const cachedRoot = resolve(
    process.env.XDG_CACHE_HOME || resolve(process.env.HOME || workspaceRoot, ".cache"),
    "oasis7",
    "wasm-bindgen-cli",
    wasmBindgenVersion,
  );
  const cachedCliPath = resolve(cachedRoot, "bin", "wasm-bindgen");

  if (await wasmBindgenVersionMatches(cachedCliPath, wasmBindgenVersion)) {
    return cachedCliPath;
  }
  if (await wasmBindgenVersionMatches(process.env.WASM_BINDGEN_BIN, wasmBindgenVersion)) {
    return process.env.WASM_BINDGEN_BIN;
  }
  if (await wasmBindgenVersionMatches(wasmBindgenCliPath, wasmBindgenVersion)) {
    return wasmBindgenCliPath;
  }

  console.log(`wasm-bindgen cli missing or stale; installing wasm-bindgen-cli ${wasmBindgenVersion}`);
  await runChecked("env", [
    "-u",
    "RUSTC_WRAPPER",
    "cargo",
    "install",
    "--locked",
    "wasm-bindgen-cli",
    "--version",
    wasmBindgenVersion,
    "--root",
    cachedRoot,
  ]);

  if (!(await wasmBindgenVersionMatches(cachedCliPath, wasmBindgenVersion))) {
    throw new Error(`failed to provision wasm-bindgen ${wasmBindgenVersion} under ${cachedRoot}`);
  }
  return cachedCliPath;
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

function compatBundleContents() {
  return [
    "// Generated compat alias; canonical bundle truth lives in ./viewer.js.",
    "import \"./viewer.js\";",
    "",
  ].join("\n");
}

await access(builtBundlePath);
const emittedFiles = (await listFilesRecursively(tempOutDir))
  .map((filePath) => relative(tempOutDir, filePath))
  .sort();
if (emittedFiles.length !== 1 || emittedFiles[0] !== "viewer.js") {
  throw new Error(`unexpected viewer canonical bundle outputs: ${emittedFiles.join(", ") || "(none)"}`);
}
await copyFile(builtBundlePath, finalCanonicalBundlePath);
await writeFile(finalCompatBundlePath, compatBundleContents(), "utf8");
await runChecked("env", [
  "-u",
  "RUSTC_WRAPPER",
  "cargo",
  "build",
  "-p",
  "pixel_world_bridge",
  "--target",
  "wasm32-unknown-unknown",
  "--release",
]);
await access(pixelWorldCompiledWasmPath);
await rm(pixelWorldRuntimeDir, { recursive: true, force: true });
await mkdir(pixelWorldRuntimeDir, { recursive: true });
const wasmBindgenCommand = await resolveWasmBindgenCommand();
await runChecked(wasmBindgenCommand, [
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

console.log(`software_safe build finalized: ${finalCanonicalBundlePath}`);
