import { describe, expect, it } from "vitest";
import { createViewerWorldScaleModule } from "./viewer_world_scale_module.js";

function createWorldScaleModule(state) {
  return createViewerWorldScaleModule({
    documentRef: document,
    finitePositionComponents: (pos) => {
      if (!pos || typeof pos !== "object") {
        return null;
      }
      const x = Number(pos.x_cm);
      const y = Number(pos.y_cm);
      const z = Number(pos.z_cm);
      return Number.isFinite(x) && Number.isFinite(y) && Number.isFinite(z) ? { x, y, z } : null;
    },
    getSearchParams: () => new URLSearchParams(),
    isLocaleZh: (locale) => locale === "zh",
    normalizeFiniteNumber: (value) => {
      const number = Number(value);
      return Number.isFinite(number) ? number : null;
    },
    softwareRendererMarkers: [],
    softwareSafeRenderModeAlias: "software_safe",
    state,
    trimFixed: (value, digits) => Number(value).toFixed(digits).replace(/\.0+$/, ""),
    viewerRenderMode: "software_safe",
  });
}

describe("viewer world scale module", () => {
  it("does not expose negative zero in centimeter labels", () => {
    const state = {
      snapshot: { model: { locations: {} } },
      uiLocale: "en",
    };
    const module = createWorldScaleModule(state);

    expect(module.formatPhysicalDistanceCm(-0.4)).toBe("0 cm");
    expect(module.formatPhysicalDistanceCm(-1.5)).toBe("-2 cm");
    expect(module.formatWorldPositionCm({ x_cm: -0.4, y_cm: 0, z_cm: 0.4 })).toBe(
      "x=0 cm · y=0 cm · z=0 cm",
    );
  });

  it("suppresses incomplete world bounds instead of exposing null dimensions", () => {
    const state = {
      snapshot: {
        config: {
          space: { depth_cm: 1_000, height_cm: 100 },
        },
        model: { locations: {} },
      },
      uiLocale: "en",
    };
    const module = createWorldScaleModule(state);

    const physicalTruth = module.buildWorldScaleSurface().physicalTruth;

    expect(physicalTruth.worldBoundsLabel).toBe(null);
    expect(physicalTruth.worldBoundsDetail).toBe("The current snapshot does not publish world bounds yet.");
  });

  it("falls back to a positioned location when the selected object has an incomplete position", () => {
    const state = {
      selectedId: "agent-0",
      selectedKind: "agent",
      selectedObject: {
        id: "agent-0",
        pos: { x_cm: 10, y_cm: "not-a-number", z_cm: 0 },
      },
      snapshot: {
        model: {
          locations: {
            base: {
              id: "base",
              name: "Base",
              pos: { x_cm: 0, y_cm: 0, z_cm: 0 },
            },
            outpost: {
              id: "outpost",
              name: "Outpost",
              pos: { x_cm: 300, y_cm: 0, z_cm: 0 },
            },
          },
        },
      },
      uiLocale: "en",
    };
    const module = createWorldScaleModule(state);

    const physicalTruth = module.buildWorldScaleSurface().physicalTruth;

    expect(physicalTruth.anchor).toMatchObject({
      id: "base",
      positionLabel: "x=0 cm · y=0 cm · z=0 cm",
    });
    expect(physicalTruth.nearestLocations).toEqual([
      expect.objectContaining({ id: "outpost", distanceCm: 300 }),
    ]);
  });

  it.each([
    ["valid selection", { id: "selected", pos: { x_cm: 10, y_cm: 0, z_cm: 0 } }, "selected"],
    ["invalid selection", { id: "selected", pos: { x_cm: "invalid", y_cm: 0, z_cm: 0 } }, "base"],
    ["no selection", null, "base"],
  ])("enumerates locations once per surface build with %s", (_name, selectedObject, anchorId) => {
    let enumerations = 0;
    const locations = new Proxy({
      base: { id: "base", pos: { x_cm: 0, y_cm: 0, z_cm: 0 } },
      outpost: { id: "outpost", pos: { x_cm: 300, y_cm: 0, z_cm: 0 } },
    }, {
      ownKeys(target) {
        enumerations += 1;
        return Reflect.ownKeys(target);
      },
    });
    const state = { selectedObject, snapshot: { model: { locations } }, uiLocale: "en" };
    const module = createWorldScaleModule(state);

    expect(module.buildWorldScaleSurface().physicalTruth.anchor.id).toBe(anchorId);
    expect(enumerations).toBe(1);
    expect(module.buildWorldScaleSurface().physicalTruth.anchor.id).toBe(anchorId);
    expect(enumerations).toBe(2);
  });

  it("preserves the first positioned fallback without an id while filtering nearest candidates", () => {
    const module = createWorldScaleModule({
      snapshot: { model: { locations: {
        omitted: null,
        unnamed: { name: "Unnamed origin", pos: { x_cm: 0, y_cm: 0, z_cm: 0 } },
        firstTie: { id: "first", pos: { x_cm: 100, y_cm: 0, z_cm: 0 } },
        secondTie: { id: "second", pos: { x_cm: -100, y_cm: 0, z_cm: 0 } },
        missingPosition: { id: "absent" },
        invalidPosition: { id: "invalid", pos: { x_cm: "invalid", y_cm: 0, z_cm: 0 } },
      } } },
      uiLocale: "en",
    });

    const truth = module.buildWorldScaleSurface().physicalTruth;

    expect(truth.anchor).toMatchObject({
      id: "Unnamed origin",
      locationId: null,
      positionLabel: "x=0 cm · y=0 cm · z=0 cm",
    });
    expect(truth.nearestLocations.map(({ id }) => id)).toEqual(["first", "second"]);
  });

  it("keeps a truthy invalid fallback position instead of silently choosing a later location", () => {
    const module = createWorldScaleModule({
      snapshot: { model: { locations: {
        invalid: { pos: { x_cm: "invalid", y_cm: 0, z_cm: 0 } },
        valid: { id: "later", pos: { x_cm: 100, y_cm: 0, z_cm: 0 } },
      } } },
      uiLocale: "en",
    });

    const truth = module.buildWorldScaleSurface().physicalTruth;

    expect(truth.anchor).toMatchObject({ id: "location", locationId: null, positionLabel: null });
    expect(truth.nearestLocations).toEqual([]);
  });

  it("does not retain an old fallback when the snapshot changes or becomes empty", () => {
    const state = {
      snapshot: { model: { locations: {
        first: { id: "first", pos: { x_cm: 0, y_cm: 0, z_cm: 0 } },
      } } },
      uiLocale: "en",
    };
    const module = createWorldScaleModule(state);
    expect(module.buildWorldScaleSurface().physicalTruth.anchor.id).toBe("first");

    state.snapshot = { model: { locations: {
      next: { id: "next", pos: { x_cm: 100, y_cm: 0, z_cm: 0 } },
    } } };
    expect(module.buildWorldScaleSurface().physicalTruth.anchor.id).toBe("next");

    state.snapshot = null;
    expect(module.buildWorldScaleSurface().physicalTruth).toMatchObject({
      anchor: null,
      nearestLocations: [],
    });
  });

  it("keeps nearest locations stable without full-array sorting", () => {
    const state = {
      selectedId: "origin",
      selectedKind: "location",
      selectedObject: {
        id: "origin",
        pos: { x_cm: 0, y_cm: 0, z_cm: 0 },
        profile: { radius_cm: 10 },
      },
      snapshot: {
        config: {
          space: { width_cm: 1_000, depth_cm: 1_000, height_cm: 100 },
        },
        model: {
          locations: {
            origin: {
              id: "origin",
              name: "Origin",
              pos: { x_cm: 0, y_cm: 0, z_cm: 0 },
              profile: { radius_cm: 10 },
            },
            far: {
              id: "far",
              name: "Far",
              pos: { x_cm: 900, y_cm: 0, z_cm: 0 },
              profile: { radius_cm: 10 },
            },
            firstTie: {
              id: "firstTie",
              name: "First tie",
              pos: { x_cm: 100, y_cm: 0, z_cm: 0 },
              profile: { radius_cm: 20 },
            },
            secondTie: {
              id: "secondTie",
              name: "Second tie",
              pos: { x_cm: 0, y_cm: 100, z_cm: 0 },
              profile: { radius_cm: 30 },
            },
            closest: {
              id: "closest",
              name: "Closest",
              pos: { x_cm: 40, y_cm: 0, z_cm: 0 },
              profile: { radius_cm: 40 },
            },
            fourth: {
              id: "fourth",
              name: "Fourth",
              pos: { x_cm: 120, y_cm: 0, z_cm: 0 },
              profile: { radius_cm: 50 },
            },
          },
        },
      },
      uiLocale: "en",
    };
    const module = createWorldScaleModule(state);
    const originalSort = Array.prototype.sort;
    Array.prototype.sort = function sortShouldNotRun() {
      throw new Error("unexpected full sort");
    };
    try {
      const nearest = module.buildWorldScaleSurface().physicalTruth.nearestLocations;
      expect(nearest.map((location) => location.id)).toEqual([
        "closest",
        "firstTie",
        "secondTie",
      ]);
      expect(nearest.map((location) => location.distanceCm)).toEqual([40, 100, 100]);
    } finally {
      Array.prototype.sort = originalSort;
    }
  });
});
