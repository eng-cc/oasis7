import { describe, expect, it } from "vitest";
import { createViewerAuthSurfaceModule } from "./viewer_auth_surface_module.js";

function createAuthSurfaceModule(hostedAccess) {
  return createViewerAuthSurfaceModule({
    getSearchParams: () => new URLSearchParams({ hosted_access: hostedAccess }),
    localeText: (_locale, zh, en) => en || zh,
    state: {
      auth: { available: false, error: null },
      hostedAccess: null,
      uiLocale: "en",
    },
    windowRef: { location: { href: "http://127.0.0.1/viewer", hostname: "127.0.0.1" } },
  });
}

describe("viewer auth surface module", () => {
  it("rejects JSON arrays passed as hosted access configuration", () => {
    const module = createAuthSurfaceModule("[]");

    expect(module.resolveHostedAccessHint()).toBe(null);
  });
});
