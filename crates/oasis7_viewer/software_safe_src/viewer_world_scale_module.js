export function createViewerWorldScaleModule({
  documentRef,
  state,
  isLocaleZh,
  normalizeFiniteNumber,
  finitePositionComponents,
  trimFixed,
  getSearchParams,
  softwareRendererMarkers,
  softwareSafeRenderModeAlias,
  viewerRenderMode,
}) {
  function formatPhysicalDistanceCm(value, locale = state.uiLocale) {
    const numeric = normalizeFiniteNumber(value);
    if (numeric == null) {
      return null;
    }
    const absolute = Math.abs(numeric);
    if (absolute >= 100_000) {
      const km = numeric / 100_000;
      const label = trimFixed(km, Math.abs(km) >= 100 ? 0 : Math.abs(km) >= 10 ? 1 : 2);
      return `${label} km`;
    }
    if (absolute >= 100) {
      const meters = numeric / 100;
      const label = trimFixed(
        meters,
        Math.abs(meters) >= 100 ? 0 : Math.abs(meters) >= 10 ? 1 : 2,
      );
      return `${label} m`;
    }
    return `${trimFixed(numeric, 0)} cm`;
  }

  function formatWorldPositionCm(pos, locale = state.uiLocale) {
    if (!pos || typeof pos !== "object") {
      return null;
    }
    const x = formatPhysicalDistanceCm(pos.x_cm, locale);
    const y = formatPhysicalDistanceCm(pos.y_cm, locale);
    const z = formatPhysicalDistanceCm(pos.z_cm, locale);
    if (!x || !y || !z) {
      return null;
    }
    return `x=${x} · y=${y} · z=${z}`;
  }

  function distanceCmBetweenPositions(a, b) {
    const left = finitePositionComponents(a);
    const right = finitePositionComponents(b);
    if (!left || !right) {
      return null;
    }
    const dx = left.x - right.x;
    const dy = left.y - right.y;
    const dz = left.z - right.z;
    return Math.max(0, Math.round(Math.sqrt((dx * dx) + (dy * dy) + (dz * dz))));
  }

  function locationRadiusCm(location) {
    return normalizeFiniteNumber(location?.profile?.radius_cm);
  }

  function snapshotSpaceConfig() {
    const space = state.snapshot?.config?.space;
    return space && typeof space === "object" ? space : null;
  }

  function selectedWorldAnchor() {
    const selected = state.selectedObject;
    if (selected && selected.pos) {
      return {
        kind: state.selectedKind || "location",
        id: state.selectedId || selected.id || selected.name || "selected",
        pos: selected.pos,
        radiusCm: locationRadiusCm(selected),
        locationId: selected.location_id || selected.id || null,
      };
    }

    const locations = Object.values(state.snapshot?.model?.locations || {});
    const fallback = locations.find((location) => location?.pos);
    if (!fallback) {
      return null;
    }
    return {
      kind: "location",
      id: fallback.id || fallback.name || "location",
      pos: fallback.pos,
      radiusCm: locationRadiusCm(fallback),
      locationId: fallback.id || null,
    };
  }

  function buildWorldScaleSurface(locale = state.uiLocale) {
    const isZh = isLocaleZh(locale);
    const space = snapshotSpaceConfig();
    const anchor = selectedWorldAnchor();
    const locations = Object.values(state.snapshot?.model?.locations || {})
      .filter((location) => location?.id && location?.pos);

    const nearestLocations = anchor
      ? locations
        .filter((location) => location.id !== anchor.locationId)
        .map((location) => {
          const distanceCm = distanceCmBetweenPositions(anchor.pos, location.pos);
          return {
            id: location.id,
            name: location.name || location.id,
            distanceCm,
            distanceLabel: formatPhysicalDistanceCm(distanceCm, locale),
            radiusCm: locationRadiusCm(location),
            radiusLabel: formatPhysicalDistanceCm(locationRadiusCm(location), locale),
          };
        })
        .filter((location) => location.distanceCm != null)
        .sort((left, right) => left.distanceCm - right.distanceCm)
        .slice(0, 3)
      : [];

    const physicalTruth = {
      canonicalUnitLabel: formatPhysicalDistanceCm(1, locale),
      canonicalUnitDetail: isZh
        ? "世界位置、距离、半径和尺寸的正式真值都按整数厘米存储。"
        : "World positions, distances, radii, and sizes are stored as integer centimeters.",
      worldBoundsLabel: space
        ? `${formatPhysicalDistanceCm(space.width_cm, locale)} × ${formatPhysicalDistanceCm(space.depth_cm, locale)} × ${formatPhysicalDistanceCm(space.height_cm, locale)}`
        : null,
      worldBoundsDetail: space
        ? isZh
          ? "来自 snapshot.config.space 的真实世界边界。"
          : "Physical world bounds derived from snapshot.config.space."
        : isZh
          ? "当前快照没有发布 world bounds。"
          : "The current snapshot does not publish world bounds yet.",
      anchor: anchor
        ? {
            kind: anchor.kind,
            id: anchor.id,
            label: anchor.kind === "agent"
              ? (isZh ? "当前选中 Agent 锚点" : "Selected agent anchor")
              : (isZh ? "当前选中地点锚点" : "Selected location anchor"),
            positionLabel: formatWorldPositionCm(anchor.pos, locale),
            radiusCm: anchor.radiusCm,
            radiusLabel: anchor.radiusCm == null ? null : formatPhysicalDistanceCm(anchor.radiusCm, locale),
            locationId: anchor.locationId,
          }
        : null,
      nearestLocations,
    };

    const presentationScale = {
      markerTruthNote: isZh
        ? "3D marker、2D overview map 和 halo 允许为了可读性被放大；请把距离/半径标签当成真值，不要把屏幕上的直径当成真实几何尺寸。"
        : "3D markers, the 2D overview map, and halos may be enlarged for readability. Treat the distance/radius labels as truth; do not read on-screen diameter as real geometry size.",
      zoomTruthNote: isZh
        ? "overview/detail 的 zoom tier 只切换表现语义，不会改写世界的厘米真值。"
        : "Overview/detail zoom tiers only switch presentation semantics; they do not rewrite centimeter truth in the world model.",
      softwareSafeNote: isZh
        ? "viewer 主入口优先给出文字和数值真值；更底层的 visual QA viewer 可以更夸张，但不应覆盖这里的物理标签。"
        : "The viewer entry prioritizes textual and numeric truth. Lower-level visual QA surfaces may exaggerate more aggressively, but they should not override the physical labels here.",
    };

    return {
      physicalTruth,
      presentationScale,
    };
  }

  function detectRendererMeta() {
    const params = getSearchParams();
    const reasonFromQuery = params.get("viewer_reason") || params.get("software_safe_reason");
    const requestedRenderMode = String(params.get("render_mode") || "").trim().toLowerCase();
    const meta = {
      renderMode:
        requestedRenderMode === softwareSafeRenderModeAlias || requestedRenderMode === viewerRenderMode
          ? viewerRenderMode
          : viewerRenderMode,
      rendererClass: "none",
      viewerReason: reasonFromQuery || "direct_viewer_entry",
      renderer: null,
      vendor: null,
      webglVersion: null,
    };

    try {
      const canvas = documentRef.createElement("canvas");
      const gl = canvas.getContext("webgl") || canvas.getContext("experimental-webgl");
      if (!gl) {
        meta.rendererClass = "none";
        meta.viewerReason = reasonFromQuery || "webgl_unavailable";
        return meta;
      }
      meta.webglVersion = gl.getParameter(gl.VERSION) || null;
      const debugInfo = gl.getExtension("WEBGL_debug_renderer_info");
      if (debugInfo) {
        meta.renderer = gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL) || null;
        meta.vendor = gl.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL) || null;
      }
      const rendererText = String(meta.renderer || "").toLowerCase();
      if (softwareRendererMarkers.some((marker) => rendererText.includes(marker))) {
        meta.rendererClass = "software";
      } else {
        meta.rendererClass = "unknown";
      }
    } catch (error) {
      meta.rendererClass = "none";
      meta.renderer = String(error);
    }
    return meta;
  }

  return {
    formatPhysicalDistanceCm,
    formatWorldPositionCm,
    buildWorldScaleSurface,
    detectRendererMeta,
  };
}
