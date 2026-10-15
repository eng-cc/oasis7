const {chromium}=require(process.env.OASIS7_PLAYWRIGHT_MODULE || 'playwright');
const assert=require('node:assert/strict');const fs=require('node:fs');const path=require('node:path');
const origin=process.env.OASIS7_SITE_QA_URL || 'http://127.0.0.1:48760';
(async()=>{
 const browser=await chromium.launch({headless:process.env.OASIS7_SITE_QA_HEADLESS === '1', ...(process.env.OASIS7_SITE_QA_BROWSER ? {executablePath:process.env.OASIS7_SITE_QA_BROWSER} : {})});
 const out=path.join(process.cwd(),'output/site-discovery-qa');const results=[];
 const record=(name)=>{results.push({name,result:'pass'});console.log('PASS '+name);};
 const context=async(opts={})=>browser.newContext({locale:'zh-CN',reducedMotion:'reduce',...opts});
 try{
 for(const locale of ['zh-CN','en-US']){
  const c=await context({locale,javaScriptEnabled:false,viewport:{width:360,height:800}});const p=await c.newPage();await p.goto(origin+(locale==='zh-CN'?'/':'/en/'));
  assert.equal(await p.locator('[data-decision-panel]:visible').count(),4);assert.equal(await p.locator('.topbar .nav a:visible').count(),4);
  assert.equal(await p.locator('noscript a:visible').count(),3);assert.equal(await p.evaluate(()=>document.documentElement.scrollWidth),360);
  await p.screenshot({path:path.join(out,locale+'-nojs-360.png'),fullPage:true});await c.close();record(locale+' no-JS complete steps/nav/manual packages');
 }
 // Composition regression: illustration must not push the reading rail below a separate image block.
 for(const locale of ['zh-CN','en-US']){
  const c=await context({locale});await c.route('https://api.github.com/**',r=>r.abort());const p=await c.newPage();
  for(const width of [390,1440]){
   await p.setViewportSize({width,height:900});await p.goto(origin+(locale==='zh-CN'?'/':'/en/'));
   const layout=await p.evaluate(()=>({proof:document.querySelector('#proof').getBoundingClientRect().top,
    fourth:document.querySelector('[data-decision-step="3"]').getBoundingClientRect().bottom,
    art:getComputedStyle(document.querySelector('.hero-visual')).position,
    rail:getComputedStyle(document.querySelector('.decision-panels')).gridTemplateColumns.split(' ').length}));
   assert.equal(await p.locator('.discovery-editorial-grid a[href*="story"]').count(),0);assert.equal(await p.locator('main > #world-laws').count(),1);assert.equal(await p.locator('#contribute #world-laws').count(),0);
   assert.equal(layout.art,'absolute');assert.ok(layout.proof<550,JSON.stringify(layout));
   assert.equal(layout.rail,width===390?1:4);assert.ok(layout.fourth<1000,JSON.stringify(layout));
  }
  await c.close();record(locale+' integrated hero and early four-step reading rail');
 }
 const files=['oasis7-windows-x64.exe','oasis7-macos-x64.dmg','oasis7-linux-x64.deb','oasis7-checksums.txt'];
 for(const scenario of ['full','no-platform','no-checksum','invalid','timeout']){
  const c=await context();await c.route('https://api.github.com/**',async r=>{
   if(scenario==='timeout')return r.abort();
   const names=files.filter(n=>!(scenario==='no-platform'&&n===files[0])&&!(scenario==='no-checksum'&&n===files[3]));
   await r.fulfill({json:{tag_name:'qa-candidate',published_at:scenario==='invalid'?'bad':'2026-10-14T00:00:00Z',assets:names.map(name=>({name,browser_download_url:`https://github.com/eng-cc/oasis7/releases/download/qa-candidate/${name}`}))}});
  });const p=await c.newPage();await p.goto(origin+'/');await p.locator('[data-download-platform-button="windows"]').click();
  await p.waitForTimeout(150);
  const tag=await p.locator('[data-release-tag]').innerText();const href=await p.locator('[data-download-primary-link]').getAttribute('href');
  if(scenario==='invalid'||scenario==='timeout'){assert.equal(tag,'2026-06-03_v2');assert.ok(href.includes('/2026-06-03_v2/'));assert.ok((await p.locator('[data-release-status]').innerText()).includes('未确认最新'));}
  else {assert.equal(tag,'qa-candidate');if(scenario==='full')assert.ok(href.includes('/qa-candidate/'));else assert.equal(href,null);}
  await c.close();record('release '+scenario+' coherent metadata/assets or safe fallback');
 }
 for (const userAgent of ['Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) Mobile', 'Mozilla/5.0 (Windows NT 10.0; ARM64)', 'Mozilla/5.0 (X11; Linux aarch64)']) {
  const c=await context({userAgent});await c.route('https://api.github.com/**',r=>r.abort());const p=await c.newPage();await p.goto(origin+'/');assert.equal(await p.locator('[data-download-primary-link]').getAttribute('href'),null);assert.equal(await p.locator('[data-download-platform-button][aria-pressed="true"]').count(),0);await c.close();record('unsupported device has no recommendation: '+userAgent);
 }
 {
  const c=await context({viewport:{width:390,height:844}});await c.route('https://api.github.com/**',r=>r.abort());const p=await c.newPage();await p.goto(origin+'/');
  for(let i=0;i<4;i++){await p.locator(`[data-decision-step="${i}"]`).focus();await p.keyboard.press('Enter');assert.equal(await p.locator('[data-decision-panel]:visible').count(),4);assert.equal(await p.locator(`[data-decision-panel="${i}"]`).getAttribute('class'),'is-active');}
  await p.locator('[data-menu-toggle]').click();assert.equal(await p.locator('.topbar .nav a:visible').count(),4);await p.keyboard.press('Escape');assert.equal(await p.locator('[data-menu-toggle]').getAttribute('aria-expanded'),'false');
  await p.locator('#proof').scrollIntoViewIfNeeded();await p.screenshot({path:path.join(out,'keyboard-proof-390.png')});
  await p.evaluate(()=>document.documentElement.style.fontSize='200%');assert.equal(await p.evaluate(()=>document.documentElement.scrollWidth),390);
  const canvas=await p.locator('[data-hero-canvas]').count();assert.equal(canvas,1);
  await c.close();record('keyboard steps/menu/Escape and 200 percent text zoom');
 }
 // Reader empty/error/retry and both late response directions.
 for(const first of ['empty','error','late-success','late-error']){
  const c=await context({locale:'en-US'});let attempts=0;
  await c.route('**/volume-01-chapter-01.md',async r=>{
   attempts++;
   if(first.startsWith('late-'))await new Promise(ok=>setTimeout(ok,700));
   if((first==='error'&&attempts===1)||first==='late-error')await r.fulfill({status:404,body:'not found'});
   else await r.fulfill({body:first==='empty'&&attempts===1?'   ':'# Current chapter\n\nReadable current text.'});
  });await c.route('**/volume-01-chapter-02.md',r=>r.fulfill({body:'# New chapter\n\nB is authoritative for this reading selection.'}));
  const p=await c.newPage();await p.goto(origin+'/story/');
  assert.ok(p.url().endsWith('/story/'));
  if(first.startsWith('late-')){await p.locator('[data-story-src$="chapter-02.md"]').first().click();await p.waitForTimeout(950);assert.ok((await p.locator('[data-story-content]').innerText()).includes('B is authoritative'));assert.equal(await p.locator('.reader-panel').getAttribute('aria-busy'),'false');}
  else {await p.waitForFunction(()=>document.querySelector('[data-story-reader]').getAttribute('data-reader-state')==='empty'||document.querySelector('[data-story-reader]').getAttribute('data-reader-state')==='error');assert.equal(await p.locator('[data-story-content]').innerText(),'');await p.locator('[data-story-retry]').click();await p.waitForFunction(()=>document.querySelector('[data-story-reader]').getAttribute('data-reader-state')==='ready');assert.ok((await p.locator('[data-story-content]').innerText()).includes('Readable current text'));}
  await c.close();record('reader '+first+' / real recovery / English does not invent route');
 }
 {
  const c=await context({locale:'en-US'});await c.route('https://api.github.com/**',r=>r.abort());
  await c.route(origin+'/project/**',async r=>{const response=await r.fetch({url:r.request().url().replace('/project/','/')});await r.fulfill({response});});
  const p=await c.newPage();await p.goto(origin+'/project/devlog/discovery-first/?from=qa#top');await p.waitForURL('**/project/en/devlog/discovery-first/index.html?from=qa#top');
  assert.ok(p.url().includes('/project/en/devlog/discovery-first/index.html?from=qa#top'));await c.close();record('explicit locale mapping preserves project prefix query and hash');
 }
 {
  const c=await context();let attempts=0;
  await c.route('**/volume-01-chapter-01.md',async r=>{attempts++;const first=attempts===1;await new Promise(ok=>setTimeout(ok,first?600:900));await r.fulfill({body:first?'   ':'# Fresh chapter\n\nFresh after retry.'});});
  const p=await c.newPage();await p.goto(origin+'/story/');await p.locator('[data-story-retry]').click();await p.waitForTimeout(1100);
  assert.ok((await p.locator('[data-story-content]').innerText()).includes('Fresh after retry'));
  await p.locator('[data-story-src$="chapter-02.md"]').first().click();await p.waitForFunction(()=>document.querySelector('[data-story-reader]').getAttribute('data-reader-state')==='ready');
  await p.locator('[data-story-src$="chapter-01.md"]').first().click();await p.waitForFunction(()=>document.querySelector('[data-story-reader]').getAttribute('data-reader-state')==='ready');
  assert.ok((await p.locator('[data-story-content]').innerText()).includes('Fresh after retry'));await c.close();record('same chapter retry ignores late cache writer');
 }
 fs.writeFileSync(path.join(out,'behavior-results.json'),JSON.stringify(results,null,2));
 }finally{await browser.close();}
})();
