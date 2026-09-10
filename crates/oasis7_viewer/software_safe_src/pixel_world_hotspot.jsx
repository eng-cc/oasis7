import { createSignal, onCleanup, onMount } from "solid-js";
import { Portal } from "solid-js/web";
import { installHotspotTooltipPlacement } from "./pixel_world_tooltip_placement.js";
import { moveFocusFromHotspotTooltip } from "./pixel_world_hotspot_focus.js";
import { pixelWorldHotspotIntersectsStage } from "./pixel_world_hotspot_projection.js";

function isZhLocale(locale) {
  return String(locale || "").trim().toLowerCase().startsWith("zh");
}

function tr(locale, zh, en) {
  return isZhLocale(locale) ? zh : en;
}

export function pixelWorldHotspotKindLabel(locale, kind) {
  const normalizedKind = String(kind || "info").trim().toLowerCase();
  if (normalizedKind === "blocker") return tr(locale, "阻塞", "Blocker");
  if (normalizedKind === "goal") return tr(locale, "目标", "Goal");
  return tr(locale, "信息", "Info");
}

export function pixelWorldHotspotAccessibleLabel(locale, hotspot) {
  const kindLabel = pixelWorldHotspotKindLabel(locale, hotspot?.kind);
  const label = String(hotspot?.label || hotspot?.id || "").trim();
  return tr(
    locale,
    `${kindLabel}热点：${label}；只读说明。`,
    `${kindLabel} hotspot: ${label}; read-only explanation.`,
  );
}

export function pixelWorldHotspotTooltipId(hotspot) {
  const id = String(hotspot?.id || hotspot?.kind || "hotspot")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-");
  return `pixel-world-hotspot-tooltip-${id || "hotspot"}`;
}

export function createHotspotFocusRestoration() {
  let trigger;
  return {
    remember: (element) => { trigger = element; },
    restore: () => {
      if (trigger) trigger.dataset.dismissedHover = "true";
      queueMicrotask(() => {
      if (!trigger || !document.contains(trigger)) return;
      trigger.dataset.restoringFocus = "true";
      trigger.focus();
      delete trigger.dataset.restoringFocus;
      });
    },
  };
}

export function PixelWorldHotspot(props) {
  let buttonRef;
  const [stageSize, setStageSize] = createSignal({ width: 960, height: 540 });
  onMount(() => {
    const stage = buttonRef.parentElement;
    const update = () => {
      const rect = stage.getBoundingClientRect();
      if (rect.width && rect.height) setStageSize(rect);
    };
    update();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(update);
    observer.observe(stage);
    onCleanup(() => observer.disconnect());
  });
  const glyphOffset = (axis, dimension) => {
    const size = stageSize()[dimension];
    const point = parseFloat(props.style?.[axis] || "50") * size / 100;
    return point - Math.max(22, Math.min(size - 22, point));
  };
  const hotspot = () => props.hotspot;
  const visible = () => pixelWorldHotspotIntersectsStage(props.style, stageSize(), Number(props.glyphSize) || 20);
  const selection = () => ({ kind: "hotspot", id: hotspot().id });
  const inspect = () => {
    delete buttonRef.dataset.dismissedHover;
    props.onHotspotTrigger?.(buttonRef);
    props.onHotspotInspect?.(selection());
    props.onHover?.(selection());
  };
  return (
    <button
      type="button"
      class="pixel-world-hotspot"
      data-hotspot-kind={hotspot().kind}
      data-hotspot-hit-target="44"
      data-renderer-target={props.rendererProjection ? 'true' : undefined}
      disabled={!visible()}
      tabIndex={visible() ? 0 : -1}
      aria-hidden={!visible()}
      ref={buttonRef}
      style={{ ...props.style,
        display: visible() ? undefined : "none",
        "--hotspot-projected-x": props.style?.left,
        "--hotspot-projected-y": props.style?.top,
        left: props.rendererProjection ? props.style?.left : `clamp(22px, ${props.style?.left || "50%"}, calc(100% - 22px))`,
        top: props.rendererProjection ? props.style?.top : `clamp(22px, ${props.style?.top || "50%"}, calc(100% - 22px))`,
      }}
      title={`${hotspot().kind}:${hotspot().label}`}
      aria-label={pixelWorldHotspotAccessibleLabel(props.locale, hotspot())}
      aria-describedby={pixelWorldHotspotTooltipId(hotspot())}
      onFocus={() => {
        if (buttonRef.dataset.restoringFocus) return;
        inspect();
      }}
      onBlur={() => props.onHover?.(null)}
      onMouseEnter={() => {
        if (buttonRef.dataset.dismissedHover) return;
        props.onHotspotTrigger?.(buttonRef);
        props.onHover?.(selection());
      }}
      onMouseMove={() => {
        delete buttonRef.dataset.dismissedHover;
        props.onHotspotHoverIntent?.();
        props.onHotspotTrigger?.(buttonRef);
        props.onHover?.(selection());
      }}
      onMouseLeave={(event) => {
        if (event.relatedTarget?.closest?.("[data-hotspot-tooltip]")?.id === pixelWorldHotspotTooltipId(hotspot())) return;
        delete buttonRef.dataset.dismissedHover;
        props.onHover?.(null);
      }}
      onKeyDown={(event) => {
        if (event.key === "Tab" && !event.shiftKey) {
          const close = document.getElementById(pixelWorldHotspotTooltipId(hotspot()))?.querySelector("button");
          if (close) {
            event.preventDefault();
            close.focus();
          }
        }
        if (event.key === "Escape") {
          event.preventDefault();
          props.onHotspotClear?.();
          props.onHover?.(null);
          props.onHotspotRestoreFocus?.();
        }
      }}
      onClick={(event) => {
        event.preventDefault();
        event.stopPropagation();
        inspect();
      }}
    >
      <span
        class="pixel-world-hotspot__glyph"
        aria-hidden="true"
        style={{
          width: `${Number(props.glyphSize) || 20}px`,
          height: `${Number(props.glyphSize) || 20}px`,
          transform: `translate(${glyphOffset("left", "width")}px, ${glyphOffset("top", "height")}px)`,
          "pointer-events": "none",
        }}
      >{hotspot().kind === "blocker" ? "!" : hotspot().kind === "goal" ? "G" : "i"}</span>
    </button>
  );
}

export function PixelWorldHotspotTooltip(props) {
  let tooltipRef;
  onMount(() => onCleanup(installHotspotTooltipPlacement(tooltipRef)));
  const hotspot = () => props.hotspot;
  return (
    <Portal>
    <div
      id={pixelWorldHotspotTooltipId(hotspot())}
      class="pixel-world-canvas__hotspot-tooltip"
      data-hotspot-tooltip
      ref={tooltipRef}
      onMouseLeave={(event) => {
        if (event.relatedTarget?.closest?.(".pixel-world-hotspot")?.getAttribute("aria-describedby") === pixelWorldHotspotTooltipId(hotspot())) return;
        props.onHoverLeave?.();
      }}
      role="status"
    >
      <span data-hotspot-tooltip-body>{`${pixelWorldHotspotKindLabel(props.locale, hotspot().kind)}: ${hotspot().label}`}</span>
      <button
        type="button"
        class="pixel-world-canvas__hotspot-tooltip-close"
        aria-label={tr(props.locale, "关闭热点说明", "Close hotspot explanation")}
        onKeyDown={(event) => {
          if (event.key === "Tab" && moveFocusFromHotspotTooltip(tooltipRef, event.shiftKey)) event.preventDefault();
          if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            props.onClose?.();
          }
        }}
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          props.onClose?.();
        }}
      >
        ×
      </button>
    </div>
    </Portal>
  );
}
