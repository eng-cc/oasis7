// Official @actions/cache CreateCacheEntry protocol; do not upload or log secrets.
const {createHash} = require("node:crypto");
async function runProbe({env = process.env, fetchImpl = fetch, log = console.log} = {}) {
  if (env.ACTIONS_CACHE_MODE !== "read") throw Error("Candidate cache probe requires read mode");
  const key = `oasis7-read-denial-trusted-v2-${env.GITHUB_RUN_ID}-${env.GITHUB_RUN_ATTEMPT}`;
  const version = createHash("sha256").update(key).digest("hex");
  const url = new URL("/twirp/github.actions.results.api.v1.CacheService/CreateCacheEntry", env.ACTIONS_RESULTS_URL);
  const response = await fetchImpl(url, {method: "POST", headers: {
    "Content-Type": "application/json", Authorization: `Bearer ${env.ACTIONS_RUNTIME_TOKEN}`
  }, body: JSON.stringify({key, version}), signal: AbortSignal.timeout(30000)});
  // An expired token, generic forbidden response, contention or quota is not a
  // policy proof. Use the same stable denial classifier as the official toolkit.
  if (!response.ok) throw Error(`Cache probe transport status ${response.status}`);
  const result = await response.json();
  if (result.ok !== false || typeof result.message !== "string" || !result.message.startsWith("cache write denied:")) {
    throw Error("Cache service did not confirm scoped write denial");
  }
  log("cache_permission_probe=write_denied");
}
module.exports = {runProbe};
if (require.main === module) runProbe().catch(() => {
  // Network error text may contain URLs. Do not print response bodies or secrets.
  console.error("cache_permission_probe=failed; service must explicitly confirm cache write denied");
  process.exitCode = 1;
});
