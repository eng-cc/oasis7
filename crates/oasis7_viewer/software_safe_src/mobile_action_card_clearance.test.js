import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("mobile gameplay action card clearance", () => {
  it("keeps targeted action cards below the sticky mobile jump rail", () => {
    const viewerHtml = readFileSync("viewer.html", "utf8");
    const compatibilityHtml = readFileSync("software_safe.html", "utf8");

    for (const html of [viewerHtml, compatibilityHtml]) {
      expect(html).toMatch(
        /\.event-card--action\s*\{[^}]*scroll-margin-top:\s*var\(--mobile-action-card-clearance,\s*0\);/,
      );
      expect(html).toMatch(
        /@media \(max-width: 1240px\)[\s\S]*?--mobile-action-card-clearance:\s*84px;/,
      );
    }
  });
});
