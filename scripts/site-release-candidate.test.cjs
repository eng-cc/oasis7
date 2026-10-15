const assert = require('node:assert/strict');
const {fromRelease, assetUrl, filenames} = require('../site/assets/release-candidate.js');
const base = 'https://github.com/eng-cc/oasis7/releases/download';
const release = (names = Object.values(filenames), tag = 'candidate-2') => ({tag_name: tag, published_at: '2026-10-14T00:00:00Z', assets: names.map(name => ({name, browser_download_url: `${base}/${tag}/${name}`}))});
const fallback = `${base}/old/${filenames.windows}`;
assert.equal(assetUrl(null, fallback), fallback);
assert.equal(assetUrl(fromRelease(release()), fallback), `${base}/candidate-2/${filenames.windows}`);
for (const names of [[filenames.checksum], [filenames.windows], []]) {
  assert.equal(assetUrl(fromRelease(release(names)), fallback), '');
}
for (const bad of [{}, {...release(), tag_name: ''}, {...release(), published_at:'invalid'}, {...release(), draft:true}, {...release(), prerelease:true}, {...release(), assets:null}]) assert.throws(() => fromRelease(bad));
for (const url of ['javascript:alert(1)', `${base}/old/${filenames.windows}`, `https://evil.test/${filenames.windows}`, `${base}/candidate-2/${filenames.windows}?redirect=x`]) {
  const data = release(); data.assets[0].browser_download_url = url;
  assert.equal(assetUrl(fromRelease(data), fallback), '');
}
const duplicate = release(); duplicate.assets.push(duplicate.assets[0]);
assert.equal(assetUrl(fromRelease(duplicate), fallback), '');
assert.equal(assetUrl(fromRelease(release([filenames.windows,filenames.checksum], 'new')), `${base}/old/${filenames.macos}`), '');
assert.equal(fromRelease({...release(),html_url:'https://evil.test'}).notes, 'https://github.com/eng-cc/oasis7/releases/tag/candidate-2');
console.log('site-release-candidate.test: OK (complete, missing, invalid, mismatched and duplicate assets)');
