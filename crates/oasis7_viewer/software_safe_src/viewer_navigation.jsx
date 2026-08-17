function focusViewerTarget(href) {
  const target = href?.startsWith("#") ? document.getElementById(href.slice(1)) : null;
  if (!target) {
    return null;
  }
  if (target instanceof HTMLDetailsElement) {
    target.open = true;
  }
  window.location.hash = href;
  const focusTarget = target instanceof HTMLDetailsElement
    ? target.querySelector("summary") || target
    : target;
  focusTarget.focus();
  target.scrollIntoView?.({ behavior: "auto", block: "start", inline: "nearest" });
  return target;
}

function focusViewerAnchor(event) {
  const href = event.currentTarget.getAttribute("href");
  if (!focusViewerTarget(href)) {
    return;
  }
  event.preventDefault();
}

function installViewerRouteController() {
  const handleKeyDown = (event) => {
    if (
      event.key !== "Escape"
      || event.isComposing
      || document.body.classList.contains("pixel-world-focus-active")
      || !["#viewer-targets-panel", "#viewer-details-panel"].includes(window.location.hash)
    ) {
      return;
    }
    event.preventDefault();
    focusViewerTarget("#viewer-stage-panel");
  };
  window.addEventListener("keydown", handleKeyDown);
  return () => window.removeEventListener("keydown", handleKeyDown);
}

function MobileJumpRail(props) {
  const locale = () => props.locale();
  const translate = (zh, en) => props.tr(locale(), zh, en);
  return (
    <nav class="mobile-rail" data-viewer-overlay={props["data-viewer-overlay"] || "navigation"} aria-label={translate("主入口分区导航", "Primary entry section navigation")}>
      <a class="mobile-rail__link" href="#viewer-stage-panel" onClick={focusViewerAnchor}>{translate("世界", "World")}</a>
      <a class="mobile-rail__link" href="#viewer-targets-panel" onClick={focusViewerAnchor}>{translate("目标", "Targets")}</a>
      <a class="mobile-rail__link" href="#viewer-details-panel" onClick={focusViewerAnchor}>{translate("指令", "Command")}</a>
    </nav>
  );
}

function SecondaryViewerNavigation(props) {
  const locale = () => props.locale();
  const translate = (zh, en) => props.tr(locale(), zh, en);
  return (
    <nav class="secondary-viewer-nav" aria-label={translate("次级查看入口", "Secondary viewer navigation")}>
      <span class="secondary-viewer-nav__more">
        {translate("更多", "More")}
      </span>
      <a class="secondary-viewer-nav__link" href="#viewer-diagnostics-panel" onClick={focusViewerAnchor}>
        {translate("诊断", "Diagnostics")}
      </a>
    </nav>
  );
}

export { MobileJumpRail, SecondaryViewerNavigation, focusViewerAnchor, installViewerRouteController };
