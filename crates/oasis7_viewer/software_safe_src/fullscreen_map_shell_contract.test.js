import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";

const VIEWER_HTML_FILES = ["viewer.html", "software_safe.html"];

function cssRules(source, selectorPattern) {
  const rules = [];
  const rulePattern = /([^{}]+)\{([^{}]*)\}/g;
  let match;
  while ((match = rulePattern.exec(source)) !== null) {
    if (selectorPattern.test(match[1])) {
      rules.push({ selector: match[1].trim(), declarations: match[2] });
    }
  }
  return rules;
}

function hasDeclaration(rule, property, valuePattern) {
  const declarationPattern = new RegExp(`(?:^|[;\\s])${property}\\s*:\\s*([^;}]*)`, "i");
  const match = rule?.declarations?.match(declarationPattern);
  return Boolean(match && (!valuePattern || valuePattern.test(match[1])));
}

function findRule(source, selectorPattern) {
  return cssRules(source, selectorPattern)[0] || null;
}

async function readViewerHtml() {
  const [viewerHtml, compatHtml, terminalShellCss] = await Promise.all([
    ...VIEWER_HTML_FILES.map((path) => readFile(path, "utf8")),
    readFile("viewer_terminal_shell.css", "utf8"),
  ]);
  return { viewerHtml, compatHtml, terminalShellCss };
}

describe("fullscreen map shell contract", () => {
  it("makes the default and visual-fixture shell a viewport-sized map base", async () => {
    const { viewerHtml, compatHtml, terminalShellCss } = await readViewerHtml();
    expect(compatHtml).toBe(viewerHtml);

    for (const html of [viewerHtml, compatHtml].map((documentHtml) => `${documentHtml}\n${terminalShellCss}`)) {
      expect(html.includes('id="app" class="viewer-shell" data-viewer-shell="player-fullscreen"')).toBe(true);
      expect(html.includes('#viewer-stage-panel[data-viewer-map-layer="base"]')).toBe(true);

      const shell = findRule(html, /\.viewer-shell(?:\[|\s|,|$)/);
      expect(shell).not.toBeNull();
      expect(hasDeclaration(shell, "position", /relative|absolute|fixed/)).toBe(true);
      expect(hasDeclaration(shell, "min-height", /100dvh|100vh/)).toBe(true);
      expect(hasDeclaration(shell, "overflow-x", /hidden|clip/)).toBe(true);
      expect(shell.declarations).not.toMatch(/display\s*:\s*grid/i);
      expect(shell.declarations).not.toMatch(/grid-template-columns\s*:/i);

      const stage = findRule(html, /#viewer-stage-panel\[data-viewer-map-layer=["']base["']\]/);
      expect(stage).not.toBeNull();
      expect(hasDeclaration(stage, "position", /absolute|fixed/)).toBe(true);
      expect(hasDeclaration(stage, "inset", /0/)).toBe(true);
      expect(hasDeclaration(stage, "min-height", /100dvh|100vh/)).toBe(true);
      expect(hasDeclaration(stage, "height", /100dvh|100vh/)).toBe(true);
    }
  });

  it("does not reintroduce a visual-fixture three-column shell exception", async () => {
    const { viewerHtml, compatHtml, terminalShellCss } = await readViewerHtml();
    for (const html of [viewerHtml, compatHtml].map((documentHtml) => `${documentHtml}\n${terminalShellCss}`)) {
      const fixtureShellGridRules = cssRules(
        html,
        /#app\[data-viewer-visual-fixture(?:=[^\]]+)?\][^{}]*\.(?:shell|viewer-shell)/,
      ).filter((rule) => /grid-template-columns\s*:/.test(rule.declarations));
      expect(fixtureShellGridRules).toEqual([]);
      expect(html).not.toMatch(
        /#app\[data-viewer-visual-fixture(?:=[^\]]+)?\]\s+\.(?:shell|viewer-shell)\s*\{[^}]*display\s*:\s*grid/i,
      );
    }
  });

  it("does not let visual fixtures override the responsive command-band layout", async () => {
    const { viewerHtml, compatHtml } = await readViewerHtml();
    for (const html of [viewerHtml, compatHtml]) {
      const fixtureCommandRules = cssRules(
        html,
        /#app\[data-viewer-visual-fixture(?:=[^\]]+)?\][^{}]*\.pixel-world-command-strip/,
      ).filter((rule) => /grid-template-columns\s*:/.test(rule.declarations));
      expect(fixtureCommandRules).toEqual([]);
    }
  });

  it("keeps map HUD, next move, receipt, feed, and navigation in overlay layers", async () => {
    const { viewerHtml, compatHtml, terminalShellCss } = await readViewerHtml();
    for (const html of [viewerHtml, compatHtml].map((documentHtml) => `${documentHtml}\n${terminalShellCss}`)) {
      for (const overlayName of ["world-hud", "next-move", "receipt", "feed", "navigation"]) {
        const rule = findRule(html, new RegExp(`\\[data-viewer-overlay=["']${overlayName}["']\\]`));
        expect(rule, `missing overlay rule for ${overlayName}`).not.toBeNull();
        expect(hasDeclaration(rule, "position", /absolute|fixed|sticky/), overlayName).toBe(true);
        expect(hasDeclaration(rule, "z-index", /[1-9]/), overlayName).toBe(true);
      }
    }
  });

  it("keeps Targets and Command as fixed drawers with stable route anchors", async () => {
    const { viewerHtml, compatHtml, terminalShellCss } = await readViewerHtml();
    for (const html of [viewerHtml, compatHtml].map((documentHtml) => `${documentHtml}\n${terminalShellCss}`)) {
      expect(html.includes('data-viewer-route-panel="targets"')).toBe(true);
      expect(html.includes('data-viewer-route-panel="command"')).toBe(true);
      for (const routePanel of ["targets", "command"]) {
        const rule = findRule(html, new RegExp(`\\[data-viewer-route-panel=["']${routePanel}["']\\]`));
        expect(rule, `missing drawer rule for ${routePanel}`).not.toBeNull();
        expect(hasDeclaration(rule, "position", /fixed|absolute/), routePanel).toBe(true);
        expect(hasDeclaration(rule, "z-index", /[1-9]/), routePanel).toBe(true);
      }
    }
  });

  it("uses viewport map geometry and on-demand bottom sheets on mobile without horizontal overflow", async () => {
    const { viewerHtml, compatHtml, terminalShellCss } = await readViewerHtml();
    for (const html of [viewerHtml, compatHtml].map((documentHtml) => `${documentHtml}\n${terminalShellCss}`)) {
      const mobileBlock = `${terminalShellCss}\n${html}`.match(/@media\s*\(max-width:\s*1240px\)[\s\S]*?(?=@media|<\/style>|$)/i)?.[0] || "";
      expect(/\.viewer-shell[^{]*\{[^}]*min-height\s*:\s*(?:100dvh|100vh)/i.test(mobileBlock)).toBe(true);
      expect(/\.pixel-world-canvas[^{]*\{[^}]*position\s*:\s*(?:absolute|fixed)/i.test(mobileBlock)).toBe(true);
      expect(/\.pixel-world-canvas[^{]*\{[^}]*inset\s*:\s*0/i.test(mobileBlock)).toBe(true);
      for (const routePanel of ["targets", "command"]) {
        const sheetRule = mobileBlock.match(
          new RegExp(`\\[data-viewer-route-panel=["']${routePanel}["']\\][^{]*\\{[^}]*\\}`),
        )?.[0] || "";
        const baseRule = findRule(html, new RegExp(`\\[data-viewer-route-panel=["']${routePanel}["']\\]`));
        const sheetDeclarations = `${baseRule?.declarations || ""} ${sheetRule}`;
        expect(/position\s*:\s*fixed/i.test(sheetDeclarations), routePanel).toBe(true);
        expect(/bottom\s*:|inset\s*:[^;]*\d/i.test(sheetDeclarations), routePanel).toBe(true);
        expect(/max-height\s*:/i.test(sheetDeclarations), routePanel).toBe(true);
      }
    }
  });

  it("keeps the desktop primary rail visible in the default fullscreen Player shell", async () => {
    const { terminalShellCss } = await readViewerHtml();
    const navigationRule = findRule(terminalShellCss, /\[data-viewer-overlay=["']navigation["']\]/);
    expect(navigationRule).not.toBeNull();
    expect(hasDeclaration(navigationRule, "display", /flex|block|inline-flex/)).toBe(true);
  });

  it("keeps Objective and Player Leverage visible in the default fullscreen Player HUD", async () => {
    const { terminalShellCss } = await readViewerHtml();

    for (const cellSelector of [
      /\.pixel-world-command-cell--objective/,
      /\.pixel-world-command-cell--leverage/,
    ]) {
      const hiddenRules = cssRules(terminalShellCss, cellSelector)
        .filter((rule) => hasDeclaration(rule, "display", /none/));
      expect(hiddenRules, `HUD cell is hidden: ${cellSelector}`).toEqual([]);
    }
  });

  it("hides raw pixel readouts in default Player mode", async () => {
    const { viewerHtml, terminalShellCss } = await readViewerHtml();
    expect(viewerHtml).toMatch(/data-viewer-shell="player-fullscreen"/);
    const readoutRule = findRule(
      terminalShellCss,
      /\[data-viewer-shell=["']player-fullscreen["']\][^{}]*\.pixel-world-readout/,
    );
    expect(readoutRule, "default Player shell must hide raw pixel readouts").not.toBeNull();
    expect(hasDeclaration(readoutRule, "display", /none/)).toBe(true);
  });

  it("keeps a mobile More route for secondary Diagnostics without a narrow-screen hide rule", async () => {
    const [navigationSource, terminalShellCss] = await Promise.all([
      readFile("software_safe_src/viewer_navigation.jsx", "utf8"),
      readFile("viewer_terminal_shell.css", "utf8"),
    ]);
    expect(navigationSource).toMatch(/More/);
    expect(navigationSource).toMatch(/Diagnostics/);
    expect(terminalShellCss).not.toMatch(
      /@media\s*\(max-width:\s*640px\)[\s\S]*?\.secondary-viewer-nav\s*\{[^}]*display:\s*none/i,
    );
  });

  it("keeps all three mobile HUD meanings visible without a scroll-only third card", async () => {
    const { terminalShellCss } = await readViewerHtml();
    const mobileBlock = terminalShellCss.match(/@media\s*\(max-width:\s*640px\)[\s\S]*$/i)?.[0] || "";
    expect(mobileBlock).toMatch(/\[data-viewer-overlay=["']next-move["']\][^{]*\{[^}]*grid-template-columns\s*:\s*repeat\(2/i);
    expect(mobileBlock).toMatch(/\.pixel-world-command-cell--next[^{]*\{[^}]*grid-column\s*:\s*1\s*\/\s*-1/i);
    expect(mobileBlock).not.toMatch(/\[data-viewer-overlay=["']next-move["']\][^{]*\{[^}]*overflow\s*:\s*auto/i);
  });
});

describe("headed visual smoke serving contract", () => {
  it("serves viewer CSS with text/css so fullscreen geometry is applied in the browser", async () => {
    const smokeSource = await readFile("scripts/pixel-world-fragment-visual-smoke.mjs", "utf8");
    expect(/case\s+["']\.css["']\s*:\s*return\s+["']text\/css(?:;\s*charset=utf-8)?["']/i.test(smokeSource)).toBe(true);
  });
});

describe("fullscreen map fixture composition", () => {
  it("retains fullscreen map and overlay anchors through fixture composition", async () => {
    const mainSource = await readFile("software_safe_src/main.jsx", "utf8");
    const pixelWorldHostSource = await readFile("software_safe_src/pixel_world_host.jsx", "utf8");
    const worldFeedSource = await readFile("software_safe_src/world_feed_panel.jsx", "utf8");
    const viewerNavigationSource = await readFile("software_safe_src/viewer_navigation.jsx", "utf8");

    expect(mainSource.includes('data-viewer-map-layer="base"')).toBe(true);
    expect(/data-viewer-overlay=.*navigation/.test(viewerNavigationSource)).toBe(true);
    expect(mainSource.includes('data-viewer-route-panel="targets"')).toBe(true);
    expect(mainSource.includes('data-viewer-route-panel="command"')).toBe(true);
    expect(pixelWorldHostSource.includes('data-viewer-overlay="world-hud"')).toBe(true);
    expect(pixelWorldHostSource.includes('data-viewer-overlay="next-move"')).toBe(true);
    expect(pixelWorldHostSource.includes('data-viewer-overlay="receipt"')).toBe(true);
    expect(worldFeedSource.includes('data-viewer-overlay="feed"')).toBe(true);
    expect(mainSource.includes("data-viewer-visual-fixture")).toBe(true);
  });
});
