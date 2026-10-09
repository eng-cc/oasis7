const assert = require("node:assert/strict");
const {runProbe} = require("../.github/actions/cache-permission-probe/index.cjs");
const env = {ACTIONS_CACHE_MODE: "read", GITHUB_RUN_ID: "1", GITHUB_RUN_ATTEMPT: "2", ACTIONS_RESULTS_URL: "https://results.example/", ACTIONS_RUNTIME_TOKEN: "never-log-this-token"};
(async () => {
  for (const [status, body] of [[401, {message: "token expired"}], [403, {message: "forbidden"}], [500, {}], [200, {ok: false}], [200, {ok: false, message: "entry already exists"}], [200, {ok: true, signedUploadUrl: "do-not-log"}]]) {
    await assert.rejects(runProbe({env, fetchImpl: async () => ({ok: status === 200, status, json: async () => body}), log: () => assert.fail("false proof logged")}));
  }
  const logs=[];
  await runProbe({env, fetchImpl: async (url, options) => {
    assert.match(url.href, /CreateCacheEntry$/);
    assert.equal(options.method, "POST");
    assert.equal(JSON.parse(options.body).key, "oasis7-read-denial-trusted-v2-1-2");
    return {ok: true, status: 200, json: async () => ({ok: false, message: "cache write denied: scoped read token"})};
  }, log: message => logs.push(message)});
  assert.deepEqual(logs, ["cache_permission_probe=write_denied"]);
  assert.ok(!logs.join().includes(env.ACTIONS_RUNTIME_TOKEN));
  console.log("ok: scoped policy denial distinguished from expired tokens and reservation failures");
})().catch(error => {console.error(error); process.exitCode=1});
